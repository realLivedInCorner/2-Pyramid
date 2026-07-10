use image::RgbaImage;

use crate::hurray::context::HurrayContext;
use crate::hurray::scheduler::{TaskTier, TaskType};

/// Overlay `icons_{size}.png` from UImage/icons/ onto `icons.png`.
pub fn overlay_icons(ctx: &HurrayContext) -> Result<(), String> {
    let temp_dir = ctx.temp_dir();
    let gui_path = temp_dir.join("assets/minecraft/textures/gui");
    let icons_path = gui_path.join("icons.png");

    crate::log_info!("overlay_icons: checking {}", icons_path.display());

    if !icons_path.exists() {
        crate::log_info!("icons.png not found, skip");
        return Ok(());
    }

    let mut base_img = image::open(&icons_path)
        .map_err(|e| format!("failed to open {}: {}", icons_path.display(), e))?
        .to_rgba8();

    let (width, height) = base_img.dimensions();
    if width != height {
        crate::log_info!("icons.png is not square ({}x{}), skip", width, height);
        return Ok(());
    }

    let overlay_filename = match width {
        256 => "icons_256.png",
        512 => "icons_512.png",
        1024 => "icons_1024.png",
        2048 => "icons_2048.png",
        _ => {
            crate::log_info!("unsupported icons.png size: {}x{}, skip", width, height);
            return Ok(());
        }
    };

    if let Ok(uimage) = super::get_uimage_path() {
        let overlay_path = uimage.join("icons").join(overlay_filename);
        crate::log_info!("looking for icons overlay: {}", overlay_path.display());

        if overlay_path.exists() {
            let overlay_img = image::open(&overlay_path)
                .map_err(|e| format!("failed to open {}: {}", overlay_path.display(), e))?
                .to_rgba8();
            // paste with alpha mask (overlay as both source and mask)
            alpha_paste(&mut base_img, &overlay_img, 0, 0);
            crate::log_info!("overlayed {} onto icons.png", overlay_filename);
        } else {
            crate::log_info!("overlay not found: {}", overlay_path.display());
        }
    } else {
        crate::log_info!("UImage path not available, skip overlay_icons");
    }

    base_img
        .save(&icons_path)
        .map_err(|e| format!("failed to save {}: {}", icons_path.display(), e))?;

    Ok(())
}

/// Paste `overlay` onto `base` at (x, y) using overlay's own alpha channel as mask.
/// Equivalent to PIL `Image.paste(overlay, (0,0), overlay)`.
fn alpha_paste(base: &mut RgbaImage, overlay: &RgbaImage, dest_x: u32, dest_y: u32) {
    let (base_w, base_h) = base.dimensions();
    let (overlay_w, overlay_h) = overlay.dimensions();

    for y in 0..overlay_h {
        let target_y = dest_y + y;
        if target_y >= base_h {
            continue;
        }
        for x in 0..overlay_w {
            let target_x = dest_x + x;
            if target_x >= base_w {
                continue;
            }
            let src_pixel = overlay.get_pixel(x, y);
            let alpha = src_pixel[3] as f64 / 255.0;
            if alpha > 0.0 {
                let dst_pixel = base.get_pixel(target_x, target_y);
                let blended = [
                    ((1.0 - alpha) * dst_pixel[0] as f64 + alpha * src_pixel[0] as f64) as u8,
                    ((1.0 - alpha) * dst_pixel[1] as f64 + alpha * src_pixel[1] as f64) as u8,
                    ((1.0 - alpha) * dst_pixel[2] as f64 + alpha * src_pixel[2] as f64) as u8,
                    dst_pixel[3].max(src_pixel[3]),
                ];
                base.put_pixel(target_x, target_y, image::Rgba(blended));
            }
        }
    }
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "overlay_icons",
        TaskType::Hybrid,
        TaskTier::Surgeon,
        |context| overlay_icons(context),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_overlay_icons() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let context = crate::hurray::context::HurrayContext::new(&temp_dir.path().to_string_lossy());
        let result = overlay_icons(&context);
        assert!(result.is_ok());
    }
}
