use tokio::task;
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;

use crate::converters::version_converter::{
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

/// Default parallelism for batch conversion, overridable per-user via
/// the `conversion_threads` config setting (1–4). 2 is a deliberate
/// balance: packs are IO-heavy (unzip → transform → rezip) so a single
/// worker starves the disk, while too many workers blow up memory
/// (every pack is fully extracted into its own tempdir) and fight with
/// the hurray engine's internal rayon pool.
const DEFAULT_CONCURRENT_PACKS: usize = 2;

/// Read the user-configured batch parallelism (clamped to 1–4).
fn concurrent_packs() -> usize {
    crate::commands::config::read_config_file()
        .ok()
        .and_then(|c| c.conversion_threads)
        .map(|n| n.clamp(1, 4) as usize)
        .unwrap_or(DEFAULT_CONCURRENT_PACKS)
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
        crate::converters::version_converter::convert_resource_pack(
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
) -> Result<Vec<serde_json::Value>, String> {
    use std::time::Instant;

    use crate::log_info;
    use crate::log_error;

    log_info!("{}", "=".repeat(60));
    log_info!("Batch conversion started");
    let parallelism = concurrent_packs();
    log_info!("Files to process: {} (parallelism: {})", file_paths.len(), parallelism);
    log_info!("Target pack_format: {} ({})", target_format, pack_format_label_for_output(target_format));
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

            match process_zip(
                file_path,
                target_format,
                None,
                1.0,
                None,
                output_path.as_deref(),
            ) {
                Ok(result_path) => {
                    let elapsed = file_start.elapsed();
                    log_info!("[{}/{}] Complete ({:.2}s): {} -> {}",
                        i + 1, file_paths.len(), elapsed.as_secs_f32(),
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
                        "time": format!("{:.2}", elapsed.as_secs_f32())
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
        let pool_size = parallelism.max(1);
        match rayon::ThreadPoolBuilder::new()
            .num_threads(pool_size)
            .thread_name(|i| format!("pack-conv-{}", i))
            .build()
        {
            Ok(pool) => pool.install(|| {
                file_paths
                    .par_iter()
                    .enumerate()
                    .map(convert_one)
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

            log_info!("{}", "=".repeat(60));
            log_info!("Batch conversion complete");
            log_info!("Results: {} success, {} failed, {} total", success_count, error_count, results.len());
            log_info!("Total time: {:.2}s", elapsed.as_secs_f32());
            log_info!("{}", "=".repeat(60));
            Ok(results)
        },
        Err(e) => {
            log_error!("Batch conversion thread failed: {}", e);
            Err(format!("Thread execution failed: {}", e))
        }
    }
}
