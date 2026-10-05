//! b2j：Bedrock 包目录 → 最新 Java 26.3 风格树（pack_format 97）。

use std::fs;
use std::path::Path;

use crate::log_info;

use super::fsutil::remove_file_quiet;
use super::metadata::{
    convert_lang_bedrock_to_java, convert_sounds_bedrock_to_java, read_manifest_info,
    strip_bedrock_only, write_pack_mcmeta,
};
use super::textures::{
    move_bedrock_font_to_java, move_bedrock_ui_to_java_gui, reorganize_bedrock_textures_for_java,
};

/// 把 Bedrock 资源包目录重组为 Java 风格树（pack_format 97 / 26.3）。
pub fn convert_bedrock_to_java(temp_dir: &Path) -> Result<(String, String), String> {
    let (pack_name, description) = read_manifest_info(temp_dir);

    let pack_icon = temp_dir.join("pack_icon.png");
    if pack_icon.exists() {
        let pack_png = temp_dir.join("pack.png");
        remove_file_quiet(&pack_png);
        fs::rename(&pack_icon, &pack_png).map_err(|e| format!("rename pack_icon failed: {}", e))?;
        log_info!("OKAY java [pack_icon.png -> pack.png]");
    }

    let minecraft = temp_dir.join("assets").join("minecraft");
    reorganize_bedrock_textures_for_java(temp_dir, &minecraft)?;
    move_bedrock_font_to_java(temp_dir, &minecraft);
    move_bedrock_ui_to_java_gui(&minecraft);
    convert_lang_bedrock_to_java(temp_dir, &minecraft);
    convert_sounds_bedrock_to_java(temp_dir, &minecraft);
    strip_bedrock_only(temp_dir, &minecraft);

    // 统一落到最新 Java 26.3（format 97），后续流水线再转到用户目标
    write_pack_mcmeta(temp_dir, 97, &description)?;
    log_info!("OKAY java [pack.mcmeta format=97]");
    Ok((pack_name, description))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_b2j_end_to_end() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("pack_icon.png"), b"p").unwrap();
        fs::write(
            root.join("manifest.json"),
            r#"{"modules":[{"type":"resources"}],"header":{"name":"基岩","description":"说明"}}"#,
        )
        .unwrap();
        fs::create_dir_all(root.join("textures/items")).unwrap();
        fs::create_dir_all(root.join("textures/blocks")).unwrap();
        fs::create_dir_all(root.join("textures/ui")).unwrap();
        fs::create_dir_all(root.join("attachables")).unwrap();
        fs::write(root.join("textures/items/apple_golden.png"), b"g").unwrap();
        fs::write(root.join("textures/blocks/stone.png"), b"s").unwrap();
        // **§9.127 回归**：`bedrock_to_java_stem` 里同样有**恒等项**（`bamboo_block` 等）。
        // 修 `rename_stems_in_dir` 之前，"block 反向改名"这一步会因恒等项把源文件删掉并报
        // `NotFound`，**整条 b2j 中止**（纯净 aeaecb2 上实测：0 产物）。
        fs::write(root.join("textures/blocks/bamboo_block.png"), b"bb").unwrap();
        fs::write(root.join("textures/ui/widgets.png"), b"w").unwrap();
        fs::write(root.join("attachables/x.json"), b"{}").unwrap();

        let (name, desc) = convert_bedrock_to_java(root).unwrap();
        assert_eq!(name, "基岩");
        assert_eq!(desc, "说明");
        assert!(root.join("pack.png").exists());
        assert!(!root.join("manifest.json").exists());
        assert!(root.join("assets/minecraft/textures/item/golden_apple.png").exists());
        assert!(root.join("assets/minecraft/textures/block/stone.png").exists());
        // 恒等映射的文件必须**原样还在**（此前会被删掉并中止整条转换）
        assert!(
            root.join("assets/minecraft/textures/block/bamboo_block.png").exists(),
            "恒等映射的 bamboo_block.png 在 b2j 里被删掉了 —— rename_stems_in_dir 的 guard 失效"
        );
        assert!(root.join("assets/minecraft/textures/gui/container/widgets.png").exists());
        assert!(!root.join("attachables").exists());
        let mc: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(root.join("pack.mcmeta")).unwrap()).unwrap();
        assert_eq!(mc["pack"]["max_format"][0], 97);
        assert_eq!(mc["pack"]["max_format"][1], 1);
        assert_eq!(mc["pack"]["description"], "说明");
    }
}
