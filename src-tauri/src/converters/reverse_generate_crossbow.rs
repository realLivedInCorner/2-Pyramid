use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_generate_crossbow(ctx: &HurrayContext) -> Result<(), String> {
    let item = ctx.temp_dir().join("assets/minecraft/textures/item");
    for name in &[
        "crossbow_standby.png",
        "crossbow_pulling_0.png",
        "crossbow_pulling_1.png",
        "crossbow_pulling_2.png",
        "crossbow_arrow.png",
        "crossbow_firework.png",
    ] {
        let f = item.join(name);
        if f.exists() {
            ctx.defer_remove_file(&f);
        }
    }
    Ok(())
}
