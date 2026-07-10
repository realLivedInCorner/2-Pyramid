use std::fs;
use std::io;
use std::path::Path;

use crate::{log_info, log_warn};

fn rename_with_mcmeta(old_png: &Path, new_png: &Path) -> io::Result<()> {
    if !old_png.exists() || old_png == new_png {
        return Ok(());
    }

    if new_png.exists() {
        fs::remove_file(new_png)?;
    }
    fs::rename(old_png, new_png)?;
    log_info!("renamed {} -> {}", old_png.display(), new_png.display());

    let old_meta = old_png.with_extension("png.mcmeta");
    let new_meta = new_png.with_extension("png.mcmeta");
    if old_meta.exists() {
        if new_meta.exists() {
            fs::remove_file(&new_meta)?;
        }
        fs::rename(&old_meta, &new_meta)?;
        log_info!("renamed {} -> {}", old_meta.display(), new_meta.display());
    }

    Ok(())
}

fn merge_or_rename_dir(source: &Path, target: &Path) -> io::Result<()> {
    if !source.exists() {
        return Ok(());
    }

    if !target.exists() {
        fs::rename(source, target)?;
        return Ok(());
    }

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());

        if source_path.is_dir() {
            fs::create_dir_all(&target_path)?;
            merge_or_rename_dir(&source_path, &target_path)?;
            if source_path.exists() {
                fs::remove_dir_all(&source_path)?;
            }
        } else {
            if target_path.exists() {
                fs::remove_file(&target_path)?;
            }
            fs::rename(&source_path, &target_path)?;
        }
    }

    if source.exists() {
        fs::remove_dir_all(source)?;
    }

    Ok(())
}

