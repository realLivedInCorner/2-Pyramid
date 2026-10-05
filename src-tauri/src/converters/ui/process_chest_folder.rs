use std::path::Path;

use image::{imageops, RgbaImage};

use crate::image_utils::paste_region;

// ---------- helpers ----------

// ── 区域变换辅助：已移入共享模块 `crate::chest_region`（§9.124）──
// 本模块与 A-ROM 原生实现（`pilots::chest_folder_gen`）现在**调用同一份代码**，
// 因此「产物不变」是构造上成立的，而不是靠测试碰运气。
pub(crate) use crate::chest_region::{
    generate_double_chest_images, hvflip_region, mirror_region, swap_and_mirror, vflip_region,
};
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
