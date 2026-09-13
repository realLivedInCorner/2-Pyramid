use std::path::Path;

use image::Rgba;

use crate::converters::scale_factor::determine_scale_factor;
use crate::hurray::context::HurrayContext;
use crate::hurray::scheduler::{TaskTier, TaskType};

pub fn fix_ui_sub_hand(context: &HurrayContext) -> Result<(), String> {
    let temp_dir = context.temp_dir().to_string_lossy().to_string();

    let widgets_path = Path::new(&temp_dir)
        .join("assets")
        .join("minecraft")
        .join("textures")
        .join("gui")
        .join("widgets.png");

    crate::log_info!("processing offhand UI in {}", widgets_path.display());

    if !widgets_path.exists() {
        crate::log_info!("widgets.png not found, skip");
        return Ok(());
    }

    let mut img = image::open(&widgets_path)
        .map_err(|e| format!("failed to open widgets.png: {}", e))?
        .to_rgba8();

    let width = img.width();
    let height = img.height();
    let (scale_factor, is_exact) = determine_scale_factor(width, height);
    crate::log_info!("offhand scale_factor={} exact={}", scale_factor, is_exact);

    let source_box = scaled_coords(1, 23, 23, 45, scale_factor);
    let dest_point_1 = scaled_point(24, 23, scale_factor);
    let dest_point_2 = scaled_point(60, 23, scale_factor);

    copy_and_paste_region(&mut img, source_box, dest_point_1);
    copy_and_paste_region(&mut img, source_box, dest_point_2);

    img.save(&widgets_path)
        .map_err(|e| format!("failed to save widgets.png: {}", e))?;

    crate::log_info!("offhand UI patch applied");
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
        "fix_ui_sub_hand",
        TaskType::Exclusive,
        TaskTier::Surgeon,
        fix_ui_sub_hand,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_fix_ui_sub_hand() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let resource_pack_path = temp_dir.path().to_string_lossy().to_string();

        let ui_path = temp_dir.path().join("assets/minecraft/textures/gui");
        fs::create_dir_all(&ui_path).expect("Failed to create test directory structure");

        let widgets_path = ui_path.join("widgets.png");
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(256, 256);
        for x in 1..23 {
            for y in 23..45 {
                img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        img.save(&widgets_path).expect("Failed to save test widgets.png");

        let context = HurrayContext::new(&resource_pack_path);
        let result = fix_ui_sub_hand(&context);
        assert!(result.is_ok());

        let modified_img = image::open(&widgets_path)
            .expect("Failed to open modified widgets.png")
            .to_rgba8();
        for x in 24..46 {
            for y in 23..45 {
                let pixel = modified_img.get_pixel(x, y);
                assert_eq!(*pixel, Rgba([255, 0, 0, 255]), "Copied region 1 not correct");
            }
        }
        for x in 60..82 {
            for y in 23..45 {
                let pixel = modified_img.get_pixel(x, y);
                assert_eq!(*pixel, Rgba([255, 0, 0, 255]), "Copied region 2 not correct");
            }
        }
    }
}