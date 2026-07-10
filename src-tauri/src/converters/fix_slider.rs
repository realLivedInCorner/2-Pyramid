use std::path::Path;

use image::Rgba;

use crate::converters::scale_factor::determine_scale_factor;

pub fn fix_slider(temp_dir: &Path) -> Result<(), String> {
    let gui_path = temp_dir
        .join("assets")
        .join("minecraft")
        .join("textures")
        .join("gui");

    let widgets_path = gui_path.join("widgets.png");
    let slider_path = gui_path.join("slider.png");

    crate::log_info!(
        "building slider texture from {} -> {}",
        widgets_path.display(),
        slider_path.display()
    );

    if !widgets_path.exists() {
        crate::log_info!("widgets.png not found in {}", gui_path.display());
        return Ok(());
    }

    let img = image::open(&widgets_path)
        .map_err(|e| format!("failed to open widgets.png: {}", e))?
        .to_rgba8();

    let width = img.width();
    let height = img.height();
    let (scale_factor, is_exact) = determine_scale_factor(width, height);
    crate::log_info!("slider scale_factor={} exact={}", scale_factor, is_exact);

    let mut slider_img = image::ImageBuffer::new(width, height);

    let source_box1 = scaled_coords(0, 46, 200, 66, scale_factor);
    let dest_point1 = scaled_point(0, 0, scale_factor);
    copy_and_paste_region(&img, &mut slider_img, source_box1, dest_point1);

    let source_box2 = scaled_coords(0, 46, 200, 106, scale_factor);
    let dest_point2 = scaled_point(0, 20, scale_factor);
    copy_and_paste_region(&img, &mut slider_img, source_box2, dest_point2);

    slider_img
        .save(&slider_path)
        .map_err(|e| format!("failed to save slider.png: {}", e))?;

    crate::log_info!("slider generated at {}", slider_path.display());
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

fn copy_and_paste_region(
    src_img: &image::ImageBuffer<Rgba<u8>, Vec<u8>>,
    dest_img: &mut image::ImageBuffer<Rgba<u8>, Vec<u8>>,
    src_coords: (u32, u32, u32, u32),
    dest_point: (u32, u32),
) {
    let (src_x1, src_y1, src_x2, src_y2) = src_coords;
    let (dest_x, dest_y) = dest_point;

    for y in 0..(src_y2 - src_y1) {
        for x in 0..(src_x2 - src_x1) {
            if let Some(src_pixel) = src_img.get_pixel_checked(src_x1 + x, src_y1 + y) {
                if let Some(dest_pixel) = dest_img.get_pixel_mut_checked(dest_x + x, dest_y + y) {
                    *dest_pixel = *src_pixel;
                }
            }
        }
    }
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "fix_slider",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Surgeon,
        |context| {
            let temp_dir = context.temp_dir();
            fix_slider(temp_dir)
        },
    );
}