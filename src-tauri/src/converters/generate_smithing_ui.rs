use std::fs;
use std::path::Path;

use image::Rgba;

use crate::converters::get_uimage_path;

fn scale_factor(size: u32) -> Option<u32> {
    match size {
        256 => Some(1),
        512 => Some(2),
        1024 => Some(4),
        2048 => Some(8),
        _ => None,
    }
}

pub fn generate_smithing_ui(resource_pack_path: &Path) -> Result<(), String> {
    let container_path = resource_pack_path.join("assets/minecraft/textures/gui/container");
    let anvil_path = container_path.join("anvil.png");
    if !anvil_path.exists() {
        return Ok(());
    }

    let mut img = image::open(&anvil_path)
        .map_err(|e| format!("failed to open {}: {}", anvil_path.display(), e))?
        .to_rgba8();
    let (width, height) = img.dimensions();
    if width != height {
        return Ok(());
    }

    let factor = match scale_factor(width) {
        Some(factor) => factor,
        None => return Ok(()),
    };

    let fill_color = *img.get_pixel(5 * factor, 4 * factor);
    let cover_box = (10 * factor, 5 * factor, 169 * factor, 37 * factor);
    for x in cover_box.0..cover_box.2 {
        for y in cover_box.1..cover_box.3 {
            img.put_pixel(x, y, fill_color);
        }
    }

    if let Ok(uimage_path) = get_uimage_path() {
        let overlay_path = uimage_path
            .join("smithing")
            .join(format!("smithing_{}.png", width));
        if overlay_path.exists() {
            let overlay_img = image::open(&overlay_path)
                .map_err(|e| format!("failed to open {}: {}", overlay_path.display(), e))?
                .to_rgba8();
            image::imageops::overlay(&mut img, &overlay_img, 0, 0);
        }
    } else {
        crate::log_info!("UImage path not available, skip smithing overlay");
    }

    let transparent_box = (0 * factor, 166 * factor, 110 * factor, 198 * factor);
    let transparent = Rgba([0, 0, 0, 0]);
    for x in transparent_box.0..transparent_box.2 {
        for y in transparent_box.1..transparent_box.3 {
            img.put_pixel(x, y, transparent);
        }
    }

    fs::create_dir_all(&container_path)
        .map_err(|e| format!("failed to create {}: {}", container_path.display(), e))?;
    let smithing_path = container_path.join("smithing.png");
    img.save(&smithing_path)
        .map_err(|e| format!("failed to save {}: {}", smithing_path.display(), e))?;

    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "generate_smithing_ui",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_smithing_ui(context.temp_dir()),
    );
}
