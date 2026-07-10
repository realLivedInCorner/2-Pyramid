use std::fs;
use std::path::Path;

use crate::converters::color_utils::{apply_netherite_transform, apply_spectral_arrow_transform};

pub fn generate_netherite_block(resource_pack_path: &Path) -> Result<(), String> {
    let block_path = resource_pack_path.join("assets/minecraft/textures/block");
    let diamond_path = block_path.join("diamond_block.png");
    let netherite_path = block_path.join("netherite_block.png");
    if !diamond_path.exists() {
        return Ok(());
    }

    fs::copy(&diamond_path, &netherite_path)
        .map_err(|e| format!("failed to copy diamond_block.png: {}", e))?;

    let mut img = image::open(&netherite_path)
        .map_err(|e| format!("failed to open {}: {}", netherite_path.display(), e))?
        .to_rgba8();
    apply_netherite_transform(&mut img);
    img.save(&netherite_path)
        .map_err(|e| format!("failed to save {}: {}", netherite_path.display(), e))?;

    let mcmeta = diamond_path.with_extension("png.mcmeta");
    if mcmeta.exists() {
        let _ = fs::copy(&mcmeta, netherite_path.with_extension("png.mcmeta"));
    }

    Ok(())
}

pub fn generate_netherite_ingot(resource_pack_path: &Path) -> Result<(), String> {
    let item_path = resource_pack_path.join("assets/minecraft/textures/item");
    let gold_path = item_path.join("gold_ingot.png");
    let netherite_path = item_path.join("netherite_ingot.png");
    if !gold_path.exists() {
        return Ok(());
    }

    fs::copy(&gold_path, &netherite_path)
        .map_err(|e| format!("failed to copy gold_ingot.png: {}", e))?;

    let mut img = image::open(&netherite_path)
        .map_err(|e| format!("failed to open {}: {}", netherite_path.display(), e))?
        .to_rgba8();
    apply_netherite_transform(&mut img);
    img.save(&netherite_path)
        .map_err(|e| format!("failed to save {}: {}", netherite_path.display(), e))?;

    let mcmeta = gold_path.with_extension("png.mcmeta");
    if mcmeta.exists() {
        let _ = fs::copy(&mcmeta, netherite_path.with_extension("png.mcmeta"));
    }

    Ok(())
}

pub fn generate_netherite_tools(resource_pack_path: &Path) -> Result<(), String> {
    let items_path = resource_pack_path.join("assets/minecraft/textures/item");
    let items = [
        "diamond_sword",
        "diamond_helmet",
        "diamond_chestplate",
        "diamond_leggings",
        "diamond_boots",
        "diamond_axe",
        "diamond_pickaxe",
        "diamond_shovel",
        "diamond_hoe",
    ];

    for item in items {
        let original_path = items_path.join(format!("{}.png", item));
        let new_path = items_path.join(format!("netherite_{}.png", &item[8..]));
        if !original_path.exists() {
            continue;
        }

        fs::copy(&original_path, &new_path)
            .map_err(|e| format!("failed to copy {}: {}", original_path.display(), e))?;
        let mut img = image::open(&new_path)
            .map_err(|e| format!("failed to open {}: {}", new_path.display(), e))?
            .to_rgba8();
        apply_netherite_transform(&mut img);
        img.save(&new_path)
            .map_err(|e| format!("failed to save {}: {}", new_path.display(), e))?;

        let mcmeta = original_path.with_extension("png.mcmeta");
        if mcmeta.exists() {
            let _ = fs::copy(&mcmeta, new_path.with_extension("png.mcmeta"));
        }
    }

    let arrow_path = items_path.join("arrow.png");
    if arrow_path.exists() {
        let spectral_path = items_path.join("spectral_arrow.png");
        fs::copy(&arrow_path, &spectral_path)
            .map_err(|e| format!("failed to copy arrow.png: {}", e))?;
        let mut img = image::open(&spectral_path)
            .map_err(|e| format!("failed to open {}: {}", spectral_path.display(), e))?
            .to_rgba8();
        apply_spectral_arrow_transform(&mut img);
        img.save(&spectral_path)
            .map_err(|e| format!("failed to save {}: {}", spectral_path.display(), e))?;

        let mcmeta = arrow_path.with_extension("png.mcmeta");
        if mcmeta.exists() {
            let _ = fs::copy(&mcmeta, spectral_path.with_extension("png.mcmeta"));
        }
    }

    Ok(())
}

pub fn generate_netherite_armor_models(resource_pack_path: &Path) -> Result<(), String> {
    let armor_path = resource_pack_path.join("assets/minecraft/textures/models/armor");
    let files = ["diamond_layer_1.png", "diamond_layer_2.png"];

    for file in files {
        let original_path = armor_path.join(file);
        let new_path = armor_path.join(file.replace("diamond", "netherite"));
        if !original_path.exists() {
            continue;
        }

        fs::copy(&original_path, &new_path)
            .map_err(|e| format!("failed to copy {}: {}", original_path.display(), e))?;
        let mut img = image::open(&new_path)
            .map_err(|e| format!("failed to open {}: {}", new_path.display(), e))?
            .to_rgba8();
        apply_netherite_transform(&mut img);
        img.save(&new_path)
            .map_err(|e| format!("failed to save {}: {}", new_path.display(), e))?;
    }

    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "generate_netherite_block",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_netherite_block(context.temp_dir()),
    );
    engine.register_task(
        "generate_netherite_ingot",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_netherite_ingot(context.temp_dir()),
    );
    engine.register_task(
        "generate_netherite_tools",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_netherite_tools(context.temp_dir()),
    );
    engine.register_task(
        "generate_netherite_armor_models",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_netherite_armor_models(context.temp_dir()),
    );
}
