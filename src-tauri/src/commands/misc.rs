use std::collections::VecDeque;
use std::sync::Mutex;

#[tauri::command]
pub fn get_logs(lines: Option<usize>) -> String {
    use crate::logger::GLOBAL_LOGGER;

    let logs = GLOBAL_LOGGER.get_logs();
    if let Some(line_count) = lines {
        let start_index = logs.len().saturating_sub(line_count);
        logs[start_index..].join("\n")
    } else {
        logs.join("\n")
    }
}

#[tauri::command]
pub fn set_dev_mode(enabled: bool) -> Result<bool, String> {
    use crate::logger::GLOBAL_LOGGER;
    GLOBAL_LOGGER.set_dev_mode(enabled);
    Ok(GLOBAL_LOGGER.is_dev_mode())
}

#[tauri::command]
pub fn get_dev_mode() -> bool {
    use crate::logger::GLOBAL_LOGGER;
    GLOBAL_LOGGER.is_dev_mode()
}

// ── Action monitor（开发者诊断）───────────────────────────────────
//
// 启用后，前端在捕获阶段记录每一次点击（元素描述 + 坐标）并通过
// `log_action` 写入日志，用于排查「按钮点了没反应」之类的问题。
// 两种启用方式：
//   1. 启动参数 `--action-monitor`
//   2. 设置页开发者选项里的「动作监视」开关
//
// 除日志外，动作还进入内存环形缓冲，可导出为 .2amr 文件
// （Action Mon3tr 可解析回放）。debug 构建下另提供
// 127.0.0.1:24159 的本地 TCP 实时流，供 Mon3tr 接管实时动作。
pub static ACTION_MONITOR: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// 一条动作记录（.2amr 帧）。
#[derive(serde::Serialize, Clone)]
pub struct ActionRecord {
    /// 距首条记录的毫秒偏移
    pub t: u64,
    #[serde(rename = "type")]
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub element: String,
    /// 点击发生时所在的 Vue 页面（home / conversion / settings / overlay …），
    /// 供 Mon3tr 复现点击逻辑时的上下文（截图不入 2amr，实时按需抓取）。
    pub page: String,
}

const ACTION_MAX_RECORDS: usize = 20_000;
const ACTION_ELEMENT_MAX: usize = 500;
const ACTION_PAGE_MAX: usize = 100;
const ACTION_LIVE_PORT: u16 = 24159;

static ACTION_RECORDS: Mutex<Option<VecDeque<ActionRecord>>> = Mutex::new(None);
static ACTION_START: Mutex<Option<std::time::Instant>> = Mutex::new(None);
static LIVE_WRITERS: Mutex<Vec<std::net::TcpStream>> = Mutex::new(Vec::new());
static LIVE_STARTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

fn sanitize_element(s: &str) -> String {
    // 记录内容视为不可信文本：剔除控制字符并截断，防止异常内容进入
    // 日志/2amr 后干扰解析或显示。
    let mut out: String = s.chars().filter(|c| !c.is_control()).take(ACTION_ELEMENT_MAX).collect();
    out.truncate(ACTION_ELEMENT_MAX);
    out
}

fn sanitize_page(s: &str) -> String {
    let mut out: String = s.chars().filter(|c| !c.is_control()).take(ACTION_PAGE_MAX).collect();
    out.truncate(ACTION_PAGE_MAX);
    out
}

fn push_action_record(kind: &str, x: f64, y: f64, element: &str, page: &str) {
    let mut recs = ACTION_RECORDS.lock().unwrap();
    let buffer = recs.get_or_insert_with(|| VecDeque::with_capacity(ACTION_MAX_RECORDS));
    let mut start = ACTION_START.lock().unwrap();
    let t0 = *start.get_or_insert_with(std::time::Instant::now);
    let t = t0.elapsed().as_millis() as u64;
    // 非有限坐标一律按 0 处理，防止异常前端数据写入记录
    let x = if x.is_finite() { x } else { 0.0 };
    let y = if y.is_finite() { y } else { 0.0 };
    let record = ActionRecord {
        t,
        kind: kind.to_string(),
        x,
        y,
        element: sanitize_element(element),
        page: sanitize_page(page),
    };
    buffer.push_back(record.clone());
    while buffer.len() > ACTION_MAX_RECORDS {
        buffer.pop_front();
    }

    // 实时流推送给已连接的 Mon3tr（仅 debug 构建有意义）
    #[cfg(debug_assertions)]
    if ACTION_MONITOR.load(std::sync::atomic::Ordering::Relaxed) {
        push_live_record(&record);
    }
}

