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
    get_perf_plan,
    open_folder,
    get_install_dir,
    read_legal_file,
    analyze_pack,
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
    get_conversion_history,
    clear_conversion_history,
    set_background,
    clear_background,
    read_image_b64,
    update_background_settings,
    set_action_monitor,
    is_action_monitor,
    log_action,
    export_action_records,
    clear_action_records,
    action_monitor_status,
    set_action_viewport,
    force_quit,
    ping,
    show_toast,
    dismiss_toast,
    dismiss_all_toasts,
    focus_main_window,
    run_toast_action,
    show_system_notification,
    foray_open_pack,
    foray_get_rom,
    foray_run_probes,
    foray_read_file,
    foray_paint_open,
    foray_paint_brush,
    foray_paint_hsv,
    foray_paint_undo,
    foray_paint_preview,
    foray_paint_commit,
    foray_export,
    foray_ai_config_get,
    foray_ai_config_set,
    foray_ai_prepare,
    foray_ai_analyze,
    foray_ai_test,
    ForayState,
};

pub mod arom;
/// 基岩 ↔ Java 结构转换（生产功能：Bedrock 目标 / Bedrock 源预检，§9.125 移出 `converters/`）。
mod bedrock_convert;
mod commands;
/// 资源包 I/O 与分析工具（原 converters/，§9.128 只保留生产必需的四块）。
mod pack;
mod foray;
pub mod native_run;
/// **原生实现主体**：46 个任务的 A-ROM 实现（原 `pilots/`，§9.127 改名 `natives/` 并分组）。
///
/// 名字即身份：这些模块**是生产实现**，不再是"试点"。目录按职责分六组
/// （`native/` `eraser/` `architect/` `surgeon/` `reverse/` `tests/`），
/// 模块路径仍是 `crate::natives::<任务名>`（分组用 `#[path]` 声明，不改路径）。
pub mod natives;
mod image_utils;
mod color_utils;
mod invoke_conversion;
mod logger;
mod overlay;
pub mod perf;
mod resource_resolver;
/// 箱子贴图区域变换（供原生实现使用，§9.124）
/// 颜色/HSV 工具（供原生实现使用，§9.124）
mod color;
mod chest_region;
/// 缩放因子（`determine_scale_factor`）：原生实现与测试共用，§9.125。
///
/// §9.128：文件已从 `converters/` 移到 crate 根（旧转换器树送走后，`converters/`
/// 只剩生产四块，这个共享工具不该再挂在那里）。
mod scale_factor;
/// **任务元数据表**（名字 / 并发类型 / 阶段）——生产驱动取阶段的唯一来源，§9.125。
mod task_registry;

mod updater;

// §9.121（M3 ②-c）：`invoke_conversion::invoke_conversion` 与其 `_ex` 变体已删除——
// 生产入口改走 `native_run`（§9.118），旧入口不再有任何调用者。
//
// §9.125–§9.129（M3 收口）：本模块只剩 `register_tasks`（52 行），**只注册元数据**
// （名字 + `TaskType` + `Tier`）——驱动取阶段要用它。
// 那 88 个旧闭包体**已彻底删除**，不是门控：`cut_gui` 折进派发表（§9.129）、
// 任务改为按计划顺序逐个派发 `Tx`，闭包签名随之作废（§9.130 删 `HurrayContext` / `TexturePool`）。
// `pack/` 下的旧实现整棵移除（§9.128），只留 `archive/legacy-converters/` 作参考快照。

/// 只读资源包结构分析（Tauri 命令与 CLI `--analyze` 共用）。
pub use pack::analysis::{analyze_dir, analyze_zip, LayerInfo, PackAnalysis, PackShape};

/// 输出对比 / 质量闸门（CLI `--pack-diff`）。
pub use pack::diff as pack_diff;

/// 无界面转换 CLI 所需的管线入口与版本解析。
pub use pack::version_converter::{
    pack_format_label_for_output, process_zip_timed, resolve_target_format, ConversionTiming,
};

/// 后台临时目录清理的等待接口（CLI 退出前调用）。
pub use pack::io::{
    pending_cleanups, set_cleanup_mode, wait_for_cleanups, CleanupMode,
};