pub fn rename_blocks_items(resource_pack_path: &Path) -> io::Result<()> {
    let textures_path = resource_pack_path.join("assets/minecraft/textures");

    let items_path = textures_path.join("items");
    let item_path = textures_path.join("item");
    if items_path.exists() {
        merge_or_rename_dir(&items_path, &item_path)?;
        log_info!("normalized textures/items -> textures/item");
    } else {
        log_warn!("textures/items not found under {}", resource_pack_path.display());
    }

    let blocks_path = textures_path.join("blocks");
    let block_path = textures_path.join("block");
    if blocks_path.exists() {
        merge_or_rename_dir(&blocks_path, &block_path)?;
        log_info!("normalized textures/blocks -> textures/block");
    } else {
        log_warn!("textures/blocks not found under {}", resource_pack_path.display());
    }

    let item_rename_pairs = [
        ("gold_sword.png", "golden_sword.png"),
        ("wood_sword.png", "wooden_sword.png"),
        ("gold_helmet.png", "golden_helmet.png"),
        ("gold_chestplate.png", "golden_chestplate.png"),
        ("gold_leggings.png", "golden_leggings.png"),
        ("gold_boots.png", "golden_boots.png"),
        ("apple_golden.png", "golden_apple.png"),
        ("bow_standby.png", "bow.png"),
        ("book_enchanted.png", "enchanted_book.png"),
        ("wood_axe.png", "wooden_axe.png"),
        ("wood_pickaxe.png", "wooden_pickaxe.png"),
        ("wood_shovel.png", "wooden_shovel.png"),
        ("wood_hoe.png", "wooden_hoe.png"),
        ("gold_axe.png", "golden_axe.png"),
        ("gold_pickaxe.png", "golden_pickaxe.png"),
        ("gold_shovel.png", "golden_shovel.png"),
        ("gold_hoe.png", "golden_hoe.png"),
        ("fishing_rod_uncast.png", "fishing_rod.png"),
        ("potion_bottle_empty.png", "glass_bottle.png"),
        ("potion_bottle_drinkable.png", "potion.png"),
        ("potion_bottle_splash.png", "splash_potion.png"),
        ("potion_bottle_lingering.png", "lingering_potion.png"),
        ("spider_eye_fermented.png", "fermented_spider_eye.png"),
        ("melon_speckled.png", "glistering_melon_slice.png"),
        ("melon.png", "melon_slice.png"),
        ("carrot_golden.png", "golden_carrot.png"),
        ("porkchop_raw.png", "porkchop.png"),
        ("porkchop_cooked.png", "cooked_porkchop.png"),
        ("chicken_raw.png", "chicken.png"),
        ("chicken_cooked.png", "cooked_chicken.png"),
        ("rabbit_raw.png", "rabbit.png"),
        ("rabbit_cooked.png", "cooked_rabbit.png"),
        ("beef_raw.png", "beef.png"),
        ("beef_cooked.png", "cooked_beef.png"),
        ("boat.png", "oak_boat.png"),
        ("book_normal.png", "book.png"),
        ("book_writable.png", "writable_book.png"),
        ("book_written.png", "written_book.png"),
        ("bucket_empty.png", "bucket.png"),
        ("bucket_lava.png", "lava_bucket.png"),
        ("bucket_water.png", "water_bucket.png"),
        ("bucket_milk.png", "milk_bucket.png"),
        ("door_acacia.png", "acacia_door.png"),
        ("door_birch.png", "birch_door.png"),
        ("door_dark_oak.png", "dark_oak_door.png"),
        ("door_iron.png", "iron_door.png"),
        ("door_jungle.png", "jungle_door.png"),
        ("door_spruce.png", "spruce_door.png"),
        ("door_wood.png", "oak_door.png"),
        ("dye_powder_black.png", "ink_sac.png"),
        ("dye_powder_blue.png", "lapis_lazuli.png"),
        ("dye_powder_brown.png", "cocoa_beans.png"),
        ("dye_powder_cyan.png", "cyan_dye.png"),
        ("dye_powder_gray.png", "gray_dye.png"),
        ("dye_powder_green.png", "green_dye.png"),
        ("dye_powder_light_blue.png", "light_blue_dye.png"),
        ("dye_powder_lime.png", "lime_dye.png"),
        ("dye_powder_magenta.png", "magenta_dye.png"),
        ("dye_powder_orange.png", "orange_dye.png"),
        ("dye_powder_pink.png", "pink_dye.png"),
        ("dye_powder_purple.png", "purple_dye.png"),
        ("dye_powder_red.png", "red_dye.png"),
        ("dye_powder_silver.png", "light_gray_dye.png"),
        ("dye_powder_white.png", "bone_meal.png"),
        ("dye_powder_yellow.png", "yellow_dye.png"),
        ("fireball.png", "fire_charge.png"),
        ("fireworks.png", "firework_rocket.png"),
        ("fireworks_charge.png", "firework_star.png"),
        ("firework_charge_overlay.png", "firework_star_overlay.png"),
        ("fish_cod_raw.png", "cod.png"),
        ("fish_cod_cooked.png", "cooked_cod.png"),
        ("fish_salmon_raw.png", "salmon.png"),
        ("fish_salmon_cooked.png", "cooked_salmon.png"),
        ("fish_clownfish_raw.png", "tropical_fish.png"),
        ("fish_pufferfish_raw.png", "pufferfish.png"),
        ("map_empty.png", "map.png"),
        ("map_filled.png", "filled_map.png"),
        ("minecart_chest.png", "chest_minecart.png"),
        ("minecart_command_block.png", "command_block_minecart.png"),
        ("minecart_furnace.png", "furnace_minecart.png"),
        ("minecart_hopper.png", "hopper_minecart.png"),
        ("minecart_normal.png", "minecart.png"),
        ("minecart_tnt.png", "tnt_minecart.png"),
        ("mutton_cooked.png", "cooked_mutton.png"),
        ("mutton_raw.png", "mutton.png"),
        ("netherbrick.png", "nether_brick.png"),
        ("potato_baked.png", "baked_potato.png"),
        ("potato_poisonous.png", "poisonous_potato.png"),
        ("record_11.png", "music_disc_11.png"),
        ("record_13.png", "music_disc_13.png"),
        ("record_blocks.png", "music_disc_blocks.png"),
        ("record_cat.png", "music_disc_cat.png"),
        ("record_chirp.png", "music_disc_chirp.png"),
        ("record_far.png", "music_disc_far.png"),
        ("record_mail.png", "music_disc_mail.png"),
        ("record_mellohi.png", "music_disc_mellohi.png"),
        ("record_stal.png", "music_disc_stal.png"),
        ("record_strad.png", "music_disc_strad.png"),
        ("record_wait.png", "music_disc_wait.png"),
        ("record_ward.png", "music_disc_ward.png"),
        ("record_mall.png", "music_disc_mall.png"),
        ("redstone_dust.png", "redstone.png"),
        ("reeds.png", "sugar_cane.png"),
        ("seeds_melon.png", "melon_seeds.png"),
        ("seeds_pumpkin.png", "pumpkin_seeds.png"),
        ("seeds_wheat.png", "wheat_seeds.png"),
        ("sign.png", "oak_sign.png"),
        ("slimeball.png", "slime_ball.png"),
        ("wooden_armorstand.png", "armor_stand.png"),
        ("gold_horse_armor.png", "golden_horse_armor.png"),
    ];

    let block_rename_pairs = [
        ("cobblestone_mossy.png", "mossy_cobblestone.png"),
        ("stone_granite_smooth.png", "polished_granite.png"),
        ("stone_diorite_smooth.png", "polished_diorite.png"),
        ("stone_andesite_smooth.png", "polished_andesite.png"),
        ("hardened_clay_stained_white.png", "white_terracotta.png"),
        ("hardened_clay_stained_orange.png", "orange_terracotta.png"),
        ("hardened_clay_stained_magenta.png", "magenta_terracotta.png"),
        ("hardened_clay_stained_light_blue.png", "light_blue_terracotta.png"),
        ("hardened_clay_stained_yellow.png", "yellow_terracotta.png"),
        ("hardened_clay_stained_lime.png", "lime_terracotta.png"),
        ("hardened_clay_stained_pink.png", "pink_terracotta.png"),
        ("hardened_clay_stained_gray.png", "gray_terracotta.png"),
        ("hardened_clay_stained_light_gray.png", "light_gray_terracotta.png"),
        ("hardened_clay_stained_cyan.png", "cyan_terracotta.png"),
        ("hardened_clay_stained_purple.png", "purple_terracotta.png"),
        ("hardened_clay_stained_blue.png", "blue_terracotta.png"),
        ("hardened_clay_stained_brown.png", "brown_terracotta.png"),
        ("hardened_clay_stained_green.png", "green_terracotta.png"),
        ("hardened_clay_stained_red.png", "red_terracotta.png"),
        ("hardened_clay_stained_black.png", "black_terracotta.png"),
        ("planks_oak.png", "oak_planks.png"),
        ("planks_birch.png", "birch_planks.png"),
        ("planks_spruce.png", "spruce_planks.png"),
        ("planks_jungle.png", "jungle_planks.png"),
        ("planks_acacia.png", "acacia_planks.png"),
        ("planks_dark_oak.png", "dark_oak_planks.png"),
        ("sapling_oak.png", "oak_sapling.png"),
        ("sapling_birch.png", "birch_sapling.png"),
        ("sapling_spruce.png", "spruce_sapling.png"),
        ("sapling_jungle.png", "jungle_sapling.png"),
        ("sapling_acacia.png", "acacia_sapling.png"),
        ("sapling_dark_oak.png", "dark_oak_sapling.png"),
        ("leaves_oak.png", "oak_leaves.png"),
        ("leaves_birch.png", "birch_leaves.png"),
        ("leaves_spruce.png", "spruce_leaves.png"),
        ("leaves_jungle.png", "jungle_leaves.png"),
        ("leaves_acacia.png", "acacia_leaves.png"),
        ("leaves_dark_oak.png", "dark_oak_leaves.png"),
        ("log_oak.png", "oak_log.png"),
        ("log_birch.png", "birch_log.png"),
        ("log_spruce.png", "spruce_log.png"),
        ("log_jungle.png", "jungle_log.png"),
        ("log_acacia.png", "acacia_log.png"),
        ("log_dark_oak.png", "dark_oak_log.png"),
    ];

    for (old_name, new_name) in &item_rename_pairs {
        let old_path = item_path.join(old_name);
        let new_path = item_path.join(new_name);
        rename_with_mcmeta(&old_path, &new_path)?;
    }

    for (old_name, new_name) in &block_rename_pairs {
        let old_path = block_path.join(old_name);
        let new_path = block_path.join(new_name);
        rename_with_mcmeta(&old_path, &new_path)?;
    }

    let _ = crate::converters::rename_and_process_blocks::rename_and_process_blocks(&block_path, false);

    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "rename_blocks_items",
        crate::hurray::scheduler::TaskType::Exclusive,
        crate::hurray::scheduler::TaskTier::Eraser,
        |context| {
            rename_blocks_items(context.temp_dir()).map_err(|e| e.to_string())
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_rename_blocks_items() {
        let temp_dir = tempdir().expect("Failed to create temp directory");

        let textures_path = temp_dir.path().join("assets/minecraft/textures");
        let items_path = textures_path.join("items");
        let blocks_path = textures_path.join("blocks");

        fs::create_dir_all(&items_path).expect("Failed to create items directory");
        fs::create_dir_all(&blocks_path).expect("Failed to create blocks directory");

        let result = rename_blocks_items(temp_dir.path());

        assert!(result.is_ok());
        assert!(!items_path.exists());
        assert!(!blocks_path.exists());
        assert!(textures_path.join("item").exists());
        assert!(textures_path.join("block").exists());
    }
}
