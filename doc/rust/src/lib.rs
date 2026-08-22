//! BedrockConverter库主模块
//!
//! 提供Java版材质包转换为基岩版格式的核心功能

pub mod ffi;
pub mod file_reorganizer;
pub mod utils;
pub mod manifest_generator;
pub mod file_analyzer;
pub mod zip_handler;
pub mod uuid_generator;

use crate::file_reorganizer::FileReorganizer;
use crate::utils::Utils;
use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::time::Instant;
use serde_json;
use zip::ZipArchive;
use uuid::Uuid;
use crate::ffi::ConversionResultFFI;

// ZIP文件处理
use zip::ZipWriter;

/// 转换结果结构体
pub struct ConversionResult {
    pub success: bool,
    pub input_file: PathBuf,
    pub output_file: Option<PathBuf>,
    pub error_message: Option<String>,
    pub conversion_time_ms: u64,
    pub warnings: Vec<String>,
}

/// Bedrock转换器主结构体
pub struct BedrockConverter {
    temp_dir: PathBuf,
    output_dir: PathBuf,
    file_reorganizer: FileReorganizer,
}

impl BedrockConverter {
    /// 创建新的转换器实例
    pub fn new(temp_dir: PathBuf, output_dir: PathBuf) -> Result<Self, String> {
        // 创建临时目录
        if let Err(e) = fs::create_dir_all(&temp_dir) {
            return Err(format!("创建临时目录失败: {}", e));
        }
        
        // 创建输出目录
        if let Err(e) = fs::create_dir_all(&output_dir) {
            return Err(format!("创建输出目录失败: {}", e));
        }
        
        Ok(BedrockConverter {
            temp_dir,
            output_dir,
            file_reorganizer: FileReorganizer::new(),
        })
    }
    
    /// 将Java版材质包转换为基岩版格式
    pub fn convert_pack(&self, java_pack_path: &Path) -> ConversionResult {
        let start_time = Instant::now();
        let mut warnings = Vec::new();
        
        // 验证输入文件
        if !java_pack_path.exists() {
            return ConversionResult {
                success: false,
                input_file: java_pack_path.to_path_buf(),
                output_file: None,
                error_message: Some("输入文件不存在".to_string()),
                conversion_time_ms: 0,
                warnings,
            };
        }
        
        if let Err(e) = self.analyze_java_pack(java_pack_path, &mut warnings) {
            return ConversionResult {
                success: false,
                input_file: java_pack_path.to_path_buf(),
                output_file: None,
                error_message: Some(format!("分析Java版材质包失败: {}", e)),
                conversion_time_ms: start_time.elapsed().as_millis() as u64,
                warnings,
            };
        }
        
        // 创建临时目录
        let temp_subdir = self.temp_dir.join("conversion_").join(&Uuid::new_v4().to_string());
        if let Err(e) = fs::create_dir_all(&temp_subdir) {
            return ConversionResult {
                success: false,
                input_file: java_pack_path.to_path_buf(),
                output_file: None,
                error_message: Some(format!("创建临时目录失败: {}", e)),
                conversion_time_ms: start_time.elapsed().as_millis() as u64,
                warnings,
            };
        }
        
        // 解压Java版材质包
        if let Err(e) = self.extract_java_pack(java_pack_path, &temp_subdir) {
            let _ = fs::remove_dir_all(&temp_subdir);
            return ConversionResult {
                success: false,
                input_file: java_pack_path.to_path_buf(),
                output_file: None,
                error_message: Some(format!("解压Java版材质包失败: {}", e)),
                conversion_time_ms: start_time.elapsed().as_millis() as u64,
                warnings,
            };
        }
        
        // 第6步：文件结构重组 - 这里包含了Java UI处理逻辑
        if let Err(e) = self.reorganize_java_to_bedrock_structure(&temp_subdir, &mut warnings) {
            let _ = fs::remove_dir_all(&temp_subdir);
            return ConversionResult {
                success: false,
                input_file: java_pack_path.to_path_buf(),
                output_file: None,
                error_message: Some(format!("文件结构重组失败: {}", e)),
                conversion_time_ms: start_time.elapsed().as_millis() as u64,
                warnings,
            };
        }
        
        // 第7步：生成manifest.json
        if let Err(e) = self.generate_manifest_json(&temp_subdir, java_pack_path) {
            let _ = fs::remove_dir_all(&temp_subdir);
            return ConversionResult {
                success: false,
                input_file: java_pack_path.to_path_buf(),
                output_file: None,
                error_message: Some(format!("生成manifest.json失败: {}", e)),
                conversion_time_ms: start_time.elapsed().as_millis() as u64,
                warnings,
            };
        }
        
        // 第8步：重新打包为基岩版格式
        let output_filename = format!("{}_bedrock.mcpack", 
            java_pack_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("converted_pack"));
        let output_path = self.output_dir.join(output_filename);
        
        if let Err(e) = self.repack_as_bedrock(&temp_subdir, &output_path) {
            let _ = fs::remove_dir_all(&temp_subdir);
            return ConversionResult {
                success: false,
                input_file: java_pack_path.to_path_buf(),
                output_file: Some(output_path),
                error_message: Some(format!("重新打包失败: {}", e)),
                conversion_time_ms: start_time.elapsed().as_millis() as u64,
                warnings,
            };
        }
        
        // 第9步：清理临时目录
        let _ = fs::remove_dir_all(&temp_subdir);
        
        ConversionResult {
            success: true,
            input_file: java_pack_path.to_path_buf(),
            output_file: Some(output_path),
            error_message: None,
            conversion_time_ms: start_time.elapsed().as_millis() as u64,
            warnings,
        }
    }
    
