use std::fs;
use std::path::Path;

use crate::converters::adjust_hue_brightness::adjust_hue_brightness;

pub fn generate_boat(resource_pack_path: &Path) -> Result<(), String> {
    let items_path = resource_pack_path.join("assets/minecraft/textures/items");
    let boat_path = items_path.join("boat.png");
    if !boat_path.exists() {
        crate::log_info!("boat.png not found, skip boat generation");
        return Ok(());
    }

    let base_img = image::open(&boat_path)
        .map_err(|e| format!("failed to open {}: {}", boat_path.display(), e))?
        .to_rgba8();

    let oak = adjust_hue_brightness(base_img.clone(), 0.0, 15.0, 0.0);
    oak.save(items_path.join("oak_boat.png"))
        .map_err(|e| format!("failed to save oak_boat.png: {}", e))?;

    let birch = adjust_hue_brightness(base_img.clone(), 0.0, 40.0, 0.0);
    birch
        .save(items_path.join("birch_boat.png"))
        .map_err(|e| format!("failed to save birch_boat.png: {}", e))?;

    let acacia = adjust_hue_brightness(base_img.clone(), -23.0, 10.0, 0.0);
    acacia
        .save(items_path.join("acacia_boat.png"))
        .map_err(|e| format!("failed to save acacia_boat.png: {}", e))?;

    let dark_oak = adjust_hue_brightness(base_img.clone(), 0.0, -15.0, 0.0);
    dark_oak
        .save(items_path.join("dark_oak_boat.png"))
        .map_err(|e| format!("failed to save dark_oak_boat.png: {}", e))?;

    let jungle = adjust_hue_brightness(base_img, -10.0, 4.6, 0.0);
    jungle
        .save(items_path.join("jungle_boat.png"))
        .map_err(|e| format!("failed to save jungle_boat.png: {}", e))?;

    let spruce_path = items_path.join("spruce_boat.png");
    if spruce_path.exists() {
        fs::remove_file(&spruce_path)
            .map_err(|e| format!("failed to remove {}: {}", spruce_path.display(), e))?;
    }

    fs::rename(&boat_path, &spruce_path).map_err(|e| {
        format!(
            "failed to rename {} -> {}: {}",
            boat_path.display(),
            spruce_path.display(),
            e
        )
    })?;

    crate::log_info!("generated boat variants");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "generate_boat",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_boat(context.temp_dir()),
    );
}
