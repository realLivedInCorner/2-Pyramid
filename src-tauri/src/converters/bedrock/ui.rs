//! Java GUI → Bedrock `textures/ui` 补全（快捷栏、HUD、容器）。
//!
//! 参考可用包布局：Bedrock 读 `textures/ui/hotbar.png`、`inventory.png`、
//! 心/饥饿等图标；Java 现代包常在 `gui/sprites/hud/`，旧包在 `gui/icons.png` 图集。

use std::fs;
use std::path::Path;

use image::RgbaImage;

use crate::log_info;

use super::fsutil::remove_dir_quiet;

/// 由 reorganize 在 gui→ui 扁平化之后调用。
pub fn convert_java_hud_to_bedrock_ui(textures_dst: &Path) {
    let ui = textures_dst.join("ui");
    let _ = fs::create_dir_all(&ui);

    copy_inventory_gui(textures_dst, &ui);
    adapt_container_screens(textures_dst, &ui);
    copy_java_sprite_hud(textures_dst, &ui);
    flatten_java120_ui_sprite_dirs(&ui);
    extract_from_icons_atlas(&ui);
    // Bedrock 容器 UV 按 256/512 POT 资源；Java 常为 176×166 等，需垫到 POT
    pad_container_textures_to_pot(&ui);
}

/// Java 1.20+ 将 GUI 拆到 `ui/<name>/container.png`、`ui/heart/full.png` 等子目录。
/// Bedrock 读的是 `ui/furnace.png`、`ui/heart_full.png` 这类扁平名。
fn flatten_java120_ui_sprite_dirs(ui: &Path) {
    // 容器：ui/<id>/container.png → ui/<id>.png
    let containers = [
        "furnace",
        "blast_furnace",
        "smoker",
        "brewing_stand",
        "enchanting_table",
        "anvil",
        "grindstone",
        "stonecutter",
        "cartography_table",
        "smithing",
        "loom",
        "beacon",
        "horse",
        "inventory",
        "generic_54",
        "generic_53",
        "crafting_table",
        "dispenser",
        "hopper",
    ];
    let mut n = 0usize;
    for id in containers {
        let src = ui.join(id).join("container.png");
        let dst = ui.join(format!("{}.png", id));
        if src.is_file() && !dst.exists() {
            if fs::copy(&src, &dst).is_ok() {
                n += 1;
            }
        }
        // 英雄条等有时叫 <id>.png 在子目录里
        let alt = ui.join(id).join(format!("{}.png", id));
        if alt.is_file() && !dst.exists() {
            if fs::copy(&alt, &dst).is_ok() {
                n += 1;
            }
        }
    }

    // 心：ui/heart/{full,half,container}.png → heart_*
    let heart = ui.join("heart");
    if heart.is_dir() {
        let maps: &[(&str, &str)] = &[
            ("full.png", "heart_full.png"),
            ("half.png", "heart_half.png"),
            ("container.png", "heart_empty.png"),
            ("hardcore_full.png", "heart_full.png"),
        ];
        for (from, to) in maps {
            let src = heart.join(from);
            let dst = ui.join(to);
            if src.is_file() && !dst.exists() && fs::copy(&src, &dst).is_ok() {
                n += 1;
            }
        }
    }

    // 饥饿：Java food_* → Bedrock hunger_*
    let food_maps: &[(&str, &str)] = &[
        ("food_full.png", "hunger_effect_full.png"),
        ("food_half.png", "hunger_effect_half.png"),
        ("food_empty.png", "hunger_effect.png"),
        ("food_full_hunger.png", "hunger_effect_flash_full.png"),
        ("food_half_hunger.png", "hunger_effect_flash_half.png"),
    ];
    for (from, to) in food_maps {
        let src = ui.join(from);
        let dst = ui.join(to);
        if src.is_file() && !dst.exists() && fs::copy(&src, &dst).is_ok() {
            n += 1;
        }
    }

    // 清理已消费的 Java 子目录
    for id in containers {
        remove_dir_quiet(&ui.join(id));
    }
    remove_dir_quiet(&heart);

    if n > 0 {
        log_info!("OKAY bedrock [Java 1.20+ ui sprite dirs × {}]", n);
    }
}