    /// 批量转换多个材质包
    pub fn convert_multiple_packs(&self, java_pack_paths: &[&Path]) -> Vec<ConversionResult> {
        java_pack_paths.iter().map(|path| self.convert_pack(path)).collect()
    }
    
    /// 获取转换器统计信息
    pub fn get_converter_stats(&self) -> ConversionStats {
        // 简化实现 - 实际可以基于历史记录统计
        ConversionStats {
            total_files: 0,
            total_size_bytes: 0,
            directories_count: 0,
            largest_file: None,
            largest_file_size: 0,
        }
    }
    
    /// 分析Java版材质包
    fn analyze_java_pack(&self, java_pack_path: &Path, warnings: &mut Vec<String>) -> Result<(), String> {
        let file = fs::File::open(java_pack_path)
            .map_err(|e| format!("打开文件失败: {}", e))?;
        
        let mut archive = ZipArchive::new(file)
            .map_err(|e| format!("读取ZIP文件失败: {}", e))?;
        
        let mut has_pack_mcmeta = false;
        let mut has_textures = false;
        
        for i in 0..archive.len() {
            let entry = archive.by_index(i).map_err(|e| format!("读取ZIP条目失败: {}", e))?;
            let name = entry.name();
            
            if name.contains("pack.mcmeta") {
                has_pack_mcmeta = true;
            }
            if name.starts_with("assets/minecraft/textures/") {
                has_textures = true;
            }
        }
        
        if !has_pack_mcmeta {
            warnings.push("警告: 未找到pack.mcmeta文件，转换可能不完整".to_string());
        }
        
        if !has_textures {
            warnings.push("警告: 未找到textures目录，这可能不是标准的Java版材质包".to_string());
        }
        
        Ok(())
    }
    
    /// 解压Java版材质包
    fn extract_java_pack(&self, java_pack_path: &Path, temp_dir: &Path) -> Result<(), String> {
        let file = fs::File::open(java_pack_path)
            .map_err(|e| format!("打开文件失败: {}", e))?;
        
        let mut archive = ZipArchive::new(file)
            .map_err(|e| format!("读取ZIP文件失败: {}", e))?;
        
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| format!("读取ZIP条目失败: {}", e))?;
            let entry_name = entry.name();
            let target_path = temp_dir.join(entry_name);
            
