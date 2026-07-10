import os
from PIL import Image
import traceback

# 注意：log函数、determine_scale_factor函数需要从原文件中提取
def generate_shulker_box_ui(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/gui/container/generic_54.png 文件，
    """
    try:
        # 定义相关路径
        container_path = os.path.join(
            temp_dir,
            "assets",
            "minecraft",
            "textures",
            "gui",
            "container"
        )
        generic_54_path = os.path.join(container_path, 'generic_54.png')
        shulker_box_path = os.path.join(container_path, 'shulker_box.png')
        
        log(f"Processing generic 54 image in: {container_path}")
        
        if os.path.exists(generic_54_path):
            log(f"Found 'generic_54.png' at {generic_54_path}")
            img = Image.open(generic_54_path).convert("RGBA")
            width, height = img.size
            log(f"Image size: {width}x{height}")

            # 确定 scale_factor
            scale_factor, is_exact = determine_scale_factor(width, height)
            log(f"Determined scale_factor: {scale_factor} (Exact match: {is_exact})")

            if not is_exact:
                log(f"Scale factor for 'generic_54.png' not exact. Proceeding with scale_factor: {scale_factor}")

            # 创建图像副本进行修改
            new_img = img.copy()

            # 1. 将 (0,71)-(176,127) 区域设为透明
            for x in range(0, 176 * scale_factor):
                for y in range(71 * scale_factor, 127 * scale_factor):
                    new_img.putpixel((x, y), (0, 0, 0, 0))
            log(f"Set pixels in (0, {71 * scale_factor}) to (176*{scale_factor}, {127 * scale_factor}) to transparent.")

            # 2. 将 (0,127)-(176,222) 区域向上移动56*scale_factor像素，并将原位置设为透明
            for x in range(0, 176 * scale_factor):
                for y in range(127 * scale_factor, 222 * scale_factor):
                    new_y = y - 56 * scale_factor
                    if new_y >= 0:
                        pixel = img.getpixel((x, y))
                        new_img.putpixel((x, new_y), pixel)
                    new_img.putpixel((x, y), (0, 0, 0, 0))
            log(f"Moved pixels in (0, {127 * scale_factor}) to (176*{scale_factor}, {222 * scale_factor}) up by {56 * scale_factor} pixels and set original area to transparent.")

            # 保存生成的 shulker_box.png
            new_img.save(shulker_box_path)
            log(f"Processed 'generic_54.png' and saved as 'shulker_box.png'")
        else:
            log(f"No 'generic_54.png' found in {container_path}")
    
    except Exception as e:
        log(f"Error processing 'generic_54.png' in '{container_path}': {e}")
        traceback.print_exc()