#[cfg(debug_assertions)]
fn push_live_record(record: &ActionRecord) {
    use std::io::Write;
    let line = serde_json::to_string(record).unwrap_or_default();
    let mut writers = LIVE_WRITERS.lock().unwrap();
    writers.retain_mut(|w| {
        w.write_all(line.as_bytes())
            .and_then(|_| w.write_all(b"\n"))
            .and_then(|_| w.flush())
            .is_ok()
    });
}

/// 启动动作实时流服务（仅 debug 构建；tauri dev 下调用）。
/// 监听 127.0.0.1:24159，接受连接后先发送头部与快照，再持续推送新动作；
/// 同时为每个连接派生读取线程，响应 Mon3tr 的 `SHOT` 请求：
/// 实时抓取主窗口截图（base64 PNG），截图不落盘、不进 2amr。
#[cfg(debug_assertions)]
pub fn ensure_action_live_server(app: tauri::AppHandle) {
    use std::io::Write;
    if LIVE_STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    let listener = match std::net::TcpListener::bind(("127.0.0.1", ACTION_LIVE_PORT)) {
        Ok(l) => l,
        Err(e) => {
            crate::log_info!("action live server bind failed: {}", e);
            LIVE_STARTED.store(false, std::sync::atomic::Ordering::SeqCst);
            return;
        }
    };
    crate::log_info!("OKAY action live server [127.0.0.1:{}]", ACTION_LIVE_PORT);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut s) = stream else { continue };
            // 协议头 + 现有记录快照，然后加入实时推送列表
            let header = format!(
                "2PYR-AMR/1 app=2-Pyramid version={} monitor={}\n",
                env!("CARGO_PKG_VERSION"),
                if ACTION_MONITOR.load(std::sync::atomic::Ordering::Relaxed) { "on" } else { "off" },
            );
            if s.write_all(header.as_bytes()).is_err() {
                continue;
            }
            {
                let recs = ACTION_RECORDS.lock().unwrap();
                if let Some(buffer) = recs.as_ref() {
                    for r in buffer.iter() {
                        let line = serde_json::to_string(r).unwrap_or_default();
                        if s.write_all(line.as_bytes())
                            .and_then(|_| s.write_all(b"\n"))
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
            let _ = s.flush();

            // 读取线程：处理 SHOT 请求（对同一 TCP 连接全双工收发）
            let reader = match s.try_clone() {
                Ok(r) => r,
                Err(_) => {
                    LIVE_WRITERS.lock().unwrap().push(s);
                    continue;
                }
            };
            let shot_app = app.clone();
            std::thread::spawn(move || {
                use std::io::{BufRead, BufReader, Write};
                let mut reader = BufReader::new(reader);
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {}
                    }
                    let req = line.trim();
                    if req.eq_ignore_ascii_case("SHOT") {
                        let resp = match capture_window_shot(&shot_app) {
                            Ok(b64) => format!("SHOT:{}\n", b64),
                            Err(_) => "SHOT:ERR\n".to_string(),
                        };
                        if reader.get_mut().write_all(resp.as_bytes()).is_err()
                            || reader.get_mut().flush().is_err()
                        {
                            break;
                        }
                    }
                }
            });

            LIVE_WRITERS.lock().unwrap().push(s);
        }
    });
}

/// 实时抓取主窗口截图（缩放至宽 800 控制体积），返回 base64 PNG。
/// 仅供 Mon3tr 实时查看使用：不写文件、不存入 2amr。
///
/// 实现：GDI PrintWindow(PW_RENDERFULLCONTENT) 抓取自身窗口（含
/// WebView2 GPU 合成内容），全黑时回退 BitBlt 屏幕拷贝。
#[cfg(debug_assertions)]
fn capture_window_shot(app: &tauri::AppHandle) -> Result<String, String> {
    use base64::Engine;
    use tauri::Manager;

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    let hwnd = window.hwnd().map_err(|e| format!("hwnd failed: {}", e))?;
    let png = capture_hwnd_png(hwnd.0)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&png))
}

