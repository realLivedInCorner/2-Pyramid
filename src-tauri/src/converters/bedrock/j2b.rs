//! j2b：Java 包目录 → Bedrock 结构。

use std::fs;
use std::path::Path;

use crate::log_info;

use super::fsutil::remove_file_quiet;
use super::metadata::{
    cleanup_empty_assets, convert_lang_java_to_bedrock, convert_sounds_java_to_bedrock,
    read_description_from_mcmeta, strip_java_only, write_manifest,
};
use super::textures::{move_java_font_to_bedrock, reorganize_java_textures_for_bedrock};

/// 把已转换到 1.21.11 的 Java 包目录重组为基岩版结构。
pub fn convert_java_to_bedrock(temp_dir: &Path, pack_name: &str) -> Result<(), String> {
    let pack_png = temp_dir.join("pack.png");
    if pack_png.exists() {
        let pack_icon = temp_dir.join("pack_icon.png");
        remove_file_quiet(&pack_icon);
        fs::rename(&pack_png, &pack_icon).map_err(|e| format!("rename pack.png failed: {}", e))?;
        log_info!("OKAY bedrock [pack.png -> pack_icon.png]");
    }
    // description 必须在删除 pack.mcmeta 之前读取
    let description = read_description_from_mcmeta(temp_dir);
    remove_file_quiet(&temp_dir.join("pack.mcmeta"));

    let minecraft = temp_dir.join("assets").join("minecraft");
    move_java_font_to_bedrock(&minecraft, temp_dir);
    reorganize_java_textures_for_bedrock(&minecraft, &temp_dir.join("textures"))?;
    convert_lang_java_to_bedrock(&minecraft, temp_dir);
    convert_sounds_java_to_bedrock(&minecraft, temp_dir);
    strip_java_only(&minecraft, temp_dir);
    cleanup_empty_assets(temp_dir);

    write_manifest(temp_dir, pack_name, &description)?;
    log_info!("OKAY bedrock [manifest.json]");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_j2b_end_to_end() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        let tex = root.join("assets/minecraft/textures");
        fs::create_dir_all(tex.join("item")).unwrap();
        fs::create_dir_all(tex.join("block")).unwrap();
        fs::create_dir_all(tex.join("gui/container")).unwrap();
        fs::create_dir_all(root.join("assets/minecraft/lang")).unwrap();
        fs::create_dir_all(root.join("assets/minecraft/blockstates")).unwrap();
        fs::write(tex.join("item/golden_apple.png"), b"g").unwrap();
        fs::write(tex.join("block/stone.png"), b"s").unwrap();
        fs::write(tex.join("gui/icons.png"), b"i").unwrap();
        fs::write(root.join("pack.png"), b"p").unwrap();
        fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"pack_format":75,"description":"测试"}}"#,
        )
        .unwrap();
        fs::write(
            root.join("assets/minecraft/lang/zh_cn.json"),
            r#"{"block.minecraft.stone":"石头"}"#,
        )
        .unwrap();
        fs::write(root.join("assets/minecraft/blockstates/stone.json"), b"{}").unwrap();

        // capture description before conversion removes mcmeta
        convert_java_to_bedrock(root, "测试").unwrap();

        assert!(root.join("pack_icon.png").exists());
        assert!(!root.join("pack.mcmeta").exists());
        assert!(root.join("textures/items/apple_golden.png").exists());
        assert!(root.join("textures/blocks/stone.png").exists());
        assert!(root.join("textures/ui/icons.png").exists());
        assert!(!root.join("assets").exists());
        assert!(root.join("manifest.json").exists());
        assert!(root.join("textures/textures_list.json").exists());
        assert!(root.join("textures/terrain_texture.json").exists());
        assert!(root.join("textures/item_texture.json").exists());
        let m: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(root.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(m["header"]["name"], "测试");
    }
}