/// 容器界面：箱子 / 工作台 / 熔炉 / 酿造 / 附魔 / 砂轮 / 铁砧 / 熔炉变体。
/// Java `gui/container/*.png` 扁平到 `ui/` 后，补齐 Bedrock 常用别名与缺失名。
fn adapt_container_screens(textures_dst: &Path, ui: &Path) {
    let _ = fs::create_dir_all(ui);
    // 仍从 gui/container、textures/container 兜底（扁平失败时）
    let mut container = ui.join("container");
    for dir in [
        textures_dst.join("gui").join("container"),
        textures_dst.join("container"),
        ui.join("container"),
    ] {
        if !dir.is_dir() {
            continue;
        }
        if dir != ui.join("container") {
            container = dir.clone();
        }
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = entry.file_name();
                let dst = ui.join(&name);
                if !dst.exists() {
                    let _ = fs::copy(&path, &dst);
                }
            }
        }
    }

    // Java 名 → 额外 Bedrock / 别名（仅在目标缺失时复制）
    let aliases: &[(&str, &[&str])] = &[
        // 大箱子
        ("generic_54.png", &["chest.png", "double_chest.png"]),
        // 小箱子有时叫 generic_53（漏斗矿车界面等，保留）
        ("generic_53.png", &["chest_small.png"]),
        // 熔炉族
        ("furnace.png", &["furnace_screen.png"]),
        ("blast_furnace.png", &["blast_furnace.png", "furnace_blast.png"]),
        ("smoker.png", &["smoker.png", "furnace_smoker.png"]),
        // 工作台 / 酿造 / 附魔
        ("crafting_table.png", &["crafting_table.png"]),
        ("brewing_stand.png", &["brewing_stand.png"]),
        ("enchanting_table.png", &["enchanting_table.png"]),
        // 砂轮 / 铁砧
        ("grindstone.png", &["grindstone.png"]),
        ("anvil.png", &["anvil.png"]),
        // 其它常用容器（有则带上）
        ("stonecutter.png", &["stonecutter.png"]),
        ("loom.png", &["loom.png"]),
        ("cartography_table.png", &["cartography_table.png"]),
        ("smithing_table.png", &["smithing_table.png"]),
        ("lectern.png", &["lectern.png"]),
        ("fletcher.png", &["fletcher.png"]),
    ];

    let mut n = 0usize;
    for (from, outs) in aliases {
        let src = ui.join(from);
        if !src.is_file() {
            continue;
        }
        for out in *outs {
            let dst = ui.join(out);
            if *out != *from && !dst.exists() && fs::copy(&src, &dst).is_ok() {
                n += 1;
            }
        }
    }
    // 源里若有 blast/smoker/grind 但 ui 下没有，再从 container 名匹配
    for name in [
        "blast_furnace.png",
        "smoker.png",
        "grindstone.png",
        "stonecutter.png",
        "smithing_table.png",
        "cartography_table.png",
        "loom.png",
    ] {
        if !ui.join(name).is_file() && container.join(name).is_file() {
            let _ = fs::copy(container.join(name), ui.join(name));
            n += 1;
        }
    }
    if n > 0 {
        log_info!("OKAY bedrock [container screens × {}]", n);
    }
}

/// 容器界面在 Bedrock 使用 2^n 尺寸（可用包为 256/512，也可能更大）。
/// 不按固定 176 判断：凡非 POT 的界面图，按**实际宽高**垫到 ≥256 的 POT；
/// 已是 POT 且 ≥256 的保留（兼容 HD 512/1024 包）。
fn pad_container_textures_to_pot(ui: &Path) {
    let Ok(entries) = fs::read_dir(ui) else { return };
    let mut n = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if !name.ends_with(".png") {
            continue;
        }
        // HUD 小图标不垫（hotbar 182×22、心/饥饿 9×9 等）
        if is_hud_icon_name(&name) {
            continue;
        }
        // 仅处理容器/面板类（含通用名与别名）
        if !is_container_texture_name(&name) {
            continue;
        }
        if pad_png_to_pot(&path).is_ok() {
            n += 1;
        }
    }
    if n > 0 {
        log_info!("OKAY bedrock [container pad-to-POT × {}]", n);
    }
}

fn is_hud_icon_name(name: &str) -> bool {
    const KEYS: &[&str] = &[
        "hotbar",
        "cross_hair",
        "crosshair",
        "heart_",
        "hunger_",
        "armor_full",
        "armor_half",
        "armor_empty",
        "experiencebar",
        "potion_overlay",
    ];
    KEYS.iter().any(|k| name.contains(k))
}

