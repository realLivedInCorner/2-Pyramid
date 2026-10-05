
use crate::hurray::context::HurrayContext;

pub fn delete_horse_folder(ctx: &HurrayContext) -> Result<(), String> {
    let horse_path = ctx.temp_dir().join("assets/minecraft/textures/entity/horse");
    if horse_path.exists() {
        crate::log_info!("deferring cleanup of {}", horse_path.display());
        ctx.defer_remove_dir(&horse_path);
    }
    Ok(())
}
