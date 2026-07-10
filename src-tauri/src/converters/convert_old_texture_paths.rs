use std::fs;
use std::path::Path;

pub fn convert_old_texture_paths(resource_pack_path: &Path) -> Result<(), String> {
    let assets_path = resource_pack_path.join("assets/minecraft");
    if !assets_path.exists() {
        crate::log_info!("assets/minecraft not found, skip old texture path conversion");
        return Ok(());
    }

    let texture_mappings = [("terrain.png", "block.png"), ("gui/items.png", "item.png")];

    for (old_path, new_path) in texture_mappings {
        let old_full_path = assets_path.join(old_path);
        let new_full_path = assets_path.join(new_path);

        if old_full_path.exists() {
            if let Some(parent) = new_full_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("failed to create parent for {}: {}", new_path, e))?;
            }

            fs::copy(&old_full_path, &new_full_path)
                .map_err(|e| format!("failed to copy {} -> {}: {}", old_path, new_path, e))?;

            crate::log_info!("converted {} -> {}", old_path, new_path);
        }
    }

    crate::log_info!("old texture path conversion completed");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "convert_old_texture_paths",
        crate::hurray::scheduler::TaskType::Exclusive,
        crate::hurray::scheduler::TaskTier::Eraser,
        |context| {
            let resource_pack_path = context.temp_dir();
            convert_old_texture_paths(resource_pack_path)
        },
    );
}