//! b2j：Bedrock 包目录 → Java 1.21.11 风格树。

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

/// 把 Bedrock 资源包目录重组为 Java 风格树（pack_format 75）。
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

    write_pack_mcmeta(temp_dir, 75, &description)?;
    log_info!("OKAY java [pack.mcmeta format=75]");
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
        fs::write(root.join("textures/ui/widgets.png"), b"w").unwrap();
        fs::write(root.join("attachables/x.json"), b"{}").unwrap();

        let (name, desc) = convert_bedrock_to_java(root).unwrap();
        assert_eq!(name, "基岩");
        assert_eq!(desc, "说明");
        assert!(root.join("pack.png").exists());
        assert!(!root.join("manifest.json").exists());
        assert!(root.join("assets/minecraft/textures/item/golden_apple.png").exists());
        assert!(root.join("assets/minecraft/textures/block/stone.png").exists());
        assert!(root.join("assets/minecraft/textures/gui/container/widgets.png").exists());
        assert!(!root.join("attachables").exists());
        let mc: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(root.join("pack.mcmeta")).unwrap()).unwrap();
        assert_eq!(mc["pack"]["pack_format"], 75);
        assert_eq!(mc["pack"]["description"], "说明");
    }
}
