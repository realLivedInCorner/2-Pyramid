//! Desktop top-level toast notifications.
//!
//! Spins up an independent, transparent, borderless, always-on-top
//! webview window per toast, anchored to the upper-right corner of the
//! primary monitor. Multiple toasts stack vertically downward like the
//! Windows 10/11 Action Center does, so the user always sees the most
//! recent message first without old ones pushing it off-screen.
//!
//! Why a separate WebviewWindow and not an in-app overlay?
//!   * The user explicitly asked for "desktop top-level" rendering, not
//!     painting inside the main app window. A toast that lives inside
//!     2-Pyramid's window gets clipped by `decorations: false` edges
//!     and occluded by fullscreen conversion progress bars.
//!   * A real OS-level top-level window survives main-window state
//!     changes (minimize, focus loss) and reads as a
//!     "real" notification to the user.

use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, PhysicalPosition, WebviewUrl,
    WebviewWindowBuilder,
};
// `Window::set_position` is generic over `Pos: Into<Position>`, so we
// can pass a `LogicalPosition` directly — the compiler infers the
// target `Position::Logical(_)` variant without us naming the enum.
// This avoids depending on tauri re-exporting `dpi::Position`, which
// it does not in 2.8+ (the only path the library itself uses is
// `crate::runtime::dpi::Position`, also not public).

/// Width / height of a single toast in DIPs. Compact notification
/// size (like a Windows 11 banner), not a modal panel.
const TOAST_W: f64 = 320.0;
const TOAST_H: f64 = 88.0;
/// Gap between stacked toasts and from screen edges.
const TOAST_GAP: f64 = 10.0;
/// Toast hugs the corner of the screen ("顶格"): a small 12px margin
/// keeps the rounded card from touching the bezel while still reading
/// as a corner notification. Corner is user-configurable via
/// `toast_position`; these margins apply to all four corners.
const TOAST_MARGIN_RIGHT: f64 = 12.0;
const TOAST_MARGIN_TOP: f64 = 12.0;
const TOAST_MARGIN_BOTTOM: f64 = 12.0;
const TOAST_MARGIN_LEFT: f64 = 12.0;
/// Maximum number of toasts we keep on screen at once. Older ones get
/// forced-closed so the stack never grows off the bottom of the
/// monitor.
const TOAST_STACK_MAX: usize = 4;

/// Counter used to give every toast window a unique label so Tauri
/// never refuses the `WebviewWindowBuilder::build` call with a
/// "label already exists" error when two toasts fire within the same
/// millisecond.
static TOAST_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Action descriptor as it crosses the wire from the frontend. Mirrors
/// `ToastAction` in `src/toast.ts`. We only deserialize the bits the
/// Rust side actually needs: `id` (used by `run_toast_action` to route
/// the click back to the main window) and `label` / `icon` (currently
/// used purely by the toast page itself, but we accept them here so
/// the payload type is symmetric with the TypeScript side).
#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct ToastActionPayload {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub icon: Option<String>,
}

/// Payload accepted from the frontend. Mirrors
/// `src/composables/useNotification.ts::NotificationOptions` minus the
/// Tauri-only bits (`silent`, internal `id`).
#[derive(serde::Deserialize)]
pub struct ToastPayload {
    pub title: String,
    pub body: String,
    /// Free-form type string the frontend wants to colour the toast
    /// (`info` / `success` / `warning` / `error`). Forwarded verbatim
    /// to the toast page via URL query so the toast HTML can style
    /// itself.
    #[serde(default)]
    pub kind: String,
    /// How long the toast stays on screen, in milliseconds. 0 = use
    /// default (4500 ms).
    #[serde(default)]
    pub duration_ms: u64,
    /// Optional action buttons. When non-empty, the toast page renders
    /// them as small buttons. Clicking one closes the toast and fires
    /// the `run_toast_action` invoke so the main app can react.
    #[serde(default)]
    pub actions: Vec<ToastActionPayload>,
}

