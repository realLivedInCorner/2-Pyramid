#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use self::commands::{
    get_logs,
    set_dev_mode,
    get_dev_mode,
    convert_zip,
    convert_resource_pack,
    test_command,
    convert_resource_packs_batch,
    cancel_conversion,
    is_conversion_running,
    open_folder,
    write_file,
    create_dir,
    delete_paths,
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
    get_app_info,
    clear_config,
    factory_reset,
    factory_reset_deep,
    get_last_backup_info,
    import_last_backup,
    force_quit,
    ping,
    show_toast,
    dismiss_toast,
    dismiss_all_toasts,
    focus_main_window,
    run_toast_action,
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

    // ── WebView2: disable background throttling ────────────────────
    // When the window is minimized / occluded, WebView2 by default
    // throttles the renderer: JS timers stop firing, which kills our
    // frontend heartbeat and makes every window-control button feel
    // dead after the window is restored. The environment variable
    // must be set BEFORE the WebView2 environment is created, so we do
    // it here at the very top of run(), before tauri::Builder::build().
    if cfg!(windows) {
        const FLAG: &str = "--disable-backgrounding-occluded-windows";
        let existing = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS")
            .unwrap_or_default();
        if !existing.contains(FLAG) {
            let combined = if existing.trim().is_empty() {
                FLAG.to_string()
            } else {
                format!("{} {}", existing.trim(), FLAG)
            };
            std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", combined);
            log_info!("webview2: background throttling disabled (browser arg set)");
        }
    }

    log_info!("========================================");
    log_info!("2-Pyramid started");
    log_info!("Started at: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
    log_info!("========================================");
    log_debug!("Debug logging enabled");

    match tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        // Single instance: a second launch of the app is not allowed
        // to spawn another process. Instead it re-activates the
        // already-running main window. NOTE: `run_silent` (context-menu
        // conversion) deliberately does NOT register this plugin — a
        // silent conversion must be able to run alongside the main
        // app as its own short-lived process.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            // Create the main window in code (not via tauri.conf.json)
            // so we can disable WebView2 background throttling — the
            // config file has no field for it. Without this, restoring
            // a minimized window can feel dead because the renderer was
            // suspended while minimized. `background_throttling(false)`
            // keeps the renderer alive at all times.
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("2-Pyramid")
            .inner_size(1200.0, 750.0)
            .min_inner_size(800.0, 600.0)
            .decorations(false)
            .transparent(true)
            .center()
            .resizable(true)
            .focused(true)
            .visible(true)
            .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled)
            .build()
            .map_err(|e| {
                crate::log_error!("failed to build main window: {}", e);
                e
            })?;
            crate::log_info!("main window created in code (background throttling disabled)");

            // Resolve and cache resource paths via Tauri resource API (required for production)
            let handle = app.handle();
            crate::resource_resolver::cache_resource_from_app(&handle, "UImage");
            crate::resource_resolver::cache_resource_from_app(&handle, "overlay");

            // Backward compat: ensure overlay/UImage at exe level for old MSI installs
            if let Err(e) = crate::resource_resolver::ensure_resources_at_exe_level() {
                crate::log_info!("ensure_resources_at_exe_level: {}", e);
            }

            // The taskbar / window icon is wired in `build.rs` via
            // `tauri_build::Attributes::windows_icon("icons/icon.ico")`,
            // which embeds icon.ico into the EXE's Win32 resource directory
            // at link time. No runtime `set_icon` is needed (that path
            // would require enabling the tauri `image-png` / `image-ico`
            // Cargo features and is redundant with build-time embedding).

            Ok(())
        })
        .invoke_handler(tauri::generate_handler!(
            get_logs,
            set_dev_mode,
            get_dev_mode,
            get_app_info,
            convert_zip,
            convert_resource_pack,
            convert_resource_packs_batch,
            cancel_conversion,
            is_conversion_running,
            test_command,
            open_folder,
            write_file,
            create_dir,
            commands::misc::delete_paths,
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
            clear_config,
            factory_reset,
            factory_reset_deep,
            get_last_backup_info,
            import_last_backup,
            force_quit,
            ping,
            show_toast,
            dismiss_toast,
            dismiss_all_toasts,
            focus_main_window,
            run_toast_action,
            updater::check_for_update,
            updater::download_update,
            updater::install_update,
            updater::get_update_channel,
            updater::set_update_channel,
            updater::check_update_marker
        ))
        .build(tauri::generate_context!())
    {
        Ok(app) => {
            // Plain run loop: closing the main window exits the app.
            // There is no tray / background-resident mode anymore, so
            // no ExitRequested interception is needed — window gone
            // means process gone, exactly what the user expects.
            app.run(|_app_handle, _event| {});
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
