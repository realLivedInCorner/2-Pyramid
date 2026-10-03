use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Mutex, atomic::{AtomicBool, Ordering}};

// ── 日志规范（2026-08 约定）───────────────────────────────────────
//
// 1. 转换 / 打包流水线（converters、hurray、invoke_conversion、
//    overlay_package）保持详细日志：进度、模块完成、批量统计。
// 2. 其他简单操作成功后只输出一条：
//        log_info!("OKAY <operation> [<关键参数>]")
//    时间戳由日志框架统一前缀（即「OKAY + 时间」）。
// 3. 错误 / 警告永远保留完整信息 —— 排查问题全靠它们。
// 4. 生命周期事件（启动、退出、工厂重置、单实例）保留简短 INFO。

/// Log level — controls filtering and display priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info  => "INFO ",
            LogLevel::Warn  => "WARN ",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Returns the log file path: `<local_data>/2-Pyramid/logs/2_pyramid_YYYY-MM-DD.log`
fn log_file_path() -> Option<PathBuf> {
    let dir = logs_dir()?;
    let today = chrono::Local::now().format("%Y-%m-%d");
    Some(dir.join(format!("2_pyramid_{}.log", today)))
}

/// 日志目录：`<local_data>/2-Pyramid/logs`（不存在时创建）。
fn logs_dir() -> Option<PathBuf> {
    let dir = dirs::data_local_dir()?.join("2-Pyramid").join("logs");
    let _ = fs::create_dir_all(&dir);
    Some(dir)
}

/// Human-readable timestamp: `2026-05-30 14:23:45.123`
fn timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string()
}

/// The core logger. Thread-safe, writes to file via BufWriter + in-memory buffer.
///
/// Uses two separate mutexes to minimize contention:
/// - `writer`: BufWriter for file I/O (batched writes)
/// - `buffer`: in-memory log buffer for frontend retrieval
pub struct Logger {
    writer: Mutex<Option<BufWriter<File>>>,
    buffer: Mutex<Vec<String>>,
    dev_mode: AtomicBool,
}

impl Logger {
    pub fn new() -> Self {
        let writer = log_file_path().and_then(|path| {
            OpenOptions::new().create(true).append(true).open(path).ok().map(BufWriter::new)
        });
        Self {
            writer: Mutex::new(writer),
            buffer: Mutex::new(Vec::new()),
            dev_mode: AtomicBool::new(false),
        }
    }

    /// Determine the minimum log level based on environment.
    fn effective_level(&self) -> LogLevel {
        if cfg!(debug_assertions) {
            LogLevel::Debug
        } else if self.dev_mode.load(Ordering::Relaxed) {
            LogLevel::Info
        } else {
            LogLevel::Warn
        }
    }

