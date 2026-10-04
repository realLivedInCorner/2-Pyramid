
use crate::hurray::context::HurrayContext;

pub fn delete_font_folder(ctx: &HurrayContext) -> Result<(), String> {
    let font_path = ctx.temp_dir().join("assets/minecraft/font");
    if font_path.exists() {
        crate::log_info!("deferring cleanup of {}", font_path.display());
        ctx.defer_remove_dir(&font_path);
    }
    Ok(())
}
