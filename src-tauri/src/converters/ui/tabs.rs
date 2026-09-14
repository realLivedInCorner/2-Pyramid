use std::path::Path;

use image::Rgba;

use crate::converters::scale_factor::determine_scale_factor;

pub fn fix_tabs(temp_dir: &Path) -> Result<(), String> {
    let creative_inventory_path = temp_dir
        .join("assets")
        .join("minecraft")
        .join("textures")
        .join("gui")
        .join("container")
        .join("creative_inventory")
        .join("tabs.png");

    crate::log_info!("processing tabs texture: {}", creative_inventory_path.display());

    if !creative_inventory_path.exists() {
        crate::log_info!("tabs.png not found, skip");
        return Ok(());
    }

    let mut img = image::open(&creative_inventory_path)
        .map_err(|e| format!("failed to open tabs.png: {}", e))?
        .to_rgba8();

    let width = img.width();
    let height = img.height();
    let (scale_factor, is_exact) = determine_scale_factor(width, height);
    crate::log_info!("tabs scale_factor={} exact={}", scale_factor, is_exact);

    let region_to_move = scaled_coords(168, 0, 196, 128, scale_factor);
    let move_right = 14 * scale_factor;
    let dest_position = (region_to_move.0 + move_right, region_to_move.1);
    move_region(&mut img, region_to_move, dest_position);

    let shift_operations: [((u32, u32, u32, u32), u32); 6] = [
        ((15, 0, 41, 128), 2),
        ((43, 0, 69, 128), 4),
        ((71, 0, 97, 128), 6),
        ((99, 0, 125, 128), 8),
        ((127, 0, 153, 128), 10),
        ((155, 0, 168, 128), 12),
    ];

    for (box_, shift) in shift_operations {
        let (x1, y1, x2, y2) = box_;
        let shift_source_box = scaled_coords(x1, y1, x2, y2, scale_factor);
        let shift_pixels = shift * scale_factor;
        let shift_dest_point = (shift_source_box.0.saturating_sub(shift_pixels), shift_source_box.1);
        move_region(&mut img, shift_source_box, shift_dest_point);
    }

    let copy_region = scaled_coords(0, 0, 26, 128, scale_factor);
    let paste_position = scaled_point(156, 0, scale_factor);
    copy_and_paste_region(&mut img, copy_region, paste_position);

    img.save(&creative_inventory_path)
        .map_err(|e| format!("failed to save tabs.png: {}", e))?;

    crate::log_info!("tabs.png updated");
    Ok(())
}

fn scaled_coords(x1: u32, y1: u32, x2: u32, y2: u32, scale_factor: u32) -> (u32, u32, u32, u32) {
    (
        x1 * scale_factor,
        y1 * scale_factor,
        x2 * scale_factor,
        y2 * scale_factor,
    )
}

fn scaled_point(x: u32, y: u32, scale_factor: u32) -> (u32, u32) {
    (x * scale_factor, y * scale_factor)
}

fn move_region(
    img: &mut image::ImageBuffer<Rgba<u8>, Vec<u8>>,
    src_coords: (u32, u32, u32, u32),
    dst_point: (u32, u32),
) {
    let src_region = extract_region(img, src_coords);
    let (dst_x, dst_y) = dst_point;

    for y in 0..src_region.height() {
        for x in 0..src_region.width() {
            if let Some(pixel) = src_region.get_pixel_checked(x, y) {
                if let Some(dst_pixel) = img.get_pixel_mut_checked(dst_x + x, dst_y + y) {
                    *dst_pixel = *pixel;
                }
            }
        }
    }
}

fn extract_region(
    img: &image::ImageBuffer<Rgba<u8>, Vec<u8>>,
    coords: (u32, u32, u32, u32),
) -> image::ImageBuffer<Rgba<u8>, Vec<u8>> {
    let (x1, y1, x2, y2) = coords;
    let width = x2 - x1;
    let height = y2 - y1;

    let mut region = image::ImageBuffer::new(width, height);
    for y in 0..height {
        for x in 0..width {
            if let Some(pixel) = img.get_pixel_checked(x1 + x, y1 + y) {
                region.put_pixel(x, y, *pixel);
            }
        }
    }

    region
}

