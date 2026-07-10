use std::io;
use std::path::Path;

use image::imageops;

use crate::hurray::context::HurrayContext;
use crate::hurray::scheduler::{TaskTier, TaskType};
use crate::image_utils::paste_region;

fn fix_brewing_stand_ui_impl(ctx: &HurrayContext) -> Result<(), String> {
    let temp_dir = ctx.temp_dir();
    let container_path = temp_dir.join("assets/minecraft/textures/gui/container");
    let shulker_box_path = container_path.join("shulker_box.png");
    let brewing_stand_new_path = container_path.join("brewing_stand.png");

    if !shulker_box_path.exists() {
        crate::log_info!("shulker_box.png not found, skip brewing stand generation");
        return Ok(());
    }

    let mut img = image::open(&shulker_box_path)
        .map_err(|e| format!("failed to open {}: {}", shulker_box_path.display(), e))?
        .to_rgba8();

    let (width, height) = img.dimensions();
    if width != height {
        crate::log_info!("shulker_box.png is not square ({}x{}), skip", width, height);
        return Ok(());
    }

    let s = match width {
        256 => 1,
        512 => 2,
        1024 => 4,
        2048 => 8,
        _ => {
            crate::log_info!("unsupported shulker_box.png size: {}x{}, skip", width, height);
            return Ok(());
        }
    };

    // Fill cover_box with color from (5*s, 4*s)
    let fill_color = *img.get_pixel(5 * s, 4 * s);
    let (cover_x1, cover_y1) = (6 * s, 16 * s);
    let (cover_x2, cover_y2) = (170 * s, 72 * s);
    for y in cover_y1..cover_y2 {
        for x in cover_x1..cover_x2 {
            img.put_pixel(x, y, fill_color);
        }
    }
    crate::log_info!("filled cover_box ({},{})-({},{})", cover_x1, cover_y1, cover_x2, cover_y2);

    // Crop region (7*s, 83*s)-(25*s, 101*s) and paste to 5 positions
    let region = imageops::crop_imm(&img, 7 * s, 83 * s, 18 * s, 18 * s).to_image();
    let paste_positions = [
        (16 * s, 16 * s),
        (78 * s, 16 * s),
        (55 * s, 50 * s),
        (78 * s, 57 * s),
        (101 * s, 50 * s),
    ];
    for &(px, py) in &paste_positions {
        paste_region(&mut img, &region, px, py).map_err(|e| format!("failed to paste region: {}", e))?;
    }

    // External overlay from brewing_stand/
    if let Ok(uimage_path) = super::get_uimage_path() {
        let overlay_candidate = uimage_path
            .join("brewing_stand")
            .join(format!("brewing_stand_{}.png", width));
        crate::log_info!("looking for brewing_stand overlay: {}", overlay_candidate.display());
        if overlay_candidate.exists() {
            let overlay_img = image::open(&overlay_candidate)
                .map_err(|e| format!("failed to open overlay {}: {}", overlay_candidate.display(), e))?
                .to_rgba8();
            imageops::overlay(&mut img, &overlay_img, 0, 0);
            crate::log_info!("overlayed brewing_stand_{}.png", width);
        } else {
            crate::log_info!("no brewing_stand overlay at {}", overlay_candidate.display());
        }
    } else {
        crate::log_info!("UImage path not available, skip brewing_stand overlay");
    }

    img.save(&brewing_stand_new_path)
        .map_err(|e| format!("failed to save {}: {}", brewing_stand_new_path.display(), e))?;

    crate::log_info!("brewing_stand UI generated: {}", brewing_stand_new_path.display());
    Ok(())
}

pub fn fix_brewing_stand_ui(path: &Path) -> io::Result<()> {
    let context = crate::hurray::context::HurrayContext::new(&path.to_string_lossy());
    fix_brewing_stand_ui_impl(&context).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "fix_brewing_stand_ui",
        TaskType::Hybrid,
        TaskTier::Surgeon,
        |context| fix_brewing_stand_ui_impl(context),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_fix_brewing_stand_ui() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let result = fix_brewing_stand_ui(temp_dir.path());
        assert!(result.is_ok());
    }
}
