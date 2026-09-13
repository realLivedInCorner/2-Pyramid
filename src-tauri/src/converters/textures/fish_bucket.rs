use std::fs;
use std::path::Path;

use image::imageops;

use crate::converters::get_uimage_path;

pub fn generate_fish_bucket(resource_pack_path: &Path) -> Result<(), String> {
    let items_path = resource_pack_path.join("assets/minecraft/textures/item");
    let water_bucket_path = items_path.join("water_bucket.png");
    if !water_bucket_path.exists() {
        return Ok(());
    }

    let base_img = image::open(&water_bucket_path)
        .map_err(|e| format!("failed to open {}: {}", water_bucket_path.display(), e))?
        .to_rgba8();
    let (width, height) = base_img.dimensions();
    if width != height || width == 0 {
        return Ok(());
    }

    let fish_types = ["axolotl", "cod", "pufferfish", "salmon", "tropical_fish", "tadpole"];
    let overlay_folder = get_uimage_path()
        .map(|p| p.join("water_bucket"))
        .ok();

    for fish in fish_types {
        let output_path = items_path.join(format!("{}_bucket.png", fish));
        fs::copy(&water_bucket_path, &output_path)
            .map_err(|e| format!("failed to copy water_bucket: {}", e))?;

        let overlay_folder = match &overlay_folder {
            Some(f) => f,
            None => continue,
        };
        let overlay_path = overlay_folder.join(format!("{}_bucket_{}.png", fish, width));
        if !overlay_path.exists() {
            continue;
        }

        let mut bucket_img = image::open(&output_path)
            .map_err(|e| format!("failed to open {}: {}", output_path.display(), e))?
            .to_rgba8();
        let mut overlay_img = image::open(&overlay_path)
            .map_err(|e| format!("failed to open {}: {}", overlay_path.display(), e))?
            .to_rgba8();

        if overlay_img.dimensions() != bucket_img.dimensions() {
            overlay_img = imageops::resize(
                &overlay_img,
                bucket_img.width(),
                bucket_img.height(),
                imageops::FilterType::Triangle,
            );
        }

        imageops::overlay(&mut bucket_img, &overlay_img, 0, 0);
        bucket_img
            .save(&output_path)
            .map_err(|e| format!("failed to save {}: {}", output_path.display(), e))?;
    }

    crate::log_info!("generated fish buckets");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "generate_fish_bucket",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_fish_bucket(context.temp_dir()),
    );
}
