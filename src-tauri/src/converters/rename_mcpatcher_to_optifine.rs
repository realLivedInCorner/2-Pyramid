use std::fs;
use std::path::Path;

pub fn rename_mcpatcher_to_optifine(resource_pack_path: &Path) -> Result<(), String> {
    let mcpatcher_path = resource_pack_path.join("assets/minecraft/mcpatcher");
    let optifine_path = resource_pack_path.join("assets/minecraft/optifine");

    if !mcpatcher_path.exists() {
        crate::log_info!("mcpatcher folder not found, skip");
        return Ok(());
    }

    if optifine_path.exists() {
        crate::log_info!("optifine folder already exists, skip rename");
        return Ok(());
    }

    fs::rename(&mcpatcher_path, &optifine_path).map_err(|e| {
        format!(
            "failed to rename {} -> {}: {}",
            mcpatcher_path.display(),
            optifine_path.display(),
            e
        )
    })?;

    crate::log_info!("renamed mcpatcher to optifine");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "rename_mcpatcher_to_optifine",
        crate::hurray::scheduler::TaskType::Exclusive,
        crate::hurray::scheduler::TaskTier::Eraser,
        |context| rename_mcpatcher_to_optifine(context.temp_dir()),
    );
}