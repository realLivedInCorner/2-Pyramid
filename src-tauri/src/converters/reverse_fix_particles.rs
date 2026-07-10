use std::collections::HashMap;
use std::path::Path;

use crate::hurray::context::HurrayContext;
use crate::hurray::scheduler::{TaskTier, TaskType};
use crate::image_utils::paste_region;

/// Reconstruct a 16x16 particle atlas from individual tile images.
/// Source files searched in particle/ and entity/ directories.
/// Output saved as particles.png in the particle directory.
/// Individual tile files are deferred for cleanup at the end of conversion.
pub fn reverse_fix_particles(ctx: &HurrayContext) -> Result<(), String> {
    let base = ctx.temp_dir();
    let particle_dir = base.join("assets/minecraft/textures/particle");
    let entity_dir = base.join("assets/minecraft/textures/entity");

    if !particle_dir.exists() {
        crate::log_info!("particle dir not found, skip reverse_fix_particles");
        return Ok(());
    }

    let particles_output = particle_dir.join("particles.png");
    if particles_output.exists() {
        crate::log_info!("particles.png already exists, skip reverse_fix_particles");
        return Ok(());
    }

    // Build filename→position map (same as split in fix_particles)
    let mut positions: HashMap<&str, (u32, u32)> = HashMap::new();
    // Row 0: generic_0..generic_7
    for c in 0u32..8 {
        positions.insert(Box::leak(format!("generic_{}.png", c).into_boxed_str()), (0, c));
    }
    // Row 1: splash_0..splash_3
    for c in 0u32..4 {
        positions.insert(Box::leak(format!("splash_{}.png", c).into_boxed_str()), (1, c + 3));
    }
    positions.insert("bubble.png", (2, 0));
    positions.insert("fishing_hook.png", (2, 1));
    positions.insert("flame.png", (3, 0));
    positions.insert("lava.png", (3, 1));
    let r4 = ["note.png", "critical_hit.png", "enchanted_hit.png"];
    for (i, n) in r4.iter().enumerate() {
        positions.insert(n, (4, i as u32));
    }
    let r5 = ["heart.png", "angry.png", "glint.png"];
    for (i, n) in r5.iter().enumerate() {
        positions.insert(n, (5, i as u32));
    }
    let r7 = ["drip_hang.png", "drip_fall.png", "drip_land.png"];
    for (i, n) in r7.iter().enumerate() {
        positions.insert(n, (7, i as u32));
    }
    for c in 0u32..8 {
        positions.insert(Box::leak(format!("effect_{}.png", c).into_boxed_str()), (8, c));
    }
    for c in 0u32..8 {
        positions.insert(Box::leak(format!("spell_{}.png", c).into_boxed_str()), (9, c));
    }
    for c in 0u32..8 {
        positions.insert(Box::leak(format!("spark_{}.png", c).into_boxed_str()), (10, c));
    }

    // Find split_size from first available tile
    let mut split_size = 0u32;
    for (filename, _) in &positions {
        let particle_fp = particle_dir.join(filename);
        let entity_fp = entity_dir.join(filename);
        if let Ok(img) = image::open(&particle_fp).or_else(|_| image::open(&entity_fp)) {
            split_size = img.width();
            break;
        }
    }

    if split_size == 0 {
        crate::log_info!("no split particle tiles found, skip reverse_fix_particles");
        return Ok(());
    }

    // Create 16x16 atlas
    let rows = 16u32;
    let cols = 16u32;
    let merged_w = cols * split_size;
    let merged_h = rows * split_size;
    let mut merged = image::RgbaImage::new(merged_w, merged_h);

    for (filename, &(row, col)) in &positions {
        let particle_fp = particle_dir.join(filename);
        let entity_fp = entity_dir.join(filename);

        let img_result = if particle_fp.exists() {
            ctx.defer_remove_file(&particle_fp);
            image::open(&particle_fp)
        } else if entity_fp.exists() {
            ctx.defer_remove_file(&entity_fp);
            image::open(&entity_fp)
        } else {
            crate::log_info!("missing tile: {}", filename);
            continue;
        };

        if let Ok(img) = img_result {
            let rgba = img.to_rgba8();
            paste_region(
                &mut merged,
                &rgba,
                col * split_size,
                row * split_size,
            ).map_err(|e| format!("failed to paste region: {}", e))?;
        }
    }

    merged
        .save(&particles_output)
        .map_err(|e| format!("failed to save {}: {}", particles_output.display(), e))?;

    crate::log_info!("reconstructed particles.png ({}x{})", merged_w, merged_h);
    crate::log_info!("reverse_fix_particles completed (source tile cleanup deferred)");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "reverse_fix_particles",
        TaskType::Hybrid,
        TaskTier::Surgeon,
        |context| reverse_fix_particles(context),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_reverse_fix_particles() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let ctx = HurrayContext::new(temp_dir.path().to_str().unwrap());
        let result = reverse_fix_particles(&ctx);
        assert!(result.is_ok());
    }
}