fn is_container_texture_name(name: &str) -> bool {
    const KEYS: &[&str] = &[
        "inventory",
        "generic_",
        "crafting",
        "furnace",
        "blast_furnace",
        "smoker",
        "brewing",
        "enchanting",
        "anvil",
        "grindstone",
        "stonecutter",
        "loom",
        "cartography",
        "smithing",
        "lectern",
        "fletcher",
        "beacon",
        "dispenser",
        "dropper",
        "hopper",
        "horse",
        "villager",
        "chest",
        "creative",
        "container",
    ];
    KEYS.iter().any(|k| name.contains(k))
}

fn next_pot(v: u32) -> u32 {
    let mut p = 1u32;
    while p < v {
        p = p.saturating_mul(2);
    }
    // 容器界面至少 256（Bedrock 常用基线）；更大的包自然用更大 POT
    p.max(256)
}

fn pad_png_to_pot(path: &Path) -> Result<(), String> {
    let img = image::open(path)
        .map_err(|e| format!("open {}: {}", path.display(), e))?
        .to_rgba8();
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return Ok(());
    }
    // 已是 POT 且 ≥256 → 通用 HD 包直接保留
    if w.is_power_of_two() && h.is_power_of_two() && w >= 256 && h >= 256 {
        return Ok(());
    }
    let tw = next_pot(w);
    let th = next_pot(h);
    if tw == w && th == h {
        return Ok(());
    }
    let mut out = RgbaImage::from_pixel(tw, th, image::Rgba([0, 0, 0, 0]));
    image::imageops::overlay(&mut out, &img, 0, 0);
    out.save(path).map_err(|e| format!("save {}: {}", path.display(), e))?;
    Ok(())
}

