import os

# 注意：此函数依赖于外部的log函数、rename_items函数和rename_and_process_blocks函数

def reverse_rename_blocks_items(temp_dir):
    textures_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures')           
    item_dir_old = os.path.join(textures_path, 'item')
    items_dir_new = os.path.join(textures_path, 'items')
    if os.path.exists(item_dir_old):
        os.rename(item_dir_old, items_dir_new)
        log(f"已将 'item' 重命名为 'items' 在 {textures_path}")
    else:
        log(f"未找到 'item' 文件夹在 {textures_path} 以重命名为 'items'")

    block_dir_old = os.path.join(textures_path, 'block')
    blocks_dir_new = os.path.join(textures_path, 'blocks')
    if os.path.exists(block_dir_old):
        os.rename(block_dir_old, blocks_dir_new)
        log(f"已将 'block' 重命名为 'blocks' 在 {textures_path}")
    else:
        log(f"未找到 'block' 文件夹在 {textures_path} 以重命名为 'blocks'")

        # 定义重命名对
    rename_pairs = {
        'gold_sword.png': 'golden_sword.png',
        'wood_sword.png': 'wooden_sword.png',
        'gold_helmet.png': 'golden_helmet.png',
        'gold_chestplate.png': 'golden_chestplate.png',
        'gold_leggings.png': 'golden_leggings.png',
        'gold_boots.png': 'golden_boots.png',
        'apple_golden.png': 'golden_apple.png',
        'bow_standby.png': 'bow.png',
        'book_enchanted.png': 'enchanted_book.png',
        'wood_axe.png': 'wooden_axe.png',
        'wood_pickaxe.png': 'wooden_pickaxe.png',
        'wood_shovel.png': 'wooden_shovel.png',
        'wood_hoe.png': 'wooden_hoe.png',
        'gold_axe.png': 'golden_axe.png',
        'gold_pickaxe.png': 'golden_pickaxe.png',
        'gold_shovel.png': 'golden_shovel.png',
        'gold_hoe.png': 'golden_hoe.png',
        'fishing_rod_uncast.png': 'fishing_rod.png',
        'potion_bottle_empty.png': 'glass_bottle.png',
        'potion_bottle_drinkable.png': 'potion.png',
        'potion_bottle_splash.png': 'splash_potion.png',
        'potion_bottle_lingering.png': 'lingering_potion.png',
        'spider_eye_fermented.png': 'fermented_spider_eye.png',
        'melon_speckled.png': 'glistering_melon_slice.png',
        'melon.png': 'melon_slice.png',
        'carrot_golden.png': 'golden_carrot.png',
        'porkchop_raw.png': 'porkchop.png',
        'porkchop_cooked.png': 'cooked_porkchop.png',
        'chicken_raw.png': 'chicken.png',
        'chicken_cooked.png': 'cooked_chicken.png',
        'rabbit_raw.png': 'rabbit.png',
        'rabbit_cooked.png': 'cooked_rabbit.png',
        'beef_raw.png': 'beef.png',
        'beef_cooked.png': 'cooked_beef.png',
        'boat.png': 'oak_boat.png',
        'book_normal.png': 'book.png',
        'book_writable.png': 'writable_book.png',
        'book_written.png': 'written_book.png',
        'bucket_empty.png': 'bucket.png',
        'bucket_lava.png': 'lava_bucket.png',
        'bucket_water.png': 'water_bucket.png',
        'bucket_milk.png': 'milk_bucket.png',
        'door_acacia.png': 'acacia_door.png',
        'door_birch.png': 'birch_door.png',
        'door_dark_oak.png': 'dark_oak_door.png',
        'door_iron.png': 'iron_door.png',
        'door_jungle.png': 'jungle_door.png',
        'door_spruce.png': 'spruce_door.png',
        'door_wood.png': 'oak_door.png',
        'dye_powder_black.png': 'ink_sac.png',
        'dye_powder_blue.png': 'lapis_lazuli.png',
        'dye_powder_brown.png': 'cocoa_beans.png',
        'dye_powder_cyan.png': 'cyan_dye.png',
        'dye_powder_gray.png': 'gray_dye.png',
        'dye_powder_green.png': 'green_dye.png',
        'dye_powder_light_blue.png': 'light_blue_dye.png',
        'dye_powder_lime.png': 'lime_dye.png',
        'dye_powder_magenta.png': 'magenta_dye.png',
        'dye_powder_orange.png': 'orange_dye.png',
        'dye_powder_pink.png': 'pink_dye.png',
        'dye_powder_purple.png': 'purple_dye.png',
        'dye_powder_red.png': 'red_dye.png',
        'dye_powder_silver.png': 'light_gray_dye.png',
        'dye_powder_white.png': 'bone_meal.png',
        'dye_powder_yellow.png': 'yellow_dye.png',
        'fireball.png': 'fire_charge.png',
        'fireworks.png': 'firework_rocket.png',
        'fireworks_charge.png': 'firework_star.png',
        'firework_charge_overlay.png': 'firework_star_overlay.png',
        'fish_cod_raw.png': 'cod.png',
        'fish_cod_cooked.png': 'cooked_cod.png',
        'fish_salmon_raw.png': 'salmon.png',
        'fish_salmon_cooked.png': 'cooked_salmon.png',
        'fish_clownfish_raw.png': 'tropical_fish.png',
        'fish_pufferfish_raw.png': 'pufferfish.png',
        'map_empty.png': 'map.png',
        'map_filled.png': 'filled_map.png',
        'minecart_chest.png': 'chest_minecart.png',
        'minecart_command_block.png': 'command_block_minecart.png',
        'minecart_furnace.png': 'furnace_minecart.png',
        'minecart_hopper.png': 'hopper_minecart.png',
        'minecart_normal.png': 'minecart.png',
        'minecart_tnt.png': 'tnt_minecart.png',
        'mutton_cooked.png': 'cooked_mutton.png',
        'mutton_raw.png': 'mutton.png',
        'netherbrick.png': 'nether_brick.png',
        'potato_baked.png': 'baked_potato.png',
        'potato_poisonous.png': 'poisonous_potato.png',
        'record_11.png': 'music_disc_11.png',
        'record_13.png': 'music_disc_13.png',
        'record_blocks.png': 'music_disc_blocks.png',
        'record_cat.png': 'music_disc_cat.png',
        'record_chirp.png': 'music_disc_chirp.png',
        'record_far.png': 'music_disc_far.png',
        'record_mail.png': 'music_disc_mail.png',
        'record_mellohi.png': 'music_disc_mellohi.png',
        'record_stal.png': 'music_disc_stal.png',
        'record_strad.png': 'music_disc_strad.png',
        'record_wait.png': 'music_disc_wait.png',
        'record_ward.png': 'music_disc_ward.png',
        'record_mall.png': 'music_disc_mall.png',
        'redstone_dust.png': 'redstone.png',
        'reeds.png': 'sugar_cane.png',
        'seeds_melon.png': 'melon_seeds.png',
        'seeds_pumpkin.png': 'pumpkin_seeds.png',
        'seeds_wheat.png': 'wheat_seeds.png',
        'sign.png': 'oak_sign.png',
        'slimeball.png': 'slime_ball.png',
        'wooden_armorstand.png': 'armor_stand.png',
        'gold_horse_armor.png': 'golden_horse_armor.png'
        }

    # 反转 rename_pairs
    rename_pairs_reversed = {v: k for k, v in rename_pairs.items()}
    log("使用反转的 rename_pairs 进行文件重命名用于1.21.4转1.8转换")

    items_path_new = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'items')
    if os.path.exists(items_path_new):
        rename_items(items_path_new, rename_pairs_reversed)
        log("已使用反转的 rename_pairs 重命名 items")
    else:
        log(f"未找到 'item' 文件夹在 {temp_dir}/assets/minecraft/textures")

    # 确保 blocks_path_new 指向 assets/minecraft/textures/block
    blocks_path_new = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'blocks')
    if os.path.exists(blocks_path_new):
        rename_and_process_blocks(blocks_path_new, reverse=True)
        log("已使用反转的 rename_pairs 重命名并处理 blocks")
    else:
        log(f"未找到 'block' 文件夹在 {temp_dir}/assets/minecraft/textures")