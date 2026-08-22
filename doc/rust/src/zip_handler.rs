//! ZIP文件处理模块
//! 
//! 专门处理ZIP文件的读取、写入和解压缩操作

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use zip::{ZipArchive, ZipWriter, CompressionMethod};
use zip::write::FileOptions;

/// ZIP文件处理结果
#[derive(Debug, Clone)]
pub struct ZipOperationResult {
    pub success: bool,
    pub files_extracted: usize,
    pub files_created: usize,
    pub error_message: Option<String>,
}

/// ZIP文件处理器
pub struct ZipHandler {
    /// 处理的文件数量统计
    processed_files: usize,
    /// 创建的文件数量统计
    created_files: usize,
}

impl ZipHandler {
    /// 创建新的ZIP处理器实例
    pub fn new() -> Self {
        Self {
            processed_files: 0,
            created_files: 0,
        }
    }

    /// 解压ZIP文件到指定目录
    pub fn extract_zip_file(&mut self, zip_path: &Path, extract_dir: &Path) -> ZipOperationResult {
        let mut result = ZipOperationResult {
            success: false,
            files_extracted: 0,
            files_created: 0,
            error_message: None,
        };

        match self._extract_zip(zip_path, extract_dir, &mut result) {
            Ok(_) => {
                result.success = true;
                log_message(&format!("✅ ZIP文件解压完成: {} 个文件", result.files_extracted), LogLevel::Info);
            }
            Err(e) => {
                result.error_message = Some(format!("解压失败: {}", e));
                log_message(&result.error_message.as_ref().unwrap(), LogLevel::Error);
            }
        }

        result
    }

    /// 创建ZIP文件
    pub fn create_zip_from_directory(&mut self, source_dir: &Path, output_path: &Path) -> ZipOperationResult {
        let mut result = ZipOperationResult {
            success: false,
            files_extracted: 0,
            files_created: 0,
            error_message: None,
        };

        match self._create_zip(source_dir, output_path, &mut result) {
            Ok(_) => {
                result.success = true;
                log_message(&format!("✅ ZIP文件创建完成: {} 个文件", result.files_created), LogLevel::Info);
            }
            Err(e) => {
                result.error_message = Some(format!("创建ZIP失败: {}", e));
                log_message(&result.error_message.as_ref().unwrap(), LogLevel::Error);
            }
        }

        result
    }

    /// 获取处理统计信息
    pub fn get_stats(&self) -> (usize, usize) {
        (self.processed_files, self.created_files)
    }

    /// 私有方法：执行ZIP解压
    fn _extract_zip(&mut self, zip_path: &Path, extract_dir: &Path, result: &mut ZipOperationResult) -> io::Result<()> {
        log_message(&format!("正在解压文件: {:?}", zip_path), LogLevel::Info);

        let file = File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;

        // 确保解压目录存在
        fs::create_dir_all(extract_dir)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_path = extract_dir.join(file.name());
            
            if file.is_dir() {
                fs::create_dir_all(&file_path)?;
            } else {
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                
                let mut out_file = File::create(&file_path)?;
                io::copy(&mut file, &mut out_file)?;
                
                result.files_extracted += 1;
            }
            self.processed_files += 1;
        }

