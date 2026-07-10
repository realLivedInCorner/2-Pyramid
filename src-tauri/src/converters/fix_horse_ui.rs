use std::path::Path;

use image::imageops::{crop_imm, overlay};

use crate::hurray::scheduler::{TaskTier, TaskType};

/// Scale factor lookup from image dimensions.
/// Matches Python: both width and height must be equal.
fn scale_factor(width: u32, height: u32) -> Option<u32> {
    match (width, height) {
        (256, 256) => Some(1),
        (512, 512) => Some(2),
        (1024, 1024) => Some(4),
        (2048, 2048) => Some(8),
        _ => None,
    }
}

pub fn fix_horse_ui(path: &Path) -> Result<(), String> {
    let container_path = path.join("assets/minecraft/textures/gui/container");
    let horse_path = container_path.join("horse.png");

    if !horse_path.exists() {
        crate::log_info!("horse.png not found, skip");
        return Ok(());
    }

    let mut img = image::open(&horse_path)
        .map_err(|e| format!("failed to open {}: {}", horse_path.display(), e))?
        .to_rgba8();

    let (width, height) = img.dimensions();
    let s = match scale_factor(width, height) {
        Some(s) => s,
        None => {
            crate::log_info!("unsupported horse.png size: {}x{}, skip", width, height);
            return Ok(());
        }
    };

    // Step 1: Move region (7,17)-(25,35) → (18,220)-(36,238)
    let move_box = (7 * s, 17 * s, 25 * s, 35 * s);
    let paste_box = (18 * s, 220 * s, 36 * s, 238 * s);
    let region = crop_imm(&img, move_box.0, move_box.1, move_box.2 - move_box.0, move_box.3 - move_box.1).to_image();
    overlay(&mut img, &region, paste_box.0 as i64, paste_box.1 as i64);
    crate::log_info!("moved region ({},{})-({},{}) -> ({},{})", move_box.0, move_box.1, move_box.2, move_box.3, paste_box.0, paste_box.1);

    // Step 2: Fill moved region with color from (7*s, 16*s)
    let fill_color = *img.get_pixel(7 * s, 16 * s);
    for y in move_box.1..move_box.3 {
        for x in move_box.0..move_box.2 {
            img.put_pixel(x, y, fill_color);
        }
    }
    crate::log_info!("filled region with color from (7,16)");

    // Step 3: Copy (36,202)-(54,220) to (36,220)-(54,238)
    let copy_region = crop_imm(&img, 36 * s, 202 * s, 18 * s, 18 * s).to_image();
    overlay(&mut img, &copy_region, (36 * s) as i64, (220 * s) as i64);
    crate::log_info!("copied region (36,202)-(54,220) to (36,220)");

    // Step 4: External overlay from horse/horse_{width}.png
    if let Ok(uimage) = super::get_uimage_path() {
        let overlay_path = uimage.join("horse").join(format!("horse_{}.png", width));
        crate::log_info!("looking for horse overlay: {}", overlay_path.display());
        if overlay_path.exists() {
            let overlay_img = image::open(&overlay_path)
                .map_err(|e| format!("failed to open overlay {}: {}", overlay_path.display(), e))?
                .to_rgba8();
            overlay(&mut img, &overlay_img, 0, 0);
            crate::log_info!("overlayed horse_{}.png", width);
        } else {
            crate::log_info!("no horse overlay at {}", overlay_path.display());
        }
    }

    img.save(&horse_path)
        .map_err(|e| format!("failed to save {}: {}", horse_path.display(), e))?;

    crate::log_info!("horse UI fixed: {}", horse_path.display());
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "fix_horse_ui",
        TaskType::Hybrid,
        TaskTier::Surgeon,
        |context| {
            let path = context.temp_dir();
            fix_horse_ui(path)
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_fix_horse_ui() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let result = fix_horse_ui(temp_dir.path());
        assert!(result.is_ok());
    }
}
