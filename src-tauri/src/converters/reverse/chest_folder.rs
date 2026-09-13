use std::path::Path;

use image::{imageops, RgbaImage};

use crate::hurray::scheduler::{TaskTier, TaskType};
use crate::image_utils::paste_region;

/// swap_and_mirror: swap two regions, then flip each LR+TB in place.
fn swap_and_mirror(img: &mut RgbaImage, b1: (u32, u32, u32, u32), b2: (u32, u32, u32, u32)) -> Result<(), String> {
    let w1 = b1.2 - b1.0;
    let h1 = b1.3 - b1.1;
    let w2 = b2.2 - b2.0;
    let h2 = b2.3 - b2.1;

    let region1 = imageops::crop_imm(img, b1.0, b1.1, w1, h1).to_image();
    let region2 = imageops::crop_imm(img, b2.0, b2.1, w2, h2).to_image();

    // Swap: paste r2→b1, r1→b2
    paste_region(img, &region2, b1.0, b1.1)?;
    paste_region(img, &region1, b2.0, b2.1)?;

    // Flip each in place
    let r1f = image::imageops::flip_horizontal(&image::imageops::flip_vertical(&imageops::crop_imm(img, b1.0, b1.1, w1, h1).to_image()));
    let r2f = image::imageops::flip_horizontal(&image::imageops::flip_vertical(&imageops::crop_imm(img, b2.0, b2.1, w2, h2).to_image()));
    imageops::overlay(img, &r1f, b1.0 as i64, b1.1 as i64);
    imageops::overlay(img, &r2f, b2.0 as i64, b2.1 as i64);
    Ok(())
}

/// Reverse the swap-and-mirror + mirror operations on single chest images.
pub fn reverse_process_chest_folder(path: &Path) -> Result<(), String> {
    let chest_path = path.join("assets/minecraft/textures/entity/chest");

    if !chest_path.exists() {
        crate::log_info!("chest dir not found, skip reverse_process_chest_folder");
        return Ok(());
    }

    let single_chest_files = ["ender.png", "normal.png", "trapped.png", "christmas.png"];

    for chest_file in &single_chest_files {
        let file_path = chest_path.join(chest_file);
        if !file_path.exists() {
            continue;
        }

        let mut img = image::open(&file_path)
            .map_err(|e| format!("failed to open {}: {}", file_path.display(), e))?
            .to_rgba8();

        let (width, _height) = img.dimensions();
        let s = match width {
            64 => 1,
            128 => 2,
            256 => 4,
            512 => 8,
            1024 => 16,
            _ => {
                crate::log_info!("unsupported chest size {} for {}, skip", width, chest_file);
                continue;
            }
        };

        let scaled = |x1: u32, y1: u32, x2: u32, y2: u32| (x1 * s, y1 * s, x2 * s, y2 * s);

        // Reverse swap_and_mirror on 4 pairs
        swap_and_mirror(&mut img, scaled(14, 0, 28, 14), scaled(28, 0, 42, 14))?;
        swap_and_mirror(&mut img, scaled(14, 14, 28, 19), scaled(42, 14, 56, 19))?;
        swap_and_mirror(&mut img, scaled(14, 19, 28, 33), scaled(28, 19, 42, 33))?;
        swap_and_mirror(&mut img, scaled(14, 33, 28, 43), scaled(42, 33, 56, 43))?;

        // Mirror 8 regions
        let mirror_boxes = [
            scaled(14, 0, 28, 14), scaled(28, 0, 42, 14),
            scaled(0, 14, 14, 19), scaled(28, 14, 42, 19),
            scaled(14, 19, 28, 33), scaled(28, 19, 42, 33),
            scaled(0, 33, 14, 43), scaled(28, 33, 42, 43),
        ];
        for &b in &mirror_boxes {
            let w = b.2 - b.0;
            let h = b.3 - b.1;
            let region = imageops::crop_imm(&img, b.0, b.1, w, h).to_image();
            let flipped = image::imageops::flip_horizontal(&image::imageops::flip_vertical(&region));
            imageops::overlay(&mut img, &flipped, b.0 as i64, b.1 as i64);
        }

        img.save(&file_path)
            .map_err(|e| format!("failed to save {}: {}", file_path.display(), e))?;
        crate::log_info!("reversed chest: {}", chest_file);
    }

    crate::log_info!("reverse_process_chest_folder completed");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "reverse_process_chest_folder",
        TaskType::Hybrid,
        TaskTier::Surgeon,
        |context| reverse_process_chest_folder(context.temp_dir()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_reverse_process_chest_folder() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let result = reverse_process_chest_folder(temp_dir.path());
        assert!(result.is_ok());
    }
}
