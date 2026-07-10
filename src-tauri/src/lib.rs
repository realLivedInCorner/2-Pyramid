#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use self::commands::{
    get_logs,
    set_dev_mode,
    get_dev_mode,
    convert_zip,
    convert_resource_pack,
    test_command,
    convert_resource_packs_batch,
    open_folder,
    write_file,
    create_dir,
    get_config,
    update_config,
    overlay_init,
    overlay_package,
    get_overlay_settings,
    save_overlay_settings,
    overlay_set_parent_pack,
    get_overlay_lang,
    save_overlay_lang,
    read_lang_file,
    save_overlay_json,
    get_overlay_json,
    get_overlay_projects,
    delete_overlay_project,
    import_lang_from_parent,
    export_overlay_share_code,
    import_overlay_share_code,
    log_notification,
    export_logs,
    get_log_path,
};

mod commands;
mod converters;
mod image_utils;
mod color_utils;
mod invoke_conversion;
mod logger;
mod overlay;
mod resource_resolver;
mod updater;
pub mod hurray;

pub use invoke_conversion::invoke_conversion;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use crate::{log_info, log_debug, log_error};

    log_info!("========================================");
    log_info!("2-Pyramid started");
    log_info!("Started at: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
    log_info!("========================================");
    log_debug!("Debug logging enabled");

    match tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // Resolve and cache resource paths via Tauri resource API (required for production)
            let handle = app.handle();
            crate::resource_resolver::cache_resource_from_app(&handle, "UImage");
            crate::resource_resolver::cache_resource_from_app(&handle, "overlay");

            // Backward compat: ensure overlay/UImage at exe level for old MSI installs
            if let Err(e) = crate::resource_resolver::ensure_resources_at_exe_level() {
                crate::log_info!("ensure_resources_at_exe_level: {}", e);
            }

            // Build the system tray icon (Show / Quit menu, left-click restores).
            // When `close_action = "minimize"`, closing the window hides it into
            // this tray instead of exiting the app — see `tray::handle_window_event`.
            if let Err(e) = crate::commands::tray::setup_tray(&app) {
                crate::log_error!("tray: setup failed: {}", e);
            }

            // The taskbar / window icon is wired in `build.rs` via
            // `tauri_build::Attributes::windows_icon("icons/icon.ico")`,
            // which embeds icon.ico into the EXE's Win32 resource directory
            // at link time. No runtime `set_icon` is needed (that path
            // would require enabling the tauri `image-png` / `image-ico`
            // Cargo features and is redundant with build-time embedding).

            Ok(())
        })
        .on_window_event(|window, event| {
            crate::commands::tray::handle_window_event(window, event);
        })
        .invoke_handler(tauri::generate_handler!(
            get_logs,
            set_dev_mode,
            get_dev_mode,
            convert_zip,
            convert_resource_pack,
            convert_resource_packs_batch,
            test_command,
            open_folder,
            write_file,
            create_dir,
            get_config,
            update_config,
            overlay_init,
            overlay_package,
            get_overlay_settings,
            save_overlay_settings,
            overlay_set_parent_pack,
            get_overlay_lang,
            save_overlay_lang,
            read_lang_file,
            save_overlay_json,
            get_overlay_json,
            get_overlay_projects,
            delete_overlay_project,
            import_lang_from_parent,
            export_overlay_share_code,
            import_overlay_share_code,
            log_notification,
            export_logs,
            get_log_path,
            commands::clear_config,
            updater::check_for_update,
            updater::download_update,
            updater::install_update,
            updater::get_update_channel,
            updater::set_update_channel,
            updater::check_update_marker
        ))
        .run(tauri::generate_context!())
    {
        Ok(_) => {
            log_info!("========================================");
            log_info!("2-Pyramid exited normally");
            log_info!("========================================");
        }
        Err(e) => {
            log_error!("========================================");
            log_error!("2-Pyramid failed: {}", e);
            log_error!("========================================");
            std::process::exit(1);
        }
    }
}

/// Context menu silent conversion: no main window, notification only
pub fn run_silent(file_path: String, format: u32) {
    use crate::{log_info, log_error};
    use crate::converters::version_converter::convert_resource_pack;

    log_info!("Silent conversion: {} -> pack_format {}", file_path, format);

    match tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(move |app| {
            let handle = app.handle();

            // Immediately hide window to avoid flicker
            {
                use tauri::Manager as _;
                handle.webview_windows().iter().for_each(|(_, w)| {
                    let _ = w.hide();
                });
            }

            // Resolve resource paths
            crate::resource_resolver::cache_resource_from_app(&handle, "UImage");
            crate::resource_resolver::cache_resource_from_app(&handle, "overlay");

            // Backward compat: ensure overlay/UImage at exe level for old MSI installs
            if let Err(e) = crate::resource_resolver::ensure_resources_at_exe_level() {
                crate::log_info!("ensure_resources_at_exe_level: {}", e);
            }

            let handle_clone = handle.clone();
            let file_clone = file_path.clone();

            tauri::async_runtime::spawn(async move {
                use tauri_plugin_notification::NotificationExt;
                use std::path::Path;

                // Execute conversion in blocking thread
                let result = tokio::task::spawn_blocking(move || {
                    convert_resource_pack(&file_clone, format)
                }).await;

                match result {
                    Ok(Ok(output)) => {
                        log_info!("Silent conversion done: {}", output);
                        let filename = Path::new(&output)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| output.clone());
                        let _ = handle_clone
                            .notification()
                            .builder()
                            .title("2-Pyramid - Conversion Complete")
                            .body(&filename)
                            .show();
                    }
                    Ok(Err(e)) => {
                        log_error!("Silent conversion failed: {}", e);
                        let _ = handle_clone
                            .notification()
                            .builder()
                            .title("2-Pyramid - Conversion Failed")
                            .body(&e)
                            .show();
                    }
                    Err(e) => {
                        log_error!("Silent conversion join error: {}", e);
                        let _ = handle_clone
                            .notification()
                            .builder()
                            .title("2-Pyramid - Conversion Failed")
                            .body(&e.to_string())
                            .show();
                    }
                }

                // Wait for notification to be sent before exiting
                std::thread::sleep(std::time::Duration::from_secs(1));
                handle_clone.exit(0);
            });

            Ok(())
        })
        .build(tauri::generate_context!())
    {
        Ok(app) => {
            app.run(|_, _| {});
        }
        Err(e) => {
            log_error!("Silent app build failed: {}", e);
            std::process::exit(1);
        }
    }
}