    /// Core log method — filters by effective level, then writes everywhere.
    pub fn log(&self, level: LogLevel, message: &str) {
        if level < self.effective_level() {
            return;
        }

        let entry = format!("[{}] [{}] {}", timestamp(), level.as_str(), message);

        // Console: debug builds show everything; release shows warn+
        #[cfg(debug_assertions)]
        {
            if level >= LogLevel::Info {
                eprintln!("{}", entry);
            }
        }
        #[cfg(not(debug_assertions))]
        {
            if level >= LogLevel::Warn {
                eprintln!("{}", entry);
            }
        }

        // File — BufWriter batches writes internally, only flushes when full
        if let Ok(mut w) = self.writer.lock() {
            if let Some(ref mut writer) = *w {
                let _ = writeln!(writer, "{}", entry);
            }
        }

        // In-memory buffer (capped at 2000)
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.push(entry);
            let len = buffer.len();
            if len > 2000 {
                buffer.drain(0..(len - 2000));
            }
        }
    }

    pub fn debug(&self, message: &str) { self.log(LogLevel::Debug, message); }
    pub fn info(&self, message: &str)  { self.log(LogLevel::Info,  message); }

    /// 把 BufWriter 里尚未落盘的日志刷出去。
    ///
    /// 短命进程（CLI）用 `std::process::exit` 退出时不会执行析构函数，
    /// 不显式 flush 会丢掉日志尾部（表现为"日志停在打包中途"）。
    pub fn flush(&self) {
        if let Ok(mut w) = self.writer.lock() {
            if let Some(ref mut writer) = *w {
                let _ = writer.flush();
            }
        }
    }
    pub fn warn(&self, message: &str)  { self.log(LogLevel::Warn,  message); }
    pub fn error(&self, message: &str) { self.log(LogLevel::Error, message); }

    pub fn get_logs(&self) -> Vec<String> {
        self.buffer.lock().map(|b| b.clone()).unwrap_or_default()
    }

    pub fn clear_logs(&self) {
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.clear();
        }
    }

    pub fn set_dev_mode(&self, enabled: bool) {
        self.dev_mode.store(enabled, Ordering::Relaxed);
        if enabled {
            self.info("Developer mode enabled — verbose logging active");
        } else {
            self.info("Developer mode disabled — minimal logging active");
        }
    }

    pub fn is_dev_mode(&self) -> bool {
        self.dev_mode.load(Ordering::Relaxed)
    }

    /// Export the current in-memory buffer to a user-specified file path.
    ///
    /// `redact = true` 时先做隐私脱敏（仅作用于导出文件，磁盘日志与界面
    /// 保持原文）。返回 `(路径, 脱敏处数)`。
    pub fn export_logs(&self, dest: &str, redact: bool) -> Result<(String, usize), String> {
        let path = PathBuf::from(dest);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        let buffer = self.buffer.lock().map_err(|e| format!("Lock error: {}", e))?;
        let content = buffer.join("\n");
        let (content, count) = if redact {
            let host = machine_name();
            redact_text(&content, host.as_deref())
        } else {
            (content, 0)
        };
        fs::write(&path, content)
            .map_err(|e| format!("Failed to write log file: {}", e))?;
        Ok((path.to_string_lossy().to_string(), count))
    }

    /// 导出**磁盘上的全部日志文件**（logs 目录下所有 `.log`，按文件名排序
    /// 拼接，带文件名分隔头）。与内存导出一样支持脱敏。
    /// 返回 `(路径, 脱敏处数, 文件数)`。
    pub fn export_disk_logs(&self, dest: &str, redact: bool) -> Result<(String, usize, usize), String> {
        let dir = logs_dir().ok_or_else(|| "logs directory not found".to_string())?;
        let mut files: Vec<PathBuf> = fs::read_dir(&dir)
            .map_err(|e| format!("Failed to read logs dir: {}", e))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().map(|x| x.eq_ignore_ascii_case("log")).unwrap_or(false))
            .collect();
        files.sort();

        let mut merged = String::new();
        for f in &files {
            let name = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            merged.push_str(&format!("===== {} =====\n", name));
            match fs::read_to_string(f) {
                Ok(text) => {
                    merged.push_str(&text);
                    if !text.ends_with('\n') {
                        merged.push('\n');
                    }
                }
                Err(e) => merged.push_str(&format!("<read failed: {}>\n", e)),
            }
            merged.push('\n');
        }

        let (content, count) = if redact {
            let host = machine_name();
            redact_text(&merged, host.as_deref())
        } else {
            (merged, 0)
        };

        let path = PathBuf::from(dest);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        fs::write(&path, content).map_err(|e| format!("Failed to write log file: {}", e))?;
        Ok((path.to_string_lossy().to_string(), count, files.len()))
    }

    /// Return the path to today's log file.
    pub fn log_file_path_str(&self) -> Option<String> {
        log_file_path().map(|p| p.to_string_lossy().to_string())
    }
}

// ── Global singleton ──────────────────────────────────────

lazy_static::lazy_static! {
    pub static ref GLOBAL_LOGGER: Logger = Logger::new();
}

// ── 导出日志脱敏 ──────────────────────────────────────────
//
// 仅在「导出日志」这一步生效：磁盘日志与界面查看保持原文，便于本地排查，
// 而用户真正分享出去的导出文件不含可识别信息。
//
// 脱敏对象（按用户确认的范围）：
//   1. Windows 用户目录名       C:\Users\张三\...        → C:\Users\<user>\...
//   2. 机器名 / 主机名           DESKTOP-ABC              → <host>
//   3. IP 地址（v4 / v6）        203.0.113.7 / fe80::1    → <ip>（保留 127.0.0.1、::1）
//   4. 邮箱地址                  someone@example.com      → <email>
//   5. API Key / token           sk-…、Authorization: Bearer …、api_key=…  → <token>/<key>
//   6. 资源包绝对路径的目录部分   D:\私密项目\包.zip        → <path>\包.zip
//      （应用自身目录 ~/.2pyr、%LOCALAPPDATA%\2-Pyramid 保留，便于排查；
//        分享码 2PYR-… 按用户要求**不**脱敏）

/// 当前机器名（Windows 用 COMPUTERNAME；其他平台回退 HOSTNAME）。
fn machine_name() -> Option<String> {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() >= 3)
}

