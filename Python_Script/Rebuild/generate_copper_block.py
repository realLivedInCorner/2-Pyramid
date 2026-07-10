import os
import shutil
from PIL import Image
import traceback

# 注意：此函数依赖于外部的log函数和adjust_copper_color函数

def generate_copper_block(temp_dir):
    log(f"Processing iron_block images in: {temp_dir}")
    try:
        blocks_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'block')
        iron_block_path = os.path.join(blocks_path, 'iron_block.png')
        
        if os.path.exists(iron_block_path):
            width, height = Image.open(iron_block_path).size
            
            # 定义不同氧化阶段的颜色（按照氧化程度依次变青）
            colors = {
                'copper': None,  # 使用默认的铜色调整函数
                'exposed': (100, 180, 160, 255),  # 轻度氧化 - 浅青绿色
                'weathered': (70, 190, 180, 255),  # 中度氧化 - 中等青色
                'oxidized': (50, 210, 210, 255)   # 重度氧化 - 纯青色
            }
            
            # 1. 普通铜方块 - 使用铜色调整函数
            copper_block_path = os.path.join(blocks_path, 'copper_block.png')
            shutil.copy(iron_block_path, copper_block_path)
            copper_img = Image.open(copper_block_path).convert("RGBA")
            
            # 使用专门的铜材质颜色调整函数
            copper_img = adjust_copper_color(copper_img)
            copper_img.save(copper_block_path)
            log(f"Generated 'copper_block.png' with copper color")
            
            # 2. 轻度氧化铜方块 - 浅青绿色调，保留纹理细节
            exposed_copper_path = os.path.join(blocks_path, 'exposed_copper.png')
            shutil.copy(iron_block_path, exposed_copper_path)
            exposed_img = Image.open(exposed_copper_path).convert("RGBA")
            exposed_pixels = exposed_img.load()
            
            # 首先应用铜色调整
            exposed_img = adjust_copper_color(exposed_img)
            
            # 然后将整体颜色调整为浅青绿色
            for y in range(height):
                for x in range(width):
                    r, g, b, a = exposed_pixels[x, y]
                    if a > 0:
                        # 应用浅青绿色调
                        new_r = int(r * 0.8 + colors['exposed'][0] * 0.2)
                        new_g = int(g * 0.7 + colors['exposed'][1] * 0.3)
                        new_b = int(b * 0.6 + colors['exposed'][2] * 0.4)
                        exposed_pixels[x, y] = (min(255, new_r), min(255, new_g), min(255, new_b), a)
            
            exposed_img.save(exposed_copper_path)
            log(f"Generated 'exposed_copper.png' with light cyan-green color")
            
            # 3. 中度氧化铜方块 - 中等青色，纹理细节较少
            weathered_copper_path = os.path.join(blocks_path, 'weathered_copper.png')
            shutil.copy(iron_block_path, weathered_copper_path)
            weathered_img = Image.open(weathered_copper_path).convert("RGBA")
            weathered_pixels = weathered_img.load()
            
            # 首先应用铜色调整
            weathered_img = adjust_copper_color(weathered_img)
            
            # 然后将整体颜色调整为中等青色
            for y in range(height):
                for x in range(width):
                    r, g, b, a = weathered_pixels[x, y]
                    if a > 0:
                        # 应用中等青色调
                        new_r = int(r * 0.6 + colors['weathered'][0] * 0.4)
                        new_g = int(g * 0.5 + colors['weathered'][1] * 0.5)
                        new_b = int(b * 0.4 + colors['weathered'][2] * 0.6)
                        weathered_pixels[x, y] = (min(255, new_r), min(255, new_g), min(255, new_b), a)
            
            weathered_img.save(weathered_copper_path)
            log(f"Generated 'weathered_copper.png' with medium cyan color")
            
            # 4. 重度氧化铜方块 - 纯青色，只有一个颜色
            oxidized_copper_path = os.path.join(blocks_path, 'oxidized_copper.png')
            shutil.copy(iron_block_path, oxidized_copper_path)
            oxidized_img = Image.open(oxidized_copper_path).convert("RGBA")
            oxidized_pixels = oxidized_img.load()
            
            # 直接设置为纯青色，不保留纹理细节
            for y in range(height):
                for x in range(width):
                    r, g, b, a = oxidized_pixels[x, y]
                    if a > 0:
                        oxidized_pixels[x, y] = colors['oxidized']  # 纯青色
            
            oxidized_img.save(oxidized_copper_path)
            log(f"Generated 'oxidized_copper.png' with full pure cyan color")
            
            # 复制 .mcmeta 文件（如果存在）
            iron_block_mcmeta_path = iron_block_path + '.mcmeta'
            if os.path.exists(iron_block_mcmeta_path):
                shutil.copy(iron_block_mcmeta_path, copper_block_path + '.mcmeta')
                shutil.copy(iron_block_mcmeta_path, exposed_copper_path + '.mcmeta')
                shutil.copy(iron_block_mcmeta_path, weathered_copper_path + '.mcmeta')
                shutil.copy(iron_block_mcmeta_path, oxidized_copper_path + '.mcmeta')
                log(f"Copied and renamed 'iron_block.png.mcmeta' for all copper block variants")
        else:
            log(f"No 'iron_block.png' found in {temp_dir}")
    except Exception as e:
        log(f"Error processing 'iron_block.png': {e}")
        traceback.print_exc()