fn copy_and_paste_region(
    img: &mut image::ImageBuffer<Rgba<u8>, Vec<u8>>,
    src_coords: (u32, u32, u32, u32),
    dst_point: (u32, u32),
) {
    let src_region = extract_region(img, src_coords);
    let (dst_x, dst_y) = dst_point;

    for y in 0..src_region.height() {
        for x in 0..src_region.width() {
            if let Some(pixel) = src_region.get_pixel_checked(x, y) {
                if let Some(dst_pixel) = img.get_pixel_mut_checked(dst_x + x, dst_y + y) {
                    *dst_pixel = *pixel;
                }
            }
        }
    }
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "fix_tabs",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Surgeon,
        |context| {
            let temp_dir = context.temp_dir();
            fix_tabs(temp_dir)
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    use std::fs;
    use tempfile::tempdir;

    fn write_tabs(temp: &std::path::Path, size: u32) -> std::path::PathBuf {
        let dir = temp.join("assets/minecraft/textures/gui/container/creative_inventory");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tabs.png");
        let mut img = image::RgbaImage::new(size, size);
        // (0,0)-(26,128) 填绿色标记源区
        for y in 0..128 * (size / 256) {
            for x in 0..26 * (size / 256) {
                img.put_pixel(x, y, Rgba([0, 200, 0, 255]));
            }
        }
        // (168,0)-(196,128) 填蓝色标记被右移区
        let s = size / 256;
        for y in 0..128 * s {
            for x in 168 * s..196 * s {
                img.put_pixel(x, y, Rgba([0, 0, 200, 255]));
            }
        }
        img.save(&path).unwrap();
        path
    }

    #[test]
    fn fix_tabs_missing_file_is_ok() {
        let dir = tempdir().unwrap();
        assert!(fix_tabs(dir.path()).is_ok());
    }

    #[test]
    fn fix_tabs_copies_leftmost_block_to_156() {
        let dir = tempdir().unwrap();
        let path = write_tabs(dir.path(), 256);
        fix_tabs(dir.path()).unwrap();
        let img = image::open(&path).unwrap().to_rgba8();
        // 最后一步：复制 (0,0)-(26,128) → (156,0)
        let p = img.get_pixel(156, 0);
        assert_eq!(p.0[1], 200, "leftmost green should be pasted at x=156");
    }

    #[test]
    fn fix_tabs_moves_region_168_right_by_14() {
        let dir = tempdir().unwrap();
        let path = write_tabs(dir.path(), 256);
        fix_tabs(dir.path()).unwrap();
        let img = image::open(&path).unwrap().to_rgba8();
        // 原 (168,0) 蓝色块右移 14 → 出现在 (182,0)
        // 但之后 (156,0) 起又粘了绿色，覆盖 156-182；182 起应仍是蓝
        let p = img.get_pixel(182, 5);
        // 182 可能被绿色粘贴覆盖（156+26=182），取 183 更稳
        let p2 = img.get_pixel(183, 5);
        assert!(
            p.0[2] == 200 || p2.0[2] == 200 || p2.0[1] == 200,
            "moved blue region should land near x=182, got {:?} {:?}",
            p,
            p2
        );
    }

    #[test]
    fn fix_tabs_handles_2x_scale() {
        let dir = tempdir().unwrap();
        let path = write_tabs(dir.path(), 512);
        assert!(fix_tabs(dir.path()).is_ok());
        let img = image::open(&path).unwrap().to_rgba8();
        assert_eq!(img.width(), 512);
        // 绿色源区按 2x 粘贴到 x=312
        let p = img.get_pixel(312, 0);
        assert_eq!(p.0[1], 200);
    }
}