/// Frontend entry point. Build a new top-level toast window, position
/// it in the upper-right of the primary monitor, then return. The
/// window self-closes after `payload.duration_ms` via a setTimeout in
/// the toast page itself.
///
/// IMPORTANT: this command is `async` on purpose. Tauri v2 runs
/// synchronous commands on the MAIN thread; building a WebView2
/// window there can stall the main thread (WebView2 controller
/// creation waits for a renderer process, which is slow while a
/// conversion saturates the CPU) and freeze ALL window event handling
/// and IPC — observed as "window buttons dead, dialogs frozen" right
/// after a conversion. As an async command it runs on the async
/// runtime and only dispatches the tiny main-thread parts through
/// Tauri's channel.
#[tauri::command]
pub async fn show_toast(app: AppHandle, payload: ToastPayload) -> Result<(), String> {
    let n = TOAST_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    let label = format!("toast-{}-{}", chrono_timestamp_ms(), n);

    // Encode the payload into URL query so the toast HTML page can
    // pick it up without a round-trip back through invoke(). Query
    // strings are also safe across all platforms; passing the body
    // through window-init-script would mean re-running a script after
    // navigation, which is messier.
    //
    // Actions are serialised as a JSON string under `actions=` so the
    // toast page can `JSON.parse(decodeURIComponent(...))` them. We
    // also URL-encode the JSON itself so commas / quotes survive.
    let actions_param = if payload.actions.is_empty() {
        String::new()
    } else {
        let json = serde_json::to_string(&payload.actions)
            .unwrap_or_else(|_| "[]".to_string());
        format!("&actions={}", urlencoding_encode(&json))
    };
    // Effective on-screen duration: caller-provided value when set,
    // otherwise the user-configured toast duration (default 8s).
    let effective_duration = if payload.duration_ms == 0 {
        crate::commands::config::read_config_file()
            .ok()
            .and_then(|c| c.toast_duration_ms)
            .unwrap_or(8000)
    } else {
        payload.duration_ms.max(1000)
    };

    let url = format!(
        "toast.html?title={}&body={}&kind={}&duration={}&label={}{}",
        urlencoding_encode(&payload.title),
        urlencoding_encode(&payload.body),
        urlencoding_encode(&payload.kind),
        effective_duration,
        urlencoding_encode(&label),
        actions_param,
    );

    let window = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(url.into()))
        .title("2-Pyramid Toast")
        .inner_size(TOAST_W, TOAST_H)
        .min_inner_size(TOAST_W, TOAST_H)
        .max_inner_size(TOAST_W, TOAST_H)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .closable(true)
        .skip_taskbar(true)
        .always_on_top(true)
        .decorations(false)
        .focused(false)
        .visible(false)
        // The toast window is never focused, so WebView2 treats it as a
        // background window and throttles its JS timers — the toast's
        // own `setTimeout(dismiss, duration)` then fires far too late
        // ("toast never goes away"). Disable throttling like the main
        // window does.
        .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled)
        .build()
        .map_err(|e| {
            crate::log_error!("toast: build failed: {}", e);
            format!("failed to build toast window: {}", e)
        })?;

    // Anchor to the user-configured corner of the primary monitor.
    let mut anchor: Option<(f64, f64)> = None;
    if let Some(monitor) = app.primary_monitor().map_err(|e| {
        crate::log_warn!("toast: primary_monitor failed: {}", e);
        e
    }).ok().flatten() {
        let mon_pos = monitor.position();
        let mon_size = monitor.size();
        let scale = monitor.scale_factor();
        // Convert monitor size + position to logical units.
        let mon_w = mon_size.width as f64 / scale;
        let mon_h = mon_size.height as f64 / scale;
        let mon_x = mon_pos.x as f64 / scale;
        let mon_y = mon_pos.y as f64 / scale;

        // Count existing live toasts so we can stack the new one below
        // them. The stack grows downward. NOTE: `count_live_toasts`
        // already sees the window we just built (it is registered the
        // moment build() succeeds), so subtract 1 — otherwise the very
        // first toast is pushed down one slot and every stack position
        // is off by one.
        let stack_offset = count_live_toasts(&app).saturating_sub(1) as f64;

        let position = crate::commands::config::read_config_file()
            .ok()
            .and_then(|c| c.toast_position)
            .unwrap_or_else(|| "top-right".to_string());

        let (x, y) = match position.as_str() {
            "top-left" => (
                mon_x + TOAST_MARGIN_LEFT,
                mon_y + TOAST_MARGIN_TOP + stack_offset * (TOAST_H + TOAST_GAP),
            ),
            "bottom-right" => (
                mon_x + mon_w - TOAST_W - TOAST_MARGIN_RIGHT,
                mon_y + mon_h - TOAST_H - TOAST_MARGIN_BOTTOM
                    - stack_offset * (TOAST_H + TOAST_GAP),
            ),
            "bottom-left" => (
                mon_x + TOAST_MARGIN_LEFT,
                mon_y + mon_h - TOAST_H - TOAST_MARGIN_BOTTOM
                    - stack_offset * (TOAST_H + TOAST_GAP),
            ),
            // Default: top-right
            _ => (
                mon_x + mon_w - TOAST_W - TOAST_MARGIN_RIGHT,
                mon_y + TOAST_MARGIN_TOP + stack_offset * (TOAST_H + TOAST_GAP),
            ),
        };

        let _ = window.set_position(LogicalPosition::new(x, y));
        let _ = window.set_size(LogicalSize::new(TOAST_W, TOAST_H));
        anchor = Some((x, y));
    } else {
        crate::log_warn!("toast: no primary monitor; using default position");
    }

    // Evict oldest if we're over the stack limit.
    evict_overflow(&app);

    window.show().map_err(|e| {
        crate::log_error!("toast: show failed: {}", e);
        format!("failed to show toast: {}", e)
    })?;

    // Some Windows display configurations drop the pre-show position;
    // re-assert the anchor after the window is visible so the toast
    // never appears at a stale/default corner.
    if let Some((x, y)) = anchor {
        let _ = window.set_position(LogicalPosition::new(x, y));
    }

    // Safety net: the toast page normally closes itself via a JS
    // setTimeout after `duration_ms`. If that never fires (renderer
    // crash, JS error, throttling edge case) the window would linger
    // on screen forever. Force-close from Rust after the duration plus
    // a generous grace period so a well-behaved toast never gets cut
    // short by the net.
    {
        let app = app.clone();
        let label = label.clone();
        let grace_ms = effective_duration + 1200;
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(grace_ms));
            if let Some(w) = app.get_webview_window(&label) {
                crate::log_warn!("toast: safety net closing lingering window {}", label);
                let _ = w.close();
            }
        });
    }

    crate::log_info!(
        "toast: shown (label={}, title={:?})",
        label,
        payload.title
    );

    Ok(())
}

