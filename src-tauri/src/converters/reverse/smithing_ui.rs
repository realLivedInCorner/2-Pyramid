use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_generate_smithing_ui(ctx: &HurrayContext) -> Result<(), String> {
    let f = ctx.temp_dir().join("assets/minecraft/textures/gui/container/smithing.png");
    if f.exists() {
        ctx.defer_remove_file(&f);
    }
    Ok(())
}
