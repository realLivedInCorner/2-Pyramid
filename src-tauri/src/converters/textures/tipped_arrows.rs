use std::fs;
use std::path::Path;

use image::RgbaImage;

use crate::converters::get_uimage_path;

pub fn generate_tipped_arrow_images(resource_pack_path: &Path) -> Result<(), String> {
    let items_path = resource_pack_path.join("assets/minecraft/textures/items");
    let arrow_path = items_path.join("arrow.png");
    if !arrow_path.exists() {
        crate::log_info!("arrow.png not found, skip tipped arrow generation");
        return Ok(());
    }

    let base_img = image::open(&arrow_path)
        .map_err(|e| format!("failed to open {}: {}", arrow_path.display(), e))?
        .to_rgba8();
    let size = base_img.width();

    let uimage_path = match get_uimage_path() {
        Ok(p) => p,
        Err(_) => {
            crate::log_info!("UImage path not available, skip tipped arrow generation");
            return Ok(());
        }
    };
    let head_path = uimage_path
        .join("tipped_arrow_head")
        .join(format!("tipped_arrow_head_{}.png", size));

    if !head_path.exists() {
        crate::log_info!("tipped arrow head not found: {}", head_path.display());
        return Ok(());
    }

    let head_img = image::open(&head_path)
        .map_err(|e| format!("failed to open {}: {}", head_path.display(), e))?
        .to_rgba8();

    let mut base_out: RgbaImage = base_img.clone();
    for (base_pixel, head_pixel) in base_out.pixels_mut().zip(head_img.pixels()) {
        if head_pixel[3] > 0 {
            base_pixel[3] = 0;
        }
    }

    let base_path = items_path.join("tipped_arrow_base.png");
    base_out
        .save(&base_path)
        .map_err(|e| format!("failed to save {}: {}", base_path.display(), e))?;

    let head_out_path = items_path.join("tipped_arrow_head.png");
    fs::copy(&head_path, &head_out_path).map_err(|e| {
        format!(
            "failed to copy {} -> {}: {}",
            head_path.display(),
            head_out_path.display(),
            e
        )
    })?;

    crate::log_info!("generated tipped_arrow_base.png and tipped_arrow_head.png");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "generate_tipped_arrow_images",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Surgeon,
        |context| generate_tipped_arrow_images(context.temp_dir()),
    );
}

