
use crate::hurray::context::HurrayContext;

pub fn delete_shaders_folder(ctx: &HurrayContext) -> Result<(), String> {
    let shaders_path = ctx.temp_dir().join("assets/minecraft/shaders");
    if shaders_path.exists() {
        crate::log_info!("deferring cleanup of {}", shaders_path.display());
        ctx.defer_remove_dir(&shaders_path);
    }
    Ok(())
}
