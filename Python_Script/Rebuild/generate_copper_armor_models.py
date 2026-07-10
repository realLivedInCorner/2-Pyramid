import os
import shutil
from PIL import Image
import traceback

# 注意：此函数依赖于外部的log函数和adjust_copper_color函数

def generate_copper_armor_models(temp_dir):
    log(f"Processing copper armor layers in: {temp_dir}")
    armor_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'models', 'armor')
    armor_files = ['iron_layer_1.png', 'iron_layer_2.png']
    # 备选材质列表，按优先级排序
    # 注意：不同材质的命名可能有所不同，需要确保使用正确的文件名格式
    alternative_materials = ['diamond', 'golden', 'chainmail', 'leather']
    
    for armor_file in armor_files:
        try:
            original_path = os.path.join(armor_path, armor_file)
            new_path = os.path.join(armor_path, armor_file.replace('iron', 'copper'))
            
            # 检查原始铁材质是否存在
            if os.path.exists(original_path):
                shutil.copy(original_path, new_path)
                log(f"Copied and renamed '{armor_file}' to '{armor_file.replace('iron', 'copper')}'")
            else:
                log(f"'{original_path}' does not exist, trying alternative materials...")
                # 尝试使用备选材质
                found_alternative = False
                for material in alternative_materials:
                    # 根据材质类型使用正确的文件名格式
                    if material == 'golden':
                        alt_file = armor_file.replace('iron', 'gold')  # 金色盔甲使用'gold'而不是'golden'
                    else:
                        alt_file = armor_file.replace('iron', material)
                    
                    alt_path = os.path.join(armor_path, alt_file)
                    if os.path.exists(alt_path):
                        shutil.copy(alt_path, new_path)
                        log(f"Copied and renamed '{alt_file}' to '{armor_file.replace('iron', 'copper')}' as alternative")
                        found_alternative = True
                        break
                
                if not found_alternative:
                    log(f"No suitable alternative found for '{armor_file}', skipping...")
                    continue
            
            # 使用铜材质专用的颜色调整函数
            img = Image.open(new_path).convert("RGBA")
            img = adjust_copper_color(img)
            img.save(new_path)
            log(f"Processed copper armor image '{armor_file.replace('iron', 'copper')}'")
        except Exception as e:
            log(f"Error processing and copying copper armor layer '{armor_file}': {e}")
            traceback.print_exc()