#[cfg(debug_assertions)]
fn capture_hwnd_png(hwnd_raw: *mut core::ffi::c_void) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC,
        DeleteObject, GetDC, GetDIBits, GetPixel, ReleaseDC, SelectObject, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, RGBQUAD, SRCCOPY,
    };
    use windows_sys::Win32::Storage::Xps::PrintWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect;

    unsafe {
        let hwnd: HWND = hwnd_raw;
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        if GetClientRect(hwnd, &mut rect) == 0 {
            return Err("GetClientRect failed".to_string());
        }
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        if w <= 0 || h <= 0 {
            return Err("窗口尺寸无效".to_string());
        }

        let screen_dc = GetDC(std::ptr::null_mut());
        if screen_dc.is_null() {
            return Err("GetDC failed".to_string());
        }
        let mem_dc = CreateCompatibleDC(screen_dc);
        let bmp = CreateCompatibleBitmap(screen_dc, w, h);
        if mem_dc.is_null() || bmp.is_null() {
            if !bmp.is_null() {
                DeleteObject(bmp);
            }
            if !mem_dc.is_null() {
                DeleteDC(mem_dc);
            }
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return Err("create dc/bitmap failed".to_string());
        }
        let old = SelectObject(mem_dc, bmp);

        // 首选 PrintWindow(PW_RENDERFULLCONTENT=2)：可拿到 GPU 合成内容
        let printed = PrintWindow(hwnd, mem_dc, 2) != 0;
        let center = if printed { GetPixel(mem_dc, w / 2, h / 2) } else { 0 };
        if !printed || center == 0 {
            // 回退：BitBlt 屏幕拷贝（窗口需可见，帧为无边框窗口即完整内容）
            let mut pt = POINT { x: rect.left, y: rect.top };
            ClientToScreen(hwnd, &mut pt);
            let _ = BitBlt(mem_dc, 0, 0, w, h, screen_dc, pt.x, pt.y, SRCCOPY);
        }

        // 32bpp BGRA 自下而上读出
        let mut buf: Vec<u8> = vec![0u8; (w * h * 4) as usize];
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..std::mem::zeroed()
            },
            bmiColors: [RGBQUAD { rgbBlue: 0, rgbGreen: 0, rgbRed: 0, rgbReserved: 0 }; 1],
        };
        let got = GetDIBits(
            mem_dc,
            bmp,
            0,
            h as u32,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            &mut bmi,
            DIB_RGB_COLORS,
        );
        SelectObject(mem_dc, old);
        DeleteObject(bmp);
        DeleteDC(mem_dc);
        ReleaseDC(std::ptr::null_mut(), screen_dc);
        if got == 0 {
            return Err("GetDIBits failed".to_string());
        }

        // BGRA → RGBA
        let mut rgba: Vec<u8> = Vec::with_capacity(buf.len());
        for px in buf.chunks_exact(4) {
            rgba.push(px[2]);
            rgba.push(px[1]);
            rgba.push(px[0]);
            rgba.push(px[3]);
        }
        let img = image::RgbaImage::from_raw(w as u32, h as u32, rgba)
            .ok_or_else(|| "invalid bitmap buffer".to_string())?;

        // 缩放到宽 800（只在更大时缩放），控制单帧体积与传输延迟
        let target_w = 800u32.min(w as u32);
        let resized = if (w as u32) > target_w {
            let nh = ((h as u32) * target_w / (w as u32)).max(1);
            image::imageops::resize(&img, target_w, nh, image::imageops::FilterType::Triangle)
        } else {
            img
        };

        let mut out: Vec<u8> = Vec::new();
        resized
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .map_err(|e| format!("png encode failed: {}", e))?;
        Ok(out)
    }
}

