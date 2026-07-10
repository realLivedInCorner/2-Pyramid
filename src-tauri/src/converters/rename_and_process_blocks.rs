use std::collections::HashMap;
use std::fs;
use std::path::Path;

use image::{Rgba, RgbaImage};

use crate::converters::color_utils::{hsv_to_rgba, rgb_to_hsv};

pub fn rename_items(items_path: &Path, rename_pairs: &HashMap<&str, &str>) -> Result<(), String> {
    for (old_name, new_name) in rename_pairs {
        let old_png = items_path.join(old_name);
        let new_png = items_path.join(new_name);
        if old_png.exists() {
            if new_png.exists() {
                let _ = fs::remove_file(&new_png);
            }
            fs::rename(&old_png, &new_png)
                .map_err(|e| format!("rename {} -> {} failed: {}", old_png.display(), new_png.display(), e))?;

            let old_meta = old_png.with_extension("png.mcmeta");
            let new_meta = new_png.with_extension("png.mcmeta");
            if old_meta.exists() {
                if new_meta.exists() {
                    let _ = fs::remove_file(&new_meta);
                }
                fs::rename(&old_meta, &new_meta)
                    .map_err(|e| format!("rename {} -> {} failed: {}", old_meta.display(), new_meta.display(), e))?;
            }
        }
    }

    for (old_name, new_name) in rename_pairs {
        let old_meta = items_path.join(format!("{}.mcmeta", old_name));
        let new_meta = items_path.join(format!("{}.mcmeta", new_name));
        if old_meta.exists() && !new_meta.exists() {
            fs::rename(&old_meta, &new_meta)
                .map_err(|e| format!("rename {} -> {} failed: {}", old_meta.display(), new_meta.display(), e))?;
        }
    }

    Ok(())
}

fn process_block_image(
    blocks_path: &Path,
    file_name: &str,
    new_name: &str,
    hue_shift: f32,
    brightness_adjust: f32,
    saturation_adjust: f32,
) -> Result<(), String> {
    let original_path = blocks_path.join(file_name);
    if !original_path.exists() {
        return Ok(());
    }

    let new_path = blocks_path.join(new_name);
    fs::copy(&original_path, &new_path)
        .map_err(|e| format!("copy {} -> {} failed: {}", original_path.display(), new_path.display(), e))?;

    let mut img = image::open(&new_path)
        .map_err(|e| format!("open {} failed: {}", new_path.display(), e))?
        .to_rgba8();

    let hue_shift_normalized = hue_shift / 360.0;
    let brightness_factor = brightness_adjust / 100.0;
    let saturation_factor = saturation_adjust / 100.0;

    for pixel in img.pixels_mut() {
        let a = pixel[3];
        let (mut h, mut s, mut v) = rgb_to_hsv(pixel[0], pixel[1], pixel[2]);
        h = (h + hue_shift_normalized).rem_euclid(1.0);
        s = (s + saturation_factor).clamp(0.0, 1.0);
        v = (v + brightness_factor).clamp(0.0, 1.0);
        let rgba = hsv_to_rgba(h, s, v, a);
        *pixel = rgba;
    }

    img.save(&new_path)
        .map_err(|e| format!("save {} failed: {}", new_path.display(), e))?;

    let original_meta = original_path.with_extension("png.mcmeta");
    if original_meta.exists() {
        let new_meta = new_path.with_extension("png.mcmeta");
        let _ = fs::copy(&original_meta, &new_meta);
    }

    Ok(())
}

fn change_white_to_yellow(img: &mut RgbaImage) {
    for pixel in img.pixels_mut() {
        if pixel[3] == 0 {
            continue;
        }
        if (180..=255).contains(&pixel[0])
            && (180..=255).contains(&pixel[1])
            && (180..=255).contains(&pixel[2])
        {
            pixel[0] = 255;
            pixel[1] = 255;
            pixel[2] = 0;
        }
    }
}

