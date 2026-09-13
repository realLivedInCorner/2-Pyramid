use std::fs;
use std::io;
use std::path::Path;

use crate::log_info;

fn rename_with_mcmeta(old_png: &Path, new_png: &Path) -> io::Result<()> {
    if !old_png.exists() || old_png == new_png {
        return Ok(());
    }

    if new_png.exists() {
        fs::remove_file(new_png)?;
    }
    fs::rename(old_png, new_png)?;

    let old_meta = old_png.with_extension("png.mcmeta");
    let new_meta = new_png.with_extension("png.mcmeta");
    if old_meta.exists() {
        if new_meta.exists() {
            fs::remove_file(&new_meta)?;
        }
        fs::rename(&old_meta, &new_meta)?;
    }

    Ok(())
}

pub fn reverse_rename_blocks_items(path: &Path) -> Result<(), String> {
    let textures_path = path.join("assets/minecraft/textures");
    let block_path = textures_path.join("block");
    let blocks_path = textures_path.join("blocks");
    let item_path = textures_path.join("item");
    let items_path = textures_path.join("items");

    // Rename directories: item→items, block→blocks
    if item_path.exists() && !items_path.exists() {
        fs::rename(&item_path, &items_path)
            .map_err(|e| format!("failed to rename item->items: {}", e))?;
        log_info!("renamed item -> items");
    }

    if block_path.exists() && !blocks_path.exists() {
        fs::rename(&block_path, &blocks_path)
            .map_err(|e| format!("failed to rename block->blocks: {}", e))?;
        log_info!("renamed block -> blocks");
    }

    // Reverse rename pairs (new→old): what the Python calls "reversed rename_pairs"
    let reverse_pairs: [(&str, &str); 128] = [
        ("golden_sword.png", "gold_sword.png"),
        ("wooden_sword.png", "wood_sword.png"),
        ("golden_helmet.png", "gold_helmet.png"),
        ("golden_chestplate.png", "gold_chestplate.png"),
        ("golden_leggings.png", "gold_leggings.png"),
        ("golden_boots.png", "gold_boots.png"),
        ("golden_apple.png", "apple_golden.png"),
        ("bow.png", "bow_standby.png"),
        ("enchanted_book.png", "book_enchanted.png"),
        ("wooden_axe.png", "wood_axe.png"),
        ("wooden_pickaxe.png", "wood_pickaxe.png"),
        ("wooden_shovel.png", "wood_shovel.png"),
        ("wooden_hoe.png", "wood_hoe.png"),
        ("golden_axe.png", "gold_axe.png"),
        ("golden_pickaxe.png", "gold_pickaxe.png"),
        ("golden_shovel.png", "gold_shovel.png"),
        ("golden_hoe.png", "gold_hoe.png"),
        ("fishing_rod.png", "fishing_rod_uncast.png"),
        ("glass_bottle.png", "potion_bottle_empty.png"),
        ("potion.png", "potion_bottle_drinkable.png"),
        ("splash_potion.png", "potion_bottle_splash.png"),
        ("lingering_potion.png", "potion_bottle_lingering.png"),
        ("fermented_spider_eye.png", "spider_eye_fermented.png"),
        ("glistering_melon_slice.png", "melon_speckled.png"),
        ("melon_slice.png", "melon.png"),
        ("golden_carrot.png", "carrot_golden.png"),
        ("porkchop.png", "porkchop_raw.png"),
        ("cooked_porkchop.png", "porkchop_cooked.png"),
        ("chicken.png", "chicken_raw.png"),
        ("cooked_chicken.png", "chicken_cooked.png"),
        ("rabbit.png", "rabbit_raw.png"),
        ("cooked_rabbit.png", "rabbit_cooked.png"),
        ("beef.png", "beef_raw.png"),
        ("cooked_beef.png", "beef_cooked.png"),
        ("oak_boat.png", "boat.png"),
        ("book.png", "book_normal.png"),
        ("writable_book.png", "book_writable.png"),
        ("written_book.png", "book_written.png"),
        ("bucket.png", "bucket_empty.png"),
        ("lava_bucket.png", "bucket_lava.png"),
        ("water_bucket.png", "bucket_water.png"),
        ("milk_bucket.png", "bucket_milk.png"),
        ("acacia_door.png", "door_acacia.png"),
        ("birch_door.png", "door_birch.png"),
        ("dark_oak_door.png", "door_dark_oak.png"),
        ("iron_door.png", "door_iron.png"),
        ("jungle_door.png", "door_jungle.png"),
        ("spruce_door.png", "door_spruce.png"),
        ("oak_door.png", "door_wood.png"),
        ("ink_sac.png", "dye_powder_black.png"),
        ("lapis_lazuli.png", "dye_powder_blue.png"),
        ("cocoa_beans.png", "dye_powder_brown.png"),
        ("cyan_dye.png", "dye_powder_cyan.png"),
        ("gray_dye.png", "dye_powder_gray.png"),
        ("green_dye.png", "dye_powder_green.png"),
        ("light_blue_dye.png", "dye_powder_light_blue.png"),
        ("lime_dye.png", "dye_powder_lime.png"),
        ("magenta_dye.png", "dye_powder_magenta.png"),
        ("orange_dye.png", "dye_powder_orange.png"),
        ("pink_dye.png", "dye_powder_pink.png"),
        ("purple_dye.png", "dye_powder_purple.png"),
        ("red_dye.png", "dye_powder_red.png"),
        ("light_gray_dye.png", "dye_powder_silver.png"),
        ("bone_meal.png", "dye_powder_white.png"),
        ("yellow_dye.png", "dye_powder_yellow.png"),
        ("fire_charge.png", "fireball.png"),
        ("firework_rocket.png", "fireworks.png"),
        ("firework_star.png", "fireworks_charge.png"),
        ("firework_star_overlay.png", "firework_charge_overlay.png"),
        ("cod.png", "fish_cod_raw.png"),
        ("cooked_cod.png", "fish_cod_cooked.png"),
        ("salmon.png", "fish_salmon_raw.png"),
        ("cooked_salmon.png", "fish_salmon_cooked.png"),
        ("tropical_fish.png", "fish_clownfish_raw.png"),
        ("pufferfish.png", "fish_pufferfish_raw.png"),
        ("map.png", "map_empty.png"),
        ("filled_map.png", "map_filled.png"),
        ("chest_minecart.png", "minecart_chest.png"),
        ("command_block_minecart.png", "minecart_command_block.png"),
        ("furnace_minecart.png", "minecart_furnace.png"),
        ("hopper_minecart.png", "minecart_hopper.png"),
        ("minecart.png", "minecart_normal.png"),
        ("tnt_minecart.png", "minecart_tnt.png"),
        ("cooked_mutton.png", "mutton_cooked.png"),
        ("mutton.png", "mutton_raw.png"),
        ("nether_brick.png", "netherbrick.png"),
        ("baked_potato.png", "potato_baked.png"),
        ("poisonous_potato.png", "potato_poisonous.png"),
        ("music_disc_11.png", "record_11.png"),
        ("music_disc_13.png", "record_13.png"),
        ("music_disc_blocks.png", "record_blocks.png"),
        ("music_disc_cat.png", "record_cat.png"),
        ("music_disc_chirp.png", "record_chirp.png"),
        ("music_disc_far.png", "record_far.png"),
        ("music_disc_mail.png", "record_mail.png"),
        ("music_disc_mellohi.png", "record_mellohi.png"),
        ("music_disc_stal.png", "record_stal.png"),
        ("music_disc_strad.png", "record_strad.png"),
        ("music_disc_wait.png", "record_wait.png"),
        ("music_disc_ward.png", "record_ward.png"),
        ("music_disc_mall.png", "record_mall.png"),
        ("redstone.png", "redstone_dust.png"),
        ("sugar_cane.png", "reeds.png"),
        ("melon_seeds.png", "seeds_melon.png"),
        ("pumpkin_seeds.png", "seeds_pumpkin.png"),
        ("wheat_seeds.png", "seeds_wheat.png"),
        ("oak_sign.png", "sign.png"),
        ("slime_ball.png", "slimeball.png"),
        ("armor_stand.png", "wooden_armorstand.png"),
        ("golden_horse_armor.png", "gold_horse_armor.png"),
        // Original renamed items from old codebase (idempotent when already named correctly)
        ("oak_planks.png", "planks_oak.png"),
        ("birch_planks.png", "planks_birch.png"),
        ("spruce_planks.png", "planks_spruce.png"),
        ("jungle_planks.png", "planks_jungle.png"),
        ("acacia_planks.png", "planks_acacia.png"),
        ("dark_oak_planks.png", "planks_big_oak.png"),
        ("cherry_planks.png", "planks_cherry.png"),
        ("bamboo_planks.png", "planks_bamboo.png"),
        ("crimson_planks.png", "planks_crimson.png"),
        ("warped_planks.png", "planks_warped.png"),
        ("mangrove_planks.png", "planks_mangrove.png"),
        ("mossy_cobblestone.png", "cobblestone_mossy.png"),
        ("chiseled_bookshelf.png", "chiseled_bookshelf_empty.png"),
        ("crafting_table_top.png", "crafting_table_top_old.png"),
        ("oak_log_top.png", "log_oak_top.png"),
        ("spruce_log_top.png", "log_spruce_top.png"),
        ("birch_log_top.png", "log_birch_top.png"),
        ("jungle_log_top.png", "log_jungle_top.png"),
    ];

    for (old, new) in &reverse_pairs {
        let old_path = items_path.join(old);
        if old_path.exists() {
            rename_with_mcmeta(&old_path, &items_path.join(new))
                .map_err(|e| format!("reverse rename item {}: {}", old, e))?;
        }
    }

    log_info!("reverse_rename_blocks_items completed");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "reverse_rename_blocks_items",
        crate::hurray::scheduler::TaskType::Exclusive,
        crate::hurray::scheduler::TaskTier::Surgeon,
        |context| reverse_rename_blocks_items(context.temp_dir()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_reverse_rename_blocks_items() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let result = reverse_rename_blocks_items(temp_dir.path());
        assert!(result.is_ok());
    }
}
