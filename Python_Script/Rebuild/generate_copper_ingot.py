import os
import shutil
from PIL import Image
import traceback

# 注意：此函数依赖于外部的log函数和adjust_copper_color函数

def generate_copper_ingot(temp_dir):
    log(f"Processing iron_ingot image in: {temp_dir}")
    try:
        iron_ingot_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'item', 'iron_ingot.png')
        copper_ingot_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'item', 'copper_ingot.png')
        
        if os.path.exists(iron_ingot_path):
            # 复制 iron_ingot.png 并处理
            shutil.copy(iron_ingot_path, copper_ingot_path)
            img = Image.open(copper_ingot_path).convert("RGBA")
            
            # 使用专门的铜材质颜色调整函数，确保效果明显
            img = adjust_copper_color(img)
            img.save(copper_ingot_path)
            log(f"Processed 'iron_ingot.png' to 'copper_ingot.png' with brass color")

            # 复制 iron_ingot.png.mcmeta 并重命名
            iron_ingot_mcmeta_path = iron_ingot_path + '.mcmeta'
            copper_ingot_mcmeta_path = copper_ingot_path + '.mcmeta'
            if os.path.exists(iron_ingot_mcmeta_path):
                shutil.copy(iron_ingot_mcmeta_path, copper_ingot_mcmeta_path)
                log(f"Copied and renamed 'iron_ingot.png.mcmeta' to 'copper_ingot.png.mcmeta'")
        else:
            log(f"No 'iron_ingot.png' found in {temp_dir}")
    except Exception as e:
        log(f"Error processing 'iron_ingot.png': {e}")
        traceback.print_exc()