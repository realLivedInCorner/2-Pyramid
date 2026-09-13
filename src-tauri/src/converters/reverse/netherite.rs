use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_generate_netherite_block(ctx: &HurrayContext) -> Result<(), String> {
    let block = ctx.temp_dir().join("assets/minecraft/textures/block");
    for p in &[block.join("netherite_block.png"), block.join("netherite_block.png.mcmeta")] {
        if p.exists() { ctx.defer_remove_file(p); }
    }
    Ok(())
}

pub fn reverse_generate_netherite_ingot(ctx: &HurrayContext) -> Result<(), String> {
    let item = ctx.temp_dir().join("assets/minecraft/textures/item");
    for p in &[item.join("netherite_ingot.png"), item.join("netherite_ingot.png.mcmeta")] {
        if p.exists() { ctx.defer_remove_file(p); }
    }
    Ok(())
}

pub fn reverse_generate_netherite_tools(ctx: &HurrayContext) -> Result<(), String> {
    let item = ctx.temp_dir().join("assets/minecraft/textures/item");
    for name in &[
        "netherite_sword.png", "netherite_helmet.png", "netherite_chestplate.png",
        "netherite_leggings.png", "netherite_boots.png", "netherite_axe.png",
        "netherite_pickaxe.png", "netherite_shovel.png", "netherite_hoe.png",
        "spectral_arrow.png",
    ] {
        let p = item.join(name);
        if p.exists() { ctx.defer_remove_file(&p); }
        let m = item.join(format!("{}.mcmeta", name));
        if m.exists() { ctx.defer_remove_file(&m); }
    }
    Ok(())
}

pub fn reverse_generate_netherite_armor_models(ctx: &HurrayContext) -> Result<(), String> {
    let armor = ctx.temp_dir().join("assets/minecraft/textures/models/armor");
    for p in &[armor.join("netherite_layer_1.png"), armor.join("netherite_layer_2.png")] {
        if p.exists() { ctx.defer_remove_file(p); }
    }
    Ok(())
}