        Ok(())
    }

    /// 私有方法：创建ZIP文件
    fn _create_zip(&mut self, source_dir: &Path, output_path: &Path, result: &mut ZipOperationResult) -> io::Result<()> {
        log_message(&format!("正在创建ZIP文件: {:?}", output_path), LogLevel::Info);

        let output_file = File::create(output_path)?;
        let mut zip_writer = ZipWriter::new(output_file);

        // 使用walkdir遍历目录
        for entry in walkdir::WalkDir::new(source_dir) {
            let entry = entry.map_err(|e| io::Error::new(io::ErrorKind::Other, format!("WalkDir error: {}", e)))?;
            let path = entry.path();
            
            if path.is_file() {
                let relative_path = path.strip_prefix(source_dir).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Strip prefix error: {}", e)))?;
                let file_name = relative_path.to_string_lossy().to_string();
                
                let mut file = File::open(path)?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                
                let file_options = FileOptions::default();
                zip_writer.start_file(&file_name, file_options)?;
                zip_writer.write_all(&buffer)?;
                
                result.files_created += 1;
                self.created_files += 1;
            }
        }

        zip_writer.finish()?;
        Ok(())
    }

    /// 检查ZIP文件是否损坏
    pub fn validate_zip_file(&self, zip_path: &Path) -> bool {
        match File::open(zip_path) {
            Ok(file) => {
                match ZipArchive::new(file) {
                    Ok(_) => {
                        log_message(&format!("✅ ZIP文件验证通过: {:?}", zip_path), LogLevel::Info);
                        true
                    }
                    Err(e) => {
                        log_message(&format!("❌ ZIP文件损坏: {:?}", e), LogLevel::Error);
                        false
                    }
                }
            }
            Err(e) => {
                log_message(&format!("❌ 无法打开ZIP文件: {}", e), LogLevel::Error);
                false
            }
        }
    }

    /// 获取ZIP文件中的文件列表
    pub fn list_zip_contents(&self, zip_path: &Path) -> Vec<String> {
        match File::open(zip_path) {
            Ok(file) => {
                match ZipArchive::new(file) {
                    Ok(mut archive) => {
                        let mut files = Vec::new();
                        for i in 0..archive.len() {
                            if let Ok(file) = archive.by_index(i) {
                                files.push(file.name().to_string());
                            }
                        }
                        log_message(&format!("ZIP文件包含 {} 个条目", files.len()), LogLevel::Info);
                        files
                    }
                    Err(_) => {
                        log_message("无法读取ZIP文件内容", LogLevel::Error);
                        Vec::new()
                    }
                }
            }
            Err(_) => {
                log_message("无法打开ZIP文件", LogLevel::Error);
                Vec::new()
            }
        }
    }

    /// 提取特定文件从ZIP
    pub fn extract_specific_file(&self, zip_path: &Path, file_name: &str, output_path: &Path) -> bool {
        match File::open(zip_path) {
            Ok(file) => {
                match ZipArchive::new(file) {
                    Ok(mut archive) => {
                        for i in 0..archive.len() {
                            let mut file = archive.by_index(i).unwrap();
                            if file.name() == file_name {
                                // 确保输出目录存在
                                if let Some(parent) = output_path.parent() {
                                    fs::create_dir_all(parent).unwrap_or(());
                                }
                                
                                let mut out_file = File::create(output_path).unwrap_or(File::open(output_path).unwrap());
                                match io::copy(&mut file, &mut out_file) {
                                    Ok(_) => {
                                        log_message(&format!("✅ 提取文件成功: {}", file_name), LogLevel::Info);
                                        return true;
                                    }
                                    Err(e) => {
                                        log_message(&format!("❌ 提取文件失败: {}", e), LogLevel::Error);
                                        return false;
                                    }
                                }
                            }
                        }
                        log_message(&format!("❌ 文件未找到: {}", file_name), LogLevel::Error);
                        false
                    }
                    Err(e) => {
                        log_message(&format!("❌ 无法读取ZIP文件: {}", e), LogLevel::Error);
                        false
                    }
                }
            }
            Err(e) => {
                log_message(&format!("❌ 无法打开ZIP文件: {}", e), LogLevel::Error);
                false
            }
        }
    }
}

impl Default for ZipHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 日志记录函数
fn log_message(message: &str, level: LogLevel) {
    match level {
        LogLevel::Info => println!("[INFO] {}", message),
        LogLevel::Warning => println!("[WARNING] {}", message),
        LogLevel::Error => println!("[ERROR] {}", message),
    }
}

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};
    use std::fs::File;
    use std::io::Write;

    fn create_test_zip(temp_dir: &TempDir) -> PathBuf {
        let zip_path = temp_dir.path().join("test.zip");
        let source_dir = temp_dir.path().join("source");
        
        // 创建源文件
        fs::create_dir_all(&source_dir).unwrap();
        let mut file = File::create(source_dir.join("test.txt")).unwrap();
        file.write_all(b"test content").unwrap();
        
        // 创建ZIP文件
        let mut handler = ZipHandler::new();
        handler.create_zip_from_directory(&source_dir, &zip_path);
        
        zip_path
    }

    #[test]
    fn test_extract_zip() {
        let temp = tempdir().unwrap();
        let zip_path = create_test_zip(&temp);
        let extract_dir = temp.path().join("extracted");
        
        let mut handler = ZipHandler::new();
        let result = handler.extract_zip_file(&zip_path, &extract_dir);
        
        assert!(result.success);
        assert!(extract_dir.join("test.txt").exists());
    }

    #[test]
    fn test_validate_zip() {
        let temp = tempdir().unwrap();
        let zip_path = create_test_zip(&temp);
        
        let handler = ZipHandler::new();
        assert!(handler.validate_zip_file(&zip_path));
    }

    #[test]
    fn test_list_contents() {
        let temp = tempdir().unwrap();
        let zip_path = create_test_zip(&temp);
        
        let handler = ZipHandler::new();
        let contents = handler.list_zip_contents(&zip_path);
        
        assert!(!contents.is_empty());
        assert!(contents.iter().any(|name| name.contains("test.txt")));
    }
}