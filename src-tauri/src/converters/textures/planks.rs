use std::fs;
use std::path::Path;

use crate::converters::color::hue::{adjust_hue_brightness, force_hue_saturation};

fn process_block_image(blocks_path: &Path, source: &str, target: &str, hue_shift: f32, brightness: f32, saturation: f32) -> Result<(), String> {
    let source_path = blocks_path.join(source);
    if !source_path.exists() {
        return Ok(());
    }

    let target_path = blocks_path.join(target);
    fs::copy(&source_path, &target_path)
        .map_err(|e| format!("failed to copy {}: {}", source_path.display(), e))?;

    let img = image::open(&target_path)
        .map_err(|e| format!("failed to open {}: {}", target_path.display(), e))?
        .to_rgba8();
    let adjusted = adjust_hue_brightness(img, hue_shift, brightness, saturation);
    adjusted
        .save(&target_path)
        .map_err(|e| format!("failed to save {}: {}", target_path.display(), e))?;

    let mcmeta = source_path.with_extension("png.mcmeta");
    if mcmeta.exists() {
        let _ = fs::copy(&mcmeta, target_path.with_extension("png.mcmeta"));
    }

    Ok(())
}

/// 任意子目录下的 recolor（source/target 相对 pack 根）。
fn recolor_rel(pack: &Path, source: &str, target: &str, h: f32, b: f32, s: f32) -> Result<(), String> {
    let src = pack.join(source);
    if !src.exists() {
        return Ok(());
    }
    let dst = pack.join(target);
    if let Some(p) = dst.parent() {
        fs::create_dir_all(p).map_err(|e| format!("mkdir {}: {}", p.display(), e))?;
    }
    fs::copy(&src, &dst).map_err(|e| format!("copy {} -> {}: {}", src.display(), dst.display(), e))?;
    let img = image::open(&dst)
        .map_err(|e| format!("open {}: {}", dst.display(), e))?
        .to_rgba8();
    adjust_hue_brightness(img, h, b, s)
        .save(&dst)
        .map_err(|e| format!("save {}: {}", dst.display(), e))?;
    let mcmeta = src.with_extension("png.mcmeta");
    if mcmeta.exists() {
        let _ = fs::copy(&mcmeta, dst.with_extension("png.mcmeta"));
    }
    Ok(())
}

/// 1.19 mangrove / 1.20 cherry+bamboo：从橡木系色相生成（旧 1.8 包无这些文件）。
/// 26.3 未发布木种不在此加入。
pub fn generate_redwood_cherry_bamboo_planks(resource_pack_path: &Path) -> Result<(), String> {
    let blocks_path = resource_pack_path.join("assets/minecraft/textures/block");
    process_block_image(&blocks_path, "oak_planks.png", "mangrove_planks.png", -59.0, -15.0, 0.0)?;
    // 樱花：原版是浅粉（约 H348° / 低饱和）。旧值 -80° 偏品红、观感偏深。
    process_block_image(&blocks_path, "oak_planks.png", "cherry_planks.png", -45.0, 45.0, -18.0)?;
    process_block_image(&blocks_path, "oak_planks.png", "bamboo_planks.png", 25.0, 20.0, 0.0)?;

    // 原木 / 竹块 / 竹马赛克
    process_block_image(&blocks_path, "oak_log.png", "mangrove_log.png", -59.0, -15.0, 0.0)?;
    process_block_image(&blocks_path, "oak_log_top.png", "mangrove_log_top.png", -59.0, -15.0, 0.0)?;
    process_block_image(&blocks_path, "oak_log.png", "cherry_log.png", -45.0, 45.0, -18.0)?;
    process_block_image(&blocks_path, "oak_log_top.png", "cherry_log_top.png", -45.0, 45.0, -18.0)?;
    process_block_image(&blocks_path, "oak_log.png", "bamboo_block.png", 25.0, 20.0, 0.0)?;
    process_block_image(&blocks_path, "oak_log_top.png", "bamboo_block_top.png", 25.0, 20.0, 0.0)?;
    process_block_image(&blocks_path, "oak_planks.png", "bamboo_mosaic.png", 25.0, 15.0, 0.0)?;
    Ok(())
}

/// 1.21.4 pale oak（苍白橡木）
pub fn generate_pale_planks(resource_pack_path: &Path) -> Result<(), String> {
    let blocks_path = resource_pack_path.join("assets/minecraft/textures/block");
    process_block_image(&blocks_path, "oak_planks.png", "pale_oak_planks.png", 0.0, 30.0, -100.0)?;
    process_block_image(&blocks_path, "oak_log.png", "pale_oak_log.png", 0.0, 30.0, -100.0)?;
    process_block_image(&blocks_path, "oak_log_top.png", "pale_oak_log_top.png", 0.0, 30.0, -100.0)?;
    Ok(())
}

