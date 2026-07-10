use std::path::Path;

use image::imageops;

use crate::hurray::scheduler::{TaskTier, TaskType};
use crate::image_utils::paste_region;

/// Reverse the creative UI fix on `tab_inventory.png`.
pub fn reverse_fix_ui_creative(path: &Path) -> Result<(), String> {
    let creative_path = path.join(
        "assets/minecraft/textures/gui/container/creative_inventory/tab_inventory.png",
    );

    if !creative_path.exists() {
        crate::log_info!("tab_inventory.png not found, skip reverse_fix_ui_creative");
        return Ok(());
    }

    let mut img = image::open(&creative_path)
        .map_err(|e| format!("failed to open {}: {}", creative_path.display(), e))?
        .to_rgba8();

    let (width, _height) = img.dimensions();
    let s = match width {
        256 => 1,
        512 => 2,
        1024 => 4,
        2048 => 8,
        _ => {
            crate::log_info!("unsupported tab_inventory.png size: {}, skip", width);
            return Ok(());
        }
    };

    let scaled = |c: u32| c * s;

    // Fill (34,19)-(52,37) with color from (164,27)
    let fill_color = *img.get_pixel(scaled(164), scaled(27));
    for y in scaled(19)..scaled(37) {
        for x in scaled(34)..scaled(52) {
            img.put_pixel(x, y, fill_color);
        }
    }

    // Crop (51,0)-(129,53) → paste at (6,0)-(84,53)
    let src_w = scaled(129) - scaled(51);
    let src_h = scaled(53);
    let region = imageops::crop_imm(&img, scaled(51), 0, src_w, src_h).to_image();
    paste_region(&mut img, &region, scaled(6), 0).map_err(|e| format!("failed to paste region: {}", e))?;

    // Fill (84,0)-(129,53) with same color
    for y in 0..scaled(53) {
        for x in scaled(84)..scaled(129) {
            img.put_pixel(x, y, fill_color);
        }
    }

    img.save(&creative_path)
        .map_err(|e| format!("failed to save {}: {}", creative_path.display(), e))?;

    crate::log_info!("reverse_fix_ui_creative completed");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "reverse_fix_ui_creative",
        TaskType::Hybrid,
        TaskTier::Surgeon,
        |context| reverse_fix_ui_creative(context.temp_dir()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_reverse_fix_ui_creative() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let result = reverse_fix_ui_creative(temp_dir.path());
        assert!(result.is_ok());
    }
}
