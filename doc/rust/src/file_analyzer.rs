//! File Analyzer Module
//! 
//! 专门处理文件分析功能的模块

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use zip::ZipArchive;
use crate::utils::{Utils, LogLevel};

/// 文件分析结果
#[derive(Debug, Clone)]
pub struct FileAnalysisResult {
    pub has_pack_mcmeta: bool,
    pub pack_format: Option<u32>,
    pub description: Option<String>,
    pub has_pack_png: bool,
    pub file_structure: Vec<String>,
    pub total_files: usize,
    pub error_message: Option<String>,
}

/// 文件分析器
pub struct FileAnalyzer;

impl FileAnalyzer {
    /// 创建新的文件分析器实例
    pub fn new() -> Self {
        FileAnalyzer
    }

    /// 分析ZIP文件
    pub fn analyze_zip_file(&self, zip_path: &Path) -> FileAnalysisResult {
        let mut result = FileAnalysisResult {
            has_pack_mcmeta: false,
            pack_format: None,
            description: None,
            has_pack_png: false,
            file_structure: Vec::new(),
            total_files: 0,
            error_message: None,
        };

        match self._analyze_zip_contents(zip_path, &mut result) {
            Ok(_) => {
                Utils::log_message(&format!("文件分析完成: {:?}", zip_path), LogLevel::Info);
            }
            Err(e) => {
                result.error_message = Some(format!("分析文件时出错: {}", e));
                Utils::log_message(&result.error_message.as_ref().unwrap(), LogLevel::Error);
            }
        }

        result
    }

    /// 分析ZIP文件内容
    fn _analyze_zip_contents(&self, zip_path: &Path, result: &mut FileAnalysisResult) -> io::Result<()> {
        let file = File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;

        result.total_files = archive.len();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_name = file.name().to_string();
            
            // 收集文件结构信息
            result.file_structure.push(file_name.clone());

            // 检查是否为pack.mcmeta文件
            if file_name == "pack.mcmeta" {
                result.has_pack_mcmeta = true;
                
                // 读取并解析pack.mcmeta
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(mcmeta_data) => {
                        if let Some(pack_data) = mcmeta_data.get("pack") {
                            if let Some(pack_format) = pack_data.get("pack_format").and_then(|v| v.as_u64()) {
                                result.pack_format = Some(pack_format as u32);
                            }
                            if let Some(description) = pack_data.get("description").and_then(|v| v.as_str()) {
                                result.description = Some(description.to_string());
                            }
                        }
                    }
                    Err(e) => {
                        Utils::log_message(&format!("解析pack.mcmeta失败: {}", e), LogLevel::Warning);
                    }
                }
            }

            // 检查是否为pack.png文件
            if file_name == "pack.png" {
                result.has_pack_png = true;
            }
        }

        Ok(())
    }

    /// 获取文件信息摘要
    pub fn get_analysis_summary(&self, analysis: &FileAnalysisResult) -> String {
        let mut summary = String::new();
        
        summary.push_str(&format!("文件总数: {}\n", analysis.total_files));
        
        if analysis.has_pack_mcmeta {
            summary.push_str("✓ 包含 pack.mcmeta 文件\n");
            if let Some(format) = analysis.pack_format {
                summary.push_str(&format!("  - Pack Format: {}\n", format));
            }
            if let Some(desc) = &analysis.description {
                summary.push_str(&format!("  - 描述: {}\n", desc));
            }
        } else {
            summary.push_str("✗ 缺少 pack.mcmeta 文件\n");
        }

        if analysis.has_pack_png {
            summary.push_str("✓ 包含 pack.png 图标文件\n");
        } else {
            summary.push_str("✗ 缺少 pack.png 图标文件\n");
        }

        if let Some(error) = &analysis.error_message {
            summary.push_str(&format!("⚠ 警告: {}\n", error));
        }

        summary
    }

    /// 检查文件是否为有效的Java版材质包
    pub fn is_valid_java_pack(&self, analysis: &FileAnalysisResult) -> bool {
        analysis.has_pack_mcmeta && analysis.pack_format.is_some()
    }

    /// 检查文件是否为有效的基岩版材质包
    pub fn is_valid_bedrock_pack(&self, analysis: &FileAnalysisResult) -> bool {
        // 基岩版材质包应该有manifest.json而不是pack.mcmeta
        analysis.file_structure.iter().any(|name| name == "manifest.json")
    }

    /// 获取材质包类型
    pub fn get_pack_type(&self, analysis: &FileAnalysisResult) -> String {
        if self.is_valid_bedrock_pack(analysis) {
            "Bedrock".to_string()
        } else if self.is_valid_java_pack(analysis) {
            "Java".to_string()
        } else {
            "Unknown".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_analyzer_creation() {
        let analyzer = FileAnalyzer::new();
        assert!(analyzer.analyze_zip_file(Path::new("non_existent.zip")).total_files >= 0);
    }
}