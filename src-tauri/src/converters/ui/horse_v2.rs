use std::fs;
use std::path::Path;

use crate::hurray::scheduler::{TaskTier, TaskType};

/// Copy horse-related slot sprites from the horse folder to the slot folder
/// with proper renaming.
///
/// Source: assets/minecraft/textures/gui/sprites/container/horse/
/// Target: assets/minecraft/textures/gui/sprites/container/slot/
pub fn fix2_horse_ui(path: &Path) -> Result<(), String> {
    let source_dir = path.join("assets/minecraft/textures/gui/sprites/container/horse");
    let target_dir = path.join("assets/minecraft/textures/gui/sprites/container/slot");

    if !source_dir.exists() {
        crate::log_info!("horse sprites dir not found, skip fix2_horse_ui");
        return Ok(());
    }

    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("failed to create {}: {}", target_dir.display(), e))?;

    let file_mappings: [(&str, &str); 3] = [
        ("armor_slot.png", "horse_armor.png"),
        ("llama_armor_slot.png", "llama_armor.png"),
        ("saddle_slot.png", "saddle.png"),
    ];

    for (src_name, dest_name) in &file_mappings {
        let src_path = source_dir.join(src_name);
        if src_path.exists() {
            let dest_path = target_dir.join(dest_name);
            fs::copy(&src_path, &dest_path)
                .map_err(|e| format!("failed to copy {} -> {}: {}", src_name, dest_path.display(), e))?;
            crate::log_info!("copied {} -> {}", src_name, dest_path.display());
        } else {
            crate::log_info!("source sprite not found: {}", src_path.display());
        }
    }

    crate::log_info!("fix2_horse_ui completed");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "fix2_horse_ui",
        TaskType::Hybrid,
        TaskTier::Surgeon,
        |context| {
            let path = context.temp_dir();
            fix2_horse_ui(path)
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_fix2_horse_ui() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let result = fix2_horse_ui(temp_dir.path());
        assert!(result.is_ok());
    }
}