lazy_static::lazy_static! {
    static ref RE_USER_DIR: regex::Regex =
        regex::Regex::new(r#"(?i)([A-Za-z]:[\\/]{1,2}Users[\\/])([^\\/\s"'<>|:]+)"#).unwrap();
    static ref RE_EMAIL: regex::Regex =
        regex::Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}").unwrap();
    static ref RE_BEARER: regex::Regex =
        regex::Regex::new(r"(?i)(authorization\s*:\s*bearer\s+)([A-Za-z0-9._\-]{6,})").unwrap();
    static ref RE_APIKEY_KV: regex::Regex =
        regex::Regex::new(r#"(?i)((?:api[_-]?key|token|secret|password)"?\s*[:=]\s*"?)([A-Za-z0-9._\-]{6,})"#).unwrap();
    static ref RE_SK: regex::Regex =
        regex::Regex::new(r"\bsk-[A-Za-z0-9_\-]{10,}").unwrap();
    static ref RE_IPV4: regex::Regex =
        regex::Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap();
    static ref RE_IPV6: regex::Regex =
        regex::Regex::new(r"\b(?:[0-9a-fA-F]{1,4}:){2,7}:[0-9a-fA-F]{0,4}\b").unwrap();
    static ref RE_WINPATH: regex::Regex =
        // 允许路径里出现 <user>/<host> 这类已插入的占位符（不把 <> 当分隔符）
        regex::Regex::new(r#"(?i)[A-Za-z]:[\\/]{1,2}[^\s"'|*?]*"#).unwrap();
    static ref RE_UNCPATH: regex::Regex =
        regex::Regex::new(r#"\\\\[^\s"'<>|]+"#).unwrap();
}

/// 应用自身目录（保留原样，便于排查；其中用户名的脱敏已在第 1 步完成）。
fn is_app_own_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("\\2-pyramid\\")
        || lower.contains("\\2-pyramid-beta\\")
        || lower.contains("\\.2pyr\\")
        || lower.contains("/2-pyramid/")
        || lower.contains("/.2pyr/")
}

/// 把绝对路径压成 `<path>\文件名`（保留最后一段文件名）。
fn shorten_path(path: &str) -> String {
    let trimmed = path.trim_end_matches(['\\', '/']);
    let last = trimmed
        .rsplit(['\\', '/'])
        .find(|s| !s.is_empty())
        .unwrap_or(trimmed);
    let sep = if path.contains('/') && !path.contains('\\') { "/" } else { "\\" };
    format!("<path>{}{}", sep, last)
}

/// 单行/IPv4 过滤：本地回环与通配地址不脱敏。
fn keep_ip(ip: &str) -> bool {
    matches!(ip, "127.0.0.1" | "0.0.0.0" | "::1")
}

/// 对文本做隐私脱敏，返回 `(脱敏后文本, 替换处数)`。
pub fn redact_text(input: &str, host: Option<&str>) -> (String, usize) {
    let mut count = 0usize;

    let mut replace_counted = |re: &regex::Regex, text: &str, rep: &str, count: &mut usize| -> String {
        let mut hits = 0usize;
        let out = re.replace_all(text, |caps: &regex::Captures| {
            hits += 1;
            if caps.len() > 2 {
                format!("{}{}", &caps[1], rep)
            } else {
                rep.to_string()
            }
        });
        *count += hits;
        out.into_owned()
    };

    let mut out = input.to_string();

    // 5. 凭据类先处理（避免后续路径规则搅乱 key 内容）
    out = replace_counted(&RE_BEARER, &out, "<token>", &mut count);
    out = replace_counted(&RE_APIKEY_KV, &out, "<key>", &mut count);
    out = replace_counted(&RE_SK, &out, "<api-key>", &mut count);

    // 4. 邮箱
    out = replace_counted(&RE_EMAIL, &out, "<email>", &mut count);

    // 1. Windows 用户目录名
    out = replace_counted(&RE_USER_DIR, &out, "<user>", &mut count);

    // 2. 机器名（只在确实出现时替换）
    if let Some(h) = host {
        if !h.is_empty() {
            let re = regex::RegexBuilder::new(&regex::escape(h))
                .case_insensitive(true)
                .build()
                .ok();
            if let Some(re) = re {
                out = replace_counted(&re, &out, "<host>", &mut count);
            }
        }
    }

    // 3. IP（保留回环/通配）
    {
        let mut hits = 0usize;
        let replaced = RE_IPV4.replace_all(&out, |caps: &regex::Captures| {
            let ip = &caps[0];
            if keep_ip(ip) {
                ip.to_string()
            } else {
                hits += 1;
                "<ip>".to_string()
            }
        });
        out = replaced.into_owned();
        count += hits;

        let mut hits6 = 0usize;
        let replaced6 = RE_IPV6.replace_all(&out, |caps: &regex::Captures| {
            let ip = &caps[0];
            if keep_ip(ip) {
                ip.to_string()
            } else {
                hits6 += 1;
                "<ip>".to_string()
            }
        });
        out = replaced6.into_owned();
        count += hits6;
    }

    // 6. 绝对路径（应用自身目录保留）
    {
        let mut hits = 0usize;
        let replaced = RE_WINPATH.replace_all(&out, |caps: &regex::Captures| {
            let p = &caps[0];
            if is_app_own_path(p) {
                p.to_string()
            } else if p.matches(['\\', '/']).count() >= 2 {
                hits += 1;
                shorten_path(p)
            } else {
                p.to_string()
            }
        });
        out = replaced.into_owned();
        count += hits;

        let mut hitsu = 0usize;
        let replacedu = RE_UNCPATH.replace_all(&out, |caps: &regex::Captures| {
            let p = &caps[0];
            if is_app_own_path(p) {
                p.to_string()
            } else {
                hitsu += 1;
                shorten_path(p)
            }
        });
        out = replacedu.into_owned();
        count += hitsu;
    }

    (out, count)
}

#[cfg(test)]
mod redact_tests {
    use super::*;

    #[test]
    fn redacts_user_dir_email_ip_and_credentials() {
        // 应用自身目录（保持可读）用来验证用户名脱敏；包路径由另一个测试覆盖
        let line = r#"[INFO ] log at C:\Users\张三\AppData\Local\2-Pyramid\logs ok, mail a.b+c@example.com, peer 203.0.113.7, key sk-abcdefghijklmnop, Authorization: Bearer abcdef123456"#;
        let (out, n) = redact_text(line, Some("DESKTOP-TEST01"));
        assert!(out.contains(r"C:\Users\<user>\AppData"), "user dir: {out}");
        assert!(!out.contains("张三"), "user name leaked: {out}");
        assert!(out.contains("127.0.0.1") || !out.contains("<ip>\n"), "loopback handling: {out}");
        assert!(out.contains("<email>") && !out.contains("a.b+c@example.com"), "email: {out}");
        assert!(out.contains("<ip>") && !out.contains("203.0.113.7"), "ip: {out}");
        assert!(out.contains("<api-key>"), "sk key: {out}");
        assert!(out.contains("Bearer <token>"), "bearer: {out}");
        assert!(n >= 4, "expected >=4 redactions, got {n}");
    }

    #[test]
    fn keeps_loopback_and_timestamps() {
        let (out, _) = redact_text("listening on 127.0.0.1:24157 at 2026-10-01 14:23:45.123", None);
        assert!(out.contains("127.0.0.1:24157"), "loopback should stay: {out}");
        assert!(out.contains("14:23:45.123"), "timestamp must stay: {out}");
    }

    #[test]
    fn shortens_pack_paths_but_keeps_app_dirs() {
        let line = r#"from D:\私密项目\我的包.zip to C:\Users\me\AppData\Local\2-Pyramid\logs\x.log"#;
        let (out, _) = redact_text(line, None);
        assert!(out.contains(r"<path>\我的包.zip"), "pack path: {out}");
        assert!(!out.contains("私密项目"), "dir leaked: {out}");
        assert!(out.contains(r"2-Pyramid\logs\x.log"), "app dir should stay: {out}");
    }

    #[test]
    fn redacts_machine_name_only_when_present() {
        let (out, n) = redact_text("host=DESKTOP-TEST01 user=desktop-test01", Some("DESKTOP-TEST01"));
        assert!(!out.contains("DESKTOP-TEST01") && out.contains("<host>"), "host: {out}");
        assert!(out.to_lowercase().contains("<host>"), "case-insensitive host: {out}");
        assert_eq!(n, 2, "both occurrences should be counted");
    }

    #[test]
    fn keeps_timestamps_intact() {
        let (out, _) = redact_text("[2026-10-01 14:23:45.123] [INFO ] started", None);
        assert!(out.contains("14:23:45.123"), "timestamp mangled: {out}");
    }
}

// ── Convenience macros ────────────────────────────────────

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => { $crate::logger::GLOBAL_LOGGER.debug(&format!($($arg)*)) };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => { $crate::logger::GLOBAL_LOGGER.info(&format!($($arg)*)) };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => { $crate::logger::GLOBAL_LOGGER.warn(&format!($($arg)*)) };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => { $crate::logger::GLOBAL_LOGGER.error(&format!($($arg)*)) };
}
