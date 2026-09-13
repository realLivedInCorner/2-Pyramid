use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_fix2_horse_ui(ctx: &HurrayContext) -> Result<(), String> {
    let slot = ctx.temp_dir().join("assets/minecraft/textures/gui/sprites/container/slot");
    for name in &["horse_armor.png", "llama_armor.png", "saddle.png"] {
        let f = slot.join(name);
        if f.exists() {
            ctx.defer_remove_file(&f);
        }
    }
    Ok(())
}
