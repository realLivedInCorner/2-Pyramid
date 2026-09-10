//! 药水物品栏：Java 单图 → Bedrock 全 effect 变体。

use std::fs;
use std::path::Path;

use crate::log_info;

use super::mapping::{
    bedrock_potion_variant_names, bedrock_splash_potion_variant_names,
};

/// items/ 下药水相关补全（rename 之后调用）。
pub fn ensure_potion_item_files(textures_dst: &Path) {
    let items = textures_dst.join("items");
    let gui = textures_dst.join("gui");
    let ui = textures_dst.join("ui");
    let _ = fs::create_dir_all(&items);

    for src in [
        ui.join("potion_overlay.png"),
        ui.join("container").join("potion_overlay.png"),
        gui.join("potion_overlay.png"),
    ] {
        if src.is_file() {
            let dst = items.join("potion_overlay.png");
            if src != dst && !dst.exists() {
                let _ = fs::copy(&src, &dst);
            }
            break;
        }
    }

    let aliases: &[(&str, &str)] = &[
        ("potion.png", "potion_bottle_drinkable.png"),
        ("splash_potion.png", "potion_bottle_splash.png"),
        ("lingering_potion.png", "potion_bottle_lingering.png"),
        ("glass_bottle.png", "potion_bottle_empty.png"),
    ];
    for (from, to) in aliases {
        let src = items.join(from);
        let dst = items.join(to);
        if src.is_file() && !dst.exists() {
            let _ = fs::copy(&src, &dst);
        }
    }

    expand_variants(&items, "potion_bottle_drinkable.png", bedrock_potion_variant_names());
    expand_variants(
        &items,
        "potion_bottle_splash.png",
        bedrock_splash_potion_variant_names(),
    );
}

/// 将基准药水贴图复制到尚未存在的 variant 文件名。
fn expand_variants(items: &Path, source_file: &str, names: &[&str]) {
    let src = items.join(source_file);
    if !src.is_file() {
        return;
    }
    let mut n = 0usize;
    for name in names {
        let dst = items.join(format!("{}.png", name));
        if !dst.exists() && fs::copy(&src, &dst).is_ok() {
            n += 1;
        }
    }
    if n > 0 {
        log_info!("OKAY bedrock [{} → {} potion variants]", source_file, n);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_expand_all_effect_potions() {
        let temp = tempdir().unwrap();
        let items = temp.path().join("items");
        fs::create_dir_all(&items).unwrap();
        fs::write(items.join("potion.png"), b"p").unwrap();
        fs::write(items.join("splash_potion.png"), b"s").unwrap();

        ensure_potion_item_files(temp.path());

        assert!(items.join("potion_bottle_drinkable.png").exists());
        assert!(items.join("potion_bottle_moveSpeed.png").exists());
        assert!(items.join("potion_bottle_regeneration.png").exists());
        assert!(items.join("potion_bottle_wither.png").exists());
        assert!(items.join("potion_bottle_splash_heal.png").exists());
        assert!(items.join("potion_bottle_splash_poison.png").exists());
        // 已存在的变体不被覆盖
        fs::write(items.join("potion_bottle_heal.png"), b"keep").unwrap();
        ensure_potion_item_files(temp.path());
        assert_eq!(fs::read(items.join("potion_bottle_heal.png")).unwrap(), b"keep");
    }
}
