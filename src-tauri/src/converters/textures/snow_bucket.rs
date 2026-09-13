use std::fs;
use std::path::Path;

use image::imageops;

use crate::converters::get_uimage_path;

pub fn generate_snow_bucket(resource_pack_path: &Path) -> Result<(), String> {
    let items_path = resource_pack_path.join("assets/minecraft/textures/item");
    let milk_path = items_path.join("milk_bucket.png");
    if !milk_path.exists() {
        return Ok(());
    }

    let base_img = image::open(&milk_path)
        .map_err(|e| format!("failed to open {}: {}", milk_path.display(), e))?
        .to_rgba8();
    let (width, height) = base_img.dimensions();
    if width != height || width == 0 {
        return Ok(());
    }

    let powder_path = items_path.join("powder_snow_bucket.png");
    fs::copy(&milk_path, &powder_path)
        .map_err(|e| format!("failed to copy milk_bucket.png: {}", e))?;

    if let Ok(uimage_path) = get_uimage_path() {
        let overlay_path = uimage_path
            .join("powder_snow_bucket")
            .join(format!("powder_snow_bucket_{}.png", width));

        if overlay_path.exists() {
            let mut bucket_img = image::open(&powder_path)
                .map_err(|e| format!("failed to open {}: {}", powder_path.display(), e))?
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
                .save(&powder_path)
                .map_err(|e| format!("failed to save {}: {}", powder_path.display(), e))?;
        }
    } else {
        crate::log_info!("UImage path not available, skip snow bucket overlay");
    }

    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "generate_snow_bucket",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_snow_bucket(context.temp_dir()),
    );
}