#[tauri::command]
pub fn set_action_monitor(app: tauri::AppHandle, enabled: bool) -> bool {
    ACTION_MONITOR.store(enabled, std::sync::atomic::Ordering::Relaxed);
    crate::log_info!("OKAY set_action_monitor [enabled={}]", enabled);
    // dev 下开启监视时同步启动实时流服务（幂等）
    #[cfg(debug_assertions)]
    if enabled {
        ensure_action_live_server(app);
    }
    ACTION_MONITOR.load(std::sync::atomic::Ordering::Relaxed)
}

#[tauri::command]
pub fn is_action_monitor() -> bool {
    ACTION_MONITOR.load(std::sync::atomic::Ordering::Relaxed)
}

#[tauri::command]
pub fn log_action(element: String, x: f64, y: f64, page: String) {
    crate::log_info!("[ACTION] ({:.0}, {:.0}) [{}] {}", x, y, page, element);
    push_action_record("click", x, y, &element, &page);
}

/// 导出内存中的动作记录为 .2amr 文件（Action Mon3tr 回放格式）。
/// 仅导出内存快照；内容在写入前经过坐标/文本防护。
#[tauri::command]
pub fn export_action_records(dest: String) -> Result<u32, String> {
    let recs = ACTION_RECORDS.lock().unwrap();
    let buffer = recs
        .as_ref()
        .ok_or_else(|| "动作监视未开启，没有可导出的动作记录".to_string())?;
    if buffer.is_empty() {
        return Err("动作监视未开启，没有可导出的动作记录".to_string());
    }
    let header = serde_json::json!({
        "format": "2amr",
        "version": 1,
        "app": "2-Pyramid",
        "app_version": env!("CARGO_PKG_VERSION"),
        "channel": option_env!("2PYR_CHANNEL").unwrap_or("stable"),
        "recorded_at": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        "window": { "width": 1200, "height": 750 },
        "frames": buffer.iter().collect::<Vec<_>>(),
    });
    let json = serde_json::to_string_pretty(&header)
        .map_err(|e| format!("序列化失败: {}", e))?;
    std::fs::write(&dest, json).map_err(|e| format!("写入失败: {}", e))?;
    let count = buffer.len() as u32;
    crate::log_info!("OKAY export_action_records [frames={} -> {}]", count, dest);
    Ok(count)
}

#[tauri::command]
pub fn log_notification(notification_type: String, title: String, body: String) {
    use crate::logger::GLOBAL_LOGGER;
    GLOBAL_LOGGER.info(&format!("[Notif] [{}] {}: {}", notification_type, title, body));
}

/// Export the in-memory log buffer to a file at `dest` path.
#[tauri::command]
pub fn export_logs(dest: String) -> Result<String, String> {
    use crate::logger::GLOBAL_LOGGER;
    GLOBAL_LOGGER.export_logs(&dest)
}

/// Return the path to today's log file.
#[tauri::command]
pub fn get_log_path() -> Option<String> {
    use crate::logger::GLOBAL_LOGGER;
    GLOBAL_LOGGER.log_file_path_str()
}

#[tauri::command]
pub fn open_folder(path: String) -> Result<(), String> {
    let path = std::path::Path::new(&path);
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        match Command::new("explorer")
            .arg(path)
            .creation_flags(0x08000000)
            .status() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to open folder: {}", e)),
        }
    }

    #[cfg(target_os = "macos")]
    {
        match std::process::Command::new("open").arg(path).status() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to open folder: {}", e)),
        }
    }

    #[cfg(target_os = "linux")]
    {
        match std::process::Command::new("xdg-open").arg(path).status() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to open folder: {}", e)),
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("Opening folders not supported on current OS".to_string())
    }
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    use std::fs;

    let path = std::path::Path::new(&path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn create_dir(path: String) -> Result<(), String> {
    use std::fs;
    let path = std::path::Path::new(&path);
    if path.exists() {
        return Ok(());
    }
    fs::create_dir_all(path).map_err(|e| format!("Failed to create directory: {}", e))?;
    Ok(())
}

/// Recursively delete one or more paths. Used by the conversion
/// page's "post-conversion source pack handling" flow when the user
/// picks `delete` or confirms the `ask` prompt.
///
/// Each path is deleted independently: a failure on one path does not
/// abort the rest, and we collect every per-path outcome so the
/// frontend can show a useful summary. The frontend only treats the
/// operation as failed if *all* paths failed.
#[tauri::command]
pub fn delete_paths(paths: Vec<String>) -> Result<Vec<DeleteResult>, String> {
    use std::fs;

    let mut out = Vec::with_capacity(paths.len());
    for p in &paths {
        let path = std::path::Path::new(p);
        if !path.exists() {
            // Treat "already gone" as success so the user doesn't get
            // a confusing error when a previous batch already removed
            // the file.
            out.push(DeleteResult {
                path: p.clone(),
                ok: true,
                error: None,
            });
            continue;
        }
        let res = if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };
        match res {
            Ok(()) => out.push(DeleteResult {
                path: p.clone(),
                ok: true,
                error: None,
            }),
            Err(e) => out.push(DeleteResult {
                path: p.clone(),
                ok: false,
                error: Some(e.to_string()),
            }),
        }
    }
    Ok(out)
}

