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

pub fn reverse_generate_poplar_planks(ctx: &HurrayContext) -> Result<(), String> {
    let root = ctx.temp_dir();
    let names = [
        // 原木 / 木板
        "assets/minecraft/textures/block/poplar_planks.png",
        "assets/minecraft/textures/block/poplar_log.png",
        "assets/minecraft/textures/block/poplar_log_top.png",
        "assets/minecraft/textures/block/stripped_poplar_log.png",
        "assets/minecraft/textures/block/stripped_poplar_log_top.png",
        // 家具 / 树叶
        "assets/minecraft/textures/block/poplar_door_top.png",
        "assets/minecraft/textures/block/poplar_door_bottom.png",
        "assets/minecraft/textures/block/poplar_trapdoor.png",
        "assets/minecraft/textures/block/poplar_shelf.png",
        "assets/minecraft/textures/block/poplar_sapling.png",
        "assets/minecraft/textures/block/poplar_sign.png",
        "assets/minecraft/textures/block/poplar_hanging_sign.png",
        "assets/minecraft/textures/block/red_poplar_leaves.png",
        "assets/minecraft/textures/block/orange_poplar_leaves.png",
        "assets/minecraft/textures/block/yellow_poplar_leaves.png",
        "assets/minecraft/textures/item/poplar_sign.png",
        "assets/minecraft/textures/item/poplar_hanging_sign.png",
        "assets/minecraft/textures/item/poplar_door.png",
        "assets/minecraft/textures/item/poplar_boat.png",
        "assets/minecraft/textures/item/poplar_chest_boat.png",
        "assets/minecraft/textures/entity/boat/poplar.png",
        "assets/minecraft/textures/entity/chest_boat/poplar.png",
    ];
    for name in names {
        let p = root.join(name);
        if p.exists() { ctx.defer_remove_file(&p); }
        let m = p.with_extension("png.mcmeta");
        if m.exists() { ctx.defer_remove_file(&m); }
    }
    Ok(())
}
