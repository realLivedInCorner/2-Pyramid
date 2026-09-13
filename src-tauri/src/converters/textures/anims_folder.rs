use std::fs;
use std::path::Path;

pub fn process_anims_folder(resource_pack_path: &Path) -> Result<(), String> {
    let anims_path = resource_pack_path.join("assets/minecraft/anims");

    if !anims_path.exists() {
        crate::log_info!("anims folder not found, skipping");
        return Ok(());
    }

    let entries = fs::read_dir(&anims_path)
        .map_err(|e| format!("failed to read anims folder: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read anims entry: {}", e))?;
        let file_name = entry
            .file_name()
            .into_string()
            .map_err(|_| "invalid unicode file name in anims folder".to_string())?;

        crate::log_info!("found anim file: {}", file_name);
        // Legacy behavior: discovery only.
    }

    crate::log_info!("anims folder processed");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "process_anims_folder",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Surgeon,
        |context| {
            let resource_pack_path = context.temp_dir();
            process_anims_folder(resource_pack_path)
        },
    );
}