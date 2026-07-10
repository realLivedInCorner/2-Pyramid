import os
import shutil
from PIL import Image
import traceback

# 注意：此函数依赖于外部的log函数和adjust_copper_color函数

def generate_copper_tools(temp_dir):
    log(f"Processing and copying copper tools/items in: {temp_dir}")
    items_path_new = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'item')
    items_to_copy_and_process = [
        'iron_sword', 'iron_helmet', 'iron_chestplate', 'iron_leggings', 'iron_boots',
        'iron_axe', 'iron_pickaxe', 'iron_shovel', 'iron_hoe', 'iron_horse_armor'
    ]
    # 备选材质列表，按优先级排序
    # 注意：不同材质的命名可能有所不同，需要确保使用正确的文件名格式
    alternative_materials = ['diamond', 'golden', 'stone', 'netherite']
    
    for item in items_to_copy_and_process:
        try:
            original_path = os.path.join(items_path_new, f'{item}.png')
            new_path = os.path.join(items_path_new, f'copper_{item[5:]}.png')
            
            # 检查原始铁材质是否存在
            if os.path.exists(original_path):
                shutil.copy(original_path, new_path)
                log(f"Copied and renamed '{item}.png' to 'copper_{item[5:]}.png'")
            else:
                log(f"'{item}.png' does not exist, trying alternative materials...")
                # 尝试使用备选材质
                found_alternative = False
                for material in alternative_materials:
                    # 根据材质类型使用正确的文件名格式
                    if material == 'golden':
                        alt_material = 'gold'  # 金色工具使用'gold'而不是'golden'
                    else:
                        alt_material = material
                    
                    alt_item = f'{alt_material}_{item[5:]}'
                    alt_path = os.path.join(items_path_new, f'{alt_item}.png')
                    if os.path.exists(alt_path):
                        shutil.copy(alt_path, new_path)
                        log(f"Copied and renamed '{alt_item}.png' to 'copper_{item[5:]}.png' as alternative")
                        found_alternative = True
                        break
                
                if not found_alternative:
                    log(f"No suitable alternative found for '{item}.png', skipping...")
                    continue
            
            # 使用铜材质专用的颜色调整函数
            img = Image.open(new_path).convert("RGBA")
            img = adjust_copper_color(img)
            img.save(new_path)
            log(f"Processed copper image 'copper_{item[5:]}.png'")

            # 复制mcmeta文件（如果存在）
            original_meta_path = original_path + '.mcmeta'
            new_meta_path = new_path + '.mcmeta'
            if os.path.exists(original_meta_path):
                shutil.copy(original_meta_path, new_meta_path)
                log(f"Copied and renamed '{item}.png.mcmeta' to 'copper_{item[5:]}.png.mcmeta'")

        except Exception as e:
            log(f"Error processing and copying copper item '{item}': {e}")
            traceback.print_exc()