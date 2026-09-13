use std::fs;
use std::path::Path;

use image::{Rgba, RgbaImage};

fn determine_scale_factor(width: u32, height: u32) -> (u32, bool) {
    let candidates = [1u32, 2, 4, 8];
    let image_size = width.max(height);
    let mut best = candidates[0];
    let mut best_delta = (candidates[0] * 256).abs_diff(image_size);
    for &candidate in &candidates[1..] {
        let delta = (candidate * 256).abs_diff(image_size);
        if delta < best_delta {
            best = candidate;
            best_delta = delta;
        }
    }
    let is_exact = best * 256 == image_size;
    (best, is_exact)
}

pub fn generate_shulker_box_ui(resource_pack_path: &Path) -> Result<(), String> {
    let container_path = resource_pack_path.join("assets/minecraft/textures/gui/container");
    let generic_path = container_path.join("generic_54.png");
    if !generic_path.exists() {
        return Ok(());
    }

    let img = image::open(&generic_path)
        .map_err(|e| format!("failed to open {}: {}", generic_path.display(), e))?
        .to_rgba8();
    let (width, height) = img.dimensions();
    if width == 0 || height == 0 {
        return Ok(());
    }

    let (scale_factor, _exact) = determine_scale_factor(width, height);
    let mut new_img: RgbaImage = img.clone();

    let x_max = 176 * scale_factor;
    let clear_start = 71 * scale_factor;
    let clear_end = 127 * scale_factor;
    for x in 0..x_max {
        for y in clear_start..clear_end {
            if x < width && y < height {
                new_img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }

    let move_start = 127 * scale_factor;
    let move_end = 222 * scale_factor;
    let move_delta = 56 * scale_factor;
    for x in 0..x_max {
        for y in move_start..move_end {
            if x < width && y < height {
                let new_y = y.saturating_sub(move_delta);
                if new_y < height {
                    let pixel = img.get_pixel(x, y);
                    new_img.put_pixel(x, new_y, *pixel);
                }
                new_img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }

    fs::create_dir_all(&container_path)
        .map_err(|e| format!("failed to create {}: {}", container_path.display(), e))?;
    let shulker_path = container_path.join("shulker_box.png");
    new_img
        .save(&shulker_path)
        .map_err(|e| format!("failed to save {}: {}", shulker_path.display(), e))?;

    crate::log_info!("generated shulker_box.png");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "generate_shulker_box_ui",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_shulker_box_ui(context.temp_dir()),
    );
}
