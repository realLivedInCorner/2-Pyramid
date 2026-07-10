use std::fs;
use std::path::Path;

use crate::converters::color_utils::adjust_copper_color;

pub fn generate_copper_ingot(resource_pack_path: &Path) -> Result<(), String> {
    let item_path = resource_pack_path.join("assets/minecraft/textures/item");
    let iron_path = item_path.join("iron_ingot.png");
    let copper_path = item_path.join("copper_ingot.png");
    if !iron_path.exists() {
        return Ok(());
    }

    fs::copy(&iron_path, &copper_path)
        .map_err(|e| format!("failed to copy iron_ingot.png: {}", e))?;
    let mut img = image::open(&copper_path)
        .map_err(|e| format!("failed to open {}: {}", copper_path.display(), e))?
        .to_rgba8();
    adjust_copper_color(&mut img);
    img.save(&copper_path)
        .map_err(|e| format!("failed to save {}: {}", copper_path.display(), e))?;

    let mcmeta = iron_path.with_extension("png.mcmeta");
    if mcmeta.exists() {
        let _ = fs::copy(&mcmeta, copper_path.with_extension("png.mcmeta"));
    }

    Ok(())
}

pub fn generate_copper_block(resource_pack_path: &Path) -> Result<(), String> {
    let block_path = resource_pack_path.join("assets/minecraft/textures/block");
    let iron_path = block_path.join("iron_block.png");
    if !iron_path.exists() {
        return Ok(());
    }

    let copper_path = block_path.join("copper_block.png");
    fs::copy(&iron_path, &copper_path)
        .map_err(|e| format!("failed to copy iron_block.png: {}", e))?;
    let mut copper_img = image::open(&copper_path)
        .map_err(|e| format!("failed to open {}: {}", copper_path.display(), e))?
        .to_rgba8();
    adjust_copper_color(&mut copper_img);
    copper_img
        .save(&copper_path)
        .map_err(|e| format!("failed to save {}: {}", copper_path.display(), e))?;

    let (width, height) = copper_img.dimensions();

    let mut exposed = copper_img.clone();
    for pixel in exposed.pixels_mut() {
        if pixel[3] == 0 {
            continue;
        }
        let r = pixel[0] as f32;
        let g = pixel[1] as f32;
        let b = pixel[2] as f32;
        let new_r = (r * 0.8 + 100.0 * 0.2).round().clamp(0.0, 255.0) as u8;
        let new_g = (g * 0.7 + 180.0 * 0.3).round().clamp(0.0, 255.0) as u8;
        let new_b = (b * 0.6 + 160.0 * 0.4).round().clamp(0.0, 255.0) as u8;
        pixel[0] = new_r;
        pixel[1] = new_g;
        pixel[2] = new_b;
    }
    exposed
        .save(block_path.join("exposed_copper.png"))
        .map_err(|e| format!("failed to save exposed_copper.png: {}", e))?;

    let mut weathered = copper_img.clone();
    for pixel in weathered.pixels_mut() {
        if pixel[3] == 0 {
            continue;
        }
        let r = pixel[0] as f32;
        let g = pixel[1] as f32;
        let b = pixel[2] as f32;
        let new_r = (r * 0.6 + 70.0 * 0.4).round().clamp(0.0, 255.0) as u8;
        let new_g = (g * 0.5 + 190.0 * 0.5).round().clamp(0.0, 255.0) as u8;
        let new_b = (b * 0.4 + 180.0 * 0.6).round().clamp(0.0, 255.0) as u8;
        pixel[0] = new_r;
        pixel[1] = new_g;
        pixel[2] = new_b;
    }
    weathered
        .save(block_path.join("weathered_copper.png"))
        .map_err(|e| format!("failed to save weathered_copper.png: {}", e))?;

    let mut oxidized = copper_img.clone();
    for pixel in oxidized.pixels_mut() {
        if pixel[3] == 0 {
            continue;
        }
        pixel[0] = 50;
        pixel[1] = 210;
        pixel[2] = 210;
    }
    oxidized
        .save(block_path.join("oxidized_copper.png"))
        .map_err(|e| format!("failed to save oxidized_copper.png: {}", e))?;

    let mcmeta = iron_path.with_extension("png.mcmeta");
    if mcmeta.exists() {
        let _ = fs::copy(&mcmeta, copper_path.with_extension("png.mcmeta"));
        let _ = fs::copy(&mcmeta, block_path.join("exposed_copper.png.mcmeta"));
        let _ = fs::copy(&mcmeta, block_path.join("weathered_copper.png.mcmeta"));
        let _ = fs::copy(&mcmeta, block_path.join("oxidized_copper.png.mcmeta"));
    }

    Ok(())
}

