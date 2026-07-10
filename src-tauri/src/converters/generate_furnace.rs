use std::fs;
use std::path::Path;

pub fn generate_furnace(resource_pack_path: &Path) -> Result<(), String> {
    let container_path = resource_pack_path.join("assets/minecraft/textures/gui/container");
    let furnace_path = container_path.join("furnace.png");
    if !furnace_path.exists() {
        return Ok(());
    }

    let blast_path = container_path.join("blast_furnace.png");
    let smoker_path = container_path.join("smoker.png");

    fs::copy(&furnace_path, &blast_path)
        .map_err(|e| format!("failed to copy to blast_furnace.png: {}", e))?;
    fs::copy(&furnace_path, &smoker_path)
        .map_err(|e| format!("failed to copy to smoker.png: {}", e))?;

    crate::log_info!("generated blast_furnace.png and smoker.png");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "generate_furnace",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_furnace(context.temp_dir()),
    );
}
