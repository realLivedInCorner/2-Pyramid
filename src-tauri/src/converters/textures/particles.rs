use std::fs;
use std::path::{Path, PathBuf};

use image::imageops;

use crate::hurray::scheduler::{TaskTier, TaskType};

/// Split particles.png (16x16 grid) into individual named tile images.
/// Saved to particle/ and entity/ directories.
fn split_particles_image(particles_path: &Path) -> Result<(), String> {
    let img = image::open(particles_path)
        .map_err(|e| format!("failed to open {}: {}", particles_path.display(), e))?
        .to_rgba8();

    let (w, h) = img.dimensions();
    if w != h || w % 16 != 0 {
        crate::log_info!("particles.png is not square /16: {}x{}, skip split", w, h);
        return Ok(());
    }

    let split_size = w / 16;
    let output_particle = particles_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let output_entity = output_particle
        .parent()
        .map(|p| p.join("entity"))
        .unwrap_or_else(|| PathBuf::from("entity"));

    fs::create_dir_all(&output_particle)
        .map_err(|e| format!("failed to create {}: {}", output_particle.display(), e))?;
    fs::create_dir_all(&output_entity)
        .map_err(|e| format!("failed to create {}: {}", output_entity.display(), e))?;

    for row in 0u32..16 {
        for col in 0u32..16 {
            let cropped = imageops::crop_imm(
                &img,
                col * split_size,
                row * split_size,
                split_size,
                split_size,
            )
            .to_image();

            let save_result: Result<(), String> = match (row, col) {
                (0, c) if c < 8 => cropped
                    .save(output_particle.join(format!("generic_{}.png", c)))
                    .map_err(|e| format!("{}", e)),
                (1, c) if (3..=6).contains(&c) => cropped
                    .save(output_particle.join(format!("splash_{}.png", c - 3)))
                    .map_err(|e| format!("{}", e)),
                (2, 0) => cropped
                    .save(output_particle.join("bubble.png"))
                    .map_err(|e| format!("{}", e)),
                (2, 1) => cropped
                    .save(output_entity.join("fishing_hook.png"))
                    .map_err(|e| format!("{}", e)),
                (3, 0) => cropped
                    .save(output_particle.join("flame.png"))
                    .map_err(|e| format!("{}", e)),
                (3, 1) => cropped
                    .save(output_particle.join("lava.png"))
                    .map_err(|e| format!("{}", e)),
                (4, c) if c < 3 => {
                    let names = ["note.png", "critical_hit.png", "enchanted_hit.png"];
                    cropped
                        .save(output_particle.join(names[c as usize]))
                        .map_err(|e| format!("{}", e))
                }
                (5, c) if c < 3 => {
                    let names = ["heart.png", "angry.png", "glint.png"];
                    cropped
                        .save(output_particle.join(names[c as usize]))
                        .map_err(|e| format!("{}", e))
                }
                (7, c) if c < 3 => {
                    let names = ["drip_hang.png", "drip_fall.png", "drip_land.png"];
                    cropped
                        .save(output_particle.join(names[c as usize]))
                        .map_err(|e| format!("{}", e))
                }
                (8, c) if c < 8 => cropped
                    .save(output_particle.join(format!("effect_{}.png", c)))
                    .map_err(|e| format!("{}", e)),
                (9, c) if c < 8 => cropped
                    .save(output_particle.join(format!("spell_{}.png", c)))
                    .map_err(|e| format!("{}", e)),
                (10, c) if c < 8 => cropped
                    .save(output_particle.join(format!("spark_{}.png", c)))
                    .map_err(|e| format!("{}", e)),
                _ => Ok(()),
            };

            if let Err(e) = save_result {
                crate::log_info!("failed to save particle tile ({},{}): {}", row, col, e);
            }
        }
    }

    // Delete original particles.png
    fs::remove_file(particles_path)
        .map_err(|e| format!("failed to remove {}: {}", particles_path.display(), e))?;

    crate::log_info!("split and removed particles.png");
    Ok(())
}

pub fn fix_particles(path: &Path) -> Result<(), String> {
    let particles_path = path.join("assets/minecraft/textures/particle/particles.png");

    if particles_path.exists() {
        crate::log_info!("processing particles.png at {}", particles_path.display());
        split_particles_image(&particles_path)?;
    } else {
        crate::log_info!("particles.png not found, skip fix_particles");
    }

    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "fix_particles",
        TaskType::Exclusive,
        TaskTier::Surgeon,
        |context| fix_particles(context.temp_dir()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_fix_particles() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let result = fix_particles(temp_dir.path());
        assert!(result.is_ok());
    }
}
