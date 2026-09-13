use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_fix_slider(ctx: &HurrayContext) -> Result<(), String> {
    let f = ctx.temp_dir().join("assets/minecraft/textures/gui/slider.png");
    if f.exists() {
        ctx.defer_remove_file(&f);
    }
    Ok(())
}
