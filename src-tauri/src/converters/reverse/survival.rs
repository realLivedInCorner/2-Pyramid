use std::path::Path;

use image::{imageops, RgbaImage};

use crate::hurray::scheduler::{TaskTier, TaskType};
use crate::image_utils::paste_region;

pub fn reverse_fix_ui_survival(path: &Path) -> Result<(), String> {
    let inventory_path = path.join("assets/minecraft/textures/gui/container/inventory.png");
    let mob_effect_path = path.join("assets/minecraft/textures/mob_effect");

    if !inventory_path.exists() {
        crate::log_info!("inventory.png not found, skip reverse_fix_ui_survival");
        return Ok(());
    }

    let mut img = image::open(&inventory_path)
        .map_err(|e| format!("failed to open {}: {}", inventory_path.display(), e))?
        .to_rgba8();

    let (width, _height) = img.dimensions();
    let s = match width {
        256 => 1,
        512 => 2,
        1024 => 4,
        2048 => 8,
        _ => {
            crate::log_info!("unsupported inventory.png size: {}, skip", width);
            return Ok(());
        }
    };

    let scaled = |c: u32| c * s;

    // Step 1: Set (0,198)-(144,254) to transparent
    for y in scaled(198)..scaled(254) {
        for x in 0..scaled(144) {
            img.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
        }
    }

    // Step 2: Paste mob effect images if folder exists
    if mob_effect_path.exists() {
        let mob_effect_images = [
            "speed.png", "slowness.png", "haste.png", "mining_fatigue.png",
            "strength.png", "weakness.png", "poison.png", "regeneration.png",
            "invisibility.png", "hunger.png", "jump_boost.png", "nausea.png",
            "night_vision.png", "blindness.png", "resistance.png", "fire_resistance.png",
            "water_breathing.png", "wither.png", "absorption.png",
        ];

        let icon_size = scaled(18);
        for (i, effect_name) in mob_effect_images.iter().enumerate() {
            let effect_path = mob_effect_path.join(effect_name);
            if effect_path.exists() {
                let effect_img = image::open(&effect_path)
                    .map_err(|e| format!("failed to open {}: {}", effect_path.display(), e))?
                    .to_rgba8();

                let resized: RgbaImage = if effect_img.dimensions() != (icon_size, icon_size) {
                    imageops::resize(&effect_img, icon_size, icon_size, imageops::FilterType::Lanczos3)
                } else {
                    effect_img
                };

                let row = (i / 8) as u32;
                let col = (i % 8) as u32;
                let x_offset = col * icon_size + scaled(0);
                let y_offset = row * icon_size + scaled(198);
                paste_region(&mut img, &resized, x_offset, y_offset).map_err(|e| format!("failed to paste region: {}", e))?;
                crate::log_info!("pasted mob effect: {}", effect_name);
            }
        }
    }

    // Step 3: Fill (76,61)-(94,79) with color from (90,10)
    let fill_color = *img.get_pixel(scaled(90), scaled(10));
    for y in scaled(61)..scaled(79) {
        for x in scaled(76)..scaled(94) {
            img.put_pixel(x, y, fill_color);
        }
    }

    // Step 4: Move (96,16)-(172,54) by (-10, 8)
    let move_w = scaled(172) - scaled(96);
    let move_h = scaled(54) - scaled(16);
    let region = imageops::crop_imm(&img, scaled(96), scaled(16), move_w, move_h).to_image();
    let dst_x = ((96i32 - 10) * s as i32) as i64;
    let dst_y = ((16i32 + 8) * s as i32) as i64;
    imageops::overlay(&mut img, &region, dst_x, dst_y);

    // Step 5: Fill (96,16)-(172,25) with color from (90,10)
    for y in scaled(16)..scaled(25) {
        for x in scaled(96)..scaled(172) {
            img.put_pixel(x, y, fill_color);
        }
    }

    // Step 6: Fill (161,25)-(172,54) with color from (90,10)
    for y in scaled(25)..scaled(54) {
        for x in scaled(161)..scaled(172) {
            img.put_pixel(x, y, fill_color);
        }
    }

    img.save(&inventory_path)
        .map_err(|e| format!("failed to save {}: {}", inventory_path.display(), e))?;

    crate::log_info!("reverse_fix_ui_survival completed");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "reverse_fix_ui_survival",
        TaskType::Hybrid,
        TaskTier::Surgeon,
        |context| reverse_fix_ui_survival(context.temp_dir()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_reverse_fix_ui_survival() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let result = reverse_fix_ui_survival(temp_dir.path());
        assert!(result.is_ok());
    }
}
