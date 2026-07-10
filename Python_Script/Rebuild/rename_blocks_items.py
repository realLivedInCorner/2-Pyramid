import os
import traceback

# 注意：log函数需要从原文件中提取
def rename_blocks_items(temp_dir):
    """
    重命名并处理 temp_dir 中的 'blocks' 和 'items' 文件夹。
    """
    try:
        assets_path = os.path.join(temp_dir, "assets", "minecraft", "textures")
        blocks_path_old = os.path.join(assets_path, 'blocks')
        blocks_path_new = os.path.join(assets_path, 'block')
        items_path_old = os.path.join(assets_path, 'items')
        items_path_new = os.path.join(assets_path, 'item')
        
        # 重命名 'blocks' 文件夹为 'block'
        if os.path.exists(blocks_path_old):
            os.rename(blocks_path_old, blocks_path_new)
            log(f"已将 'blocks' 重命名为 'block' 在 {temp_dir}")
        else:
            log(f"未找到 {temp_dir} 中的 'blocks' 文件夹")
        
        # 重命名 'items' 文件夹为 'item'
        if os.path.exists(items_path_old):
            os.rename(items_path_old, items_path_new)
            log(f"已将 'items' 重命名为 'item' 在 {temp_dir}")
        else:
            log(f"未找到 {temp_dir} 中的 'items' 文件夹")
        
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
        }

        # 执行重命名
        for old_name, new_name in rename_pairs.items():
            old_path = os.path.join(items_path_new, old_name)
            new_path = os.path.join(items_path_new, new_name)
            if os.path.exists(old_path):
                if os.path.exists(new_path):
                    os.remove(new_path)
                os.rename(old_path, new_path)
                log(f"已将 '{old_name}' 重命名为 '{new_name}'")

    except Exception as e:
        log(f"重命名文件时出错: {e}")
        traceback.print_exc()