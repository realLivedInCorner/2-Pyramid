//! Java GUI → Bedrock `textures/ui` 补全（快捷栏、HUD、容器）。
//!
//! 参考可用包布局：Bedrock 读 `textures/ui/hotbar.png`、`inventory.png`、
//! 心/饥饿等图标；Java 现代包常在 `gui/sprites/hud/`，旧包在 `gui/icons.png` 图集。

use std::fs;
use std::path::Path;

use image::RgbaImage;

use crate::log_info;

/// 由 reorganize 在 gui→ui 扁平化之后调用。
pub fn convert_java_hud_to_bedrock_ui(textures_dst: &Path) {
    let ui = textures_dst.join("ui");
    let _ = fs::create_dir_all(&ui);

    copy_inventory_gui(textures_dst, &ui);
    copy_java_sprite_hud(textures_dst, &ui);
    extract_from_icons_atlas(&ui);
}

/// 背包：container/inventory.png（扁平后可能已在 ui/）。
fn copy_inventory_gui(textures_dst: &Path, ui: &Path) {
    let dst = ui.join("inventory.png");
    if dst.is_file() {
        return;
    }
    for src in [
        textures_dst.join("gui").join("container").join("inventory.png"),
        textures_dst.join("gui").join("inventory.png"),
    ] {
        if src.is_file() {
            let _ = fs::copy(&src, &dst);
            log_info!("OKAY bedrock [ui/inventory.png]");
            return;
        }
    }
}

/// Java 1.20+ sprites/hud/* → Bedrock ui/*（同名优先）。
fn copy_java_sprite_hud(textures_dst: &Path, ui: &Path) {
    let hud = textures_dst.join("gui").join("sprites").join("hud");
    if !hud.is_dir() {
        return;
    }
    // 常见精灵名 → Bedrock ui 文件名
    let map: &[(&str, &str)] = &[
        ("hotbar.png", "hotbar.png"),
        ("hotbar_selection.png", "hotbar_selection.png"),
        ("crosshair.png", "cross_hair.png"),
        ("heart_full.png", "heart_full.png"),
        ("heart_half.png", "heart_half.png"),
        ("heart_empty.png", "heart_empty.png"),
        ("hunger_full.png", "hunger_effect_full.png"),
        ("hunger_half.png", "hunger_effect_half.png"),
        ("hunger_empty.png", "hunger_effect.png"),
        ("armor_full.png", "armor_full.png"),
        ("armor_half.png", "armor_half.png"),
        ("armor_empty.png", "armor_empty.png"),
    ];
    // 递归收集 hud 下 png
    let mut copied = 0usize;
    copy_dir_rename(&hud, ui, map, &mut copied);
    // 原名文件也拷一份（保证路径存在）
    merge_keep_names(&hud, ui, &mut copied);
    if copied > 0 {
        log_info!("OKAY bedrock [gui/sprites/hud → ui × {}]", copied);
    }
    // sprites 本身不再需要（已消费）
    let _ = fs::remove_dir_all(textures_dst.join("gui").join("sprites"));
}

fn copy_dir_rename(
    src: &Path,
    dst_dir: &Path,
    map: &[(&str, &str)],
    copied: &mut usize,
) {
    let Ok(entries) = fs::read_dir(src) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // 子目录扁平到 ui/
            copy_dir_rename(&path, dst_dir, map, copied);
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.to_ascii_lowercase().ends_with(".png") {
            continue;
        }
        if let Some((_, out)) = map.iter().find(|(from, _)| *from == name) {
            let _ = fs::copy(&path, dst_dir.join(out));
            *copied += 1;
        }
    }
}

fn merge_keep_names(src: &Path, dst_dir: &Path, copied: &mut usize) {
    let Ok(entries) = fs::read_dir(src) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            merge_keep_names(&path, dst_dir, copied);
            continue;
        }
        let name = entry.file_name();
        let dst = dst_dir.join(&name);
        if path.is_file() && !dst.exists() {
            if fs::copy(&path, &dst).is_ok() {
                *copied += 1;
            }
        }
    }
}

/// icons.png 标准 UV（256 坐标系；按图集倍数放大）。
struct IconsUv {
    scale: u32,
    img: RgbaImage,
}

impl IconsUv {
    fn open(path: &Path) -> Option<Self> {
        let img = image::open(path).ok()?.to_rgba8();
        let w = img.width();
        let scale = if w >= 1024 {
            4
        } else if w >= 512 {
            2
        } else {
            1
        };
        Some(Self { scale, img })
    }

    fn crop(&self, x: u32, y: u32, w: u32, h: u32) -> Option<RgbaImage> {
        let s = self.scale;
        let (x, y, w, h) = (x * s, y * s, w * s, h * s);
        if self.img.width() < x + w || self.img.height() < y + h {
            return None;
        }
        Some(image::imageops::crop_imm(&self.img, x, y, w, h).to_image())
    }
}

