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
}

const ACTION_MAX_RECORDS: usize = 20_000;
const ACTION_ELEMENT_MAX: usize = 500;
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

fn push_action_record(kind: &str, x: f64, y: f64, element: &str) {
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

/// 启动动作实时流服务（仅 debug 构建；tauri dev + 动作监视开启时调用）。
/// 监听 127.0.0.1:24159，接受连接后先发送头部与快照，再持续推送新动作。
#[cfg(debug_assertions)]
pub fn ensure_action_live_server() {
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
                "2PYR-AMR/1 app=2-Pyramid version={}\n",
                env!("CARGO_PKG_VERSION")
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
            LIVE_WRITERS.lock().unwrap().push(s);
        }
    });
}

#[tauri::command]
pub fn set_action_monitor(enabled: bool) -> bool {
    ACTION_MONITOR.store(enabled, std::sync::atomic::Ordering::Relaxed);
    crate::log_info!("OKAY set_action_monitor [enabled={}]", enabled);
    // dev 下开启监视时同步启动实时流服务（幂等）
    #[cfg(debug_assertions)]
    if enabled {
        ensure_action_live_server();
    }
    ACTION_MONITOR.load(std::sync::atomic::Ordering::Relaxed)
}

#[tauri::command]
pub fn is_action_monitor() -> bool {
    ACTION_MONITOR.load(std::sync::atomic::Ordering::Relaxed)
}

#[tauri::command]
pub fn log_action(element: String, x: f64, y: f64) {
    crate::log_info!("[ACTION] ({:.0}, {:.0}) {}", x, y, element);
    push_action_record("click", x, y, &element);
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
