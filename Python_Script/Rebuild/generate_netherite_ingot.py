import os
import shutil
from PIL import Image
import traceback

# 注意：log函数、rgba_to_hsv函数、hsv_to_rgba函数需要从原文件中提取
def generate_netherite_ingot(temp_dir):
    log(f"Processing gold_ingot image in: {temp_dir}")
    try:
        gold_ingot_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'item', 'gold_ingot.png')
        netherite_ingot_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'item', 'netherite_ingot.png')
        
        if os.path.exists(gold_ingot_path):
            # 复制 gold_ingot.png 并处理
            shutil.copy(gold_ingot_path, netherite_ingot_path)
            img = Image.open(netherite_ingot_path).convert("RGBA")
            
            new_image_data = []
            for item in img.getdata():
                if item[3] == 0:
                    new_image_data.append(item)
                else:
                    hsva = rgba_to_hsv(item)
                    new_hue = 310 / 360.0  # 色相改为恒定310
                    new_saturation = hsva[1] / 3
                    new_value = hsva[2] / 3
                    new_image_data.append(hsv_to_rgba((new_hue, new_saturation, new_value, hsva[3])))
            new_image = Image.new("RGBA", img.size)
            new_image.putdata(new_image_data)
            new_image.save(netherite_ingot_path)
            log(f"Processed 'gold_ingot.png' to 'netherite_ingot.png' with adjusted HSV values")

            # 复制 gold_ingot.png.mcmeta 并重命名
            gold_ingot_mcmeta_path = gold_ingot_path + '.mcmeta'
            netherite_ingot_mcmeta_path = netherite_ingot_path + '.mcmeta'
            if os.path.exists(gold_ingot_mcmeta_path):
                shutil.copy(gold_ingot_mcmeta_path, netherite_ingot_mcmeta_path)
                log(f"Copied and renamed 'gold_ingot.png.mcmeta' to 'netherite_ingot.png.mcmeta'")
        else:
            log(f"No 'gold_ingot.png' found in {temp_dir}")
    except Exception as e:
        log(f"Error processing 'gold_ingot.png': {e}")
        traceback.print_exc()