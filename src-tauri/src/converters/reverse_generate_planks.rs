use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_generate_redwood_cherry_bamboo_planks(ctx: &HurrayContext) -> Result<(), String> {
    let block = ctx.temp_dir().join("assets/minecraft/textures/block");
    for name in &["mangrove_planks.png", "cherry_planks.png", "bamboo_planks.png"] {
        let p = block.join(name);
        if p.exists() { ctx.defer_remove_file(&p); }
        let m = block.join(format!("{}.mcmeta", name));
        if m.exists() { ctx.defer_remove_file(&m); }
    }
    Ok(())
}

pub fn reverse_generate_pale_planks(ctx: &HurrayContext) -> Result<(), String> {
    let block = ctx.temp_dir().join("assets/minecraft/textures/block");
    let p = block.join("pale_oak_planks.png");
    if p.exists() { ctx.defer_remove_file(&p); }
    let m = block.join("pale_oak_planks.png.mcmeta");
    if m.exists() { ctx.defer_remove_file(&m); }
    Ok(())
}