            if entry.is_dir() {
                fs::create_dir_all(&target_path)
                    .map_err(|e| format!("创建目录失败: {}", e))?;
            } else {
                // 创建父目录
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("创建父目录失败: {}", e))?;
                }
                
                let mut file = fs::File::create(&target_path)
                    .map_err(|e| format!("创建文件失败: {}", e))?;
                
                io::copy(&mut entry, &mut file)
                    .map_err(|e| format!("复制文件失败: {}", e))?;
            }
        }
        
        Ok(())
    }
    
    /// 重组Java文件结构为基岩版结构（包含Java UI处理）
    fn reorganize_java_to_bedrock_structure(&self, temp_dir: &Path, warnings: &mut Vec<String>) -> Result<(), String> {
        // 第1步：Java UI处理 - 必须先执行，因为它需要检查minecraft目录
        if let Err(e) = self.file_reorganizer.reorganize_java_to_bedrock_structure(temp_dir, warnings) {
            return Err(format!("Java UI处理失败: {}", e));
        }
        
        // 第2步：重命名textures目录
        self.rename_textures_directory(temp_dir, warnings)?;
        
        // 第3步：提升目录层级
        self.move_assets_content_to_root(temp_dir, warnings)?;
        
        // 第4步：合并相关的材质包文件
        self.merge_pack_files(temp_dir, warnings)?;
        
        // 第5步：清理空目录
        if let Err(e) = self.cleanup_empty_directories(temp_dir) {
            return Err(format!("清理空目录失败: {}", e));
        }
        
        Ok(())
    }
    
    /// 提升assets内容到根目录
    fn move_assets_content_to_root(&self, temp_dir: &Path, warnings: &mut Vec<String>) -> Result<(), String> {
        let assets_dir = temp_dir.join("assets");
        if !assets_dir.exists() {
            warnings.push("警告: 未找到assets目录".to_string());
            return Ok(());
        }
        
        // 移动minecraft目录内容到temp_dir根目录
        let minecraft_dir = assets_dir.join("minecraft");
        if minecraft_dir.exists() {
            self.move_directory_contents(&minecraft_dir, temp_dir, warnings)?;
            // 删除空的assets目录
            let _ = fs::remove_dir_all(&assets_dir);
        }
        
        Ok(())
    }
    
    /// 移动目录内容
    fn move_directory_contents(&self, src: &Path, dst: &Path, warnings: &mut Vec<String>) -> Result<(), String> {
        if !src.exists() {
            return Ok(());
        }
        
        let entries = match fs::read_dir(src) {
            Ok(entries) => entries,
            Err(e) => return Err(format!("读取目录失败: {}", e)),
        };
            
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    warnings.push(format!("警告: 读取目录条目失败: {}", e));
                    continue;
                },
            };
            let src_path = entry.path();
            let file_name = match src_path.file_name() {
                Some(name) => name,
                None => {
                    warnings.push("警告: 无效文件名".to_string());
                    continue;
                },
            };
            let dst_path = dst.join(file_name);
            
            if src_path.is_dir() {
                if dst_path.exists() {
                    // 目录已存在，递归合并
                    if let Err(e) = self.move_directory_contents(&src_path, &dst_path, warnings) {
                        warnings.push(format!("警告: 递归移动目录失败: {}", e));
                    }
                    let _ = fs::remove_dir_all(&src_path);
                } else {
                    if let Err(e) = fs::rename(&src_path, &dst_path) {
                        return Err(format!("移动目录失败: {}", e));
                    }
                }
            } else {
                // 文件已存在则跳过或替换
                if dst_path.exists() {
                    warnings.push(format!("警告: 文件 {} 已存在，跳过", file_name.to_string_lossy()));
                    let _ = fs::remove_file(&src_path);
                } else {
                    if let Err(e) = fs::rename(&src_path, &dst_path) {
                        return Err(format!("移动文件失败: {}", e));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// 重命名textures目录
    fn rename_textures_directory(&self, temp_dir: &Path, warnings: &mut Vec<String>) -> Result<(), String> {
        let old_textures = temp_dir.join("assets").join("minecraft").join("textures");
        let new_textures = temp_dir.join("textures");
        
        if old_textures.exists() {
            if new_textures.exists() {
                warnings.push("警告: textures目录已存在，尝试合并".to_string());
                self.move_directory_contents(&old_textures, &new_textures, warnings)?;
                let _ = fs::remove_dir_all(&old_textures);
            } else {
                fs::rename(&old_textures, &new_textures)
                    .map_err(|e| format!("重命名textures目录失败: {}", e))?;
            }
            
            // 不再直接删除assets目录结构，而是交给cleanup_empty_directories方法处理
            // 这样可以保留assets/minecraft下的其他重要内容（如sounds、models等）
        }
        
        Ok(())
    }
    
    /// 合并pack相关文件
    fn merge_pack_files(&self, temp_dir: &Path, warnings: &mut Vec<String>) -> Result<(), String> {
        // 查找所有pack相关文件
        let pack_mcmeta = temp_dir.join("pack.mcmeta");
        if !pack_mcmeta.exists() {
            warnings.push("警告: 未找到pack.mcmeta文件，将创建默认版本".to_string());
            // 创建默认pack.mcmeta
            let default_content = r#"{"pack":{"pack_format":3,"description":"Converted from Java Edition"}}"#;
            fs::write(&pack_mcmeta, default_content)
                .map_err(|e| format!("创建pack.mcmeta失败: {}", e))?;
        }
        
        Ok(())
    }
    
    /// 清理空目录
    fn cleanup_empty_directories(&self, temp_dir: &Path) -> Result<(), String> {
        self.remove_empty_directories(temp_dir)?;
        Ok(())
    }
    
    /// 递归删除空目录
    fn remove_empty_directories(&self, dir: &Path) -> Result<(), String> {
        if !dir.exists() || !dir.is_dir() {
            return Ok(());
        }
        
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => return Err(format!("读取目录失败: {}", e)),
        };
            
        let mut is_empty = true;
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    // 继续处理其他条目，但记录错误
                    continue;
                },
            };
            let path = entry.path();
            
            if path.is_dir() {
                if let Err(e) = self.remove_empty_directories(&path) {
                    // 记录错误但继续处理
                }
                if path.exists() {
                    is_empty = false;
                }
            } else {
                is_empty = false;
            }
        }
        
        if is_empty {
            let _ = fs::remove_dir(dir);
        }
        
        Ok(())
    }
    
    /// 生成manifest.json
    fn generate_manifest_json(&self, temp_dir: &Path, java_pack_path: &Path) -> Result<(), String> {
        // 读取pack.mcmeta获取版本信息
        let pack_mcmeta_path = temp_dir.join("pack.mcmeta");
        let pack_format = if pack_mcmeta_path.exists() {
            match fs::read_to_string(&pack_mcmeta_path) {
                Ok(content) => {
                    // 简化解析，提取pack_format
                    if content.contains("\"pack_format\":4") || content.contains("'pack_format':4") {
                        4
                    } else if content.contains("\"pack_format\":3") || content.contains("'pack_format':3") {
                        3
                    } else {
                        4 // 默认使用基岩版支持的最新格式
                    }
                }
                Err(_) => 4,
            }
        } else {
            4
        };
        
        // 生成基岩版manifest.json
        let manifest = serde_json::json!({
            "format_version": 2,
            "header": {
                "description": format!("基岩版转换自 {}", java_pack_path.file_name().unwrap_or_default().to_string_lossy()),
                "name": "基岩版材质包",
                "uuid": Uuid::new_v4().to_string(),
                "version": [1, 0, 0],
                "min_engine_version": [1, 2, 0]
            },
            "modules": [
                {
                    "description": "基岩版材质包模块",
                    "type": "resources",
                    "uuid": Uuid::new_v4().to_string(),
                    "version": [1, 0, 0]
                }
            ]
        });
        
        let manifest_path = temp_dir.join("manifest.json");
        let manifest_content = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("生成manifest.json失败: {}", e))?;
            
        fs::write(&manifest_path, manifest_content)
            .map_err(|e| format!("写入manifest.json失败: {}", e))?;
            
        Ok(())
    }
    
    /// 重新打包为基岩版格式
    fn repack_as_bedrock(&self, temp_dir: &Path, output_path: &Path) -> Result<(), String> {
        let output_file = fs::File::create(output_path)
            .map_err(|e| format!("创建输出文件失败: {}", e))?;
            
        let mut zip_writer = ZipWriter::new(output_file);
        
        self.add_directory_to_zip(&mut zip_writer, temp_dir, "", temp_dir, "")?;
        zip_writer.finish()
            .map_err(|e| format!("写入ZIP文件失败: {}", e))?;
            
        Ok(())
    }
    
    /// 添加目录到ZIP文件
    fn add_directory_to_zip(&self, zip_writer: &mut ZipWriter<fs::File>, base_dir: &Path, current_path: &str, dir: &Path, zip_path: &str) -> Result<(), String> {
        let entries = fs::read_dir(dir)
            .map_err(|e| format!("读取目录失败: {}", e))?;
            
        for entry in entries {
            let entry = entry.map_err(|e| format!("读取目录条目失败: {}", e))?;
            let path = entry.path();
            let file_name = path.file_name()
                .ok_or("无效文件名")?;
                
            let relative_path = if current_path.is_empty() {
                file_name.to_string_lossy().to_string()
            } else {
                format!("{}/{}", current_path, file_name.to_string_lossy())
            };
            
            if path.is_dir() {
                self.add_directory_to_zip(zip_writer, base_dir, &relative_path, &path, &relative_path)?;
            } else {
                let file = fs::File::open(&path)
                    .map_err(|e| format!("打开文件失败: {}", e))?;
                
                let options = zip::write::FileOptions::default();
                zip_writer.start_file(&relative_path, options)
                    .map_err(|e| format!("创建ZIP条目失败: {}", e))?;
                
                let mut buf_reader = BufReader::new(file);
                io::copy(&mut buf_reader, zip_writer)
                    .map_err(|e| format!("复制到ZIP文件失败: {}", e))?;
            }
        }
        
        Ok(())
    }
}

