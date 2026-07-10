import os
import sys
from PIL import Image
import traceback

# 注意：log函数需要从原文件中提取
def fix_brewing_stand_ui(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/gui/container/shulker_box.png 文件，
    """
    try:
        # 定义相关路径
        container_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "gui", "container")
        shulker_box_path = os.path.join(container_path, 'shulker_box.png')
        brewing_stand_new_path = os.path.join(container_path, 'brewing_stand.png')
        # 获取脚本或exe的路径
        if getattr(sys, 'frozen', False):
            exe_folder = os.path.dirname(sys.executable)
        else:
            exe_folder = os.path.dirname(os.path.abspath(__file__))
        # 生成brewing_stand.png的路径
        brewing_stand_path = os.path.join(exe_folder, 'brewing_stand')
        
        log(f"Processing brewing stand image in: {container_path}")
        
        if os.path.exists(shulker_box_path):
            log(f"Found 'shulker_box.png' at {shulker_box_path}")
            img = Image.open(shulker_box_path).convert("RGBA")
            width, height = img.size
            log(f"Image size: {width}x{height}")

            if width == height:
                if width == 256:
                    scale_factor = 1
                    brewing_stand_image_path = os.path.join(brewing_stand_path, 'brewing_stand_256.png')
                elif width == 512:
                    scale_factor = 2
                    brewing_stand_image_path = os.path.join(brewing_stand_path, 'brewing_stand_512.png')
                elif width == 1024:
                    scale_factor = 4
                    brewing_stand_image_path = os.path.join(brewing_stand_path, 'brewing_stand_1024.png')
                elif width == 2048:
                    scale_factor = 8
                    brewing_stand_image_path = os.path.join(brewing_stand_path, 'brewing_stand_2048.png')
                else:
                    log(f"Unsupported image size for 'shulker_box.png': {width}x{height}")
                    return

                img_copy = img.copy()
                cover_box = (6 * scale_factor, 16 * scale_factor, 170 * scale_factor, 72 * scale_factor)
                fill_color = img.getpixel((5 * scale_factor, 4 * scale_factor))
                
                # 填充 cover_box 区域的颜色
                for x in range(cover_box[0], cover_box[2]):
                    for y in range(cover_box[1], cover_box[3]):
                        img_copy.putpixel((x, y), fill_color)
                log(f"Filled cover_box {cover_box} with color {fill_color}")
                
                # 复制并粘贴区域
                region = img_copy.crop((7 * scale_factor, 83 * scale_factor, 25 * scale_factor, 101 * scale_factor))
                img_copy.paste(region, (16 * scale_factor, 16 * scale_factor))
                img_copy.paste(region, (78 * scale_factor, 16 * scale_factor))
                img_copy.paste(region, (55 * scale_factor, 50 * scale_factor))
                img_copy.paste(region, (78 * scale_factor, 57 * scale_factor))
                img_copy.paste(region, (101 * scale_factor, 50 * scale_factor))
                log(f"Pasted region to multiple positions with scale_factor {scale_factor}")

                if os.path.exists(brewing_stand_image_path):
                    overlay_img = Image.open(brewing_stand_image_path).convert("RGBA")
                    img_copy.paste(overlay_img, (0, 0), overlay_img)
                    log(f"Overlayed '{brewing_stand_image_path}' onto brewing_stand.png")
                else:
                    log(f"No overlay image found at '{brewing_stand_image_path}'")

                # 保存 brewing_stand.png
                img_copy.save(brewing_stand_new_path)
                log(f"Processed 'shulker_box.png' and saved as 'brewing_stand.png'")
            else:
                log(f"'shulker_box.png' is not a square image: {width}x{height}. Skipping processing.")
        else:
            log(f"No 'shulker_box.png' found in {container_path}")

    except Exception as e:
        log(f"Error processing brewing stand image in '{container_path}': {e}")
        traceback.print_exc()