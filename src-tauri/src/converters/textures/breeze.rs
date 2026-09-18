//! 1.21 Tricky Trials / 旋风系贴图生成（升级旧资源包时补齐）。
//!
//! 依据 Minecraft 1.21.11 原版资源均色，从包内已有贴图 recolor/force：
//! - 好做：breeze_rod / wind_charge / trial_key / spawn_egg / copper_bulb / 效果图标
//! - 中等：heavy_core / breeze 实体 / flow trim / ominous_bottle
//! - 不做：mace、trial_spawner、vault、crafter、粒子（轮廓独特，recolor 会穿帮）
//!
//! 目标文件已存在时跳过，不覆盖玩家/原版自定义。

use std::fs;
use std::path::Path;

use crate::converters::color::hue::{adjust_hue_brightness, force_hue_saturation};

enum Tint {
    /// 相对色相偏移（源必须有一定饱和度）
    Shift { h: f32, b: f32, s: f32 },
    /// 钉色相（源近灰时用）
    Force { hue: f32, sat: f32, v_min: f32, v_max: f32 },
}

fn recolor_skip_existing(
    pack: &Path,
    src_rel: &str,
    dst_rel: &str,
    tint: Tint,
) -> Result<(), String> {
    let src = pack.join(src_rel);
    if !src.exists() {
        return Ok(());
    }
    let dst = pack.join(dst_rel);
    if dst.exists() {
        return Ok(());
    }
    if let Some(p) = dst.parent() {
        fs::create_dir_all(p).map_err(|e| format!("mkdir {}: {}", p.display(), e))?;
    }
    fs::copy(&src, &dst)
        .map_err(|e| format!("copy {} -> {}: {}", src.display(), dst.display(), e))?;
    let img = image::open(&dst)
        .map_err(|e| format!("open {}: {}", dst.display(), e))?
        .to_rgba8();
    let out = match tint {
        Tint::Shift { h, b, s } => adjust_hue_brightness(img, h, b, s),
        Tint::Force {
            hue,
            sat,
            v_min,
            v_max,
        } => force_hue_saturation(img, hue, 6.0, sat, v_min, v_max),
    };
    out.save(&dst)
        .map_err(|e| format!("save {}: {}", dst.display(), e))?;
    let mcmeta = src.with_extension("png.mcmeta");
    if mcmeta.exists() {
        let _ = fs::copy(&mcmeta, dst.with_extension("png.mcmeta"));
    }
    Ok(())
}