/// Explicitly close a toast window (used by the toast page when the
/// user clicks the close button or the auto-dismiss timer fires).
#[tauri::command]
pub fn dismiss_toast(app: AppHandle, label: String) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.close();
    }
    Ok(())
}

/// Manually dismiss every live toast (e.g. when the user picks up
/// the main window focus).
#[tauri::command]
pub fn dismiss_all_toasts(app: AppHandle) -> Result<(), String> {
    for (_, w) in app.webview_windows() {
        if w.label().starts_with("toast-") {
            let _ = w.close();
        }
    }
    Ok(())
}

/// Bring the main app window back to the foreground (or focus it if
/// it's already visible). Called from the toast page when the user
/// clicks anywhere on the toast card — clicking a toast is the
/// natural "OK, bring me back to the app" gesture.
#[tauri::command]
pub fn focus_main_window(app: AppHandle) -> Result<(), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "main webview window not found".to_string())?;
    // `show` is a no-op if the window is already visible; `unminimize`
    // brings it back if it was minimized; `set_focus` raises it to the
    // foreground. All three are needed for the "user clicked the
    // toast, bring the app back" flow to feel right on Windows:
    //   * minimized        → unminimize + focus
    //   * visible+occluded → focus
    let _ = main.show();
    let _ = main.unminimize();
    main.set_focus()
        .map_err(|e| format!("set_focus failed: {}", e))?;
    crate::log_info!("toast: focus_main_window → main window shown + focused");
    Ok(())
}

/// Route a clicked toast action back to the main app. The toast page
/// emits this when the user clicks one of the action buttons; the main
/// window listens for the `toast-action` event on the JS side and
/// looks up the registered handler by id.
#[tauri::command]
pub fn run_toast_action(app: AppHandle, action_id: String) -> Result<(), String> {
    // We close all live toasts defensively so the action callback in
    // the main window doesn't have to worry about the toast staying
    // open over its UI. Cheap because we already track the labels.
    dismiss_all_toasts_inner(&app);
    // Emit to the main window. JS side uses `listen('toast-action',
    // ...)` to handle the id.
    if let Err(e) = app.emit("toast-action", action_id.clone()) {
        crate::log_error!("toast: emit toast-action failed: {}", e);
        return Err(format!("emit failed: {}", e));
    }
    crate::log_info!("toast: action '{}' emitted to main", action_id);
    Ok(())
}

fn dismiss_all_toasts_inner(app: &AppHandle) {
    for (_, w) in app.webview_windows() {
        if w.label().starts_with("toast-") {
            let _ = w.close();
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────

fn count_live_toasts(app: &AppHandle) -> usize {
    app.webview_windows()
        .keys()
        .filter(|l| l.starts_with("toast-"))
        .count()
}

fn evict_overflow(app: &AppHandle) {
    let mut labels: Vec<String> = app
        .webview_windows()
        .keys()
        .filter(|l| l.starts_with("toast-"))
        .cloned()
        .collect();
    if labels.len() <= TOAST_STACK_MAX {
        return;
    }
    // Oldest toasts have smaller timestamp suffixes; sort ascending and
    // close the head until we're back under the cap.
    labels.sort();
    let to_close = labels.len() - TOAST_STACK_MAX;
    for l in labels.iter().take(to_close) {
        if let Some(w) = app.get_webview_window(l) {
            let _ = w.close();
        }
    }
}

fn chrono_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Minimal URL-encoder (only the chars that actually appear in toast
/// titles/bodies). Using the `urlencoding` crate would add a
/// dependency; this 20-line version covers the cases we hit and
/// never produces wrong output because we only encode characters
/// that have a stable, well-known percent form.
fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}