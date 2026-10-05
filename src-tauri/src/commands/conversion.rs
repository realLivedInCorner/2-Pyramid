use tokio::task;
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;

use crate::pack::version_converter::{
    process_zip,
    pack_format_label_for_output,
    build_output_path_for_batch,
};

/// Global cancel flag for the batch conversion. The frontend calls
/// `cancel_conversion` to set it; the batch loop checks it before each
/// file and aborts early, returning the files already processed.
static CONVERSION_CANCELLED: AtomicBool = AtomicBool::new(false);

/// Global "a conversion is currently running" flag. The frontend asks
/// `is_conversion_running` before closing the window so it can warn the
/// user that quitting will interrupt the batch. Set/cleared by a RAII
/// guard so every exit path (success, error, cancelled) resets it.
static CONVERSION_RUNNING: AtomicBool = AtomicBool::new(false);

/// 当前的性能档位解析结果（平衡 / 性能 → 线程预算与并发包数）。
///
/// 引擎的并行是单一总预算：这里的 `threads` 是 rayon 池大小，包级与包内
/// 并行（含 PNG 编码）共用它；`packs` 只是再限制"同时展开几个包"（内存保护）。
fn perf_plan() -> crate::perf::PerfPlan {
    let cfg = crate::commands::config::read_config_file().ok();
    let mode = cfg.as_ref().and_then(|c| c.performance_mode.clone());
    // 旧字段 conversion_threads（1–4）作为迁移来源：≤2 → 平衡，≥3 → 性能
    let legacy = cfg.as_ref().and_then(|c| c.conversion_threads);
    crate::perf::resolve(mode.as_deref(), legacy)
}

/// 批处理同时转换的包数。
fn concurrent_packs() -> usize {
    perf_plan().packs
}

/// **并发包数闸门**（§9.151）：一个朴素的计数信号量。
///
/// 为什么需要它：rayon 池现在按 **CPU 总预算**（`plan.threads`）建，
/// 因此 `par_iter` 会**同时展开尽可能多的包**——而 `plan.packs` 的本意是**内存保护**
/// （每个包解压 + 图片解码有峰值内存），不是 CPU 保护。池变大之后，这道保护必须由
/// 调用方自己兜住，否则一次会把所有包都解压开。
///
/// 用 `Mutex + Condvar` 而非第三方信号量：依赖里没有现成的，而这里只需要"计数 + 等待"。
struct PackGate {
    limit: usize,
    state: std::sync::Mutex<usize>,
    cv: std::sync::Condvar,
}

impl PackGate {
    fn new(limit: usize) -> Self {
        Self {
            limit: limit.max(1),
            state: std::sync::Mutex::new(0),
            cv: std::sync::Condvar::new(),
        }
    }

    fn acquire(&self) -> PackTicket<'_> {
        let mut used = self.state.lock().unwrap_or_else(|e| e.into_inner());
        while *used >= self.limit {
            used = self.cv.wait(used).unwrap_or_else(|e| e.into_inner());
        }
        *used += 1;
        drop(used);
        PackTicket { gate: self }
    }
}

/// 闸门票据：drop 时归还名额（即使转换 panic 也会归还，因为 drop 在 unwind 时仍会跑）。
struct PackTicket<'a> {
    gate: &'a PackGate,
}

impl Drop for PackTicket<'_> {
    fn drop(&mut self) {
        let mut used = self.gate.state.lock().unwrap_or_else(|e| e.into_inner());
        *used = used.saturating_sub(1);
        drop(used);
        self.gate.cv.notify_one();
    }
}

/// 暴露给前端的性能档位信息（用于设置页显示解析后的线程/包数）。
#[tauri::command]
pub fn get_perf_plan() -> crate::perf::PerfPlan {
    let plan = perf_plan();
    crate::log_info!(
        "OKAY get_perf_plan [mode={} cores={} threads={} packs={}]{}",
        plan.mode,
        plan.cores,
        plan.threads,
        plan.packs,
        plan.note
            .as_ref()
            .map(|n| format!(" note={}", n))
            .unwrap_or_default()
    );
    plan
}

/// RAII guard that keeps `CONVERSION_RUNNING` true for the lifetime of
/// one conversion command, then clears it on drop — even if the
/// command panics or the frontend stops waiting.
struct RunningGuard;

