use std::fs;
use std::path::Path;

use crate::hurray::scheduler::{TaskTier, TaskType};

/// Reverse the armor models fix: move files from humanoid/ and humanoid_leggings/
/// back to models/armor/ with the original layer_1/layer_2 naming.
pub fn reverse_fix_armor_models(path: &Path) -> Result<(), String> {
    let armor_target_dir = path.join("assets/minecraft/textures/models/armor");
    let humanoid_source_dir = path.join("assets/minecraft/textures/entity/equipment/humanoid");
    let leggings_source_dir = path.join("assets/minecraft/textures/entity/equipment/humanoid_leggings");

    fs::create_dir_all(&armor_target_dir)
        .map_err(|e| format!("failed to create {}: {}", armor_target_dir.display(), e))?;

    // humanoid → layer_1
    let layer1_mappings: [(&str, &str); 8] = [
        ("chainmail.png", "chainmail_layer_1.png"),
        ("diamond.png", "diamond_layer_1.png"),
        ("iron.png", "iron_layer_1.png"),
        ("gold.png", "gold_layer_1.png"),
        ("leather.png", "leather_layer_1.png"),
        ("leather_overlay.png", "leather_layer_1_overlay.png"),
        ("netherite.png", "netherite_layer_1.png"),
        ("copper.png", "copper_layer_1.png"),
    ];

    // humanoid_leggings → layer_2
    let layer2_mappings: [(&str, &str); 8] = [
        ("chainmail.png", "chainmail_layer_2.png"),
        ("diamond.png", "diamond_layer_2.png"),
        ("iron.png", "iron_layer_2.png"),
        ("gold.png", "gold_layer_2.png"),
        ("leather.png", "leather_layer_2.png"),
        ("leather_overlay.png", "leather_layer_2_overlay.png"),
        ("netherite.png", "netherite_layer_2.png"),
        ("copper.png", "copper_layer_2.png"),
    ];

    for (src_name, dest_name) in &layer1_mappings {
        let src_path = humanoid_source_dir.join(src_name);
        if src_path.exists() {
            let dest_path = armor_target_dir.join(dest_name);
            fs::rename(&src_path, &dest_path)
                .map_err(|e| format!("failed to move {} -> {}: {}", src_name, dest_path.display(), e))?;
            crate::log_info!("reverse moved {} -> {}", src_name, dest_path.display());
        }
    }

    for (src_name, dest_name) in &layer2_mappings {
        let src_path = leggings_source_dir.join(src_name);
        if src_path.exists() {
            let dest_path = armor_target_dir.join(dest_name);
            fs::rename(&src_path, &dest_path)
                .map_err(|e| format!("failed to move {} -> {}: {}", src_name, dest_path.display(), e))?;
            crate::log_info!("reverse moved {} -> {}", src_name, dest_path.display());
        }
    }

    crate::log_info!("reverse_fix_armor_models completed");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "reverse_fix_armor_models",
        TaskType::Hybrid,
        TaskTier::Surgeon,
        |context| reverse_fix_armor_models(context.temp_dir()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_reverse_fix_armor_models() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let result = reverse_fix_armor_models(temp_dir.path());
        assert!(result.is_ok());
    }
}
