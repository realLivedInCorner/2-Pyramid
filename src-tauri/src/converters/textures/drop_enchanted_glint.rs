use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn delete_enchanted_item_glint(ctx: &HurrayContext) -> Result<(), String> {
    let glint_path = ctx.temp_dir()
        .join("assets/minecraft/textures/misc/enchanted_item_glint.png");
    if glint_path.exists() {
        crate::log_info!("deferring cleanup of {}", glint_path.display());
        ctx.defer_remove_file(&glint_path);
    }
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "delete_enchanted_item_glint",
        crate::hurray::scheduler::TaskType::Exclusive,
        crate::hurray::scheduler::TaskTier::Eraser,
        |context| delete_enchanted_item_glint(context),
    );
}
