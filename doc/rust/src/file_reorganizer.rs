//! File Reorganizer Module
//! 
//! 专门处理文件结构重组的模块

use std::fs;
use std::io;
use std::path::Path;
use crate::utils::{Utils, LogLevel};

/// 文件重组操作结果
#[derive(Debug, Clone)]
pub struct ReorganizationResult {
    pub success: bool,
    pub renamed_files: Vec<String>,
    pub moved_directories: Vec<String>,
    pub removed_empty_dirs: Vec<String>,
    pub error_message: Option<String>,
}

/// 文件结构重组器
pub struct FileReorganizer;

impl FileReorganizer {
    /// 创建新的文件重组器实例
    pub fn new() -> Self {
        FileReorganizer
    }

    /// 重组Java版材质包为基岩版格式的文件结构
    pub fn reorganize_java_to_bedrock_structure(&self, temp_dir: &Path, warnings: &mut Vec<String>) -> Result<(), String> {
        // 检查minecraft目录是否存在
        let minecraft_dir = temp_dir.join("assets").join("minecraft");
        if !minecraft_dir.exists() {
            let error_msg = "minecraft目录未找到".to_string();
            Utils::log_message(&error_msg, LogLevel::Error);
            warnings.push(format!("警告: {}", error_msg));
            return Ok(());
        }

        let mut result = ReorganizationResult {
            success: true,
            renamed_files: Vec::new(),
            moved_directories: Vec::new(),
            removed_empty_dirs: Vec::new(),
            error_message: None,
        };

        // 1. 将pack.png重命名为pack_icon.png
        if let Err(e) = self._rename_pack_icon(temp_dir, &mut result) {
            let error_msg = format!("重命名pack.png失败: {}", e);
            Utils::log_message(&error_msg, LogLevel::Error);
            warnings.push(format!("警告: {}", error_msg));
            return Ok(());
        }

        // 2. 提升minecraft/textures/font到一级目录
        if let Err(e) = self._promote_font_directory(&minecraft_dir, temp_dir, &mut result) {
            let error_msg = format!("提升font目录失败: {}", e);
            Utils::log_message(&error_msg, LogLevel::Error);
            warnings.push(format!("警告: {}", error_msg));
            return Ok(());
        }

        // 3. 提升minecraft/textures到一级目录
        if let Err(e) = self._promote_textures_directory(&minecraft_dir, temp_dir, &mut result) {
            let error_msg = format!("提升textures目录失败: {}", e);
            Utils::log_message(&error_msg, LogLevel::Error);
            warnings.push(format!("警告: {}", error_msg));
            return Ok(());
        }

        // 4. 处理Java UI相关的文件重组
        if let Err(e) = self._handle_java_ui_reorganization(temp_dir, &mut result) {
            let error_msg = format!("Java UI重组失败: {}", e);
            Utils::log_message(&error_msg, LogLevel::Error);
            warnings.push(format!("警告: {}", error_msg));
            return Ok(());
        }

        // 5. 清理空的assets目录
        if let Err(e) = self._cleanup_empty_assets_dir(temp_dir, &mut result) {
            let error_msg = format!("清理assets目录失败: {}", e);
            Utils::log_message(&error_msg, LogLevel::Error);
            warnings.push(format!("警告: {}", error_msg));
            return Ok(());
        }

        Utils::log_message(&format!("文件结构重组完成，重命名了{}个文件，移动了{}个目录", 
                           result.renamed_files.len(), result.moved_directories.len()), LogLevel::Info);
        
        // 添加结果信息到warnings
        for file in &result.renamed_files {
            warnings.push(format!("已重命名文件: {}", file));
        }
        for dir in &result.moved_directories {
            warnings.push(format!("已移动目录: {}", dir));
        }
        
        Ok(())
    }

    /// 重命名pack.png为pack_icon.png
    fn _rename_pack_icon(&self, temp_dir: &Path, result: &mut ReorganizationResult) -> io::Result<()> {
        let pack_png = temp_dir.join("pack.png");
        if pack_png.exists() {
            let pack_icon = temp_dir.join("pack_icon.png");
            fs::rename(&pack_png, &pack_icon)?;
            result.renamed_files.push("pack.png -> pack_icon.png".to_string());
            Utils::log_message("已将pack.png重命名为pack_icon.png", LogLevel::Info);
        }
        Ok(())
    }

