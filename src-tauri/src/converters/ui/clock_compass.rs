use std::fs;
use std::path::Path;

use image::imageops;

use crate::hurray::context::HurrayContext;
use crate::hurray::scheduler::{TaskTier, TaskType};

/// Split a vertical strip image into `retain_num` evenly distributed frames.
/// Saves frames as `{prefix}_{j:02d}.png` in `output_dir`, then deletes the
/// original image and its `.mcmeta` companion.
fn split_image<P: AsRef<Path>>(
    image_path: P,
    output_dir: P,
    prefix: &str,
    retain_num: u32,
) -> Result<(), String> {
    let image_path = image_path.as_ref();
    let output_dir = output_dir.as_ref();

    let img = image::open(image_path)
        .map_err(|e| format!("failed to open {}: {}", image_path.display(), e))?
        .to_rgba8();

    let (img_width, img_height) = img.dimensions();
    if img_width == 0 {
        return Err(format!("invalid image width 0 for {}", image_path.display()));
    }

    let num_splits = img_height / img_width;
    let split_height = img_height / num_splits.max(1);

    let indices: Vec<u32> = if num_splits > retain_num {
        let step = num_splits as f64 / retain_num as f64;
        (0..retain_num)
            .map(|i| ((i as f64) * step) as u32)
            .map(|idx| idx.min(num_splits - 1))
            .collect()
    } else {
        (0..num_splits).collect()
    };

    for (j, &i) in indices.iter().enumerate() {
        let y = i * split_height;
        let cropped = imageops::crop_imm(&img, 0, y, img_width, split_height).to_image();
        let out_path = output_dir.join(format!("{}_{:02}.png", prefix, j));
        fs::create_dir_all(output_dir)
            .map_err(|e| format!("failed to create output dir {}: {}", output_dir.display(), e))?;
        cropped
            .save(&out_path)
            .map_err(|e| format!("failed to save {}: {}", out_path.display(), e))?;
    }

    crate::log_info!(
        "split {} into {} frames (num_splits={}, retain_num={})",
        image_path.display(),
        indices.len(),
        num_splits,
        retain_num
    );

    // Delete original image
    fs::remove_file(image_path)
        .map_err(|e| format!("failed to remove {}: {}", image_path.display(), e))?;

    // Delete .mcmeta companion if exists
    let mcmeta_path = image_path.with_extension("png.mcmeta");
    if mcmeta_path.exists() {
        fs::remove_file(&mcmeta_path)
            .map_err(|e| format!("failed to remove {}: {}", mcmeta_path.display(), e))?;
    }

    Ok(())
}

pub fn fix_clock_compass(ctx: &HurrayContext) -> Result<(), String> {
    let assets_path = ctx.temp_dir().join("assets/minecraft/textures");
    let items_path = assets_path.join("items");

    let clock_path = items_path.join("clock.png");
    let compass_path = items_path.join("compass.png");

    crate::log_info!("processing clock/compass textures in {}", items_path.display());

    if clock_path.exists() {
        crate::log_info!("found clock.png, splitting...");
        split_image(&clock_path, &items_path, "clock", 64)?;
    } else {
        crate::log_info!("clock.png not found, skip");
    }

    if compass_path.exists() {
        crate::log_info!("found compass.png, splitting...");
        split_image(&compass_path, &items_path, "compass", 32)?;
    } else {
        crate::log_info!("compass.png not found, skip");
    }

    crate::log_info!("clock/compass pass finished");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "fix_clock_compass",
        TaskType::Hybrid,
        TaskTier::Surgeon,
        |context| fix_clock_compass(context),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_fix_clock_compass() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let context = crate::hurray::context::HurrayContext::new(&temp_dir.path().to_string_lossy());
        let result = fix_clock_compass(&context);
        assert!(result.is_ok());
    }
}