/// 1.21 旋风系：从旧包已有材质近似生成。源缺失则静默跳过。
pub fn generate_tricky_trials_breeze(resource_pack_path: &Path) -> Result<(), String> {
    let pack = resource_pack_path;

    // ── 第 1 批：好做 ──
    // 旋风棒：烈焰棒（橙黄）→ 蓝灰棍（原版均色 ~#6D79A9）
    recolor_skip_existing(
        pack,
        "assets/minecraft/textures/item/blaze_rod.png",
        "assets/minecraft/textures/item/breeze_rod.png",
        Tint::Shift {
            h: 185.0,
            b: -22.0,
            s: -18.0,
        },
    )?;

    // 风弹：雪球 → 淡蓝球（~#BFC6DA）
    recolor_skip_existing(
        pack,
        "assets/minecraft/textures/item/snowball.png",
        "assets/minecraft/textures/item/wind_charge.png",
        Tint::Force {
            hue: 222.0,
            sat: 0.14,
            v_min: 0.55,
            v_max: 0.95,
        },
    )?;

    // 风弹投掷物（entity 粒子/snowball 同源）
    recolor_skip_existing(
        pack,
        "assets/minecraft/textures/entity/snowball.png",
        "assets/minecraft/textures/entity/projectiles/wind_charge.png",
        Tint::Force {
            hue: 222.0,
            sat: 0.14,
            v_min: 0.5,
            v_max: 0.95,
        },
    )?;

    // 试炼钥匙：无更早钥匙，用金锭轮廓近似铜棕（~#725142）
    recolor_skip_existing(
        pack,
        "assets/minecraft/textures/item/gold_ingot.png",
        "assets/minecraft/textures/item/trial_key.png",
        Tint::Force {
            hue: 22.0,
            sat: 0.38,
            v_min: 0.28,
            v_max: 0.62,
        },
    )?;

    // 不祥试炼钥匙：在 trial_key 之上压暗偏绿
    let ominous_src = if pack
        .join("assets/minecraft/textures/item/trial_key.png")
        .exists()
    {
        "assets/minecraft/textures/item/trial_key.png"
    } else {
        "assets/minecraft/textures/item/gold_ingot.png"
    };
    recolor_skip_existing(
        pack,
        ominous_src,
        "assets/minecraft/textures/item/ominous_trial_key.png",
        Tint::Force {
            hue: 160.0,
            sat: 0.16,
            v_min: 0.18,
            v_max: 0.45,
        },
    )?;

    // 旋风刷怪蛋：任意已有刷怪蛋 → 蓝灰
    for egg in [
        "chicken_spawn_egg.png",
        "spider_spawn_egg.png",
        "cow_spawn_egg.png",
        "creeper_spawn_egg.png",
    ] {
        let rel = format!("assets/minecraft/textures/item/{}", egg);
        if pack.join(&rel).exists() {
            recolor_skip_existing(
                pack,
                &rel,
                "assets/minecraft/textures/item/breeze_spawn_egg.png",
                Tint::Force {
                    hue: 230.0,
                    sat: 0.28,
                    v_min: 0.35,
                    v_max: 0.75,
                },
            )?;
            break;
        }
    }

    // 风充能状态图标
    for src in [
        "assets/minecraft/textures/mob_effect/speed.png",
        "assets/minecraft/textures/mob_effect/jump_boost.png",
        "assets/minecraft/textures/mob_effect/absorption.png",
    ] {
        if pack.join(src).exists() {
            recolor_skip_existing(
                pack,
                src,
                "assets/minecraft/textures/mob_effect/wind_charged.png",
                Tint::Force {
                    hue: 220.0,
                    sat: 0.28,
                    v_min: 0.45,
                    v_max: 0.9,
                },
            )?;
            break;
        }
    }

    // 铜灯泡族：红石灯 / 铜块 → copper_bulb*（含氧化与蜡变体）
    // 熄灭：偏铜；点亮：铜 + 灯芯亮部
    let lamp_off = [
        "assets/minecraft/textures/block/redstone_lamp.png",
        "assets/minecraft/textures/block/redstone_lamp_off.png",
    ];
    let lamp_on = [
        "assets/minecraft/textures/block/redstone_lamp_on.png",
        "assets/minecraft/textures/block/redstone_lamp.png",
    ];
    let copper_base = [
        "assets/minecraft/textures/block/copper_block.png",
        "assets/minecraft/textures/block/cut_copper.png",
    ];

    let mut off_src: Option<&str> = None;
    for s in lamp_off {
        if pack.join(s).exists() {
            off_src = Some(s);
            break;
        }
    }
    if off_src.is_none() {
        for s in copper_base {
            if pack.join(s).exists() {
                off_src = Some(s);
                break;
            }
        }
    }
    if let Some(src) = off_src {
        for dst in [
            "copper_bulb.png",
            "copper_bulb_powered.png",
            "exposed_copper_bulb.png",
            "exposed_copper_bulb_powered.png",
            "weathered_copper_bulb.png",
            "weathered_copper_bulb_powered.png",
            "oxidized_copper_bulb.png",
            "oxidized_copper_bulb_powered.png",
            "waxed_copper_bulb.png",
            "waxed_exposed_copper_bulb.png",
            "waxed_weathered_copper_bulb.png",
            "waxed_oxidized_copper_bulb.png",
        ] {
            let (hue, sat, vmin, vmax) = match dst {
                d if d.contains("oxidized") => (145.0, 0.22, 0.25, 0.55),
                d if d.contains("weathered") => (120.0, 0.28, 0.3, 0.6),
                d if d.contains("exposed") => (35.0, 0.32, 0.35, 0.65),
                _ => (18.0, 0.42, 0.35, 0.7),
            };
            recolor_skip_existing(
                pack,
                src,
                &format!("assets/minecraft/textures/block/{}", dst),
                Tint::Force {
                    hue,
                    sat,
                    v_min: vmin,
                    v_max: vmax,
                },
            )?;
        }
    }

    let mut on_src: Option<&str> = None;
    for s in lamp_on {
        if pack.join(s).exists() {
            on_src = Some(s);
            break;
        }
    }
    if let Some(src) = on_src {
        for dst in [
            "copper_bulb_lit.png",
            "copper_bulb_lit_powered.png",
            "exposed_copper_bulb_lit.png",
            "exposed_copper_bulb_lit_powered.png",
            "weathered_copper_bulb_lit.png",
            "weathered_copper_bulb_lit_powered.png",
            "oxidized_copper_bulb_lit.png",
            "oxidized_copper_bulb_lit_powered.png",
            "waxed_copper_bulb_lit.png",
            "waxed_exposed_copper_bulb_lit.png",
            "waxed_weathered_copper_bulb_lit.png",
            "waxed_oxidized_copper_bulb_lit.png",
        ] {
            let (hue, sat, vmin, vmax) = match dst {
                d if d.contains("oxidized") => (145.0, 0.2, 0.45, 0.95),
                d if d.contains("weathered") => (120.0, 0.25, 0.5, 0.95),
                d if d.contains("exposed") => (35.0, 0.3, 0.55, 0.98),
                _ => (18.0, 0.38, 0.55, 0.98),
            };
            recolor_skip_existing(
                pack,
                src,
                &format!("assets/minecraft/textures/block/{}", dst),
                Tint::Force {
                    hue,
                    sat,
                    v_min: vmin,
                    v_max: vmax,
                },
            )?;
        }
    }

    // ── 第 2 批：中等 ──
    // 重核方块：铁块/深板岩 → 深灰金属（~#52565E）
    for src in [
        "assets/minecraft/textures/block/iron_block.png",
        "assets/minecraft/textures/block/deepslate.png",
        "assets/minecraft/textures/block/polished_deepslate.png",
    ] {
        if pack.join(src).exists() {
            recolor_skip_existing(
                pack,
                src,
                "assets/minecraft/textures/block/heavy_core.png",
                Tint::Force {
                    hue: 220.0,
                    sat: 0.1,
                    v_min: 0.22,
                    v_max: 0.48,
                },
            )?;
            break;
        }
    }

    // 旋风实体贴图：烈焰人 → 蓝紫旋风
    for (src, dst) in [
        (
            "assets/minecraft/textures/entity/blaze.png",
            "assets/minecraft/textures/entity/breeze/breeze.png",
        ),
        (
            "assets/minecraft/textures/entity/blaze_blaze.png",
            "assets/minecraft/textures/entity/breeze/breeze_eyes.png",
        ),
    ] {
        recolor_skip_existing(
            pack,
            src,
            dst,
            Tint::Force {
                hue: 235.0,
                sat: 0.26,
                v_min: 0.35,
                v_max: 0.85,
            },
        )?;
    }

    // flow 盔甲纹饰模板：从已有纹饰模板染蓝紫
    for src in [
        "assets/minecraft/textures/item/armor_trim_smithing_template.png",
        "assets/minecraft/textures/item/silence_armor_trim_smithing_template.png",
        "assets/minecraft/textures/item/bolt_armor_trim_smithing_template.png",
        "assets/minecraft/textures/item/dune_armor_trim_smithing_template.png",
    ] {
        if pack.join(src).exists() {
            recolor_skip_existing(
                pack,
                src,
                "assets/minecraft/textures/item/flow_armor_trim_smithing_template.png",
                Tint::Force {
                    hue: 225.0,
                    sat: 0.3,
                    v_min: 0.35,
                    v_max: 0.8,
                },
            )?;
            break;
        }
    }

    // 不祥之瓶：药水瓶 → 深色不祥
    for src in [
        "assets/minecraft/textures/item/potion.png",
        "assets/minecraft/textures/item/splash_potion.png",
        "assets/minecraft/textures/item/awkward_potion.png",
    ] {
        if pack.join(src).exists() {
            recolor_skip_existing(
                pack,
                src,
                "assets/minecraft/textures/item/ominous_bottle.png",
                Tint::Force {
                    hue: 280.0,
                    sat: 0.22,
                    v_min: 0.2,
                    v_max: 0.5,
                },
            )?;
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;
    use tempfile::tempdir;

    fn write_png(path: &Path, rgb: [u8; 3]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut img = RgbaImage::new(4, 4);
        for p in img.pixels_mut() {
            *p = image::Rgba([rgb[0], rgb[1], rgb[2], 255]);
        }
        img.save(path).unwrap();
    }

    #[test]
    fn test_generate_breeze_rod_from_blaze() {
        let temp = tempdir().unwrap();
        let pack = temp.path();
        write_png(
            &pack.join("assets/minecraft/textures/item/blaze_rod.png"),
            [240, 180, 40],
        );
        generate_tricky_trials_breeze(pack).unwrap();
        assert!(pack
            .join("assets/minecraft/textures/item/breeze_rod.png")
            .is_file());
    }

    #[test]
    fn test_skip_existing_target() {
        let temp = tempdir().unwrap();
        let pack = temp.path();
        write_png(
            &pack.join("assets/minecraft/textures/item/blaze_rod.png"),
            [240, 180, 40],
        );
        write_png(
            &pack.join("assets/minecraft/textures/item/breeze_rod.png"),
            [1, 2, 3],
        );
        generate_tricky_trials_breeze(pack).unwrap();
        let img = image::open(pack.join("assets/minecraft/textures/item/breeze_rod.png"))
            .unwrap()
            .to_rgba8();
        assert_eq!(img.get_pixel(0, 0)[0], 1);
    }

    #[test]
    fn test_copper_bulb_from_lamp() {
        let temp = tempdir().unwrap();
        let pack = temp.path();
        write_png(
            &pack.join("assets/minecraft/textures/block/redstone_lamp.png"),
            [80, 80, 80],
        );
        write_png(
            &pack.join("assets/minecraft/textures/block/redstone_lamp_on.png"),
            [220, 180, 80],
        );
        generate_tricky_trials_breeze(pack).unwrap();
        assert!(pack
            .join("assets/minecraft/textures/block/copper_bulb.png")
            .is_file());
        assert!(pack
            .join("assets/minecraft/textures/block/copper_bulb_lit.png")
            .is_file());
        assert!(pack
            .join("assets/minecraft/textures/block/oxidized_copper_bulb.png")
            .is_file());
    }
}
