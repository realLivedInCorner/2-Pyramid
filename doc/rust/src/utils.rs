//! Utils Module
//! 
//! 包含各种工具函数和共享功能的模块

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 日志级别
#[derive(Debug, Clone)]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
}

/// 转换结果
#[derive(Debug, Clone)]
pub struct ConversionResult {
    pub success: bool,
    pub input_file: PathBuf,
    pub output_file: Option<PathBuf>,
    pub error_message: Option<String>,
    pub conversion_time_ms: u64,
    pub warnings: Vec<String>,
}

/// 文件统计信息
#[derive(Debug, Clone)]
pub struct FileStats {
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub directories_count: usize,
    pub largest_file: Option<PathBuf>,
    pub largest_file_size: u64,
}

/// 进度信息
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    pub current: usize,
    pub total: usize,
    pub percentage: f32,
    pub current_file: Option<String>,
    pub stage: String,
}

/// 通用工具函数
pub struct Utils;

impl Utils {
    /// 初始化日志系统
    pub fn init_logging() {
        env_logger::init();
    }

    /// 记录日志消息
    pub fn log_message(message: &str, level: LogLevel) {
        let timestamp = Self::get_current_timestamp();
        let level_str = match level {
            LogLevel::Error => "ERROR",
            LogLevel::Warning => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
        };
        
        println!("[{}] {}: {}", timestamp, level_str, message);
        
        // 可以选择将日志写入文件
        Self::write_log_to_file(&format!("[{}] {}: {}", timestamp, level_str, message));
    }

    /// 获取当前时间戳字符串
    fn get_current_timestamp() -> String {
        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH).unwrap();
        let millis = duration.as_millis();
        format!("{}.{:03}", millis / 1000, millis % 1000)
    }

    /// 写入日志到文件
    fn write_log_to_file(log_line: &str) {
        if let Ok(mut file) = File::options().append(true).create(true).open("kpt_conversion.log") {
            let _ = writeln!(file, "{}", log_line);
        }
    }

    /// 创建临时目录
    pub fn create_temp_dir(prefix: &str) -> Result<PathBuf, io::Error> {
        let temp_dir = std::env::temp_dir().join(format!("kpt_{}_{}", prefix, std::process::id()));
        fs::create_dir_all(&temp_dir)?;
        Ok(temp_dir)
    }

    /// 清理临时目录
    pub fn cleanup_temp_dir(temp_dir: &Path) -> Result<(), io::Error> {
        if temp_dir.exists() {
            fs::remove_dir_all(temp_dir)?;
        }
        Ok(())
    }

    /// 计算文件或目录的大小
    pub fn calculate_directory_size(dir_path: &Path) -> io::Result<u64> {
        let mut total_size = 0u64;
        
        if dir_path.is_file() {
            return Ok(dir_path.metadata()?.len());
        }
        
        if dir_path.is_dir() {
            for entry in fs::read_dir(dir_path)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_dir() {
                    total_size += Self::calculate_directory_size(&path)?;
                } else {
                    total_size += path.metadata()?.len();
                }
            }
        }
        
        Ok(total_size)
    }

    /// 获取文件统计信息
    pub fn get_file_stats(path: &Path) -> io::Result<FileStats> {
        let mut stats = FileStats {
            total_files: 0,
            total_size_bytes: 0,
            directories_count: 0,
            largest_file: None,
            largest_file_size: 0,
        };
        
        Self::_collect_file_stats(path, &mut stats)?;
        Ok(stats)
    }

    /// 递归收集文件统计信息
    fn _collect_file_stats(path: &Path, stats: &mut FileStats) -> io::Result<()> {
        if path.is_file() {
            stats.total_files += 1;
            let file_size = path.metadata()?.len();
            stats.total_size_bytes += file_size;
            
            if file_size > stats.largest_file_size {
                stats.largest_file_size = file_size;
                stats.largest_file = Some(path.to_path_buf());
            }
        } else if path.is_dir() {
            stats.directories_count += 1;
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                Self::_collect_file_stats(&entry.path(), stats)?;
            }
        }
        
        Ok(())
    }

    /// 格式化文件大小
    pub fn format_file_size(size_bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = size_bytes as f64;
        let mut unit_index = 0;
        
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        
        if unit_index == 0 {
            format!("{} {}", size_bytes, UNITS[unit_index])
        } else {
            format!("{:.2} {}", size, UNITS[unit_index])
        }
    }

    /// 验证文件是否为ZIP格式
    pub fn is_valid_zip_file(file_path: &Path) -> bool {
        if !file_path.exists() || !file_path.is_file() {
            return false;
        }
        
        if let Ok(file) = File::open(file_path) {
            if let Ok(mut reader) = zip::ZipArchive::new(file) {
                return reader.len() > 0; // 如果能正常打开ZIP且不为空
            }
        }
        
        false
    }

    /// 安全的文件名（移除非法字符）
    pub fn sanitize_filename(filename: &str) -> String {
        let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];
        let mut sanitized = filename.to_string();
        
        for ch in invalid_chars {
            sanitized = sanitized.replace(ch, "_");
        }
        
        // 移除控制字符
        sanitized = sanitized.chars().filter(|c| !c.is_control()).collect();
        
        // 限制长度
        if sanitized.len() > 255 {
            let extension_pos = sanitized.rfind('.').unwrap_or(sanitized.len());
            let base_len = if extension_pos > 200 { 200 } else { extension_pos };
            sanitized = sanitized.chars().take(base_len).collect();
        }
        
        sanitized
    }

    /// 创建进度信息
    pub fn create_progress_info(current: usize, total: usize, stage: &str, current_file: Option<&str>) -> ProgressInfo {
        let percentage = if total > 0 {
            (current as f32 / total as f32) * 100.0
        } else {
            0.0
        };
        
        ProgressInfo {
            current,
            total,
            percentage,
            current_file: current_file.map(|s| s.to_string()),
            stage: stage.to_string(),
        }
    }

    /// 格式化进度信息为字符串
    pub fn format_progress_info(progress: &ProgressInfo) -> String {
        let mut info = format!("[{}] {}: {}/{} ({:.1}%)", 
                             progress.stage,
                             progress.current_file.as_deref().unwrap_or(""),
                             progress.current,
                             progress.total,
                             progress.percentage);
        
        info
    }

    /// 睡眠指定毫秒数
    pub fn sleep_ms(milliseconds: u64) {
        std::thread::sleep(std::time::Duration::from_millis(milliseconds));
    }

    /// 生成安全的输出文件名
    pub fn generate_safe_output_filename(input_path: &Path, prefix: &str, extension: &str) -> PathBuf {
        let stem = input_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
            
        let sanitized_stem = Self::sanitize_filename(stem);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        let filename = format!("{}_{}_{}", prefix, sanitized_stem, timestamp);
        let final_filename = if extension.is_empty() {
            filename
        } else {
            format!("{}.{}", filename, extension)
        };
        
        std::env::temp_dir().join(final_filename)
    }
}