fn process_redstone_dust_cross_image(blocks_path: &Path) -> Result<(), String> {
    let cross_path = blocks_path.join("redstone_dust_cross.png");
    if !cross_path.exists() {
        return Ok(());
    }

    let mut img = image::open(&cross_path)
        .map_err(|e| format!("open {} failed: {}", cross_path.display(), e))?
        .to_rgba8();

    if img.dimensions() != (16, 16) {
        return Ok(());
    }

    for x in 0..16 {
        for y in 0..16 {
            let diag1 = x == y && (5..=11).contains(&x);
            let diag2 = x + y == 16 && (5..=11).contains(&x);
            if !(diag1 || diag2) {
                img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }

    let new_path = blocks_path.join("red_dust_dot.png");
    img.save(&new_path)
        .map_err(|e| format!("save {} failed: {}", new_path.display(), e))?;

    Ok(())
}

fn process_redstone_dust_line_image(blocks_path: &Path) -> Result<(), String> {
    let line_path = blocks_path.join("redstone_dust_line.png");
    if !line_path.exists() {
        return Ok(());
    }

    let img = image::open(&line_path)
        .map_err(|e| format!("open {} failed: {}", line_path.display(), e))?
        .to_rgba8();

    let line_0 = image::imageops::rotate90(&img);
    let line_1 = image::imageops::rotate270(&img);

    line_0
        .save(blocks_path.join("redstone_dust_line0.png"))
        .map_err(|e| format!("save redstone_dust_line0.png failed: {}", e))?;
    line_1
        .save(blocks_path.join("redstone_dust_line1.png"))
        .map_err(|e| format!("save redstone_dust_line1.png failed: {}", e))?;

    Ok(())
}

pub fn rename_and_process_blocks(blocks_path: &Path, reverse: bool) -> Result<(), String> {
    let mut rename_pairs: HashMap<&str, &str> = HashMap::new();
    rename_pairs.insert("stone_granite.png", "granite.png");
    rename_pairs.insert("stone_granite_smooth.png", "polished_granite.png");
    rename_pairs.insert("stone_diorite.png", "diorite.png");
    rename_pairs.insert("stone_diorite_smooth.png", "polished_diorite.png");
    rename_pairs.insert("stone_andesite.png", "andesite.png");
    rename_pairs.insert("stone_andesite_smooth.png", "polished_andesite.png");
    rename_pairs.insert("grass_side.png", "grass_block_side.png");
    rename_pairs.insert("grass_top.png", "grass_block_top.png");
    rename_pairs.insert("dirt_podzol_side.png", "podzol_side.png");
    rename_pairs.insert("dirt_podzol_top.png", "podzol_top.png");
    rename_pairs.insert("planks_acacia.png", "acacia_planks.png");
    rename_pairs.insert("planks_big_oak.png", "dark_oak_planks.png");
    rename_pairs.insert("planks_birch.png", "birch_planks.png");
    rename_pairs.insert("planks_jungle.png", "jungle_planks.png");
    rename_pairs.insert("planks_spruce.png", "spruce_planks.png");
    rename_pairs.insert("planks_oak.png", "oak_planks.png");
    rename_pairs.insert("quartz_ore.png", "nether_quartz_ore.png");
    rename_pairs.insert("sponge_wet.png", "wet_sponge.png");
    rename_pairs.insert("sandstone_normal.png", "sandstone.png");
    rename_pairs.insert("sandstone_carved.png", "chiseled_sandstone.png");
    rename_pairs.insert("sandstone_smooth.png", "cut_sandstone.png");
    rename_pairs.insert("red_sandstone_normal.png", "red_sandstone.png");
    rename_pairs.insert("red_sandstone_carved.png", "chiseled_red_sandstone.png");
    rename_pairs.insert("red_sandstone_smooth.png", "cut_red_sandstone.png");
    rename_pairs.insert("wool_colored_black.png", "black_wool.png");
    rename_pairs.insert("wool_colored_blue.png", "blue_wool.png");
    rename_pairs.insert("wool_colored_brown.png", "brown_wool.png");
    rename_pairs.insert("wool_colored_cyan.png", "cyan_wool.png");
    rename_pairs.insert("wool_colored_gray.png", "gray_wool.png");
    rename_pairs.insert("wool_colored_green.png", "green_wool.png");
    rename_pairs.insert("wool_colored_light_blue.png", "light_blue_wool.png");
    rename_pairs.insert("wool_colored_lime.png", "lime_wool.png");
    rename_pairs.insert("wool_colored_magenta.png", "magenta_wool.png");
    rename_pairs.insert("wool_colored_orange.png", "orange_wool.png");
    rename_pairs.insert("wool_colored_pink.png", "pink_wool.png");
    rename_pairs.insert("wool_colored_purple.png", "purple_wool.png");
    rename_pairs.insert("wool_colored_red.png", "red_wool.png");
    rename_pairs.insert("wool_colored_silver.png", "light_gray_wool.png");
    rename_pairs.insert("wool_colored_white.png", "white_wool.png");
    rename_pairs.insert("wool_colored_yellow.png", "yellow_wool.png");
    rename_pairs.insert("stone_slab_side.png", "smooth_stone_slab_side.png");
    rename_pairs.insert("stone_slab_top.png", "smooth_stone.png");
    rename_pairs.insert("brick.png", "bricks.png");
    rename_pairs.insert("nether_brick.png", "nether_bricks.png");
    rename_pairs.insert("stonebrick.png", "stone_bricks.png");
    rename_pairs.insert("stonebrick_carved.png", "chiseled_stone_bricks.png");
    rename_pairs.insert("stonebrick_mossy.png", "mossy_stone_bricks.png");
    rename_pairs.insert("quartz_block_chiseled.png", "chiseled_quartz_block.png");
    rename_pairs.insert("quartz_block_lines.png", "quartz_pillar.png");
    rename_pairs.insert("quartz_block_lines_top.png", "quartz_pillar_top.png");
    rename_pairs.insert("prismarine_dark.png", "dark_prismarine.png");
    rename_pairs.insert("prismarine_rough.png", "prismarine.png");
    rename_pairs.insert("prismarine_rough.png.mcmeta", "prismarine.png.mcmeta");
    rename_pairs.insert("anvil_base.png", "anvil.png");
    rename_pairs.insert("anvil_top_damaged_0.png", "anvil_top.png");
    rename_pairs.insert("anvil_top_damaged_1.png", "chipped_anvil_top.png");
    rename_pairs.insert("anvil_top_damaged_2.png", "damaged_anvil_top.png");
    rename_pairs.insert("carrots_stage_0.png", "carrots_stage0.png");
    rename_pairs.insert("carrots_stage_1.png", "carrots_stage1.png");
    rename_pairs.insert("carrots_stage_2.png", "carrots_stage2.png");
    rename_pairs.insert("carrots_stage_3.png", "carrots_stage3.png");
    rename_pairs.insert("cobblestone_mossy.png", "mossy_cobblestone.png");
    rename_pairs.insert("cocoa_stage_0.png", "cocoa_stage0.png");
    rename_pairs.insert("cocoa_stage_1.png", "cocoa_stage1.png");
    rename_pairs.insert("cocoa_stage_2.png", "cocoa_stage2.png");
    rename_pairs.insert("comparator_off.png", "comparator.png");
    rename_pairs.insert("deadbush.png", "dead_bush.png");
    rename_pairs.insert("dispenser_front_horizontal.png", "dispenser_horizontal.png");
    rename_pairs.insert("door_acacia_lower.png", "acacia_door_bottom.png");
    rename_pairs.insert("door_acacia_upper.png", "acacia_door_top.png");
    rename_pairs.insert("door_birch_lower.png", "birch_door_bottom.png");
    rename_pairs.insert("door_birch_upper.png", "birch_door_top.png");
    rename_pairs.insert("door_dark_oak_lower.png", "dark_oak_door_bottom.png");
    rename_pairs.insert("door_dark_oak_upper.png", "dark_oak_door_top.png");
    rename_pairs.insert("door_iron_lower.png", "iron_door_bottom.png");
    rename_pairs.insert("door_iron_upper.png", "iron_door_top.png");
    rename_pairs.insert("door_jungle_lower.png", "jungle_door_bottom.png");
    rename_pairs.insert("door_jungle_upper.png", "jungle_door_top.png");
    rename_pairs.insert("door_spruce_lower.png", "spruce_door_bottom.png");
    rename_pairs.insert("door_spruce_upper.png", "spruce_door_top.png");
    rename_pairs.insert("door_wood_lower.png", "oak_door_bottom.png");
    rename_pairs.insert("door_wood_upper.png", "oak_door_top.png");
    rename_pairs.insert("double_plant_fern_bottom.png", "large_fern_bottom.png");
    rename_pairs.insert("double_plant_fern_top.png", "large_fern_top.png");
    rename_pairs.insert("double_plant_grass_bottom.png", "tall_grass_bottom.png");
    rename_pairs.insert("double_plant_grass_top.png", "tall_grass_top.png");
    rename_pairs.insert("double_plant_paeonia_bottom.png", "peony_bottom.png");
    rename_pairs.insert("double_plant_paeonia_top.png", "peony_top.png");
    rename_pairs.insert("double_plant_rose_bottom.png", "rose_bush_bottom.png");
    rename_pairs.insert("double_plant_rose_top.png", "rose_bush_top.png");
    rename_pairs.insert("double_plant_sunflower_back.png", "sunflower_back.png");
    rename_pairs.insert("double_plant_sunflower_bottom.png", "sunflower_bottom.png");
    rename_pairs.insert("double_plant_sunflower_top.png", "sunflower_top.png");
    rename_pairs.insert("double_plant_sunflower_front.png", "sunflower_front.png");
    rename_pairs.insert("double_plant_syringa_bottom.png", "lilac_bottom.png");
    rename_pairs.insert("double_plant_syringa_top.png", "lilac_top.png");
    rename_pairs.insert("dropper_front_horizontal.png", "dropper_front.png");
    rename_pairs.insert("endframe_eye.png", "end_portal_frame_eye.png");
    rename_pairs.insert("endframe_side.png", "end_portal_frame_side.png");
    rename_pairs.insert("endframe_top.png", "end_portal_frame_top.png");
    rename_pairs.insert("farmland_dry.png", "farmland.png");
    rename_pairs.insert("farmland_wet.png", "farmland_moist.png");
    rename_pairs.insert("fire_layer_0.png", "fire_0.png");
    rename_pairs.insert("fire_layer_1.png", "fire_1.png");
    rename_pairs.insert("flower_allium.png", "allium.png");
    rename_pairs.insert("flower_blue_orchid.png", "blue_orchid.png");
    rename_pairs.insert("flower_dandelion.png", "dandelion.png");
    rename_pairs.insert("flower_houstonia.png", "azure_bluet.png");
    rename_pairs.insert("flower_oxeye_daisy.png", "oxeye_daisy.png");
    rename_pairs.insert("flower_rose.png", "poppy.png");
    rename_pairs.insert("flower_tulip_orange.png", "orange_tulip.png");
    rename_pairs.insert("flower_tulip_pink.png", "pink_tulip.png");
    rename_pairs.insert("flower_tulip_red.png", "red_tulip.png");
    rename_pairs.insert("flower_tulip_white.png", "white_tulip.png");
    rename_pairs.insert("furnace_front_off.png", "furnace_front.png");
    rename_pairs.insert("glass_black.png", "black_stained_glass.png");
    rename_pairs.insert("glass_blue.png", "blue_stained_glass.png");
    rename_pairs.insert("glass_brown.png", "brown_stained_glass.png");
    rename_pairs.insert("glass_cyan.png", "cyan_stained_glass.png");
    rename_pairs.insert("glass_gray.png", "gray_stained_glass.png");
    rename_pairs.insert("glass_green.png", "green_stained_glass.png");
    rename_pairs.insert("glass_light_blue.png", "light_blue_stained_glass.png");
    rename_pairs.insert("glass_lime.png", "lime_stained_glass.png");
    rename_pairs.insert("glass_magenta.png", "magenta_stained_glass.png");
    rename_pairs.insert("glass_orange.png", "orange_stained_glass.png");
    rename_pairs.insert("glass_pink.png", "pink_stained_glass.png");
    rename_pairs.insert("glass_purple.png", "purple_stained_glass.png");
    rename_pairs.insert("glass_red.png", "red_stained_glass.png");
    rename_pairs.insert("glass_silver.png", "light_gray_stained_glass.png");
    rename_pairs.insert("glass_white.png", "white_stained_glass.png");
    rename_pairs.insert("glass_yellow.png", "yellow_stained_glass.png");
    rename_pairs.insert("glass_pane_top_black.png", "black_stained_glass_pane_top.png");
    rename_pairs.insert("glass_pane_top_blue.png", "blue_stained_glass_pane_top.png");
    rename_pairs.insert("glass_pane_top_brown.png", "brown_stained_glass_pane_top.png");
    rename_pairs.insert("glass_pane_top_cyan.png", "cyan_stained_glass_pane_top.png");
    rename_pairs.insert("glass_pane_top_gray.png", "gray_stained_glass_pane_top.png");
    rename_pairs.insert("glass_pane_top_green.png", "green_stained_glass_pane_top.png");
    rename_pairs.insert("glass_pane_top_light_blue.png", "light_blue_stained_glass_pane_top.png");
    rename_pairs.insert("glass_pane_top_lime.png", "lime_stained_glass_pane_top.png");
    rename_pairs.insert("glass_pane_top_magenta.png", "magenta_stained_glass_pane_top.png");
    rename_pairs.insert("glass_pane_top_orange.png", "orange_stained_glass_pane_top.png");
    rename_pairs.insert("glass_pane_top_pink.png", "pink_stained_glass_pane_top.png");
    rename_pairs.insert("glass_pane_top_purple.png", "purple_stained_glass_pane_top.png");
    rename_pairs.insert("glass_pane_top_red.png", "red_stained_glass_pane_top.png");
    rename_pairs.insert("glass_pane_top_silver.png", "light_gray_stained_glass_pane_top.png");
    rename_pairs.insert("glass_pane_top_white.png", "white_stained_glass_pane_top.png");
    rename_pairs.insert("glass_pane_top_yellow.png", "yellow_stained_glass_pane_top.png");
    rename_pairs.insert("grass_side_overlay.png", "grass_block_side_overlay.png");
    rename_pairs.insert("grass_side_snowed.png", "grass_block_snow.png");
    rename_pairs.insert("hardened_clay.png", "terracotta.png");
    rename_pairs.insert("hardened_clay_stained_black.png", "black_terracotta.png");
    rename_pairs.insert("hardened_clay_stained_blue.png", "blue_terracotta.png");
    rename_pairs.insert("hardened_clay_stained_brown.png", "brown_terracotta.png");
    rename_pairs.insert("hardened_clay_stained_cyan.png", "cyan_terracotta.png");
    rename_pairs.insert("hardened_clay_stained_gray.png", "gray_terracotta.png");
    rename_pairs.insert("hardened_clay_stained_green.png", "green_terracotta.png");
    rename_pairs.insert("hardened_clay_stained_light_blue.png", "light_blue_terracotta.png");
    rename_pairs.insert("hardened_clay_stained_lime.png", "lime_terracotta.png");
    rename_pairs.insert("hardened_clay_stained_magenta.png", "magenta_terracotta.png");
    rename_pairs.insert("hardened_clay_stained_orange.png", "orange_terracotta.png");
    rename_pairs.insert("hardened_clay_stained_pink.png", "pink_terracotta.png");
    rename_pairs.insert("hardened_clay_stained_purple.png", "purple_terracotta.png");
    rename_pairs.insert("hardened_clay_stained_red.png", "red_terracotta.png");
    rename_pairs.insert("hardened_clay_stained_silver.png", "light_gray_terracotta.png");
    rename_pairs.insert("hardened_clay_stained_white.png", "white_terracotta.png");
    rename_pairs.insert("hardened_clay_stained_yellow.png", "yellow_terracotta.png");
    rename_pairs.insert("ice_packed.png", "packed_ice.png");
    rename_pairs.insert("itemframe_background.png", "item_frame.png");
    rename_pairs.insert("leaves_acacia.png", "acacia_leaves.png");
    rename_pairs.insert("leaves_big_oak.png", "dark_oak_leaves.png");
    rename_pairs.insert("leaves_birch.png", "birch_leaves.png");
    rename_pairs.insert("leaves_jungle.png", "jungle_leaves.png");
    rename_pairs.insert("leaves_oak.png", "oak_leaves.png");
    rename_pairs.insert("leaves_spruce.png", "spruce_leaves.png");
    rename_pairs.insert("log_acacia.png", "acacia_log.png");
    rename_pairs.insert("log_acacia_top.png", "acacia_log_top.png");
    rename_pairs.insert("log_big_oak.png", "dark_oak_log.png");
    rename_pairs.insert("log_big_oak_top.png", "dark_oak_log_top.png");
    rename_pairs.insert("log_spruce.png", "spruce_log.png");
    rename_pairs.insert("log_spruce_top.png", "spruce_log_top.png");
    rename_pairs.insert("log_birch.png", "birch_log.png");
    rename_pairs.insert("log_birch_top.png", "birch_log_top.png");
    rename_pairs.insert("log_jungle.png", "jungle_log.png");
    rename_pairs.insert("log_jungle_top.png", "jungle_log_top.png");
    rename_pairs.insert("log_oak.png", "oak_log.png");
    rename_pairs.insert("log_oak_top.png", "oak_log_top.png");
    rename_pairs.insert("melon_stem_connected.png", "attached_melon_stem.png");
    rename_pairs.insert("melon_stem_disconnected.png", "melon_stem.png");
    rename_pairs.insert("mob_spawner.png", "spawner.png");
    rename_pairs.insert("mushroom_block_skin_brown.png", "brown_mushroom_block.png");
    rename_pairs.insert("mushroom_block_skin_red.png", "red_mushroom_block.png");
    rename_pairs.insert("mushroom_block_skin_stem.png", "mushroom_stem.png");
    rename_pairs.insert("mushroom_brown.png", "brown_mushroom.png");
    rename_pairs.insert("mushroom_red.png", "red_mushroom.png");
    rename_pairs.insert("nether_wart_stage_0.png", "nether_wart_stage0.png");
    rename_pairs.insert("nether_wart_stage_1.png", "nether_wart_stage1.png");
    rename_pairs.insert("nether_wart_stage_2.png", "nether_wart_stage2.png");
    rename_pairs.insert("noteblock.png", "note_block.png");
    rename_pairs.insert("piston_top_normal.png", "piston_top.png");
    rename_pairs.insert("portal.png", "nether_portal.png");
    rename_pairs.insert("potatoes_stage_0.png", "potatoes_stage0.png");
    rename_pairs.insert("potatoes_stage_1.png", "potatoes_stage1.png");
    rename_pairs.insert("potatoes_stage_2.png", "potatoes_stage2.png");
    rename_pairs.insert("potatoes_stage_3.png", "potatoes_stage3.png");
    rename_pairs.insert("pumpkin_face_off.png", "carved_pumpkin.png");
    rename_pairs.insert("pumpkin_face_on.png", "jack_o_lantern.png");
    rename_pairs.insert("pumpkin_stem_connected.png", "attached_pumpkin_stem.png");
    rename_pairs.insert("pumpkin_stem_disconnected.png", "pumpkin_stem.png");
    rename_pairs.insert("rail_activator.png", "activator_rail.png");
    rename_pairs.insert("rail_activator_powered.png", "activator_rail_on.png");
    rename_pairs.insert("rail_detector.png", "detector_rail.png");
    rename_pairs.insert("rail_detector_powered.png", "detector_rail_on.png");
    rename_pairs.insert("rail_golden.png", "powered_rail.png");
    rename_pairs.insert("rail_golden_powered.png", "powered_rail_on.png");
    rename_pairs.insert("rail_normal.png", "rail.png");
    rename_pairs.insert("rail_normal_turned.png", "rail_corner.png");
    rename_pairs.insert("redstone_dust_cross_overlay.png", "redstone_dust_overlay.png");
    rename_pairs.insert("redstone_lamp_off.png", "redstone_lamp.png");
    rename_pairs.insert("redstone_torch_on.png", "redstone_torch.png");
    rename_pairs.insert("reeds.png", "sugar_cane.png");
    rename_pairs.insert("repeater_off.png", "repeater.png");
    rename_pairs.insert("sapling_acacia.png", "acacia_sapling.png");
    rename_pairs.insert("sapling_birch.png", "birch_sapling.png");
    rename_pairs.insert("sapling_jungle.png", "jungle_sapling.png");
    rename_pairs.insert("sapling_oak.png", "oak_sapling.png");
    rename_pairs.insert("sapling_roofed_oak.png", "dark_oak_sapling.png");
    rename_pairs.insert("sapling_spruce.png", "spruce_sapling.png");
    rename_pairs.insert("stonebrick_cracked.png", "cracked_stone_bricks.png");
    rename_pairs.insert("slime.png", "slime_block.png");
    rename_pairs.insert("tallgrass.png", "grass.png");
    rename_pairs.insert("torch_on.png", "torch.png");
    rename_pairs.insert("trapdoor.png", "oak_trapdoor.png");
    rename_pairs.insert("trip_wire_source.png", "tripwire_hook.png");
    rename_pairs.insert("waterlily.png", "lily_pad.png");
    rename_pairs.insert("web.png", "cobweb.png");
    rename_pairs.insert("wheat_stage_0.png", "wheat_stage0.png");
    rename_pairs.insert("wheat_stage_1.png", "wheat_stage1.png");
    rename_pairs.insert("wheat_stage_2.png", "wheat_stage2.png");
    rename_pairs.insert("wheat_stage_3.png", "wheat_stage3.png");
    rename_pairs.insert("wheat_stage_4.png", "wheat_stage4.png");
    rename_pairs.insert("wheat_stage_5.png", "wheat_stage5.png");
    rename_pairs.insert("wheat_stage_6.png", "wheat_stage6.png");
    rename_pairs.insert("wheat_stage_7.png", "wheat_stage7.png");

    if reverse {
        let mut reversed = HashMap::new();
        for (k, v) in &rename_pairs {
            reversed.insert(*v, *k);
        }
        rename_pairs = reversed;
    }

    rename_items(blocks_path, &rename_pairs)?;

    process_redstone_dust_cross_image(blocks_path)?;
    process_redstone_dust_line_image(blocks_path)?;

    process_block_image(blocks_path, "oak_planks.png", "warped_planks.png", 130.0, -33.0, 0.0)?;
    process_block_image(blocks_path, "oak_planks.png", "crimson_planks.png", -59.0, -30.0, 0.0)?;

    for ore in [
        "coal_ore",
        "iron_ore",
        "gold_ore",
        "diamond_ore",
        "emerald_ore",
        "redstone_ore",
        "lapis_ore",
    ] {
        process_block_image(blocks_path, &format!("{}.png", ore), &format!("deepslate_{}.png", ore), 0.0, -20.0, 0.0)?;
        if ore == "redstone_ore" {
            process_block_image(blocks_path, "redstone_ore.png", "copper_ore.png", 26.0, 0.0, 0.0)?;
            process_block_image(blocks_path, "copper_ore.png", "deepslate_copper_ore.png", 0.0, -20.0, 0.0)?;
        }
    }

    let quartz_path = blocks_path.join("nether_quartz_ore.png");
    let gold_path = blocks_path.join("nether_gold_ore.png");
    if quartz_path.exists() {
        fs::copy(&quartz_path, &gold_path)
            .map_err(|e| format!("copy {} -> {} failed: {}", quartz_path.display(), gold_path.display(), e))?;
        let mut gold_img = image::open(&gold_path)
            .map_err(|e| format!("open {} failed: {}", gold_path.display(), e))?
            .to_rgba8();
        change_white_to_yellow(&mut gold_img);
        gold_img
            .save(&gold_path)
            .map_err(|e| format!("save {} failed: {}", gold_path.display(), e))?;
    }

    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "rename_and_process_blocks",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| rename_and_process_blocks(context.temp_dir(), false),
    );
}
