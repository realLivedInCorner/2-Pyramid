use std::collections::VecDeque;
use std::path::Path;
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

// ── Action monitor（开发者诊断 / Action Mon3tr）─────────────────
//
// 启用后，前端在捕获阶段记录交互事件并写入内存环形缓冲：
//   * click / input / keydown / change / page
// 两种启用方式：
//   1. 启动参数 `--action-monitor` 或环境变量 `2PYR_ACTION_MONITOR=1`
//   2. 设置页开发者选项里的「动作监视」开关
//   3. 实时流协议 `MON ON`（供 Mon3tr 远程打开）
//
// 导出 .2amr 供 Mon3tr 逐帧回放；debug 构建提供 127.0.0.1:24159
// 实时流，协议见 `ensure_action_live_server`。
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
    /// 可选补充说明：输入内容摘要、按键名、切换开关值等。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

const ACTION_MAX_RECORDS: usize = 20_000;
const ACTION_ELEMENT_MAX: usize = 500;
const ACTION_PAGE_MAX: usize = 100;
const ACTION_DETAIL_MAX: usize = 200;
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

fn sanitize_detail(s: &str) -> String {
    let mut out: String = s.chars().filter(|c| !c.is_control()).take(ACTION_DETAIL_MAX).collect();
    out.truncate(ACTION_DETAIL_MAX);
    out
}

fn push_action_record(kind: &str, x: f64, y: f64, element: &str, page: &str, detail: Option<&str>) {
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
        detail: detail.map(sanitize_detail).filter(|s| !s.is_empty()),
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

            // 读取线程：处理 Mon3tr 控制指令（对同一 TCP 连接全双工收发）
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
                    let raw = line.trim();
                    if raw.is_empty() {
                        continue;
                    }
                    let upper = raw.to_ascii_uppercase();
                    let resp: String = if upper == "SHOT" {
                        match capture_window_shot(&shot_app) {
                            Ok(b64) => format!("SHOT:{}\n", b64),
                            Err(e) => format!("SHOT:ERR {}\n", e),
                        }
                    } else if upper == "STATUS" {
                        format!("STATUS monitor={} frames={}\n",
                            if ACTION_MONITOR.load(std::sync::atomic::Ordering::Relaxed) { "on" } else { "off" },
                            action_frame_count(),
                        )
                    } else if upper == "CLEAR" {
                        clear_action_records_inner();
                        "CLEAR:OK\n".to_string()
                    } else if upper == "MON ON" {
                        ACTION_MONITOR.store(true, std::sync::atomic::Ordering::Relaxed);
                        "MON:ON\n".to_string()
                    } else if upper == "MON OFF" {
                        ACTION_MONITOR.store(false, std::sync::atomic::Ordering::Relaxed);
                        "MON:OFF\n".to_string()
                    } else if let Some(rest) = upper.strip_prefix("CLICK ") {
                        // 回放驱动：在主窗口客户区坐标合成一次点击，让真实 UI 跟着变
                        let mut parts = rest.split_whitespace();
                        let x: f64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(f64::NAN);
                        let y: f64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(f64::NAN);
                        if !x.is_finite() || !y.is_finite() {
                            "CLICK:ERR bad-coords\n".to_string()
                        } else {
                            match inject_client_click(&shot_app, x, y) {
                                Ok(()) => "CLICK:OK\n".to_string(),
                                Err(e) => format!("CLICK:ERR {}\n", e),
                            }
                        }
                    } else {
                        format!("ERR unknown {}\n", raw)
                    };
                    if reader.get_mut().write_all(resp.as_bytes()).is_err()
                        || reader.get_mut().flush().is_err()
                    {
                        break;
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
/// WebView2 GPU 合成内容）；失败或明显异常时回退 BitBlt 屏幕拷贝。
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

/// 在主窗口客户区坐标 (x, y) 合成一次左键点击。
/// Mon3tr 回放 2amr 时调用，让真实 2-Pyramid UI 跟着切换。
#[cfg(debug_assertions)]
fn inject_client_click(app: &tauri::AppHandle, x: f64, y: f64) -> Result<(), String> {
    use tauri::Manager;
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetCursorPos, SetForegroundWindow};

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    let hwnd = window.hwnd().map_err(|e| format!("hwnd failed: {}", e))?;
    // 尽量前置，保证点击落在目标窗口
    let _ = window.set_focus();
    unsafe {
        SetForegroundWindow(hwnd.0);
        let mut pt = POINT {
            x: x.round() as i32,
            y: y.round() as i32,
        };
        if ClientToScreen(hwnd.0, &mut pt) == 0 {
            return Err("ClientToScreen failed".into());
        }
        let _ = SetCursorPos(pt.x, pt.y);
        // 短暂等待焦点/光标稳定
        std::thread::sleep(std::time::Duration::from_millis(30));

        let mk_input = |flags: u32| INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let mut inputs = [mk_input(MOUSEEVENTF_LEFTDOWN), mk_input(MOUSEEVENTF_LEFTUP)];
        let sent = SendInput(
            inputs.len() as u32,
            inputs.as_mut_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        );
        if sent != inputs.len() as u32 {
            return Err("SendInput failed".into());
        }
    }
    Ok(())
}

#[cfg(debug_assertions)]
fn capture_hwnd_png(hwnd_raw: *mut core::ffi::c_void) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC,
        DeleteObject, GetDC, GetDIBits, ReleaseDC, SelectObject, BITMAPINFO,
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

        // PrintWindow(PW_RENDERFULLCONTENT=2)：优先拿 GPU 合成内容
        let printed = PrintWindow(hwnd, mem_dc, 2) != 0;
        // 不再用「中心像素 == 0」判断失败（深色 UI 也会是 0）。
        // PrintWindow 成功就采用；失败再 BitBlt 屏幕拷贝。
        if !printed {
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

        // 缩放到宽 720（只在更大时缩放），控制单帧体积与传输延迟
        let target_w = 720u32.min(w as u32);
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

fn action_frame_count() -> usize {
    ACTION_RECORDS
        .lock()
        .map(|r| r.as_ref().map(|b| b.len()).unwrap_or(0))
        .unwrap_or(0)
}

fn clear_action_records_inner() {
    let mut recs = ACTION_RECORDS.lock().unwrap();
    if let Some(buffer) = recs.as_mut() {
        buffer.clear();
    }
    // 重置时间轴，导出回放时 t 从 0 重新计
    *ACTION_START.lock().unwrap() = None;
}

/// 清空内存中的动作记录（Mon3tr / 设置页共用）。
#[tauri::command]
pub fn clear_action_records() -> u32 {
    let before = action_frame_count();
    clear_action_records_inner();
    crate::log_info!("OKAY clear_action_records [dropped={}]", before);
    before as u32
}

/// 当前监视状态与帧数（设置页 / Mon3tr STATUS）。
#[tauri::command]
pub fn action_monitor_status() -> serde_json::Value {
    serde_json::json!({
        "enabled": ACTION_MONITOR.load(std::sync::atomic::Ordering::Relaxed),
        "frames": action_frame_count(),
        "maxFrames": ACTION_MAX_RECORDS,
        "livePort": ACTION_LIVE_PORT,
    })
}

/// 记录一次交互。`kind`：click / input / keydown / change / page。
#[tauri::command]
pub fn log_action(
    element: String,
    x: f64,
    y: f64,
    page: String,
    kind: Option<String>,
    detail: Option<String>,
) {
    let k = kind.as_deref().unwrap_or("click");
    let detail_ref = detail.as_deref();
    let detail_log = detail_ref.unwrap_or("");
    crate::log_info!(
        "[ACTION] {} ({:.0}, {:.0}) [{}] {}{}",
        k,
        x,
        y,
        page,
        element,
        if detail_log.is_empty() {
            String::new()
        } else {
            format!(" | {}", detail_log)
        }
    );
    push_action_record(k, x, y, &element, &page, detail_ref);
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

    // 尽量写入真实视口尺寸，便于 Mon3tr 按比例回放坐标
    let window = try_main_window_size()
        .map(|(w, h)| serde_json::json!({ "width": w, "height": h }))
        .unwrap_or_else(|| serde_json::json!({ "width": 1200, "height": 750 }));

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
        "window": window,
        "frames": buffer.iter().collect::<Vec<_>>(),
    });
    let json = serde_json::to_string_pretty(&header)
        .map_err(|e| format!("序列化失败: {}", e))?;
    std::fs::write(&dest, json).map_err(|e| format!("写入失败: {}", e))?;
    let count = buffer.len() as u32;
    crate::log_info!("OKAY export_action_records [frames={} -> {}]", count, dest);
    Ok(count)
}

