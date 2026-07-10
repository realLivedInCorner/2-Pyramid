use std::fs;
use std::path::Path;

use crate::converters::adjust_hue_brightness::adjust_hue_brightness;
use crate::hurray::scheduler::{TaskTier, TaskType};

/// Generate 11 wood-variant sign entity textures from the base sign.png using
/// hue/brightness/saturation adjustment, then delete the original.
pub fn fix_sign_entities(path: &Path) -> Result<(), String> {
    let entity_path = path.join("assets/minecraft/textures/entity");
    let sign_path = entity_path.join("sign.png");

    if !sign_path.exists() {
        crate::log_info!("sign.png not found in entity/, skip fix_sign_entities");
        return Ok(());
    }

    let base_img = image::open(&sign_path)
        .map_err(|e| format!("failed to open {}: {}", sign_path.display(), e))?
        .to_rgba8();

    let signs_folder = entity_path.join("signs");
    fs::create_dir_all(&signs_folder)
        .map_err(|e| format!("failed to create {}: {}", signs_folder.display(), e))?;

    let sign_variants: [(&str, f32, f32, f32); 11] = [
        ("oak.png", 0.0, 15.0, 0.0),
        ("birch.png", 0.0, 40.0, 0.0),
        ("acacia.png", -23.0, 10.0, 0.0),
        ("dark_oak.png", 0.0, -15.0, 0.0),
        ("jungle.png", -10.0, 4.6, 0.0),
        ("crimson.png", -59.0, -30.0, 0.0),
        ("warped.png", 130.0, -33.0, 0.0),
        ("mangrove.png", -59.0, -10.0, 0.0),
        ("pale_oak.png", 0.0, 30.0, -100.0),
        ("bamboo.png", 25.0, 20.0, 0.0),
        ("cherry.png", -80.0, 20.0, 0.0),
    ];

    for (filename, hue, bright, sat) in &sign_variants {
        let adjusted = adjust_hue_brightness(base_img.clone(), *hue, *bright, *sat);
        let output_path = signs_folder.join(filename);
        adjusted
            .save(&output_path)
            .map_err(|e| format!("failed to save {}: {}", output_path.display(), e))?;
        crate::log_info!("generated sign variant: {}", filename);
    }

    // py: rename original sign.png -> signs/spruce.png (so the original becomes
    // the spruce variant; without this step the spruce variant is missing and
    // the texture pack would lose its spruce sign entity texture)
    let spruce_path = signs_folder.join("spruce.png");
    if !spruce_path.exists() {
        fs::rename(&sign_path, &spruce_path)
            .map_err(|e| format!("failed to rename sign.png to spruce.png: {}", e))?;
        crate::log_info!("renamed sign.png -> signs/spruce.png");
    } else {
        // spruce.png already present (re-run) — just drop the source
        fs::remove_file(&sign_path)
            .map_err(|e| format!("failed to remove {}: {}", sign_path.display(), e))?;
        crate::log_info!("spruce.png already present, removed sign.png");
    }

    crate::log_info!("fix_sign_entities completed");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "fix_sign_entities",
        TaskType::Exclusive,
        TaskTier::Surgeon,
        |context| fix_sign_entities(context.temp_dir()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_fix_sign_entities() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let result = fix_sign_entities(temp_dir.path());
        assert!(result.is_ok());
    }
}