/// 26.3 poplar（杨树）全套：原木/木板/去皮 + 家具 + 近似树叶。
/// 源必须是 1.8 已有木种（橡木 / 丛林木）。
///
/// 色相参数来自原版 16x 与 1.8 木种的 HSV 均值差：
/// - 木板 / 横截面：丛林木（H 已接近，dS≈-35）
/// - 树皮 / 去皮：橡木（白桦皮几乎全白，不可用）
/// - 家具（门/活板门/告示牌/船/shelf/树苗）：与木板同一套 jungle 参数
/// - 树叶：橡木叶几乎无饱和，强行拉高 S 并钉色相（近似，非原版落叶纹理）
pub fn generate_poplar_planks(resource_pack_path: &Path) -> Result<(), String> {
    let blocks_path = resource_pack_path.join("assets/minecraft/textures/block");
    // ── 原木 / 木板 ──
    process_block_image(&blocks_path, "jungle_planks.png", "poplar_planks.png", -1.0, -4.0, -35.0)?;
    process_block_image(&blocks_path, "jungle_log_top.png", "poplar_log_top.png", -3.0, -5.0, -33.0)?;
    process_block_image(&blocks_path, "oak_log.png", "poplar_log.png", -7.0, -12.0, -8.0)?;
    process_block_image(&blocks_path, "stripped_oak_log.png", "stripped_poplar_log.png", -7.0, -5.0, -36.0)?;
    process_block_image(&blocks_path, "stripped_oak_log_top.png", "stripped_poplar_log_top.png", -10.0, -3.0, -37.0)?;

    // ── 家具：与木板同一套 jungle 参数 ──
    // 门（优先 jungle，回退 oak）
    for (src, dst) in [
        ("jungle_door_top.png", "poplar_door_top.png"),
        ("jungle_door_bottom.png", "poplar_door_bottom.png"),
        ("jungle_trapdoor.png", "poplar_trapdoor.png"),
        ("jungle_shelf.png", "poplar_shelf.png"),
        ("jungle_sapling.png", "poplar_sapling.png"),
    ] {
        if blocks_path.join(src).exists() {
            process_block_image(&blocks_path, src, dst, -1.0, -4.0, -35.0)?;
        } else {
            let oak = src.replace("jungle_", "oak_");
            process_block_image(&blocks_path, &oak, dst, -7.0, -5.0, -36.0)?;
        }
    }

    // 物品图标 / 实体船：优先 jungle，缺失再 oak（避免 oak 覆盖）
    let prefer_jungle_then_oak: &[(&str, &str, &str, &str)] = &[
        ("assets/minecraft/textures/item/jungle_sign.png", "assets/minecraft/textures/item/oak_sign.png", "assets/minecraft/textures/item/poplar_sign.png", "item"),
        ("assets/minecraft/textures/item/jungle_hanging_sign.png", "assets/minecraft/textures/item/oak_hanging_sign.png", "assets/minecraft/textures/item/poplar_hanging_sign.png", "item"),
        ("assets/minecraft/textures/item/jungle_door.png", "assets/minecraft/textures/item/oak_door.png", "assets/minecraft/textures/item/poplar_door.png", "item"),
        ("assets/minecraft/textures/item/jungle_boat.png", "assets/minecraft/textures/item/oak_boat.png", "assets/minecraft/textures/item/poplar_boat.png", "item"),
        ("assets/minecraft/textures/item/jungle_chest_boat.png", "assets/minecraft/textures/item/oak_chest_boat.png", "assets/minecraft/textures/item/poplar_chest_boat.png", "item"),
        ("assets/minecraft/textures/block/jungle_sign.png", "assets/minecraft/textures/block/oak_sign.png", "assets/minecraft/textures/block/poplar_sign.png", "block"),
        ("assets/minecraft/textures/block/jungle_hanging_sign.png", "assets/minecraft/textures/block/oak_hanging_sign.png", "assets/minecraft/textures/block/poplar_hanging_sign.png", "block"),
        ("assets/minecraft/textures/entity/boat/jungle.png", "assets/minecraft/textures/entity/boat/oak.png", "assets/minecraft/textures/entity/boat/poplar.png", "entity"),
        ("assets/minecraft/textures/entity/chest_boat/jungle.png", "assets/minecraft/textures/entity/chest_boat/oak.png", "assets/minecraft/textures/entity/chest_boat/poplar.png", "entity"),
    ];
    for (jungle, oak, target, _kind) in prefer_jungle_then_oak {
        if resource_pack_path.join(jungle).exists() {
            recolor_rel(resource_pack_path, jungle, target, -1.0, -4.0, -35.0)?;
        } else if resource_pack_path.join(oak).exists() {
            recolor_rel(resource_pack_path, oak, target, -7.0, -5.0, -36.0)?;
        }
    }

    // ── 树叶：钉色相，禁止相对 shift（橡木叶近灰，shift 会漂成蓝紫）──
    // 原版参考：红 ~8° / 橙 ~28° / 黄 ~45°
    let leaves = |src: &str, dst: &str, hue: f32, sat: f32| -> Result<(), String> {
        let source_path = blocks_path.join(src);
        if !source_path.exists() {
            return Ok(());
        }
        let target_path = blocks_path.join(dst);
        let img = image::open(&source_path)
            .map_err(|e| format!("open {}: {}", source_path.display(), e))?
            .to_rgba8();
        force_hue_saturation(img, hue, 6.0, sat, 0.22, 0.88)
            .save(&target_path)
            .map_err(|e| format!("save {}: {}", target_path.display(), e))?;
        let mcmeta = source_path.with_extension("png.mcmeta");
        if mcmeta.exists() {
            let _ = fs::copy(&mcmeta, target_path.with_extension("png.mcmeta"));
        }
        Ok(())
    };
    leaves("oak_leaves.png", "red_poplar_leaves.png", 8.0, 0.72)?;
    leaves("oak_leaves.png", "orange_poplar_leaves.png", 28.0, 0.80)?;
    leaves("oak_leaves.png", "yellow_poplar_leaves.png", 45.0, 0.78)?;

    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "generate_redwood_cherry_bamboo_planks",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_redwood_cherry_bamboo_planks(context.temp_dir()),
    );
    engine.register_task(
        "generate_pale_planks",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_pale_planks(context.temp_dir()),
    );
    engine.register_task(
        "generate_poplar_planks",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_poplar_planks(context.temp_dir()),
    );
}
