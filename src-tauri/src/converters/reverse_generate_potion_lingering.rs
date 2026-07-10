use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_generate_potion_lingering(ctx: &HurrayContext) -> Result<(), String> {
    let items = ctx.temp_dir().join("assets/minecraft/textures/items");
    for name in &[
        "lingering_potion.png",
        "lingering_potion.png.mcmeta",
        "potion_bottle_lingering.png",
        "potion_bottle_lingering.png.mcmeta",
    ] {
        let f = items.join(name);
        if f.exists() {
            ctx.defer_remove_file(&f);
        }
    }
    Ok(())
}