impl RunningGuard {
    fn new() -> Self {
        CONVERSION_RUNNING.store(true, Ordering::SeqCst);
        RunningGuard
    }
}

impl Drop for RunningGuard {
    fn drop(&mut self) {
        CONVERSION_RUNNING.store(false, Ordering::SeqCst);
    }
}

/// Frontend-driven cancellation of the running batch conversion.
#[tauri::command]
pub fn cancel_conversion() {
    CONVERSION_CANCELLED.store(true, Ordering::SeqCst);
    crate::log_warn!("conversion: cancel requested — aborting remaining files");
}

/// Query used by the window-close flow: is a conversion running right
/// now? The frontend shows a “conversion in progress” warning when
/// this is true so the user can abort the quit instead of silently
/// losing a half-finished batch.
#[tauri::command]
pub fn is_conversion_running() -> bool {
    CONVERSION_RUNNING.load(Ordering::SeqCst)
}


#[tauri::command]
pub fn test_command(message: String) -> String {
    format!("Received message: {} - Test success!", message)
}

#[tauri::command]
pub async fn convert_zip(
    original_file_path: String,
    pack_format2: u32,
    parent_folder_path: Option<String>,
) -> Result<String, String> {
    use crate::log_info;
    use crate::log_error;

    let _running = RunningGuard::new();

    let result = tokio::task::spawn_blocking(move || {
        process_zip(
            &original_file_path,
            pack_format2,
            None,
            1.0,
            parent_folder_path.as_deref(),
            None,
            false,
            true,
        )
    }).await;

    match result {
        Ok(Ok(output_path)) => {
            log_info!("Conversion complete, result: {}", output_path);
            Ok(format!("Conversion success! Output: {}", output_path))
        }
        Ok(Err(e)) => {
            log_error!("Conversion failed: {}", e);
            Err(format!("Conversion failed: {}", e))
        }
        Err(e) => {
            log_error!("Thread execution failed: {}", e);
            Err(format!("Thread execution failed: {}", e))
        }
    }
}

#[tauri::command]
pub async fn convert_resource_pack(
    file_path: String,
    target_format: u32,
) -> Result<String, String> {
    use crate::log_info;
    use crate::log_error;

    let _running = RunningGuard::new();

    let result = task::spawn_blocking(move || {
        crate::pack::version_converter::convert_resource_pack(
            &file_path,
            target_format,
        )
    }).await;

    match result {
        Ok(Ok(output)) => {
            log_info!("Conversion success: {}", output);
            Ok(output)
        }
        Ok(Err(e)) => {
            log_error!("Conversion failed: {}", e);
            Err(e)
        }
        Err(e) => {
            log_error!("Thread execution failed: {}", e);
            Err(format!("Thread execution failed: {}", e))
        }
    }
}

