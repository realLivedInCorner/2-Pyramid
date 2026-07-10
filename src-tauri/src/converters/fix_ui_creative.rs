use std::path::Path;

use image::Rgba;

use crate::converters::scale_factor::determine_scale_factor;
use crate::hurray::context::HurrayContext;
use crate::hurray::scheduler::{TaskTier, TaskType};

pub fn fix_ui_creative(context: &HurrayContext) -> Result<(), String> {
    let temp_dir = context.temp_dir().to_string_lossy().to_string();

    let creative_inventory_path = Path::new(&temp_dir)
        .join("assets")
        .join("minecraft")
        .join("textures")
        .join("gui")
        .join("container")
        .join("creative_inventory")
        .join("tab_inventory.png");

    crate::log_info!("processing creative inventory: {}", creative_inventory_path.display());

    if !creative_inventory_path.exists() {
        crate::log_info!("tab_inventory.png missing, skip");
        return Ok(());
    }

    let mut img = image::open(&creative_inventory_path)
        .map_err(|e| format!("failed to open tab_inventory.png: {}", e))?
        .to_rgba8();

    let width = img.width();
    let height = img.height();
    let (scale_factor, is_exact) = determine_scale_factor(width, height);
    crate::log_info!("creative scale_factor={} exact={}", scale_factor, is_exact);

    let source_box = scaled_coords(6, 0, 84, 53, scale_factor);
    let dest_box = scaled_coords(51, 0, 129, 53, scale_factor);
    copy_and_paste_region(&mut img, source_box, (dest_box.0, dest_box.1));

    let fill_box = scaled_coords(6, 0, 53, 53, scale_factor);
    let fill_color = get_pixel(&img, scaled_point(164, 27, scale_factor))
        .unwrap_or(Rgba([0, 0, 0, 0]));
    fill_region(&mut img, fill_box, fill_color);

    let source_box_18x18 = scaled_coords(53, 5, 71, 23, scale_factor);
    let dest_position = scaled_point(34, 19, scale_factor);
    copy_and_paste_region(&mut img, source_box_18x18, dest_position);

    img.save(&creative_inventory_path)
        .map_err(|e| format!("failed to save tab_inventory.png: {}", e))?;

    crate::log_info!("creative inventory UI adjusted");
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

fn get_pixel(img: &image::ImageBuffer<Rgba<u8>, Vec<u8>>, point: (u32, u32)) -> Option<Rgba<u8>> {
    img.get_pixel_checked(point.0, point.1).copied()
}

fn fill_region(
    img: &mut image::ImageBuffer<Rgba<u8>, Vec<u8>>,
    region: (u32, u32, u32, u32),
    color: Rgba<u8>,
) {
    let (x1, y1, x2, y2) = region;
    for y in y1..y2 {
        for x in x1..x2 {
            if let Some(pixel) = img.get_pixel_mut_checked(x, y) {
                *pixel = color;
            }
        }
    }
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

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "fix_ui_creative",
        TaskType::Exclusive,
        TaskTier::Surgeon,
        fix_ui_creative,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_fix_ui_creative() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let resource_pack_path = temp_dir.path().to_string_lossy().to_string();

        let creative_dir = temp_dir
            .path()
            .join("assets/minecraft/textures/gui/container/creative_inventory");
        fs::create_dir_all(&creative_dir).expect("Failed to create creative directory");

        let test_file = creative_dir.join("tab_inventory.png");
        let img = image::RgbaImage::new(256, 256);
        img.save(&test_file).expect("Failed to create test image");

        let context = HurrayContext::new(&resource_pack_path);
        let result = fix_ui_creative(&context);
        assert!(result.is_ok());
    }
}