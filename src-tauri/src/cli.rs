//! 无界面转换 CLI：`2-pyramid.exe --convert <路径> [--to <版本>] [--out <目录>] [--report <file.json>]`
//!
//! 供拖放脚本使用：把资源包（或含资源包的文件夹）拖到脚本上 → 跑完整转换 →
//! 输出**结构化 JSON 报告**（结构分析 + 两个耗时口径 + 逐任务画像 + 体积变化）。
//!
//! 与 GUI 完全同一条管线（`process_zip_timed`），因此报告里的数字可直接用于
//! 性能优化前后的对比。

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;

use two_pyramid_lib::arom::engine::scheduler::TaskTiming;
use two_pyramid_lib::{
    analyze_zip, pack_format_label_for_output, process_zip_timed, resolve_target_format,
    ConversionTiming, PackAnalysis,
};

/// 单个任务的画像条目（报告用）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskProfileEntry {
    task: String,
    tier: String,
    seconds: f32,
    parallel: bool,
}

impl From<TaskTiming> for TaskProfileEntry {
    fn from(t: TaskTiming) -> Self {
        Self {
            task: t.task,
            tier: t.tier.to_string(),
            seconds: (t.seconds * 1000.0).round() / 1000.0,
            parallel: t.parallel,
        }
    }
}

/// 单个资源包的转换结果。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackReport {
    input: String,
    output: Option<String>,
    status: String,
    error: Option<String>,
    input_bytes: Option<u64>,
    output_bytes: Option<u64>,
    /// 纯转换 / 总时间（含 IO）与 IO 分解。
    timing: Option<ConversionTiming>,
    /// 转换前的结构分析（分层/区间/折叠目录/多根）。
    structure: Option<PackAnalysis>,
    /// 逐任务耗时（按耗时降序，**完整列表**，便于分析长尾）。
    task_profile: Vec<TaskProfileEntry>,
    /// 任务画像条目数与耗时合计（秒），用于判断"长尾"占比。
    task_count: usize,
    task_sum_s: f32,
}

/// 整份报告。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConvertReport {
    generated_at: String,
    target_format: u32,
    target_label: String,
    out_dir: Option<String>,
    packs: Vec<PackReport>,
    summary: ReportSummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportSummary {
    packs: usize,
    success: usize,
    failed: usize,
    pure_sum_s: f32,
    total_sum_s: f32,
    io_sum_s: f32,
    wall_clock_s: f32,
    input_bytes: u64,
    output_bytes: u64,
}

