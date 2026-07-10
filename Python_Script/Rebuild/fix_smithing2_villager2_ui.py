import os
import sys
import shutil
from PIL import Image
import traceback

# 注意：此函数依赖于外部的log函数和determine_scale_factor函数

def fix_smithing2_villager2_ui(temp_dir):
    container_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container')
    log(f"Processing smithing2 image in: {container_path}")
    try:
        anvil_path = os.path.join(container_path, 'anvil.png')
        smithing2_new_path = os.path.join(container_path, 'smithing.png')

        # 获取 crossbow_dir，假设 crossbow 文件夹在 .exe 同目录下
        if getattr(sys, 'frozen', False):
            # 如果是打包后的 .exe
            application_path = os.path.dirname(sys.executable)
        else:
            # 如果是未打包的脚本
            application_path = os.path.dirname(os.path.abspath(__file__))
        
        smithing2_path = os.path.join(application_path, 'smithing2')
        
        if os.path.exists(anvil_path):
            img = Image.open(anvil_path).convert("RGBA")
            width, height = img.size

            if width == 256 and height == 256:
                scale_factor = 1
                smithing2_image_path = os.path.join('smithing2', 'smithing2_256.png')
            elif width == 512 and height == 512:
                scale_factor = 2
                smithing2_image_path = os.path.join('smithing2', 'smithing2_512.png')
            elif width == 1024 and height == 1024:
                scale_factor = 4
                smithing2_image_path = os.path.join('smithing2', 'smithing2_1024.png')
            elif width == 2048 and height == 2048:
                scale_factor = 8
                smithing2_image_path = os.path.join('smithing2', 'smithing2_2048.png')
            else:
                log(f"Unsupported image size for 'anvil.png': {width}x{height}")
                return

            img_copy = img.copy()
            cover_box = (5 * scale_factor, 5 * scale_factor, 171 * scale_factor, 72 * scale_factor)
            fill_color = img.getpixel((5 * scale_factor, 4 * scale_factor))
            
            for x in range(cover_box[0], cover_box[2]):
                for y in range(cover_box[1], cover_box[3]):
                    img_copy.putpixel((x, y), fill_color)
                    
            region = img_copy.crop((7 * scale_factor, 83 * scale_factor, 25 * scale_factor, 101 * scale_factor))
            img_copy.paste(region, (7 * scale_factor, 47 * scale_factor))
            img_copy.paste(region, (25 * scale_factor, 47 * scale_factor))
            img_copy.paste(region, (43 * scale_factor, 47 * scale_factor))
            img_copy.paste(region, (97 * scale_factor, 47 * scale_factor))
            
            if os.path.exists(smithing2_image_path):
                overlay_img = Image.open(smithing2_image_path).convert("RGBA")
                img_copy.paste(overlay_img, (0, 0), overlay_img)
                img_copy.save(smithing2_new_path)
                log(f"Processed 'anvil.png' and saved as 'smithing.png'")
            else:
                log(f"No overlay image found for size {width}x{height}")
        else:
            log(f"No 'anvil.png' found in {container_path}")

        villager_path = os.path.join(container_path, 'villager.png')
        if os.path.exists(villager_path):
            img = Image.open(villager_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            scale_factor, is_exact = determine_scale_factor(width, height)

            log(f"Processing villager.png, size: {width}x{height}, scale_factor: {scale_factor} (Exact match: {is_exact})")

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            def scaled_point(x, y):
                """
    根据 scale_factor 缩放单点坐标
    """
                return (x * scale_factor, y * scale_factor)

            # 定义新图像的尺寸（宽度翻倍，保持高度不变）
            new_width = width * 2
            new_height = height
            temp_villager_path = os.path.join(container_path, 'villager_temp.png')  # 临时保存路径

            # 创建一个新的透明图像
            villager2_img = Image.new('RGBA', (new_width, new_height), (0, 0, 0, 0))
            log(f"Created new transparent image: {temp_villager_path}, size: {villager2_img.size}")

            # 裁剪原图并粘贴到新图像
            crop_region = scaled_box(0, 0, 240, 166)
            paste_position = (100 * scale_factor, 0)  # (100, 0) scaled
            try:
                cropped_img = img.crop(crop_region)
                villager2_img.paste(cropped_img, paste_position)
                log(f"Pasted cropped region {crop_region} to {paste_position}")
            except Exception as e:
                log(f"Error cropping and pasting regions: {e}")
                traceback.print_exc()
                return

            # 获取脚本所在目录（假设覆盖图像在与exe同目录的villager2文件夹中）
            if getattr(sys, 'frozen', False):
                # 如果是打包后的exe
                script_dir = os.path.dirname(sys.executable)
            else:
                # 如果是脚本运行
                script_dir = os.path.dirname(os.path.abspath(__file__))
            
            overlay_folder = os.path.join(script_dir, 'villager2')
            overlay_image_name = f'villager2_{256 * scale_factor}.png'
            overlay_img_path = os.path.join(overlay_folder, overlay_image_name)

            if os.path.exists(overlay_img_path):
                try:
                    overlay_img = Image.open(overlay_img_path).convert("RGBA")
                    villager2_img.paste(overlay_img, (0, 0), overlay_img)
                    log(f"Overlayed {overlay_image_name} onto temp_villager.png")
                except Exception as e:
                    log(f"Error overlaying image {overlay_image_name}: {e}")
                    traceback.print_exc()
                    return
            else:
                log(f"No overlay image found: {overlay_img_path}")

            # 新步骤1：覆盖颜色区域 (185,17)-(186,18) -> (186,24)-(208,39)
            try:
                # 定义源颜色区域 (185,17)-(186,18) scaled
                source_box = scaled_box(185, 17, 186, 18)  # (185,17)-(186,18), 1x1 像素

                # 获取源颜色
                source_color = villager2_img.getpixel((185 * scale_factor, 17 * scale_factor))
                log(f"Color at (185,17) scaled: {source_color}")

                # 定义覆盖区域 (186,24)-(208,39) scaled
                cover_box = scaled_box(186, 24, 208, 39)
                cover_x1, cover_y1, cover_x2, cover_y2 = cover_box

                cover_width = cover_x2 - cover_x1
                cover_height = cover_y2 - cover_y1

                # 创建一个单色图像用于覆盖
                cover_img = Image.new('RGBA', (cover_width, cover_height), source_color)

                # 粘贴覆盖图像到 villager2_img
                villager2_img.paste(cover_img, (cover_x1, cover_y1))
                log(f"Covered region {cover_box} in villager2_img with color {source_color}")
            except Exception as e:
                log(f"Error covering region (186,24)-(208,39) in 'villager2_img': {e}")
                traceback.print_exc()

            # 新步骤2：移动矩形区域 (133,48)-(242,76) 向上平移16个像素
            try:
                # 定义原始区域 (133,48)-(242,76) scaled
                original_region = scaled_box(133, 48, 242, 76)
                orig_x1, orig_y1, orig_x2, orig_y2 = original_region

                # 定义目标区域，向上平移16个像素 scaled
                shift_y = 16 * scale_factor
                target_position = (orig_x1, orig_y1 - shift_y)

                # 裁剪原始区域
                cropped_move = villager2_img.crop(original_region)

                # 粘贴到目标区域
                villager2_img.paste(cropped_move, target_position)
                log(f"Moved region {original_region} to {target_position} in villager2_img")
            except Exception as e:
                log(f"Error moving region (133,48)-(242,76) in 'villager2_img': {e}")
                traceback.print_exc()

            # 新步骤3：覆盖颜色区域 (132,60)-(133,61) -> (133,60)-(242,76)
            try:
                # 定义源颜色区域 (132,60)-(133,61) scaled
                source_box_2 = scaled_box(132, 60, 133, 61)  # (132,60)-(133,61), 1x1 像素

                # 获取源颜色
                source_color_2 = villager2_img.getpixel((132 * scale_factor, 60 * scale_factor))
                log(f"Color at (132,60) scaled: {source_color_2}")

                # 定义覆盖区域 (133,60)-(242,76) scaled
                cover_box_2 = scaled_box(133, 60, 242, 76)
                cover_x1_2, cover_y1_2, cover_x2_2, cover_y2_2 = cover_box_2

                cover_width_2 = cover_x2_2 - cover_x1_2
                cover_height_2 = cover_y2_2 - cover_y1_2

                # 创建一个单色图像用于覆盖
                cover_img_2 = Image.new('RGBA', (cover_width_2, cover_height_2), source_color_2)

                # 粘贴覆盖图像到 villager2_img
                villager2_img.paste(cover_img_2, (cover_x1_2, cover_y1_2))
                log(f"Covered region {cover_box_2} in villager2_img with color {source_color_2}")
            except Exception as e:
                log(f"Error covering region (133,60)-(242,76) in 'villager2_img': {e}")
                traceback.print_exc()

            # 新步骤4：将 (0,166)-(110,198) 的区域设置为全透明（根据 scale_factor 缩放）
            try:
                transparent_box = (
                    0 * scale_factor,
                    166 * scale_factor,
                    110 * scale_factor,
                    198 * scale_factor
                )
                log(f"Setting region {transparent_box} to fully transparent in villager2_img")
                # 创建一个全透明的图像
                transparent_region = Image.new('RGBA', (transparent_box[2] - transparent_box[0], 
                                                      transparent_box[3] - transparent_box[1]), 
                                               (0, 0, 0, 0))
                # 粘贴到指定区域
                villager2_img.paste(transparent_region, (transparent_box[0], transparent_box[1]))
                log(f"Region {transparent_box} set to transparent")
            except Exception as e:
                log(f"Error setting transparency in 'villager2_img': {e}")
                traceback.print_exc()

            # 新步骤5：在保存前处理 anvil.png 并粘贴到 villager.png（根据 scale_factor 缩放）
            try:
                anvil_path = os.path.join(container_path, 'anvil.png')
                if os.path.exists(anvil_path):
                    # 打开 anvil.png 并等比缩放到与 villager2_img 相同的尺寸
                    anvil_img = Image.open(anvil_path).convert("RGBA")
                    if anvil_img.size != villager2_img.size:
                        anvil_img = anvil_img.resize(villager2_img.size, Image.Resampling.LANCZOS)
                        log(f"Resized 'anvil.png' from {anvil_img.size} to {villager2_img.size}")

                    # 定义源区域 (176,0)-(204,21) scaled
                    source_box = (
                        176 * scale_factor,
                        0 * scale_factor,
                        204 * scale_factor,
                        21 * scale_factor
                    )
                    cropped_region = anvil_img.crop(source_box)
                    log(f"Cropped region {source_box} from 'anvil.png'")

                    # 定义目标位置 (176,0) scaled
                    target_position = (176 * scale_factor, 0 * scale_factor)

                    # 粘贴裁剪后的区域到 villager2_img
                    villager2_img.paste(cropped_region, target_position, cropped_region)
                    log(f"Pasted cropped region {source_box} from 'anvil.png' to {target_position} in 'villager2_img'")
                else:
                    log(f"No 'anvil.png' found in {container_path} for paste step")
            except Exception as e:
                log(f"Error processing and pasting from 'anvil.png': {e}")
                traceback.print_exc()

            # 备份原始 villager.png
            try:
                backup_villager_path = os.path.join(container_path, 'villager_backup.png')
                if not os.path.exists(backup_villager_path):
                    shutil.copy(villager_path, backup_villager_path)
                    log(f"Backup of original 'villager.png' created at {backup_villager_path}")
                else:
                    log(f"Backup already exists at {backup_villager_path}")
            except Exception as e:
                log(f"Error creating backup of 'villager.png': {e}")
                traceback.print_exc()

            # 保存生成的新的 villager.png，覆盖原有的 villager.png
            try:
                villager2_img.save(villager_path)
                log(f"Saved new 'villager.png' at {villager_path}")
            except Exception as e:
                log(f"Error saving new 'villager.png': {e}")
                traceback.print_exc()
                return
        else:
            log(f"No 'villager.png' found in {container_path}")

    except Exception as e:
        log(f"Error processing smithing2 villager2 image in '{container_path}': {e}")
        traceback.print_exc()