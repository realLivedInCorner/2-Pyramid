use std::fs;
use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_generate_boat(ctx: &HurrayContext) -> Result<(), String> {
    let items = ctx.temp_dir().join("assets/minecraft/textures/items");

    // Defer deletion of generated boat variants
    for name in &["oak_boat.png", "birch_boat.png", "acacia_boat.png", "dark_oak_boat.png", "jungle_boat.png"] {
        let f = items.join(name);
        if f.exists() {
            ctx.defer_remove_file(&f);
        }
    }

    // Rename spruce_boat.png back to boat.png (part of conversion logic, immediate)
    let spruce = items.join("spruce_boat.png");
    let boat = items.join("boat.png");
    if spruce.exists() && !boat.exists() {
        fs::rename(&spruce, &boat).map_err(|e| format!("rename spruce_boat->boat: {}", e))?;
    }

    Ok(())
}