fn collect_inputs(path: &Path) -> Result<Vec<PathBuf>, String> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    if !path.is_dir() {
        return Err(format!("路径不存在：{}", path.display()));
    }
    let mut packs: Vec<PathBuf> = walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .filter(|p| {
            let ext = p
                .extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            ext == "zip" || ext == "mcpack"
        })
        .collect();
    packs.sort();
    if packs.is_empty() {
        return Err(format!("目录中没有找到 .zip / .mcpack：{}", path.display()));
    }
    Ok(packs)
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// `--convert` 入口。返回进程退出码。
pub fn run_convert(args: &[String], idx: usize) -> i32 {
    // CLI 是短命进程：临时目录清理交给脱离进程的 rmdir 子进程，
    // 既不等待、也不会留下残留。
    two_pyramid_lib::set_cleanup_mode(two_pyramid_lib::CleanupMode::Detached);

    let input = match args.get(idx + 1).filter(|v| !v.starts_with("--")) {
        Some(v) => v.clone(),
        None => {
            eprintln!(
                "用法: 2-pyramid.exe --convert <资源包.zip | 目录> [--to <版本|pack_format>] [--out <目录>] [--report <报告.json>]"
            );
            return 2;
        }
    };

    let target = match arg_value(args, "--to") {
        Some(spec) => match resolve_target_format(&spec) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("目标版本解析失败：{}", e);
                return 2;
            }
        },
        // 未指定时默认最新 Java 26.3（format 97）
        None => 97,
    };

    let out_dir = arg_value(args, "--out").map(PathBuf::from);
    let report_path = arg_value(args, "--report").map(PathBuf::from);

    let inputs = match collect_inputs(Path::new(&input)) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}", e);
            return 2;
        }
    };

    // 进程内阶段计时：转换之外的开销（结构分析、报告写入）一目了然
    let process_start = Instant::now();
    let mut analyze_secs = 0.0f32;
    let mut convert_secs = 0.0f32;

    println!("==> 2-Pyramid CLI 转换");
    println!("    输入：{}（{} 个资源包）", input, inputs.len());
    println!(
        "    目标：{}（pack_format {}）",
        pack_format_label_for_output(target),
        target
    );
    if let Some(dir) = &out_dir {
        println!("    输出目录：{}", dir.display());
    }
    println!();

    let wall_start = Instant::now();
    let mut packs: Vec<PackReport> = Vec::new();

    for (i, pack) in inputs.iter().enumerate() {
        let name = pack
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        println!("[{}/{}] {}", i + 1, inputs.len(), name);

        let analyze_start = Instant::now();
        let structure = analyze_zip(pack).ok();
        analyze_secs += analyze_start.elapsed().as_secs_f32();
        if let Some(s) = &structure {
            println!("    结构：{}", s.summary());
            for w in &s.warnings {
                println!("      ! {}", w);
            }
        }
        let input_bytes = std::fs::metadata(pack).ok().map(|m| m.len());

        let convert_start = Instant::now();
        match process_zip_timed(
            &pack.to_string_lossy(),
            target,
            None,
            1.0,
            None,
            out_dir.as_ref().map(|d| d.to_string_lossy().to_string()).as_deref(),
            false,
            true,
        ) {
            Ok((output, timing)) => {
                convert_secs += convert_start.elapsed().as_secs_f32();
                let output_bytes = std::fs::metadata(&output).ok().map(|m| m.len());
                let profile: Vec<TaskProfileEntry> = timing
                    .task_profile
                    .iter()
                    .cloned()
                    .map(TaskProfileEntry::from)
                    .collect();
                let task_count = profile.len();
                let task_sum_s: f32 = profile.iter().map(|t| t.seconds).sum();
                println!(
                    "    ✓ 纯转换 {:.2}s / 总时间 {:.2}s（IO {:.2}s）",
                    timing.pure_s,
                    timing.total_s,
                    timing.extract_s + timing.pack_s
                );
                for t in profile.iter().take(5) {
                    println!(
                        "      · {:<28} {:.2}s{}",
                        t.task,
                        t.seconds,
                        if t.parallel { " (parallel)" } else { "" }
                    );
                }
                if let (Some(a), Some(b)) = (input_bytes, output_bytes) {
                    println!(
                        "    ✓ 体积 {:.2} MB → {:.2} MB",
                        a as f64 / 1048576.0,
                        b as f64 / 1048576.0
                    );
                }
                println!("    ✓ 输出：{}", output);
                println!(
                    "    · 任务画像：{} 个任务，合计 {:.2}s（引擎纯转换 {:.2}s）",
                    task_count, task_sum_s, timing.pure_s
                );
                packs.push(PackReport {
                    input: pack.to_string_lossy().to_string(),
                    output: Some(output),
                    status: "success".to_string(),
                    error: None,
                    input_bytes,
                    output_bytes,
                    timing: Some(timing),
                    structure,
                    task_profile: profile,
                    task_count,
                    task_sum_s,
                });
            }
            Err(e) => {
                convert_secs += convert_start.elapsed().as_secs_f32();
                let profile: Vec<TaskProfileEntry> = Vec::new();
                println!("    ✗ 失败：{}", e);
                packs.push(PackReport {
                    input: pack.to_string_lossy().to_string(),
                    output: None,
                    status: "error".to_string(),
                    error: Some(e),
                    input_bytes,
                    output_bytes: None,
                    timing: None,
                    structure,
                    task_profile: profile,
                    task_count: 0,
                    task_sum_s: 0.0,
                });
            }
        }
        println!();
    }

    let wall = wall_start.elapsed().as_secs_f32();
    let process_wall = process_start.elapsed().as_secs_f32();
    let success = packs.iter().filter(|p| p.status == "success").count();
    let pure_sum: f32 = packs
        .iter()
        .filter_map(|p| p.timing.as_ref())
        .map(|t| t.pure_s)
        .sum();
    let total_sum: f32 = packs
        .iter()
        .filter_map(|p| p.timing.as_ref())
        .map(|t| t.total_s)
        .sum();
    let io_sum: f32 = packs
        .iter()
        .filter_map(|p| p.timing.as_ref())
        .map(|t| t.extract_s + t.pack_s)
        .sum();
    let input_bytes: u64 = packs.iter().filter_map(|p| p.input_bytes).sum();
    let output_bytes: u64 = packs.iter().filter_map(|p| p.output_bytes).sum();

    let summary = ReportSummary {
        packs: packs.len(),
        success,
        failed: packs.len() - success,
        pure_sum_s: (pure_sum * 100.0).round() / 100.0,
        total_sum_s: (total_sum * 100.0).round() / 100.0,
        io_sum_s: (io_sum * 100.0).round() / 100.0,
        wall_clock_s: (wall * 100.0).round() / 100.0,
        input_bytes,
        output_bytes,
    };

    println!("================ 汇总 ================");
    println!(
        "资源包：{} 个（成功 {}，失败 {}）",
        summary.packs, summary.success, summary.failed
    );
    println!(
        "纯转换时间合计：{:.2}s ｜ 总时间合计（含 IO）：{:.2}s ｜ IO {:.2}s",
        summary.pure_sum_s, summary.total_sum_s, summary.io_sum_s
    );
    println!("墙钟时间：{:.2}s", summary.wall_clock_s);
    if summary.input_bytes > 0 {
        println!(
            "体积：{:.2} MB → {:.2} MB",
            summary.input_bytes as f64 / 1048576.0,
            summary.output_bytes as f64 / 1048576.0
        );
    }

    let report = ConvertReport {
        generated_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        target_format: target,
        target_label: pack_format_label_for_output(target).to_string(),
        out_dir: out_dir.map(|d| d.to_string_lossy().to_string()),
        packs,
        summary,
    };

    if let Some(path) = report_path {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(&report) {
            Ok(text) => match std::fs::write(&path, text) {
                Ok(()) => println!("\n报告已写入：{}", path.display()),
                Err(e) => eprintln!("报告写入失败：{}", e),
            },
            Err(e) => eprintln!("报告序列化失败：{}", e),
        }
    }

    // 阶段耗时：区分"转换"与"转换之外"（结构分析、报告写入、进程自身开销）。
    // 注意：临时目录清理默认交给脱离进程的 rmdir 子进程，不计入本行，
    // 因此某些外壳（会等待整个进程树）测到的墙钟会比这里更长。
    println!("阶段耗时：结构分析 {:.2}s · 转换 {:.2}s · 其他 {:.2}s = 进程内 {:.2}s",
        analyze_secs,
        convert_secs,
        (process_wall - analyze_secs - convert_secs).max(0.0),
        process_wall
    );

    if report.summary.failed > 0 { 1 } else { 0 }
}

/// CLI 退出前不做任何等待：清理已交给**脱离本进程**的 `rmdir` 子进程，
/// 进程立刻返回，删除在后台继续完成（因此也不会有临时目录残留）。
pub fn finish_pending_cleanups() {
    // 若仍有后台线程模式的清理在跑（例如模式被环境变量改过），最多等 3 秒
    if two_pyramid_lib::pending_cleanups() == 0 {
        return;
    }
    let _ = two_pyramid_lib::wait_for_cleanups(std::time::Duration::from_secs(3));
}
