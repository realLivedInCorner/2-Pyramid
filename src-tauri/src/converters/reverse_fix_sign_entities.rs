use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_fix_sign_entities(ctx: &HurrayContext) -> Result<(), String> {
    let signs_dir = ctx.temp_dir().join("assets/minecraft/textures/entity/signs");
    if signs_dir.exists() {
        ctx.defer_remove_dir(&signs_dir);
    }
    Ok(())
}
