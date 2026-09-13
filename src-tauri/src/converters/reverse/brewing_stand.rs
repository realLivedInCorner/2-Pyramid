use std::path::Path;

use image::imageops;

use crate::hurray::scheduler::{TaskTier, TaskType};
use crate::image_utils::paste_region;

/// Reverse the brewing_stand UI fix by doing forwared pixel operations in reverse.
pub fn reverse_fix_brewing_stand_ui(path: &Path) -> Result<(), String> {
    let container_path = path.join("assets/minecraft/textures/gui/container");
    let brewing_stand_path = container_path.join("brewing_stand.png");

    if !brewing_stand_path.exists() {
        crate::log_info!("brewing_stand.png not found, skip reverse");
        return Ok(());
    }

    let mut img = image::open(&brewing_stand_path)
        .map_err(|e| format!("failed to open {}: {}", brewing_stand_path.display(), e))?
        .to_rgba8();

    let (width, height) = img.dimensions();
    let s = match width {
        256 => 1,
        512 => 2,
        1024 => 4,
        2048 => 8,
        _ => {
            crate::log_info!("unsupported brewing_stand.png size: {}x{}, skip", width, height);
            return Ok(());
        }
    };

    // Fill color from (7*s, 4*s)
    let fill_color = *img.get_pixel(7 * s, 4 * s);

    // Fill (41,43)-(79,49)
    for y in (43 * s)..(49 * s) {
        for x in (41 * s)..(79 * s) {
            img.put_pixel(x, y, fill_color);
        }
    }

    // Fill (14,14)-(55,43)
    for y in (14 * s)..(43 * s) {
        for x in (14 * s)..(55 * s) {
            img.put_pixel(x, y, fill_color);
        }
    }

    // Move region (55,50)-(119,75) up by 5 pixels → (55,45)
    let move_w = (119 - 55) * s;
    let move_h = (75 - 50) * s;
    let region = imageops::crop_imm(&img, 55 * s, 50 * s, move_w, move_h).to_image();
    paste_region(&mut img, &region, 55 * s, 45 * s).map_err(|e| format!("failed to paste region: {}", e))?;

    // Fill vacated region (55,70)-(119,75)
    for y in (70 * s)..(75 * s) {
        for x in (55 * s)..(119 * s) {
            img.put_pixel(x, y, fill_color);
        }
    }

    img.save(&brewing_stand_path)
        .map_err(|e| format!("failed to save {}: {}", brewing_stand_path.display(), e))?;

    crate::log_info!("reverse_fix_brewing_stand_ui completed");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "reverse_fix_brewing_stand_ui",
        TaskType::Hybrid,
        TaskTier::Surgeon,
        |context| reverse_fix_brewing_stand_ui(context.temp_dir()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_reverse_fix_brewing_stand_ui() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let result = reverse_fix_brewing_stand_ui(temp_dir.path());
        assert!(result.is_ok());
    }
}
