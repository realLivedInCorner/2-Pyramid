import os
import shutil
from PIL import Image
import traceback

# 注意：log函数需要从原文件中提取
def generate_netherite_block(temp_dir):
    log(f"Processing diamond_block image in: {temp_dir}")
    try:
        diamond_block_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'block', 'diamond_block.png')
        netherite_block_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'block', 'netherite_block.png')
        
        if os.path.exists(diamond_block_path):
            # 复制 diamond_block.png 并处理
            shutil.copy(diamond_block_path, netherite_block_path)
            img = Image.open(netherite_block_path).convert("RGBA")
            
            # 对图像进行处理，例如调整亮度、色相等
            # 这里可以根据需要添加具体的处理逻辑
            
            img.save(netherite_block_path)
            log(f"Generated 'netherite_block.png' from 'diamond_block.png'")
        else:
            log(f"No 'diamond_block.png' found in {temp_dir}")

    except Exception as e:
        log(f"Error processing netherite block image: {e}")
        traceback.print_exc()