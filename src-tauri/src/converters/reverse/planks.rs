use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_generate_redwood_cherry_bamboo_planks(ctx: &HurrayContext) -> Result<(), String> {
    let block = ctx.temp_dir().join("assets/minecraft/textures/block");
    for name in &[
        "mangrove_planks.png",
        "cherry_planks.png",
        "bamboo_planks.png",
        "mangrove_log.png",
        "mangrove_log_top.png",
        "cherry_log.png",
        "cherry_log_top.png",
        "bamboo_block.png",
        "bamboo_block_top.png",
        "bamboo_mosaic.png",
    ] {
        let p = block.join(name);
        if p.exists() { ctx.defer_remove_file(&p); }
        let m = block.join(format!("{}.mcmeta", name));
        if m.exists() { ctx.defer_remove_file(&m); }
    }
    Ok(())
}

pub fn reverse_generate_pale_planks(ctx: &HurrayContext) -> Result<(), String> {
    let block = ctx.temp_dir().join("assets/minecraft/textures/block");
    for name in &["pale_oak_planks.png", "pale_oak_log.png", "pale_oak_log_top.png"] {
        let p = block.join(name);
        if p.exists() { ctx.defer_remove_file(&p); }
        let m = block.join(format!("{}.mcmeta", name));
        if m.exists() { ctx.defer_remove_file(&m); }
    }
    Ok(())
}
