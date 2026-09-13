use std::fs;
use std::path::Path;

use crate::hurray::scheduler::{TaskTier, TaskType};

/// Move and rename armor model files from the old directory structure to the new one.
///
/// layer_1 → entity/equipment/humanoid/
/// layer_2 → entity/equipment/humanoid_leggings/
pub fn fix_armor_models(path: &Path) -> Result<(), String> {
    let armor_source_dir = path.join("assets/minecraft/textures/models/armor");
    let humanoid_target_dir = path.join("assets/minecraft/textures/entity/equipment/humanoid");
    let leggings_target_dir = path.join("assets/minecraft/textures/entity/equipment/humanoid_leggings");

    if !armor_source_dir.exists() {
        crate::log_info!("armor models dir not found, skip");
        return Ok(());
    }

    // Layer 1 → humanoid
    let layer1_mappings: [(&str, &str); 8] = [
        ("chainmail_layer_1.png", "chainmail.png"),
        ("diamond_layer_1.png", "diamond.png"),
        ("iron_layer_1.png", "iron.png"),
        ("gold_layer_1.png", "gold.png"),
        ("leather_layer_1.png", "leather.png"),
        ("leather_layer_1_overlay.png", "leather_overlay.png"),
        ("netherite_layer_1.png", "netherite.png"),
        ("copper_layer_1.png", "copper.png"),
    ];

    // Layer 2 → humanoid_leggings
    let layer2_mappings: [(&str, &str); 8] = [
        ("chainmail_layer_2.png", "chainmail.png"),
        ("diamond_layer_2.png", "diamond.png"),
        ("iron_layer_2.png", "iron.png"),
        ("gold_layer_2.png", "gold.png"),
        ("leather_layer_2.png", "leather.png"),
        ("leather_layer_2_overlay.png", "leather_overlay.png"),
        ("netherite_layer_2.png", "netherite.png"),
        ("copper_layer_2.png", "copper.png"),
    ];

    for (src_name, dest_name) in &layer1_mappings {
        let src_path = armor_source_dir.join(src_name);
        if src_path.exists() {
            fs::create_dir_all(&humanoid_target_dir)
                .map_err(|e| format!("failed to create {}: {}", humanoid_target_dir.display(), e))?;
            let dest_path = humanoid_target_dir.join(dest_name);
            fs::rename(&src_path, &dest_path)
                .map_err(|e| format!("failed to move {} -> {}: {}", src_name, dest_path.display(), e))?;
            crate::log_info!("moved {} -> {}", src_name, dest_path.display());
        } else {
            crate::log_info!("source file not found: {}", src_path.display());
        }
    }

    for (src_name, dest_name) in &layer2_mappings {
        let src_path = armor_source_dir.join(src_name);
        if src_path.exists() {
            fs::create_dir_all(&leggings_target_dir)
                .map_err(|e| format!("failed to create {}: {}", leggings_target_dir.display(), e))?;
            let dest_path = leggings_target_dir.join(dest_name);
            fs::rename(&src_path, &dest_path)
                .map_err(|e| format!("failed to move {} -> {}: {}", src_name, dest_path.display(), e))?;
            crate::log_info!("moved {} -> {}", src_name, dest_path.display());
        } else {
            crate::log_info!("source file not found: {}", src_path.display());
        }
    }

    crate::log_info!("armor model move/rename completed");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "fix_armor_models",
        TaskType::Hybrid,
        TaskTier::Surgeon,
        |context| {
            let path = context.temp_dir();
            fix_armor_models(path).map_err(|e| e.to_string())
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_fix_armor_models() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let result = fix_armor_models(temp_dir.path());
        assert!(result.is_ok());
    }
}
