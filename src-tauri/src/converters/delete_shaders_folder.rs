use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn delete_shaders_folder(ctx: &HurrayContext) -> Result<(), String> {
    let shaders_path = ctx.temp_dir().join("assets/minecraft/shaders");
    if shaders_path.exists() {
        crate::log_info!("deferring cleanup of {}", shaders_path.display());
        ctx.defer_remove_dir(&shaders_path);
    }
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "delete_shaders_folder",
        crate::hurray::scheduler::TaskType::Exclusive,
        crate::hurray::scheduler::TaskTier::Eraser,
        |context| delete_shaders_folder(context),
    );
}
