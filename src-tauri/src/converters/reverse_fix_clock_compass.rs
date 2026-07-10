use std::fs;
use std::io::Write;
use std::path::Path;

use crate::hurray::context::HurrayContext;
use crate::hurray::scheduler::{TaskTier, TaskType};
use crate::image_utils::paste_region;

/// Vertically merge a list of image file paths into a single output image,
/// then defer deletion of the individual source files to the end of conversion.
fn merge_images<P: AsRef<Path>>(image_paths: &[P], output_path: &Path, ctx: &HurrayContext) -> Result<(), String> {
    let mut images: Vec<image::RgbaImage> = Vec::new();
    let mut max_width = 0u32;
    let mut total_height = 0u32;

    for path in image_paths {
        let path = path.as_ref();
        if !path.exists() {
            continue;
        }
        let img = image::open(path)
            .map_err(|e| format!("failed to open {}: {}", path.display(), e))?
            .to_rgba8();
        let (w, h) = img.dimensions();
        max_width = max_width.max(w);
        total_height += h;
        images.push(img);
    }

    if images.is_empty() {
        crate::log_info!("merge_images: no source images found for {}", output_path.display());
        return Ok(());
    }

    let mut merged = image::RgbaImage::new(max_width, total_height);
    let mut y_offset = 0u32;
    for img in &images {
        let h = img.height();
        paste_region(&mut merged, img, 0, y_offset).map_err(|e| format!("failed to paste region: {}", e))?;
        y_offset += h;
    }

    merged
        .save(output_path)
        .map_err(|e| format!("failed to save {}: {}", output_path.display(), e))?;

    crate::log_info!("merged {} images into {}", images.len(), output_path.display());

    // Defer deletion of individual source files to end of conversion
    for path in image_paths {
        let path = path.as_ref();
        if path.exists() {
            ctx.defer_remove_file(path);
        }
    }

    Ok(())
}

/// Create a .mcmeta file with an empty animation JSON object.
fn create_mcmeta_file(output_path: &Path) -> Result<(), String> {
    let content = r#"{"animation":{}}"#;
    let mut file = fs::File::create(output_path)
        .map_err(|e| format!("failed to create {}: {}", output_path.display(), e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("failed to write {}: {}", output_path.display(), e))?;
    crate::log_info!("created mcmeta: {}", output_path.display());
    Ok(())
}

/// Reverse the clock/compass fix: merge individual frames back into the
/// unified clock.png and compass.png, then create .mcmeta files.
/// Source frame files are deferred for cleanup at the end of conversion.
pub fn reverse_fix_clock_compass(ctx: &HurrayContext) -> Result<(), String> {
    let items_path = ctx.temp_dir().join("assets/minecraft/textures/items");

    if !items_path.exists() {
        crate::log_info!("items dir not found, skip reverse_fix_clock_compass");
        return Ok(());
    }

    // Merge compass: 32 frames (00-31)
    let compass_images: Vec<std::path::PathBuf> = (0..32)
        .map(|i| items_path.join(format!("compass_{:02}.png", i)))
        .filter(|p| p.exists())
        .collect();

    if !compass_images.is_empty() {
        let compass_output = items_path.join("compass.png");
        merge_images(&compass_images, &compass_output, ctx)?;
        create_mcmeta_file(&items_path.join("compass.png.mcmeta"))?;
    }

    // Merge clock: 64 frames (00-63)
    let clock_images: Vec<std::path::PathBuf> = (0..64)
        .map(|i| items_path.join(format!("clock_{:02}.png", i)))
        .filter(|p| p.exists())
        .collect();

    if !clock_images.is_empty() {
        let clock_output = items_path.join("clock.png");
        merge_images(&clock_images, &clock_output, ctx)?;
        create_mcmeta_file(&items_path.join("clock.png.mcmeta"))?;
    }

    crate::log_info!("reverse_fix_clock_compass completed");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "reverse_fix_clock_compass",
        TaskType::Hybrid,
        TaskTier::Surgeon,
        |context| reverse_fix_clock_compass(context),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_reverse_fix_clock_compass() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let ctx = HurrayContext::new(temp_dir.path().to_str().unwrap());
        let result = reverse_fix_clock_compass(&ctx);
        assert!(result.is_ok());
    }
}
