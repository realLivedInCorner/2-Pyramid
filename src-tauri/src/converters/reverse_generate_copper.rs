use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_generate_copper_ingot(ctx: &HurrayContext) -> Result<(), String> {
    let item = ctx.temp_dir().join("assets/minecraft/textures/item");
    for p in &[item.join("copper_ingot.png"), item.join("copper_ingot.png.mcmeta")] {
        if p.exists() { ctx.defer_remove_file(p); }
    }
    Ok(())
}

pub fn reverse_generate_copper_block(ctx: &HurrayContext) -> Result<(), String> {
    let block = ctx.temp_dir().join("assets/minecraft/textures/block");
    for name in &["copper_block.png", "exposed_copper.png", "weathered_copper.png", "oxidized_copper.png"] {
        let p = block.join(name);
        if p.exists() { ctx.defer_remove_file(&p); }
        let m = block.join(format!("{}.mcmeta", name));
        if m.exists() { ctx.defer_remove_file(&m); }
    }
    Ok(())
}

pub fn reverse_generate_copper_tools(ctx: &HurrayContext) -> Result<(), String> {
    let item = ctx.temp_dir().join("assets/minecraft/textures/item");
    for name in &[
        "copper_sword.png", "copper_helmet.png", "copper_chestplate.png",
        "copper_leggings.png", "copper_boots.png", "copper_axe.png",
        "copper_pickaxe.png", "copper_shovel.png", "copper_hoe.png",
        "copper_horse_armor.png",
    ] {
        let p = item.join(name);
        if p.exists() { ctx.defer_remove_file(&p); }
        let m = item.join(format!("{}.mcmeta", name));
        if m.exists() { ctx.defer_remove_file(&m); }
    }
    Ok(())
}

pub fn reverse_generate_copper_armor_models(ctx: &HurrayContext) -> Result<(), String> {
    let armor = ctx.temp_dir().join("assets/minecraft/textures/models/armor");
    for p in &[armor.join("copper_layer_1.png"), armor.join("copper_layer_2.png")] {
        if p.exists() { ctx.defer_remove_file(p); }
    }
    Ok(())
}