/// Batch resource pack conversion command. Runs in the blocking
/// thread pool so the UI never stalls; packs are converted in parallel
/// (bounded by `CONCURRENT_PACKS`) and the loop checks the global
/// cancel flag before each file so the user can abort a large batch.
#[tauri::command]
pub async fn convert_resource_packs_batch(
    file_paths: Vec<String>,
    target_format: u32,
    output_dirs: Option<Vec<String>>,
    fix_alpha_layers: Option<bool>,
    adapt_shaders: Option<bool>,
) -> Result<Vec<serde_json::Value>, String> {
    use std::time::Instant;

    use crate::log_info;
    use crate::log_error;

    let fix_alpha = fix_alpha_layers.unwrap_or(false);
    // 实验项：默认开启，保持 2.1.1 以来的着色器适配行为
    let adapt_shaders = adapt_shaders.unwrap_or(true);

    log_info!("{}", "=".repeat(60));
    log_info!("Batch conversion started");
    let plan = perf_plan();
    // **§9.151：池大小用 `threads`（CPU 总预算），不是 `packs`（内存保护）。**
    //
    // `perf.rs` 的模块文档把两者的语义写得很清楚：
    //   * `threads` 是「**总活跃线程上限**」，包内并行任务与 PNG 并行编码都在这个池里跑；
    //   * `packs` 「只是在此基础上再限制**同时展开几个包**」，理由是内存（每包解压 +
    //     图片解码有峰值内存），**不是 CPU 保护**。
    //
    // 但这里原先写成 `let parallelism = plan.packs;` 并直接拿它建池，于是本机
    // （24 核 / 性能档：threads=24、packs=6）的池只有 **6 个线程**——
    // **单包转换时 18 个核闲置，用户选的"性能"档没有兑现**。
    //
    // 实测可见的后果：GUI 日志里并行批的记录是 `architect 并行批（6 项，含落盘）`、
    // 各项 0.007–0.012s，正好卡在 6 路并行上。
    //
    // 修法：**池 = `threads`（CPU 总预算）**，而并发**包数**另用信号量限到 `packs`
    // （保住它的内存保护语义）。两者分离后：单包能用满线程预算，多包也不会同时展开过多。
    let effective_packs = plan.packs;
    let pool_size = plan.threads.max(effective_packs).max(1);
    let parallelism = effective_packs;
    log_info!(
        "Performance mode: {} (cores={}, thread budget={}, concurrent packs={}, rayon pool={})",
        plan.mode,
        plan.cores,
        plan.threads,
        plan.packs,
        pool_size
    );
    if let Some(note) = &plan.note {
        crate::log_warn!("Performance note: {}", note);
    }
    log_info!("Files to process: {} (parallelism: {})", file_paths.len(), parallelism);
    log_info!("Target pack_format: {} ({})", target_format, pack_format_label_for_output(target_format));
    log_info!("fix_alpha_layers: {}, adapt_shaders: {}", fix_alpha, adapt_shaders);
    log_info!("{}", "=".repeat(60));

    let start_time = Instant::now();
    // Reset the cancel flag at the start of every batch.
    CONVERSION_CANCELLED.store(false, Ordering::SeqCst);

    // Mark the batch as running for the whole command (cleared on drop).
    let _running = RunningGuard::new();

    let result = task::spawn_blocking(move || {
        let output_dirs = output_dirs.as_ref();

        // Per-file conversion, shared by both the parallel and the
        // fallback serial path.
        let convert_one = |(i, file_path): (usize, &String)| -> serde_json::Value {
            // Honour user cancellation between files.
            if CONVERSION_CANCELLED.load(Ordering::SeqCst) {
                crate::log_warn!(
                    "conversion: cancelled by user — skipping {}/{} ({})",
                    i + 1,
                    file_paths.len(),
                    file_path
                );
                return serde_json::json!({
                    "input": file_path,
                    "status": "cancelled",
                    "error": "cancelled",
                });
            }

            let file_start = Instant::now();
            let file_name = std::path::Path::new(file_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();

            log_info!("[{}/{}] Processing: {}", i + 1, file_paths.len(), file_name);

            let output_path = output_dirs
                .and_then(|dirs| dirs.get(i))
                .cloned()
                .and_then(|s| if s.is_empty() { None } else { Some(s) });

            match crate::pack::version_converter::process_zip_timed(
                file_path,
                target_format,
                None,
                1.0,
                None,
                output_path.as_deref(),
                fix_alpha,
                adapt_shaders,
                // §9.138：GUI 命令保持「完整转换」语义（GUI 手术开）
                true,
            ) {
                Ok((result_path, timing)) => {
                    let elapsed = file_start.elapsed();
                    // 同时给出两个口径：pure = 纯转换（引擎），total = 含 IO。
                    log_info!("[{}/{}] Complete (pure {:.2}s / total {:.2}s): {} -> {}",
                        i + 1, file_paths.len(), timing.pure_s, timing.total_s,
                        file_name, result_path);

                    if let Some(ref dir) = output_path {
                        match build_output_path_for_batch(std::path::Path::new(file_path), target_format, None, Some(dir)) {
                            Ok(p) => log_info!("  Output location: {}", p.display()),
                            Err(e) => log_info!("  Output location error: {}", e),
                        }
                    }

                    serde_json::json!({
                        "input": file_path,
                        "status": "success",
                        "output": result_path,
                        "time": format!("{:.2}", elapsed.as_secs_f32()),
                        "pure": format!("{:.2}", timing.pure_s),
                        "total": format!("{:.2}", timing.total_s),
                        "io": format!("{:.2}", timing.extract_s + timing.pack_s + timing.cleanup_s),
                    })
                }
                Err(e) => {
                    let elapsed = file_start.elapsed();
                    log_error!("[{}/{}] Failed ({:.2}s): {} - {}",
                        i + 1, file_paths.len(), elapsed.as_secs_f32(),
                        file_name, e);

                    serde_json::json!({
                        "input": file_path,
                        "status": "error",
                        "error": e,
                        "time": format!("{:.2}", elapsed.as_secs_f32())
                    })
                }
            }
        };

        // Parallel path: bounded pool sized by the user's
        // `conversion_threads` setting. rayon preserves the input
        // order in the collected Vec, so the frontend can still pair
        // results with items 1:1. If the pool cannot be built for any
        // reason we fall back to serial execution so the batch never
        // hard-fails on a pool error.
        // Parallel path: pool sized by the user's **CPU budget** (`plan.threads`), while the
        // number of packs in flight at once is bounded separately by the pack-count guard.
        // rayon preserves the input order in the collected Vec, so the frontend can still pair
        // results with items 1:1. If the pool cannot be built for any reason we fall back to
        // serial execution so the batch never hard-fails on a pool error.
        let pack_gate = PackGate::new(parallelism);
        match rayon::ThreadPoolBuilder::new()
            .num_threads(pool_size)
            .thread_name(|i| format!("pack-conv-{}", i))
            .build()
        {
            Ok(pool) => pool.install(|| {
                file_paths
                    .par_iter()
                    .enumerate()
                    .map(|(i, p)| {
                        // 内存保护：同时展开的包数不超过 `packs`。池变大（= CPU 预算）之后
                        // 必须由这里兜住，否则一次会把所有包都解压开来。
                        let _ticket = pack_gate.acquire();
                        convert_one((i, p))
                    })
                    .collect::<Vec<_>>()
            }),
            Err(e) => {
                crate::log_warn!(
                    "conversion: parallel pool unavailable ({}) — falling back to serial",
                    e
                );
                file_paths.iter().enumerate().map(convert_one).collect()
            }
        }
    }).await;

    match result {
        Ok(results) => {
            let elapsed = start_time.elapsed();
            let success_count = results.iter().filter(|r| r["status"] == "success").count();
            let error_count = results.iter().filter(|r| r["status"] == "error").count();

            // Record the journal entries (one per input file). Done here
            // on the async runtime after the pool has finished so the
            // file writes never race with the rayon workers.
            for r in &results {
                let duration_s = r["time"]
                    .as_str()
                    .and_then(|t| t.parse::<f64>().ok())
                    .unwrap_or(0.0);
                crate::commands::history::record_entry(crate::commands::history::HistoryEntry {
                    input: r["input"].as_str().unwrap_or_default().to_string(),
                    output: r["output"].as_str().map(String::from),
                    status: r["status"].as_str().unwrap_or("error").to_string(),
                    error: r["error"].as_str().map(String::from),
                    time: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    duration_s,
                });
            }

            log_info!("{}", "=".repeat(60));
            log_info!("Batch conversion complete");
            log_info!("Results: {} success, {} failed, {} total", success_count, error_count, results.len());
            // 两个口径：pure = 各包纯转换时间之和（引擎，不含 IO），
            //          wall = 批处理墙钟时间（含解压/打包 IO 与并行调度）。
            let pure_sum: f64 = results
                .iter()
                .filter_map(|r| r["pure"].as_str())
                .filter_map(|s| s.parse::<f64>().ok())
                .sum();
            // IO 合计 = 解压 + 打包 + 临时目录清理（各包纯转换之外的实测工作）
            let io_sum: f64 = results
                .iter()
                .filter_map(|r| r["io"].as_str())
                .filter_map(|s| s.parse::<f64>().ok())
                .sum();
            log_info!("Pure conversion time: {:.2}s (sum of engine work)", pure_sum);
            log_info!(
                "IO time: {:.2}s (extract + pack + temp cleanup)",
                io_sum
            );
            log_info!("Total time (incl. IO): {:.2}s", elapsed.as_secs_f32());
            log_info!(
                "Unaccounted: {:.2}s (scheduling, report writing, per-pack overhead){}",
                (elapsed.as_secs_f64() - pure_sum - io_sum).max(0.0),
                if parallelism > 1 {
                    format!(" · wall-clock with parallelism={}", parallelism)
                } else {
                    String::new()
                }
            );
            log_info!("{}", "=".repeat(60));
            Ok(results)
        },
        Err(e) => {
            log_error!("Batch conversion thread failed: {}", e);
            Err(format!("Thread execution failed: {}", e))
        }
    }
}
