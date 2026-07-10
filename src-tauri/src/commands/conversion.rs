use tokio::task;

use crate::converters::version_converter::{
    process_zip,
    pack_format_label_for_output,
    build_output_path_for_batch,
};


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

/// Batch resource pack conversion command using Rayon for parallel processing
#[tauri::command]
pub async fn convert_resource_packs_batch(
    file_paths: Vec<String>,
    target_format: u32,
    output_dirs: Option<Vec<String>>,
) -> Result<Vec<serde_json::Value>, String> {
    use std::time::Instant;
    use rayon::prelude::*;

    use crate::log_info;
    use crate::log_error;

    log_info!("{}", "=".repeat(60));
    log_info!("Batch conversion started");
    log_info!("Files to process: {}", file_paths.len());
    log_info!("Target pack_format: {} ({})", target_format, pack_format_label_for_output(target_format));
    log_info!("{}", "=".repeat(60));

    let start_time = Instant::now();

    let result = task::spawn_blocking(move || {
        let results: Vec<serde_json::Value> = file_paths.par_iter().enumerate().map(|(i, file_path)| {
            let file_start = Instant::now();
            let file_name = std::path::Path::new(file_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();

            log_info!("[{}/{}] Processing: {}", i + 1, file_paths.len(), file_name);

            let output_path = output_dirs.as_ref()
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

                    // If output was redirected, show that info
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
        }).collect();

        results
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