/// icons.png 缩放倍数：按实际宽度相对 256 计算，兼容 256/512/1024/2048 及非整数倍包。
fn icons_scale(width: u32) -> u32 {
    if width == 0 {
        return 1;
    }
    if width % 256 == 0 {
        return (width / 256).max(1);
    }
    ((width as f32 / 256.0).round() as u32).max(1)
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

/// Java 1.20+ sprites/hud/* → Bedrock ui/*。
/// 注意：调用时 gui 可能已合并进 ui，故同时扫 ui/sprites/hud 与 gui/sprites/hud。
fn copy_java_sprite_hud(textures_dst: &Path, ui: &Path) {
    let mut hud_dirs = vec![
        ui.join("sprites").join("hud"),
        textures_dst.join("gui").join("sprites").join("hud"),
    ];
    // 已扁平的 hud 文件也在 ui/ 根
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
    let mut copied = 0usize;
    for hud in &hud_dirs {
        if !hud.is_dir() {
            continue;
        }
        copy_dir_rename(hud, ui, map, &mut copied);
        merge_keep_names(hud, ui, &mut copied);
    }
    if copied > 0 {
        log_info!("OKAY bedrock [sprites/hud → ui × {}]", copied);
    }
    // 消费后清理
    remove_dir_quiet(&ui.join("sprites"));
    let _ = fs::remove_dir_all(textures_dst.join("gui").join("sprites"));
    hud_dirs.clear();
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
        let scale = icons_scale(w);
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

    #[test]
    fn test_container_screens_adapted() {
        let temp = tempdir().unwrap();
        let textures = temp.path();
        let container = textures.join("gui/container");
        let ui = textures.join("ui");
        fs::create_dir_all(&container).unwrap();
        for name in [
            "generic_54.png",
            "crafting_table.png",
            "furnace.png",
            "blast_furnace.png",
            "smoker.png",
            "brewing_stand.png",
            "enchanting_table.png",
            "grindstone.png",
            "anvil.png",
        ] {
            fs::write(container.join(name), name.as_bytes()).unwrap();
        }

        adapt_container_screens(textures, &ui);

        assert!(ui.join("generic_54.png").exists());
        assert!(ui.join("chest.png").exists(), "箱子别名");
        assert!(ui.join("crafting_table.png").exists());
        assert!(ui.join("furnace.png").exists());
        assert!(ui.join("blast_furnace.png").exists(), "熔炉变体");
        assert!(ui.join("smoker.png").exists(), "熔炉变体");
        assert!(ui.join("brewing_stand.png").exists());
        assert!(ui.join("enchanting_table.png").exists());
        assert!(ui.join("grindstone.png").exists());
        assert!(ui.join("anvil.png").exists());
    }

    #[test]
    fn test_pad_container_to_pot() {
        let temp = tempdir().unwrap();
        let ui = temp.path().join("ui");
        fs::create_dir_all(&ui).unwrap();
        // 模拟 Java 176×166 背包
        let img = RgbaImage::from_pixel(176, 166, image::Rgba([10, 20, 30, 255]));
        img.save(ui.join("inventory.png")).unwrap();
        // 已是 512 的不改
        let big = RgbaImage::from_pixel(512, 512, image::Rgba([1, 2, 3, 255]));
        big.save(ui.join("furnace.png")).unwrap();
        // hotbar 182×22 不应被 pad
        let hb = RgbaImage::from_pixel(182, 22, image::Rgba([9, 9, 9, 255]));
        hb.save(ui.join("hotbar.png")).unwrap();

        pad_container_textures_to_pot(&ui);

        let inv = image::open(ui.join("inventory.png")).unwrap();
        assert_eq!(inv.width(), 256);
        assert_eq!(inv.height(), 256);
        let fur = image::open(ui.join("furnace.png")).unwrap();
        assert_eq!(fur.width(), 512);
        let hot = image::open(ui.join("hotbar.png")).unwrap();
        assert_eq!(hot.width(), 182);
        assert_eq!(hot.height(), 22);
    }

    #[test]
    fn test_generic_scale_and_pad_various_sizes() {
        // 384 宽（1.5×256）→ scale 取整为 2；非 176 整数倍也可处理
        assert_eq!(icons_scale(256), 1);
        assert_eq!(icons_scale(512), 2);
        assert_eq!(icons_scale(1024), 4);
        assert_eq!(icons_scale(2048), 8);
        assert_eq!(icons_scale(384), 2); // round(384/256)=1.5→2
        assert_eq!(icons_scale(128), 1);

        let temp = tempdir().unwrap();
        let ui = temp.path().join("ui");
        fs::create_dir_all(&ui).unwrap();
        // 2x Java 背包 352×332 → 512 POT
        let img2 = RgbaImage::from_pixel(352, 332, image::Rgba([5, 5, 5, 255]));
        img2.save(ui.join("inventory.png")).unwrap();
        // 已是 1024 的 HD 容器保留
        let hd = RgbaImage::from_pixel(1024, 1024, image::Rgba([8, 8, 8, 255]));
        hd.save(ui.join("furnace.png")).unwrap();
        // 非容器文件不处理
        let other = RgbaImage::from_pixel(100, 100, image::Rgba([1, 1, 1, 255]));
        other.save(ui.join("custom_panel.png")).unwrap();

        pad_container_textures_to_pot(&ui);

        let inv = image::open(ui.join("inventory.png")).unwrap();
        assert_eq!(inv.width(), 512);
        assert_eq!(inv.height(), 512);
        let fur = image::open(ui.join("furnace.png")).unwrap();
        assert_eq!(fur.width(), 1024);
        let custom = image::open(ui.join("custom_panel.png")).unwrap();
        assert_eq!(custom.width(), 100, "非容器名不垫 POT");
    }

    #[test]
    fn test_flatten_heart_and_food_and_furnace_dir() {
        let temp = tempdir().unwrap();
        let textures = temp.path();
        let ui = textures.join("ui");
        fs::create_dir_all(ui.join("heart")).unwrap();
        fs::create_dir_all(ui.join("furnace")).unwrap();
        fs::write(ui.join("heart/full.png"), b"hf").unwrap();
        fs::write(ui.join("heart/half.png"), b"hh").unwrap();
        fs::write(ui.join("heart/container.png"), b"he").unwrap();
        fs::write(ui.join("furnace/container.png"), b"fu").unwrap();
        fs::write(ui.join("food_full.png"), b"ff").unwrap();
        fs::write(ui.join("food_half.png"), b"fh").unwrap();
        fs::write(ui.join("food_empty.png"), b"fe").unwrap();

        flatten_java120_ui_sprite_dirs(&ui);

        assert!(ui.join("heart_full.png").exists(), "血量满心");
        assert!(ui.join("heart_half.png").exists(), "血量半心");
        assert!(ui.join("heart_empty.png").exists(), "血量空心容器");
        assert!(ui.join("hunger_effect_full.png").exists(), "饱食度");
        assert!(ui.join("hunger_effect_half.png").exists());
        assert!(ui.join("hunger_effect.png").exists());
        assert!(ui.join("furnace.png").exists(), "熔炉容器");
        assert!(!ui.join("heart").exists());
        assert!(!ui.join("furnace").exists());
    }
}