/// 刷出日志缓冲（短命进程退出前必须调用，否则丢日志尾部）。
pub fn flush_logs() {
    logger::GLOBAL_LOGGER.flush();
}

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

    // `--action-monitor` 启动参数：记录前端所有点击行为（开发者诊断）。
    // 也可以之后在设置页开发者选项里开关。
    //
    // 传参方式（Tauri CLI 规则：第二个 `--` 之后才是应用参数）：
    //   npm run tauri dev -- -- -- --action-monitor
    // 或直接设置环境变量：
    //   $env:2PYR_ACTION_MONITOR = "1"; npm run tauri dev
    let action_monitor_arg = std::env::args().any(|a| a == "--action-monitor")
        || std::env::var("2PYR_ACTION_MONITOR")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
    if action_monitor_arg {
        crate::commands::misc::ACTION_MONITOR.store(true, std::sync::atomic::Ordering::Relaxed);
        log_info!("action monitor enabled via --action-monitor / env");
    }

    match tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(ForayState::default())
        .setup(|app| {
            // debug 构建（tauri dev）始终启动本地动作流端口 127.0.0.1:24159：
            // Action Mon3tr 用它检测「tauri dev 下的 2-Pyramid」，并按需抓取
            // 主窗口实时截图（截图不落盘）。动作监视未开启时连接照常建立，
            // 只是没有帧推送（协议头带 monitor=off 标记）。
            #[cfg(debug_assertions)]
            crate::commands::misc::ensure_action_live_server(app.handle().clone());
            // Single instance: bind a fixed localhost TCP port before
            // creating any window. The first process to bind owns the
            // lock; a second process fails to bind, pokes the port,
            // waits for our "ok" ack and exits immediately — the first
            // instance then brings its main window to the foreground.
            //
            // Implemented with plain std (no extra crate, no network
            // access needed at build time).
            install_single_instance(app);

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
            // beta 渠道在窗口标题上带 Beta 标识（任务栏悬停可见）
            .title(if option_env!("2PYR_CHANNEL") == Some("beta") {
                "2-Pyramid (Beta)"
            } else {
                "2-Pyramid"
            })
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
        .on_window_event(|window, event| {
            use tauri::Manager;
            // Closing the main window must exit the WHOLE app. Tauri
            // only shuts the process down once ALL windows are gone —
            // a lingering toast notification window (up to 10s) would
            // otherwise keep the process alive in the background after
            // the user already "closed" the app. Force a full exit as
            // soon as the main window is destroyed; exit(0) tears down
            // every remaining webview window as well.
            if window.label() == "main" {
                if let tauri::WindowEvent::Destroyed = event {
                    crate::log_info!("main window destroyed → exiting process");
                    window.app_handle().exit(0);
                }
            }
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
            get_perf_plan,
            test_command,
            open_folder,
            get_install_dir,
            read_legal_file,
            analyze_pack,
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
            get_conversion_history,
            clear_conversion_history,
            set_background,
            clear_background,
            read_image_b64,
            update_background_settings,
            set_action_monitor,
            is_action_monitor,
            log_action,
            export_action_records,
            clear_action_records,
            action_monitor_status,
            set_action_viewport,
            force_quit,
            ping,
            show_toast,
            dismiss_toast,
            dismiss_all_toasts,
            focus_main_window,
            run_toast_action,
            show_system_notification,
            foray_open_pack,
            foray_get_rom,
            foray_run_probes,
            foray_read_file,
            foray_paint_open,
            foray_paint_brush,
            foray_paint_hsv,
            foray_paint_undo,
            foray_paint_preview,
            foray_paint_commit,
            foray_export,
            foray_ai_config_get,
            foray_ai_config_set,
            foray_ai_prepare,
            foray_ai_analyze,
            foray_ai_test,
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

/// Fixed localhost port used as the single-instance lock. Chosen to be
/// unlikely to collide with other software; if something else already
/// holds it, both sides fall back gracefully (see below).
const SINGLE_INSTANCE_PORT: u16 = 24157;

/// Single-instance enforcement without external crates:
///
/// * First instance binds `127.0.0.1:SINGLE_INSTANCE_PORT` and spawns a
///   thread that accepts "wake" pokes; on each poke it replies "ok" and
///   brings the main window to the foreground.
/// * A second instance fails to bind. It connects to the port, sends
///   "wake" and waits briefly for an "ok" ack. If acked, another
///   2-Pyramid instance is definitely running → exit immediately. If
///   not acked (the port is held by unrelated software), start
///   normally — app availability beats strict single-instance.
fn install_single_instance(app: &tauri::App) {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::time::Duration;
    use tauri::Manager;

    let addr = SocketAddr::from(([127, 0, 0, 1], SINGLE_INSTANCE_PORT));

    match TcpListener::bind(addr) {
        Ok(listener) => {
            // First instance — own the lock and wake the window whenever
            // a second instance knocks.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    match stream {
                        Ok(mut s) => {
                            // Read the (tiny) wake payload, then ack so
                            // the caller knows it reached a real
                            // 2-Pyramid instance.
                            let mut buf = [0u8; 4];
                            let _ = s.set_read_timeout(Some(Duration::from_millis(500)));
                            let _ = s.read(&mut buf);
                            let _ = s.write_all(b"ok");
                            crate::log_info!(
                                "single-instance: wake poke received → focusing main window"
                            );
                            if let Some(w) = handle.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.unminimize();
                                let _ = w.set_focus();
                            }
                        }
                        Err(_) => break, // listener closed (app shutting down)
                    }
                }
            });
            crate::log_info!(
                "single-instance: lock acquired on port {}",
                SINGLE_INSTANCE_PORT
            );
        }
        Err(_) => {
            // Could not bind — maybe another 2-Pyramid instance holds it.
            // Verify by poking and awaiting the "ok" ack before giving up.
            if let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_millis(400)) {
                let _ = s.write_all(b"wake");
                let mut ack = [0u8; 2];
                if s.read(&mut ack).map(|n| &ack[..n] == b"ok").unwrap_or(false) {
                    crate::log_info!(
                        "single-instance: another instance confirmed — exiting this process"
                    );
                    std::process::exit(0);
                }
            }
            crate::log_warn!(
                "single-instance: port {} unavailable but no 2-Pyramid instance answered — starting normally",
                SINGLE_INSTANCE_PORT
            );
        }
    }
}