/// 转换器统计信息
pub struct ConversionStats {
    pub total_files: u32,
    pub total_size_bytes: u64,
    pub directories_count: u32,
    pub largest_file: Option<PathBuf>,
    pub largest_file_size: u64,
}

/// 便利函数
pub fn convert_java_to_bedrock<P: AsRef<Path>>(java_pack_path: P, temp_dir: P, output_dir: P) -> ConversionResult {
    let converter = BedrockConverter::new(
        temp_dir.as_ref().to_path_buf(),
        output_dir.as_ref().to_path_buf()
    ).expect("创建转换器失败");
    
    converter.convert_pack(java_pack_path.as_ref())
}

/// 批量转换便利函数
pub fn convert_multiple_java_to_bedrock<P: AsRef<Path>>(java_pack_paths: &[P], temp_dir: P, output_dir: P) -> Vec<ConversionResult> {
    let converter = BedrockConverter::new(
        temp_dir.as_ref().to_path_buf(),
        output_dir.as_ref().to_path_buf()
    ).expect("创建转换器失败");
    
    let path_refs: Vec<&Path> = java_pack_paths.iter().map(|p| p.as_ref()).collect();
    converter.convert_multiple_packs(&path_refs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_converter_creation() {
        let temp_dir = tempdir().unwrap();
        let output_dir = tempdir().unwrap();
        
        let converter = BedrockConverter::new(
            temp_dir.path().to_path_buf(),
            output_dir.path().to_path_buf()
        );
        
        assert!(converter.is_ok());
    }
    
    #[test]
    fn test_convenience_functions() {
        let temp_dir = tempdir().unwrap();
        let output_dir = tempdir().unwrap();
        
        // 测试便利函数
        let result = convert_java_to_bedrock(
            "nonexistent.zip",
            temp_dir.path(),
            output_dir.path()
        );
        
        assert!(!result.success);
        assert!(result.error_message.is_some());
    }
}

// 添加 FFI 函数的重新导出（如果需要）
pub use crate::ffi::{
    create_empty_conversion_result,
    free_conversion_result,
    convert_java_to_bedrock_ffi,
    convert_multiple_packs_ffi,
    get_converter_stats_ffi,
    free_string,
    init_logging_ffi,
};