    /// 提升minecraft/textures/font到一级目录
    fn _promote_font_directory(&self, minecraft_dir: &Path, temp_dir: &Path, result: &mut ReorganizationResult) -> io::Result<()> {
        let textures_font_dir = minecraft_dir.join("textures").join("font");
        if textures_font_dir.exists() {
            let target_font_dir = temp_dir.join("font");
            if target_font_dir.exists() {
                fs::remove_dir_all(&target_font_dir)?;
            }
            fs::create_dir_all(target_font_dir.parent().unwrap())?;
            fs::rename(&textures_font_dir, &target_font_dir)?;
            result.moved_directories.push("textures/font -> font".to_string());
            Utils::log_message("已提升font目录到一级目录", LogLevel::Info);
        }
        Ok(())
    }

    /// 提升minecraft/textures到一级目录
    fn _promote_textures_directory(&self, minecraft_dir: &Path, temp_dir: &Path, result: &mut ReorganizationResult) -> io::Result<()> {
        let textures_dir = minecraft_dir.join("textures");
        if textures_dir.exists() {
            let target_textures_dir = temp_dir.join("textures");
            if target_textures_dir.exists() {
                // 移动文件而不是替换整个目录
                self._merge_directories(&textures_dir, &target_textures_dir)?;
                fs::remove_dir_all(&textures_dir)?;
            } else {
                fs::rename(&textures_dir, &target_textures_dir)?;
            }
            result.moved_directories.push("minecraft/textures -> textures".to_string());
            Utils::log_message("已提升textures目录到一级目录", LogLevel::Info);
        }
        Ok(())
    }

    /// 合并目录（保留目标目录中的文件）
    fn _merge_directories(&self, src: &Path, dst: &Path) -> io::Result<()> {
        if !dst.exists() {
            fs::create_dir_all(dst)?;
        }

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                self._merge_directories(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }
        Ok(())
    }

    /// 处理Java UI相关的文件重组
    fn _handle_java_ui_reorganization(&self, temp_dir: &Path, result: &mut ReorganizationResult) -> io::Result<()> {
        // 1. 将/textures/item重命名为/textures/items
        let textures_dir = temp_dir.join("textures");
        let item_dir = textures_dir.join("item");
        if item_dir.exists() {
            let items_dir = textures_dir.join("items");
            // 如果items目录已存在，先删除
            if items_dir.exists() {
                fs::remove_dir_all(&items_dir)?;
            }
            // 重命名item目录为items
            fs::rename(&item_dir, &items_dir)?;
            result.moved_directories.push("textures/item -> textures/items".to_string());
            Utils::log_message("已将/textures/item重命名为/textures/items", LogLevel::Info);
        }

        // 2. 检查java_ui目录是否存在
        let java_ui_dir = temp_dir.join("java_ui");
        if !java_ui_dir.exists() {
            Utils::log_message("java_ui目录不存在，跳过Java UI处理", LogLevel::Info);
            return Ok(());
        }

        Utils::log_message("开始处理Java UI文件替换", LogLevel::Info);

        // 2. 将/textures/gui/container移动到/textures/ui
        let textures_gui_dir = temp_dir.join("textures").join("gui");
        let container_dir = textures_gui_dir.join("container");
        let textures_ui_dir = temp_dir.join("textures").join("ui");
        
        if container_dir.exists() {
            // 如果ui目录已存在，先删除
            if textures_ui_dir.exists() {
                fs::remove_dir_all(&textures_ui_dir)?;
            }
            // 移动container目录到textures目录下，命名为ui
            fs::rename(&container_dir, &textures_ui_dir)?;
            result.moved_directories.push("textures/gui/container -> textures/ui".to_string());
            Utils::log_message("已将/textures/gui/container移动到/textures/ui", LogLevel::Info);
        }

        // 3. 提取textures/gui/creative_inventory内容到textures/gui同级
        let creative_inventory_dir = textures_gui_dir.join("creative_inventory");
        
        if creative_inventory_dir.exists() {
            // 将creative_inventory内容提取到gui目录同级
            self._extract_creative_inventory_contents(&creative_inventory_dir, &textures_gui_dir, result)?;
            // 删除空的creative_inventory目录
            fs::remove_dir_all(&creative_inventory_dir)?;
            result.removed_empty_dirs.push("textures/gui/creative_inventory".to_string());
            Utils::log_message("已提取creative_inventory内容到textures/gui同级", LogLevel::Info);
        }

        // 3. 将java_ui/ui文件夹复制到bedrock材质包根目录
        let java_ui_ui_dir = java_ui_dir.join("ui");
        let root_ui_dir = temp_dir.join("ui");
        
        if java_ui_ui_dir.exists() {
            // 如果根目录的ui文件夹已存在，先删除
            if root_ui_dir.exists() {
                fs::remove_dir_all(&root_ui_dir)?;
            }
            // 复制java_ui/ui文件夹到根目录
            self._copy_with_force_replace(&java_ui_ui_dir, &root_ui_dir, result)?;
            result.moved_directories.push("java_ui/ui -> /ui (根目录)".to_string());
            Utils::log_message("已将java_ui/ui文件夹复制到bedrock材质包根目录", LogLevel::Info);
        }

        // 4. 复制java_ui/textures/ui目录到/textures/ui并强制替换同名文件
        let java_ui_textures_ui = java_ui_dir.join("textures").join("ui");
        let target_ui_dir = temp_dir.join("textures").join("ui");
        
        if java_ui_textures_ui.exists() {
            // 确保目标目录存在
            if !target_ui_dir.exists() {
                fs::create_dir_all(&target_ui_dir)?;
            }
            
            // 递归复制并强制替换同名文件
            self._copy_with_force_replace(&java_ui_textures_ui, &target_ui_dir, result)?;
            result.moved_directories.push("java_ui/textures/ui -> textures/ui (强制替换)".to_string());
            Utils::log_message("已复制java_ui/textures/ui到textures/ui并强制替换同名文件", LogLevel::Info);
        }

        // 4. 将java_ui/gui/container的内容合并到textures/gui/container（保留原有结构）
        let java_ui_gui_container = java_ui_dir.join("gui").join("container");
        let target_container_dir = textures_gui_dir.join("container");
        
        if java_ui_gui_container.exists() {
            // 确保目标目录存在
            if !target_container_dir.exists() {
                fs::create_dir_all(&target_container_dir)?;
            }
            
            // 合并java_ui的container内容到目标目录（保留原有文件）
            self._merge_directories_preserve(&java_ui_gui_container, &target_container_dir)?;
            result.moved_directories.push("java_ui/gui/container -> textures/gui/container (合并)".to_string());
            Utils::log_message("已合并java_ui/gui/container内容到textures/gui/container", LogLevel::Info);
        }

        // 5. 将java_ui/gui/sprites文件夹复制到/textures/gui下（强行替换）
        let java_ui_gui_sprites = java_ui_dir.join("gui").join("sprites");
        let target_sprites_dir = textures_gui_dir.join("sprites");
        
        if java_ui_gui_sprites.exists() {
            // 确保目标目录存在
            if !target_sprites_dir.exists() {
                fs::create_dir_all(&target_sprites_dir)?;
            }
            
            // 递归复制sprites目录并强制替换同名文件
            self._copy_with_force_replace(&java_ui_gui_sprites, &target_sprites_dir, result)?;
            result.moved_directories.push("java_ui/gui/sprites -> textures/gui/sprites (强制替换)".to_string());
            Utils::log_message("已将java_ui/gui/sprites复制到/textures/gui/sprites（强行替换）", LogLevel::Info);
        }

        Ok(())
    }

