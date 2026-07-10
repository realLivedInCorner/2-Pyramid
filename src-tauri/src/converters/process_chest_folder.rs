use std::path::Path;

use image::{imageops, RgbaImage};

use crate::hurray::scheduler::{TaskTier, TaskType};
use crate::image_utils::paste_region;

// ---------- helpers ----------

/// Swap two regions, then flip each horizontally+vertically in place.
fn swap_and_mirror(img: &mut RgbaImage, b1: (u32, u32, u32, u32), b2: (u32, u32, u32, u32)) -> Result<(), String> {
    let w1 = b1.2 - b1.0;
    let h1 = b1.3 - b1.1;
    let w2 = b2.2 - b2.0;
    let h2 = b2.3 - b2.1;

    let r1 = imageops::crop_imm(img, b1.0, b1.1, w1, h1).to_image();
    let r2 = imageops::crop_imm(img, b2.0, b2.1, w2, h2).to_image();

    paste_region(img, &r2, b1.0, b1.1)?;
    paste_region(img, &r1, b2.0, b2.1)?;

    let r1f = image::imageops::flip_horizontal(&image::imageops::flip_vertical(&r1));
    let r2f = image::imageops::flip_horizontal(&image::imageops::flip_vertical(&r2));
    imageops::overlay(img, &r1f, b1.0 as i64, b1.1 as i64);
    imageops::overlay(img, &r2f, b2.0 as i64, b2.1 as i64);
    Ok(())
}

/// Flip a region horizontally+vertically in place.
fn mirror_region(img: &mut RgbaImage, b: (u32, u32, u32, u32)) {
    let w = b.2 - b.0;
    let h = b.3 - b.1;
    let r = imageops::crop_imm(img, b.0, b.1, w, h).to_image();
    let f = image::imageops::flip_horizontal(&image::imageops::flip_vertical(&r));
    imageops::overlay(img, &f, b.0 as i64, b.1 as i64);
}

/// Flip a region vertically.
fn vflip_region(r: &RgbaImage) -> RgbaImage {
    image::imageops::flip_vertical(r)
}

/// Flip a region horizontally+vertically.
fn hvflip_region(r: &RgbaImage) -> RgbaImage {
    image::imageops::flip_horizontal(&image::imageops::flip_vertical(r))
}

/// Generate left and right chest images from a double chest texture.
fn generate_double_chest_images(
    left: &mut RgbaImage,
    right: &mut RgbaImage,
    img: &RgbaImage,
    s: u32,
) {
    let sb = |x1: u32, y1: u32, x2: u32, y2: u32| (x1 * s, y1 * s, x2 * s, y2 * s);

    // --- Left chest ---
    let crop = |b: (u32, u32, u32, u32)| imageops::crop_imm(img, b.0, b.1, b.2 - b.0, b.3 - b.1).to_image();

    imageops::overlay(left, &vflip_region(&crop(sb(29, 0, 44, 14))), (29 * s) as i64, 0);
    imageops::overlay(left, &vflip_region(&crop(sb(59, 0, 74, 14))), (14 * s) as i64, 0);
    imageops::overlay(left, &hvflip_region(&crop(sb(29, 14, 44, 19))), (43 * s) as i64, (14 * s) as i64);
    imageops::overlay(left, &hvflip_region(&crop(sb(44, 14, 58, 19))), (29 * s) as i64, (14 * s) as i64);
    imageops::overlay(left, &hvflip_region(&crop(sb(58, 14, 73, 19))), (14 * s) as i64, (14 * s) as i64);
    imageops::overlay(left, &vflip_region(&crop(sb(29, 19, 44, 33))), (29 * s) as i64, (19 * s) as i64);
    imageops::overlay(left, &vflip_region(&crop(sb(59, 19, 74, 33))), (14 * s) as i64, (19 * s) as i64);
    imageops::overlay(left, &hvflip_region(&crop(sb(29, 33, 44, 43))), (43 * s) as i64, (33 * s) as i64);
    imageops::overlay(left, &hvflip_region(&crop(sb(44, 33, 58, 43))), (29 * s) as i64, (33 * s) as i64);
    imageops::overlay(left, &hvflip_region(&crop(sb(58, 33, 73, 43))), (14 * s) as i64, (33 * s) as i64);

    // Additional left transforms
    imageops::overlay(left, &hvflip_region(&crop(sb(2, 1, 5, 5))), (1 * s) as i64, (1 * s) as i64);
    imageops::overlay(left, &crop(sb(2, 0, 3, 1)), (2 * s) as i64, 0);
    imageops::overlay(left, &crop(sb(4, 0, 5, 1)), (1 * s) as i64, 0);
    imageops::overlay(left, &vflip_region(&crop(sb(5, 1, 6, 5))), (1 * s) as i64, (1 * s) as i64);
    imageops::overlay(left, &crop(sb(1, 0, 2, 1)), (2 * s) as i64, 0);
    imageops::overlay(left, &crop(sb(3, 0, 4, 1)), (1 * s) as i64, 0);

    // --- Right chest ---
    imageops::overlay(right, &vflip_region(&crop(sb(44, 0, 59, 14))), (14 * s) as i64, 0);
    imageops::overlay(right, &vflip_region(&crop(sb(14, 0, 29, 14))), (29 * s) as i64, 0);
    imageops::overlay(right, &hvflip_region(&crop(sb(0, 14, 14, 19))), 0, (14 * s) as i64);
    imageops::overlay(right, &hvflip_region(&crop(sb(73, 14, 88, 19))), (14 * s) as i64, (14 * s) as i64);
    imageops::overlay(right, &hvflip_region(&crop(sb(14, 14, 29, 19))), (43 * s) as i64, (14 * s) as i64);
    imageops::overlay(right, &vflip_region(&crop(sb(14, 19, 29, 33))), (29 * s) as i64, (19 * s) as i64);
    imageops::overlay(right, &vflip_region(&crop(sb(44, 19, 59, 33))), (14 * s) as i64, (19 * s) as i64);
    imageops::overlay(right, &hvflip_region(&crop(sb(14, 33, 29, 43))), (43 * s) as i64, (33 * s) as i64);
    imageops::overlay(right, &hvflip_region(&crop(sb(0, 33, 14, 43))), 0, (33 * s) as i64);

    // Additional right transforms
    imageops::overlay(right, &hvflip_region(&crop(sb(10, 1, 13, 5))), (43 * s) as i64, (1 * s) as i64);
    imageops::overlay(right, &crop(sb(13, 0, 14, 1)), 0, 0);
    imageops::overlay(right, &crop(sb(11, 0, 12, 1)), 0, 0);
    imageops::overlay(right, &vflip_region(&crop(sb(9, 1, 10, 5))), (43 * s) as i64, (1 * s) as i64);
    imageops::overlay(right, &crop(sb(14, 0, 15, 1)), 0, 0);
    imageops::overlay(right, &crop(sb(12, 0, 13, 1)), 0, 0);
    imageops::overlay(right, &hvflip_region(&crop(sb(10, 0, 11, 1))), (43 * s) as i64, 0);
}

