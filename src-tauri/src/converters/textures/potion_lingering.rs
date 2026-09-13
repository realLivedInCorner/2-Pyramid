use std::fs;
use std::path::Path;

use image::{Rgba, RgbaImage};

pub fn generate_potion_lingering(resource_pack_path: &Path) -> Result<(), String> {
    let items_path = resource_pack_path.join("assets/minecraft/textures/items");
    let targets = [
        ("potion.png", "lingering_potion.png"),
        ("potion_bottle_drinkable.png", "potion_bottle_lingering.png"),
    ];

    for (original, new_name) in targets {
        let original_path = items_path.join(original);
        if !original_path.exists() {
            continue;
        }

        let new_path = items_path.join(new_name);
        fs::copy(&original_path, &new_path)
            .map_err(|e| format!("failed to copy {}: {}", original_path.display(), e))?;

        let mut img = image::open(&new_path)
            .map_err(|e| format!("failed to open {}: {}", new_path.display(), e))?
            .to_rgba8();
        let (width, height) = img.dimensions();

        if width == 0 || height == 0 {
            continue;
        }

        if width == height {
            apply_top_third_transparency(&mut img, width, height);
        } else if height % width == 0 {
            let squares = height / width;
            for square in 0..squares {
                let y_offset = square * width;
                apply_top_third_transparency_region(&mut img, width, y_offset);
            }
        } else {
            continue;
        }

        img.save(&new_path)
            .map_err(|e| format!("failed to save {}: {}", new_path.display(), e))?;

        let original_mcmeta = original_path.with_extension("png.mcmeta");
        if original_mcmeta.exists() {
            let new_mcmeta = new_path.with_extension("png.mcmeta");
            let _ = fs::copy(&original_mcmeta, &new_mcmeta);
        }
    }

    crate::log_info!("generated lingering potion textures");
    Ok(())
}

fn apply_top_third_transparency(img: &mut RgbaImage, width: u32, height: u32) {
    let cutoff = height / 3;
    for y in 0..height {
        for x in 0..width {
            if y < cutoff {
                img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }
}

fn apply_top_third_transparency_region(img: &mut RgbaImage, width: u32, y_offset: u32) {
    let cutoff = width / 3;
    for y in 0..width {
        for x in 0..width {
            if y < cutoff {
                img.put_pixel(x, y_offset + y, Rgba([0, 0, 0, 0]));
            }
        }
    }
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "generate_potion_lingering",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_potion_lingering(context.temp_dir()),
    );
}
