//! System tray icon + window close interception.
//!
//! When `close_action = "minimize"` (UI label: 后台到托盘 / "Background to Tray"),
//! the user expects the window to disappear into the system tray (the small
//! icons next to the clock) instead of the taskbar. We achieve this by:
//!
//! 1. Building a `TrayIcon` at startup with a "Show" + "Quit" menu and a
//!    left-click handler that restores the main window.
//! 2. Hooking `on_window_event` so that whenever the OS asks the window to
//!    close (title bar X, Alt+F4, taskbar right-click → Close, task manager,
//!    logoff, etc.) we can intercept it via `api.prevent_close()` and
//!    `window.hide()` instead of letting the app exit.
//!
//! Reading the latest `close_action` on every event is intentional: the
//! user may flip the setting in Settings and the next close attempt should
//! honor it. The settings.json file is tiny (<10 KB) so a synchronous read
//! per event is well below any perceptible threshold.

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

use crate::commands::config::read_config_file;

const TRAY_ID: &str = "main-tray";
const MENU_SHOW: &str = "show";
const MENU_QUIT: &str = "quit";

// Embedded ICO so the tray has a valid icon even when the runtime cannot
// decode PNGs (dev mode + `tauri` features without `image-png`) or when
// `app.default_window_icon()` returns `None` for any reason. ICO is a
// well-supported format that `tauri::image::Image::from_bytes` decodes
// without any additional Cargo features.
const FALLBACK_ICON_ICO: &[u8] = include_bytes!("../../icons/icon.ico");

/// Build the system tray icon and attach all handlers. Called from
/// `tauri::Builder::setup` after window state is initialized.
///
/// If tray creation fails for any reason (e.g. no system tray in the
/// current environment, icon decode error) we log the failure but return
/// `Ok(())` so the rest of the app can still start. The frontend's own
/// `currentWindow.hide()` path is what actually hides the window; the
/// tray is purely a UX nicety on top.
pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    // Resolve an icon with a robust fallback chain:
    //   1. Tauri's default window icon (PNG, requires `image-png` feature)
    //   2. Embedded ICO bytes (always available; ICO needs no features)
    //   3. Error: tray without icon — `TrayIconBuilder::with_id(...).build`
    //      will reject an icon we can't decode; we then give up entirely.
    let icon = match app.default_window_icon().cloned() {
        Some(ic) => ic,
        None => match Image::from_bytes(FALLBACK_ICON_ICO) {
            Ok(ic) => {
                crate::log_info!("tray: using embedded ICO fallback (default icon unavailable)");
                ic
            }
            Err(e) => {
                crate::log_error!("tray: icon decode failed: {}", e);
                return Err(tauri::Error::AssetNotFound(format!(
                    "default icon: {}, fallback ICO: {}",
                    "None", e
                )));
            }
        },
    };

    let show_item = MenuItem::with_id(app, MENU_SHOW, "Show 2-Pyramid", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    match TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(false) // keep colors; the icon is already a styled PNG/ICO
        .tooltip("2-Pyramid")
        .menu(&menu)
        .show_menu_on_left_click(false) // left click = restore window, right click = menu
        .on_menu_event(|app, event| match event.id.as_ref() {
            MENU_SHOW => show_main_window(app),
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Single left-click (not double, not right) brings the window back.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)
    {
        Ok(_) => {
            crate::log_info!("tray: system tray initialized");
            Ok(())
        }
        Err(e) => {
            // Don't take down the whole app if the tray subsystem fails
            // (some headless / kiosk Windows configurations have no tray).
            // The frontend still hides the window; the user just has no
            // tray entry point to restore it.
            crate::log_error!("tray: build failed: {} — continuing without tray", e);
            Ok(())
        }
    }
}

/// Restore + focus the main window. No-op if the window label can't be found.
fn show_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Window-level event hook. Called from `Builder::on_window_event` on every
/// window event the OS delivers to our main webview window.
///
/// We only care about `CloseRequested`: when close_action is "minimize",
/// prevent the actual close and hide the window so the app keeps running in
/// the tray. For "close" or "ask" we let the close proceed; the frontend
/// handles the "ask" prompt via its own close button handler.
pub fn handle_window_event(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        let action = read_config_file()
            .ok()
            .and_then(|c| c.close_action)
            .unwrap_or_else(|| "ask".to_string());

        match action.as_str() {
            "minimize" => {
                api.prevent_close();
                match window.hide() {
                    Ok(()) => crate::log_info!("tray: close intercepted → hide to tray"),
                    Err(e) => crate::log_error!("tray: window.hide() failed: {}", e),
                }
            }
            // "ask" / "close" / unknown → let it close. The frontend's
            // custom close button will still show the ask dialog when
            // applicable.
            _ => {}
        }
    }
}