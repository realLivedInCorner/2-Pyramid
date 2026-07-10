use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn delete_font_folder(ctx: &HurrayContext) -> Result<(), String> {
    let font_path = ctx.temp_dir().join("assets/minecraft/font");
    if font_path.exists() {
        crate::log_info!("deferring cleanup of {}", font_path.display());
        ctx.defer_remove_dir(&font_path);
    }
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "delete_font_folder",
        crate::hurray::scheduler::TaskType::Exclusive,
        crate::hurray::scheduler::TaskTier::Eraser,
        |context| delete_font_folder(context),
    );
}