/// 前端上报的最近视口尺寸（window.innerWidth/Height），导出 2amr 时写入。
static ACTION_VIEWPORT: Mutex<Option<(u32, u32)>> = Mutex::new(None);

fn try_main_window_size() -> Option<(u32, u32)> {
    *ACTION_VIEWPORT.lock().ok()?
}

/// 前端上报当前视口尺寸。
#[tauri::command]
pub fn set_action_viewport(width: f64, height: f64) {
    let w = width.round().clamp(1.0, 20000.0) as u32;
    let h = height.round().clamp(1.0, 20000.0) as u32;
    if let Ok(mut vp) = ACTION_VIEWPORT.lock() {
        *vp = Some((w, h));
    }
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

/// 主程序所在目录（安装后为安装目录；开发时为 target/debug 等）。
#[tauri::command]
pub fn get_install_dir() -> Option<String> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_string_lossy().to_string()))
}

/// 读取安装目录 legal/ 下的法律文本（开发时回落到仓库根 legal/）。
#[tauri::command]
pub fn read_legal_file(filename: String) -> Result<String, String> {
    let name = Path::new(&filename)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if name.is_empty() || name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err("invalid filename".into());
    }
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Some(dir) = get_install_dir() {
        candidates.push(std::path::PathBuf::from(dir).join("legal").join(name));
    }
    candidates.push(std::path::PathBuf::from("legal").join(name));
    for p in candidates {
        if p.is_file() {
            return std::fs::read_to_string(&p).map_err(|e| format!("read {}: {}", p.display(), e));
        }
    }
    Err(format!("legal file not found: {name}"))
}

#[tauri::command]
pub fn open_folder(path: String) -> Result<(), String> {
    let path = std::path::Path::new(&path);
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    // Store 包不拉起 explorer/open/xdg-open（WACK 已阻止的可执行文件）。
    #[cfg(feature = "store")]
    {
        Err(format!(
            "商店版不支持打开文件夹，请手动访问: {}",
            path.display()
        ))
    }

    #[cfg(not(feature = "store"))]
    {
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
