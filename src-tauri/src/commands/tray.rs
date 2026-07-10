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
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

use crate::commands::config::read_config_file;

const TRAY_ID: &str = "main-tray";
const MENU_SHOW: &str = "show";
const MENU_QUIT: &str = "quit";

/// Build the system tray icon and attach all handlers. Called from
/// `tauri::Builder::setup` after window state is initialized.
pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    // Fall back to the bundled default icon if the window doesn't expose
    // one (e.g. tests or unusual startup paths).
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".into()))?;

    let show_item = MenuItem::with_id(app, MENU_SHOW, "Show 2-Pyramid", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    TrayIconBuilder::with_id(TRAY_ID)
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
        .build(app)?;

    crate::log_info!("tray: system tray initialized");
    Ok(())
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
                let _ = window.hide();
                crate::log_info!("tray: close intercepted → hide to tray");
            }
            // "ask" / "close" / unknown → let it close. The frontend's
            // custom close button will still show the ask dialog when
            // applicable.
            _ => {}
        }
    }
}