/// 错误处理宏
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::utils::Utils::log_message(&format!($($arg)*), LogLevel::Error)
    };
}

/// 警告处理宏
#[macro_export]
macro_rules! log_warning {
    ($($arg:tt)*) => {
        $crate::utils::Utils::log_message(&format!($($arg)*), LogLevel::Warning)
    };
}

/// 信息处理宏
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::utils::Utils::log_message(&format!($($arg)*), LogLevel::Info)
    };
}

/// 调试处理宏
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::utils::Utils::log_message(&format!($($arg)*), LogLevel::Debug)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_file_size_formatting() {
        assert_eq!(Utils::format_file_size(1024), "1.00 KB");
        assert_eq!(Utils::format_file_size(1048576), "1.00 MB");
        assert_eq!(Utils::format_file_size(512), "512 B");
    }

    #[test]
    fn test_filename_sanitization() {
        assert_eq!(Utils::sanitize_filename("test<>file?.txt"), "test__file_.txt");
        assert_eq!(Utils::sanitize_filename("normal_file.png"), "normal_file.png");
    }

    #[test]
    fn test_progress_info_creation() {
        let progress = Utils::create_progress_info(50, 100, "Converting", Some("test.zip"));
        assert_eq!(progress.current, 50);
        assert_eq!(progress.total, 100);
        assert_eq!(progress.percentage, 50.0);
        assert_eq!(progress.stage, "Converting");
    }

    #[test]
    fn test_temp_dir_creation() {
        let temp_dir = Utils::create_temp_dir("test").unwrap();
        assert!(temp_dir.exists());
        assert!(temp_dir.to_string_lossy().contains("kpt_test"));
        
        Utils::cleanup_temp_dir(&temp_dir).unwrap();
        assert!(!temp_dir.exists());
    }
}