/// 从 icons.png 裁出 Bedrock HUD 常用图。
/// 坐标来自 Java IngameGui / GuiGraphics blit 表（256 图集）。
fn extract_from_icons_atlas(ui: &Path) {
    let icons = ui.join("icons.png");
    let Some(atlas) = IconsUv::open(&icons) else {
        return;
    };

    // (x, y, w, h, out_name) — 256 坐标系
    let cuts: &[(u32, u32, u32, u32, &str)] = &[
        // 快捷栏底与选中框
        (0, 0, 182, 22, "hotbar.png"),
        (0, 22, 24, 24, "hotbar_selection.png"),
        // 准星（与快捷栏同区不同尺寸，vanilla 亦如此；仅在非 hotbar 尺寸包时有效）
        (0, 0, 15, 15, "cross_hair.png"),
        // 生命 9×9：空 16,0 / 满 52,0 / 半 61,0
        (16, 0, 9, 9, "heart_empty.png"),
        (52, 0, 9, 9, "heart_full.png"),
        (61, 0, 9, 9, "heart_half.png"),
        // 饥饿：空 16,27 / 满 52,27 / 半 61,27
        (16, 27, 9, 9, "hunger_effect.png"),
        (52, 27, 9, 9, "hunger_effect_full.png"),
        (61, 27, 9, 9, "hunger_effect_half.png"),
        // 护甲：空 16,9 / 半 34,9 / 满 43,9
        (16, 9, 9, 9, "armor_empty.png"),
        (34, 9, 9, 9, "armor_half.png"),
        (43, 9, 9, 9, "armor_full.png"),
        // 经验条
        (0, 64, 182, 5, "experiencebarempty.png"),
        (0, 69, 182, 5, "experiencebarfull.png"),
    ];

    let mut n = 0usize;
    for &(x, y, w, h, name) in cuts {
        let dst = ui.join(name);
        // 已有 sprites 拷来的文件不覆盖
        if dst.exists() && name != "hotbar.png" && name != "cross_hair.png" {
            continue;
        }
        // hotbar 始终可重裁（sprites 优先时上面已写过，这里仅当缺失）
        if dst.exists() && (name == "hotbar.png" || name == "cross_hair.png") {
            // sprites 已提供则跳过
            if name == "cross_hair.png" && ui.join("crosshair.png").exists() {
                continue;
            }
            if name == "hotbar.png" {
                continue;
            }
        }
        if let Some(crop) = atlas.crop(x, y, w, h) {
            if crop.save(&dst).is_ok() {
                n += 1;
            }
        }
    }
    // 准星：若 sprites 未提供且 hotbar 已占 0,0，从同区裁 15×15 仍可用
    if !ui.join("cross_hair.png").exists() {
        if let Some(c) = atlas.crop(0, 0, 15, 15) {
            let _ = c.save(ui.join("cross_hair.png"));
            n += 1;
        }
    }
    log_info!("OKAY bedrock [icons.png HUD crops × {}]", n);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_icons_256(path: &Path) {
        let mut img = RgbaImage::from_pixel(256, 256, image::Rgba([0, 0, 0, 0]));
        // 快捷栏区红色
        for y in 0..22u32 {
            for x in 0..182u32 {
                img.put_pixel(x, y, image::Rgba([200, 30, 30, 255]));
            }
        }
        // 选中框绿色
        for y in 22..46u32 {
            for x in 0..24u32 {
                img.put_pixel(x, y, image::Rgba([30, 200, 30, 255]));
            }
        }
        img.save(path).unwrap();
    }

    #[test]
    fn test_icons_crops_hotbar_and_selection() {
        let temp = tempdir().unwrap();
        let ui = temp.path().join("ui");
        fs::create_dir_all(&ui).unwrap();
        write_icons_256(&ui.join("icons.png"));

        extract_from_icons_atlas(&ui);

        let hb = image::open(ui.join("hotbar.png")).unwrap();
        assert_eq!(hb.width(), 182);
        assert_eq!(hb.height(), 22);
        let sel = image::open(ui.join("hotbar_selection.png")).unwrap();
        assert_eq!(sel.width(), 24);
        assert_eq!(sel.height(), 24);
        assert!(ui.join("heart_full.png").exists());
        assert!(ui.join("hunger_effect_full.png").exists());
        assert!(ui.join("armor_full.png").exists());
        assert!(ui.join("experiencebarfull.png").exists());
        assert!(ui.join("cross_hair.png").exists());
    }

    #[test]
    fn test_sprites_hud_copied() {
        let temp = tempdir().unwrap();
        let textures = temp.path();
        let hud = textures.join("gui/sprites/hud");
        let ui = textures.join("ui");
        fs::create_dir_all(&hud).unwrap();
        fs::write(hud.join("hotbar.png"), b"h").unwrap();
        fs::write(hud.join("crosshair.png"), b"c").unwrap();
        fs::create_dir_all(textures.join("gui/container")).unwrap();
        fs::write(textures.join("gui/container/inventory.png"), b"i").unwrap();

        convert_java_hud_to_bedrock_ui(textures);

        assert!(ui.join("hotbar.png").exists());
        assert!(ui.join("cross_hair.png").exists());
        assert!(ui.join("inventory.png").exists());
        assert!(!textures.join("gui/sprites").exists());
    }
}