#[derive(serde::Serialize)]
pub struct DeleteResult {
    pub path: String,
    pub ok: bool,
    pub error: Option<String>,
}

// (Context-menu commands removed — see src-tauri/src/registry.rs deletion
// note in commit message. The right-click entry-point is gone by design.)

// ── App version / build info ──────────────────────────────────────

/// Aggregate struct returned to the frontend so the UI can show a
/// single canonical `version + build` stamp without having to know
/// about Cargo or BUILD files.
#[derive(serde::Serialize)]
pub struct AppInfo {
    /// Semantic version from `Cargo.toml` (e.g. "2.0.0").
    pub version: String,
    /// Raw build number injected by `build.rs` from the repo-root
    /// `BUILD` file. May be "dev", "dev.5", or a plain integer string.
    pub build: String,
    /// Convenience field: `"{version}+build.{build}"` (or just
    /// `"{version}+{build}"` when build is empty).
    pub full: String,
    /// True when built without `--release`.
    pub is_dev: bool,
    /// 构建渠道：stable（正式版）/ beta（测试版）。
    /// 由发布流水线在编译时经环境变量 2PYR_CHANNEL 注入。
    pub channel: String,
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    let version = env!("CARGO_PKG_VERSION").to_string();
    let build_raw = env!("BUILD_NUMBER").to_string();
    // `build.rs` prefixes debug builds with "dev.", strip it for display.
    let build = build_raw
        .strip_prefix("dev.")
        .map(str::to_string)
        .unwrap_or_else(|| build_raw.clone());

    let full = if build.is_empty() || build == "0" {
        version.clone()
    } else if build == "dev" {
        // No BUILD file present — emit `version+dev` so the marker still shows.
        format!("{}+dev", version)
    } else {
        format!("{}+build.{}", version, build)
    };

    let is_dev = cfg!(debug_assertions);
    const CHANNEL: &str = match option_env!("2PYR_CHANNEL") {
        Some(v) => v,
        None => "stable",
    };

    AppInfo {
        version,
        build,
        full,
        is_dev,
        channel: CHANNEL.to_string(),
    }
}

// ── Process lifecycle ─────────────────────────────────────────────
//
// There is no tray / background-resident mode anymore: closing the
// main window exits the process (see `lib.rs`). These two commands
// are what remains of the old lifecycle surface:
//
//   * `force_quit` — hard exit, used by the frontend's Ctrl+Shift+Q
//     shortcut. `app.exit(0)` tears down webviews + the Tauri runtime
//     cleanly so in-flight commands get a chance to abort via normal
//     destructors.
//   * `ping` — trivial IPC liveness probe, kept as a general
//     diagnostic for the webview↔Rust bridge.

#[tauri::command]
pub fn force_quit(app: tauri::AppHandle) {
    crate::log_info!("force_quit: user requested hard exit (Ctrl+Shift+Q)");
    app.exit(0);
}

/// Trivial liveness probe so the frontend can tell whether the
/// webview↔Rust IPC bridge is still alive.
#[tauri::command]
pub fn ping() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    crate::log_warn!("ping: IPC alive at {}", ms);
    ms
}
