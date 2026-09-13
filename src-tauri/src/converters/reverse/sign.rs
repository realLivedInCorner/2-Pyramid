use std::fs;
use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_fix_sign(ctx: &HurrayContext) -> Result<(), String> {
    let item = ctx.temp_dir().join("assets/minecraft/textures/item");

    // Defer deletion of the 11 generated sign variants
    for name in &[
        "oak_sign.png", "birch_sign.png", "acacia_sign.png", "dark_oak_sign.png",
        "jungle_sign.png", "crimson_sign.png", "warped_sign.png", "mangrove_sign.png",
        "pale_oak_sign.png", "bamboo_sign.png", "cherry_sign.png",
    ] {
        let f = item.join(name);
        if f.exists() {
            ctx.defer_remove_file(&f);
        }
    }

    // Rename spruce_sign.png back to oak_sign.png (part of conversion logic, immediate)
    let spruce = item.join("spruce_sign.png");
    let oak = item.join("oak_sign.png");
    if spruce.exists() {
        fs::rename(&spruce, &oak).map_err(|e| format!("rename spruce_sign->oak_sign: {}", e))?;
    }

    Ok(())
}
