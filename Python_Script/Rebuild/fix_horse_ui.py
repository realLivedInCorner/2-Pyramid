import os
import sys
from PIL import Image
import traceback

# 注意：log函数需要从原文件中提取
def fix_horse_ui(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/gui/container/horse.png 文件，
    """
    try:
        # 定义相关路径
        container_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "gui", "container")
        horse_new_path = os.path.join(container_path, 'horse.png')
        
        # 获取脚本或exe的路径
        if getattr(sys, 'frozen', False):
            exe_folder = os.path.dirname(sys.executable)
        else:
            exe_folder = os.path.dirname(os.path.abspath(__file__))
        # 生成brewing_stand.png的路径
        horse_path = os.path.join(exe_folder, 'horse')

        log(f"Processing horse image in: {container_path}")
        
        if os.path.exists(horse_new_path):
            log(f"Found 'horse.png' at {horse_new_path}")
            img = Image.open(horse_new_path).convert("RGBA")
            width, height = img.size
            log(f"Image size: {width}x{height}")

            # 根据分辨率设置 scale_factor 和输出路径
            if width == 256 and height == 256:
                scale_factor = 1
                horse_image_path = os.path.join(horse_path, 'horse_256.png')
            elif width == 512 and height == 512:
                scale_factor = 2
                horse_image_path = os.path.join(horse_path, 'horse_512.png')
            elif width == 1024 and height == 1024:
                scale_factor = 4
                horse_image_path = os.path.join(horse_path, 'horse_1024.png')
            elif width == 2048 and height == 2048:
                scale_factor = 8
                horse_image_path = os.path.join(horse_path, 'horse_2048.png')
            else:
                log(f"Unsupported image size for 'horse.png': {width}x{height}")
                return

            img_copy = img.copy()

            # 步骤1: 将 (7,17) 到 (25,35) 的矩形区域平移到 (18,220) 到 (36,238)
            try:
                move_box = (7 * scale_factor, 17 * scale_factor, 25 * scale_factor, 35 * scale_factor)
                paste_box = (18 * scale_factor, 220 * scale_factor, 36 * scale_factor, 238 * scale_factor)
                region = img.crop(move_box)
                img_copy.paste(region, paste_box)
                log(f"Moved region {move_box} to {paste_box}")
            except Exception as e:
                log(f"Error moving region: {e}")

            # 步骤2: 填充 (7,17) 到 (25,35) 区域的颜色
            try:
                fill_color = img.getpixel((5 * scale_factor, 5 * scale_factor))
                for x in range(move_box[0], move_box[2]):
                    for y in range(move_box[1], move_box[3]):
                        img_copy.putpixel((x, y), fill_color)
                log(f"Filled region {move_box} with color {fill_color}")
            except Exception as e:
                log(f"Error filling region: {e}")

            # 步骤3: 粘贴 horse_xxx.png 到目标位置
            if os.path.exists(horse_image_path):
                try:
                    overlay_img = Image.open(horse_image_path).convert("RGBA")
                    img_copy.paste(overlay_img, (0, 0), overlay_img)
                    log(f"Overlayed '{horse_image_path}' onto horse.png")
                except Exception as e:
                    log(f"Error overlaying image: {e}")
            else:
                log(f"No overlay image found at '{horse_image_path}'")

            # 保存修改后的 horse.png
            try:
                img_copy.save(horse_new_path)
                log(f"Processed 'horse.png' and saved changes")
            except Exception as e:
                log(f"Error saving image: {e}")
        else:
            log(f"No 'horse.png' found in {container_path}")

    except Exception as e:
        log(f"Error processing horse image: {e}")
        traceback.print_exc()