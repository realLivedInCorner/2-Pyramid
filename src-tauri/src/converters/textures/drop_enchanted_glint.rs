
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
