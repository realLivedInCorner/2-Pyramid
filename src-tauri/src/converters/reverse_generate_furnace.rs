use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_generate_furnace(ctx: &HurrayContext) -> Result<(), String> {
    let gui = ctx.temp_dir().join("assets/minecraft/textures/gui/container");
    for name in &["blast_furnace.png", "smoker.png"] {
        let f = gui.join(name);
        if f.exists() {
            ctx.defer_remove_file(&f);
        }
    }
    Ok(())
}