pub fn generate_copper_tools(resource_pack_path: &Path) -> Result<(), String> {
    let items_path = resource_pack_path.join("assets/minecraft/textures/item");
    let items = [
        "iron_sword",
        "iron_helmet",
        "iron_chestplate",
        "iron_leggings",
        "iron_boots",
        "iron_axe",
        "iron_pickaxe",
        "iron_shovel",
        "iron_hoe",
        "iron_horse_armor",
    ];
    let alternatives = ["diamond", "gold", "stone", "netherite"];

    for item in items {
        let original_path = items_path.join(format!("{}.png", item));
        let new_path = items_path.join(format!("copper_{}.png", &item[5..]));
        if original_path.exists() {
            fs::copy(&original_path, &new_path)
                .map_err(|e| format!("failed to copy {}: {}", original_path.display(), e))?;
        } else {
            let mut copied = false;
            for material in alternatives {
                let alt_name = format!("{}_{}.png", material, &item[5..]);
                let alt_path = items_path.join(&alt_name);
                if alt_path.exists() {
                    fs::copy(&alt_path, &new_path)
                        .map_err(|e| format!("failed to copy {}: {}", alt_path.display(), e))?;
                    copied = true;
                    break;
                }
            }
            if !copied {
                continue;
            }
        }

        let mut img = image::open(&new_path)
            .map_err(|e| format!("failed to open {}: {}", new_path.display(), e))?
            .to_rgba8();
        adjust_copper_color(&mut img);
        img.save(&new_path)
            .map_err(|e| format!("failed to save {}: {}", new_path.display(), e))?;

        let mcmeta = original_path.with_extension("png.mcmeta");
        if mcmeta.exists() {
            let _ = fs::copy(&mcmeta, new_path.with_extension("png.mcmeta"));
        }
    }

    Ok(())
}

pub fn generate_copper_armor_models(resource_pack_path: &Path) -> Result<(), String> {
    let armor_path = resource_pack_path.join("assets/minecraft/textures/models/armor");
    let armor_files = ["iron_layer_1.png", "iron_layer_2.png"];
    let alternatives = ["diamond", "gold", "chainmail", "leather"];

    for armor_file in armor_files {
        let original_path = armor_path.join(armor_file);
        let new_path = armor_path.join(armor_file.replace("iron", "copper"));
        if original_path.exists() {
            fs::copy(&original_path, &new_path)
                .map_err(|e| format!("failed to copy {}: {}", original_path.display(), e))?;
        } else {
            let mut copied = false;
            for material in alternatives {
                let alt_file = if material == "gold" {
                    armor_file.replace("iron", "gold")
                } else {
                    armor_file.replace("iron", material)
                };
                let alt_path = armor_path.join(&alt_file);
                if alt_path.exists() {
                    fs::copy(&alt_path, &new_path)
                        .map_err(|e| format!("failed to copy {}: {}", alt_path.display(), e))?;
                    copied = true;
                    break;
                }
            }
            if !copied {
                continue;
            }
        }

        let mut img = image::open(&new_path)
            .map_err(|e| format!("failed to open {}: {}", new_path.display(), e))?
            .to_rgba8();
        adjust_copper_color(&mut img);
        img.save(&new_path)
            .map_err(|e| format!("failed to save {}: {}", new_path.display(), e))?;
    }

    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "generate_copper_ingot",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_copper_ingot(context.temp_dir()),
    );
    engine.register_task(
        "generate_copper_block",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_copper_block(context.temp_dir()),
    );
    engine.register_task(
        "generate_copper_tools",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_copper_tools(context.temp_dir()),
    );
    engine.register_task(
        "generate_copper_armor_models",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_copper_armor_models(context.temp_dir()),
    );
}
