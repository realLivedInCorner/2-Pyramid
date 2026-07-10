use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Mutex, atomic::{AtomicBool, Ordering}};

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
    let dir = dirs::data_local_dir()?.join("2-Pyramid").join("logs");
    let _ = fs::create_dir_all(&dir);
    let today = chrono::Local::now().format("%Y-%m-%d");
    Some(dir.join(format!("2_pyramid_{}.log", today)))
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
    pub fn export_logs(&self, dest: &str) -> Result<String, String> {
        let path = PathBuf::from(dest);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        let buffer = self.buffer.lock().map_err(|e| format!("Lock error: {}", e))?;
        let content = buffer.join("\n");
        fs::write(&path, content)
            .map_err(|e| format!("Failed to write log file: {}", e))?;
        Ok(path.to_string_lossy().to_string())
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
