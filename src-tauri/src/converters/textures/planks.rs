use std::fs;
use std::path::Path;

use crate::converters::color::hue::adjust_hue_brightness;

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
}
