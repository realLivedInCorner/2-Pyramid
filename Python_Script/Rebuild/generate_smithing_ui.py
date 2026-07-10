import os
from PIL import Image
import traceback

# 注意：log函数需要从原文件中提取
def generate_smithing_ui(temp_dir):
    log(f"Processing smithing image in: {temp_dir}")
    try:
        anvil_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container', 'anvil.png')
        smithing_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container', 'smithing.png')

        if os.path.exists(anvil_path):
            img = Image.open(anvil_path).convert("RGBA")
            width, height = img.size

            if width == 256 and height == 256:
                scale_factor = 1
                smithing_image_path = os.path.join('smithing', 'smithing_256.png')
            elif width == 512 and height == 512:
                scale_factor = 2
                smithing_image_path = os.path.join('smithing', 'smithing_512.png')
            elif width == 1024 and height == 1024:
                scale_factor = 4
                smithing_image_path = os.path.join('smithing', 'smithing_1024.png')
            elif width == 2048 and height == 2048:
                scale_factor = 8
                smithing_image_path = os.path.join('smithing', 'smithing_2048.png')
            else:
                log(f"Unsupported image size for 'anvil.png': {width}x{height}")
                return

            img_copy = img.copy()

            # 定义覆盖区域并填充颜色
            cover_box = (10 * scale_factor, 5 * scale_factor, 169 * scale_factor, 37 * scale_factor)
            fill_color = img.getpixel((5 * scale_factor, 4 * scale_factor))  # 获取 (5,4) 的像素颜色

            log(f"Filling region {cover_box} with color {fill_color}")
            for x in range(cover_box[0], cover_box[2]):
                for y in range(cover_box[1], cover_box[3]):
                    img_copy.putpixel((x, y), fill_color)

            # 覆盖图像（如果存在）
            if os.path.exists(smithing_image_path):
                overlay_img = Image.open(smithing_image_path).convert("RGBA")
                img_copy.paste(overlay_img, (0, 0), overlay_img)
                log(f"Overlayed {smithing_image_path} onto smithing.png")
            else:
                log(f"No overlay image found for size {width}x{height}")

            # 新步骤1：将 (0,166)-(110,198) 的区域设置为全透明（根据 scale_factor 缩放）
            try:
                transparent_box = (
                    0 * scale_factor,
                    166 * scale_factor,
                    110 * scale_factor,
                    198 * scale_factor
                )
                log(f"Setting region {transparent_box} to fully transparent in grindstone.png")
                # 创建一个全透明的图像
                transparent_region = Image.new('RGBA', (transparent_box[2] - transparent_box[0], 
                                                      transparent_box[3] - transparent_box[1]), 
                                               (0, 0, 0, 0))
                # 粘贴到指定区域
                img_copy.paste(transparent_region, (transparent_box[0], transparent_box[1]))
                log(f"Region {transparent_box} set to transparent")
            except Exception as e:
                log(f"Error setting transparency in 'grindstone.png': {e}")
                traceback.print_exc()

            # 保存生成的 smithing.png
            try:
                img_copy.save(smithing_path)
                log(f"Processed 'anvil.png' and saved as 'smithing.png'")
            except Exception as e:
                log(f"Error saving 'smithing.png': {e}")
                return
        else:
            log(f"No 'anvil.png' found in {temp_dir}")

    except Exception as e:
        log(f"Error processing smithing image in '{temp_dir}': {e}") 
        traceback.print_exc()