    /// 提取creative_inventory内容到同级目录
    fn _extract_creative_inventory_contents(&self, creative_inventory_dir: &Path, target_gui_dir: &Path, result: &mut ReorganizationResult) -> io::Result<()> {
        for entry in fs::read_dir(creative_inventory_dir)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = target_gui_dir.join(entry.file_name());

            if src_path.is_dir() {
                // 递归处理子目录
                self._extract_creative_inventory_contents(&src_path, &dst_path, result)?;
            } else {
                // 复制文件到gui目录同级
                fs::copy(&src_path, &dst_path)?;
            }
        }
        Ok(())
    }

    /// 合并目录但保留目标目录中的文件（不覆盖）
    fn _merge_directories_preserve(&self, src: &Path, dst: &Path) -> io::Result<()> {
        if !dst.exists() {
            fs::create_dir_all(dst)?;
        }

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                // 递归处理子目录
                self._merge_directories_preserve(&src_path, &dst_path)?;
            } else {
                // 只复制不存在的文件，保留已存在的文件
                if !dst_path.exists() {
                    fs::copy(&src_path, &dst_path)?;
                }
            }
        }
        Ok(())
    }

    /// 递归复制目录并强制替换同名文件
    fn _copy_with_force_replace(&self, src: &Path, dst: &Path, result: &mut ReorganizationResult) -> io::Result<()> {
        if !dst.exists() {
            fs::create_dir_all(dst)?;
        }

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                // 递归处理子目录
                self._copy_with_force_replace(&src_path, &dst_path, result)?;
            } else {
                // 如果目标文件存在，先删除
                if dst_path.exists() {
                    fs::remove_file(&dst_path)?;
                }
                // 复制文件
                fs::copy(&src_path, &dst_path)?;
            }
        }
        Ok(())
    }

    /// 清理空的assets目录
    fn _cleanup_empty_assets_dir(&self, temp_dir: &Path, result: &mut ReorganizationResult) -> io::Result<()> {
        let assets_dir = temp_dir.join("assets");
        if assets_dir.exists() {
            // 检查assets目录是否为空或只包含minecraft
            let entries = fs::read_dir(&assets_dir)?;
            let mut is_empty_or_only_minecraft = true;
            
            for entry in entries {
                let entry = entry?;
                let file_name = entry.file_name();
                if file_name != "minecraft" {
                    is_empty_or_only_minecraft = false;
                    break;
                }
            }
            
            if is_empty_or_only_minecraft {
                // 移除minecraft目录
                let minecraft_path = assets_dir.join("minecraft");
                if minecraft_path.exists() {
                    fs::remove_dir_all(&minecraft_path)?;
                    result.removed_empty_dirs.push("minecraft".to_string());
                }
                
                // 如果assets目录为空，则删除
                if fs::read_dir(&assets_dir).unwrap().next().is_none() {
                    fs::remove_dir(&assets_dir)?;
                    result.removed_empty_dirs.push("assets".to_string());
                    Utils::log_message("已删除空的assets目录", LogLevel::Info);
                }
            }
        }
        Ok(())
    }

    /// 创建Java版材质包目录结构
    pub fn create_java_structure(&self, temp_dir: &Path) -> io::Result<()> {
        // 创建标准的Java版材质包目录结构
        let dirs_to_create = vec![
            temp_dir.join("assets"),
            temp_dir.join("assets").join("minecraft"),
            temp_dir.join("assets").join("minecraft").join("textures"),
            temp_dir.join("assets").join("minecraft").join("textures").join("block"),
            temp_dir.join("assets").join("minecraft").join("textures").join("item"),
            temp_dir.join("assets").join("minecraft").join("textures").join("gui"),
            temp_dir.join("assets").join("minecraft").join("textures").join("font"),
        ];

        for dir in dirs_to_create {
            fs::create_dir_all(&dir)?;
        }

        Utils::log_message("已创建Java版材质包目录结构", LogLevel::Info);
        Ok(())
    }

    /// 创建基岩版材质包目录结构
    pub fn create_bedrock_structure(&self, temp_dir: &Path) -> io::Result<()> {
        // 创建标准的基岩版材质包目录结构
        let dirs_to_create = vec![
            temp_dir.join("textures"),
            temp_dir.join("textures").join("blocks"),
            temp_dir.join("textures").join("items"),
            temp_dir.join("textures").join("gui"),
            temp_dir.join("textures").join("font"),
            temp_dir.join("models"),
            temp_dir.join("font"),
        ];

        for dir in dirs_to_create {
            fs::create_dir_all(&dir)?;
        }

        Utils::log_message("已创建基岩版材质包目录结构", LogLevel::Info);
        Ok(())
    }

    /// 获取重组操作的详细报告
    pub fn get_reorganization_report(&self, result: &ReorganizationResult) -> String {
        let mut report = String::new();
        
        if result.success {
            report.push_str("✅ 文件结构重组成功\n\n");
        } else {
            report.push_str("❌ 文件结构重组失败\n\n");
        }

        if !result.renamed_files.is_empty() {
            report.push_str("📝 重命名文件:\n");
            for file in &result.renamed_files {
                report.push_str(&format!("  - {}\n", file));
            }
            report.push_str("\n");
        }

        if !result.moved_directories.is_empty() {
            report.push_str("📁 移动目录:\n");
            for dir in &result.moved_directories {
                report.push_str(&format!("  - {}\n", dir));
            }
            report.push_str("\n");
        }

        if !result.removed_empty_dirs.is_empty() {
            report.push_str("🗑️ 删除空目录:\n");
            for dir in &result.removed_empty_dirs {
                report.push_str(&format!("  - {}\n", dir));
            }
            report.push_str("\n");
        }

        if let Some(error) = &result.error_message {
            report.push_str(&format!("⚠️ 错误: {}\n", error));
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_file_reorganizer_creation() {
        let reorganizer = FileReorganizer::new();
        assert!(reorganizer.reorganize_java_to_bedrock_structure(Path::new("non_existent")).success);
    }

    #[test]
    fn test_structure_creation() {
        let reorganizer = FileReorganizer::new();
        let temp = tempdir().unwrap();
        
        assert!(reorganizer.create_java_structure(temp.path()).is_ok());
        assert!(reorganizer.create_bedrock_structure(temp.path()).is_ok());
    }
}