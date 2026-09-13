use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn delete_horse_folder(ctx: &HurrayContext) -> Result<(), String> {
    let horse_path = ctx.temp_dir().join("assets/minecraft/textures/entity/horse");
    if horse_path.exists() {
        crate::log_info!("deferring cleanup of {}", horse_path.display());
        ctx.defer_remove_dir(&horse_path);
    }
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "delete_horse_folder",
        crate::hurray::scheduler::TaskType::Exclusive,
        crate::hurray::scheduler::TaskTier::Eraser,
        |context| delete_horse_folder(context),
    );
}