// ---------- entry point ----------

pub fn process_chest_folder(path: &Path) -> Result<(), String> {
    let chest_path = path.join("assets/minecraft/textures/entity/chest");

    if !chest_path.exists() {
        crate::log_info!("chest dir not found, skip process_chest_folder");
        return Ok(());
    }

    crate::log_info!("processing chest textures");

    let single_files = ["ender.png", "normal.png", "trapped.png", "christmas.png"];

    for chest_file in &single_files {
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
                crate::log_info!("unsupported single chest size {} for {}, skip", width, chest_file);
                continue;
            }
        };

        let sb = |x1: u32, y1: u32, x2: u32, y2: u32| (x1 * s, y1 * s, x2 * s, y2 * s);

        // swap_and_mirror on 4 pairs
        swap_and_mirror(&mut img, sb(14, 0, 28, 14), sb(28, 0, 42, 14))?;
        swap_and_mirror(&mut img, sb(14, 14, 28, 19), sb(42, 14, 56, 19))?;
        swap_and_mirror(&mut img, sb(14, 19, 28, 33), sb(28, 19, 42, 33))?;
        swap_and_mirror(&mut img, sb(14, 33, 28, 43), sb(42, 33, 56, 43))?;

        // mirror 8 regions
        let mbs = [
            sb(14, 0, 28, 14), sb(28, 0, 42, 14),
            sb(0, 14, 14, 19), sb(28, 14, 42, 19),
            sb(14, 19, 28, 33), sb(28, 19, 42, 33),
            sb(0, 33, 14, 43), sb(28, 33, 42, 43),
        ];
        for &b in &mbs {
            mirror_region(&mut img, b);
        }

        img.save(&file_path)
            .map_err(|e| format!("failed to save {}: {}", file_path.display(), e))?;
        crate::log_info!("processed single chest: {}", chest_file);
    }

    // Double chest processing
    let double_files = ["normal_double.png", "trapped_double.png", "christmas_double.png"];

    for chest_file in &double_files {
        let file_path = chest_path.join(chest_file);
        if !file_path.exists() {
            continue;
        }

        let img = image::open(&file_path)
            .map_err(|e| format!("failed to open {}: {}", file_path.display(), e))?
            .to_rgba8();

        let (width, height) = img.dimensions();
        let s = match (width, height) {
            (128, 64) => 1,
            (256, 128) => 2,
            (512, 256) => 4,
            (1024, 512) => 8,
            _ => {
                crate::log_info!("unsupported double chest size {}x{} for {}, skip", width, height, chest_file);
                continue;
            }
        };

        let left_size = (64 * s, 64 * s);
        let right_size = (64 * s, 64 * s);
        let mut left_img = RgbaImage::new(left_size.0, left_size.1);
        let mut right_img = RgbaImage::new(right_size.0, right_size.1);

        let prefix = if chest_file.contains("christmas") {
            "christmas"
        } else if chest_file.contains("normal") {
            "normal"
        } else {
            "trapped"
        };

        generate_double_chest_images(&mut left_img, &mut right_img, &img, s);

        left_img
            .save(chest_path.join(format!("{}_left.png", prefix)))
            .map_err(|e| format!("failed to save {}_left.png: {}", prefix, e))?;
        right_img
            .save(chest_path.join(format!("{}_right.png", prefix)))
            .map_err(|e| format!("failed to save {}_right.png: {}", prefix, e))?;
        crate::log_info!("processed double chest: {}", chest_file);
    }

    crate::log_info!("process_chest_folder completed");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "process_chest_folder",
        TaskType::Exclusive,
        TaskTier::Surgeon,
        |context| process_chest_folder(context.temp_dir()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_process_chest_folder() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let result = process_chest_folder(temp_dir.path());
        assert!(result.is_ok());
    }
}
