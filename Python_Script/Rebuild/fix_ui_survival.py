import os
import sys
from PIL import Image
import traceback

# 注意：log函数、move_region函数、color_fill_region函数、copy_and_paste_region函数需要从原文件中提取
def fix_ui_survival(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/gui/inventory.png 文件，
    """
    try:
        # 定义相关路径
        gui_path = os.path.join(temp_dir, "assets/minecraft/textures/gui/container")
        inventory_path = os.path.join(gui_path, 'inventory.png')
        
        log(f"Processing inventory image in: {gui_path}")
        
        if os.path.exists(inventory_path):
            log(f"Found 'inventory.png' at {inventory_path}")
            img = Image.open(inventory_path).convert("RGBA")
            width, height = img.size
            log(f"Image size: {width}x{height}")

            # Determine scale factor based on image size
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'inventory.png': {width}x{height}")
                return

            # Adjust mob effect image size based on the scale_factor
            image_width = 18 * scale_factor  # Each mob effect image is adjusted based on the scale_factor
            image_height = 18 * scale_factor

            def scaled_coords(x1, y1, x2, y2):
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            def scaled_point(x, y):
                return (x * scale_factor, y * scale_factor)

            # Step 1: Extract the mob effect region from (0,198) to (144,254)
            effect_box = scaled_coords(0, 198, 144, 254)
            effect_region = img.crop(effect_box)
            log(f"Extracted mob effect region from {effect_box}")

            # Step 2: Define the effect image names for each row
            mob_effect_images = [
                # First row
                ["speed.png", "slowness.png", "haste.png", "mining_fatigue.png", "strength.png", "weakness.png", "poison.png", "regeneration.png"],
                # Second row
                ["invisibility.png", "hunger.png", "jump_boost.png", "nausea.png", "night_vision.png", "blindness.png", "resistance.png", "fire_resistance.png"],
                # Third row
                ["water_breathing.png", "wither.png", "absorption.png"]
            ]

            # Step 3: Set up mob effect path
            mob_effect_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'mob_effect')

            log(f"Saving mob effect images to: {mob_effect_path}")

            # Create the output folder for mob effect images if it doesn't exist
            if not os.path.exists(mob_effect_path):
                os.makedirs(mob_effect_path)
                log(f"Created folder for mob effect images: {mob_effect_path}")
            
            # Step 4: Loop through each effect and save it as individual images
            for row_idx, row in enumerate(mob_effect_images):
                for col_idx, effect_image_name in enumerate(row):
                    # Calculate position of each image in the effect region
                    x_offset = col_idx * image_width
                    y_offset = row_idx * image_height
                    effect_img = effect_region.crop((x_offset, y_offset, x_offset + image_width, y_offset + image_height))

                    # Save the image to the mob effect folder
                    effect_img.save(os.path.join(mob_effect_path, effect_image_name))
                    log(f"Saved {effect_image_name} in {mob_effect_path}")

            # Step 5: Process the rest of the inventory image as needed (moving, color filling, etc.)
            # Move regions
            move_region(img, *scaled_coords(86, 24, 162, 62), *scaled_point(10, -8))

            # Color fill regions
            color_fill_region(img, *scaled_coords(75, 6, 96, 80), *scaled_point(90, 10))
            color_fill_region(img, *scaled_coords(96, 54, 161, 62), *scaled_point(90, 10))

            # Copy and paste regions
            copy_and_paste_region(img, *scaled_coords(152, 26, 172, 46), *scaled_point(75, 60))


            # ===== 新增步骤：根据尺寸覆盖 'inventory.png' =====

            # 获取脚本或exe的路径
            if getattr(sys, 'frozen', False):
                exe_folder = os.path.dirname(sys.executable)
            else:
                exe_folder = os.path.dirname(os.path.abspath(__file__))
            inventory_folder = os.path.join(exe_folder, 'inventory')

            log(f"Looking for inventory size files in: {inventory_folder}")

            # 根据尺寸选择对应的 inventory_xxx.png 文件名
            if scale_factor == 1:
                inventory_size_file = 'inventory_256.png'
            elif scale_factor == 2:
                inventory_size_file = 'inventory_512.png'
            elif scale_factor == 4:
                inventory_size_file = 'inventory_1024.png'
            elif scale_factor == 8:
                inventory_size_file = 'inventory_2048.png'
            else:
                # 这一步实际上不会被执行，因为之前已经检查了尺寸
                log(f"No corresponding inventory_xxx.png for scale_factor: {scale_factor}")
                return

            # 构建要覆盖的文件路径
            inventory_size_path = os.path.join(inventory_folder, inventory_size_file)

            # 检查对应的 inventory_xxx.png 是否存在，并进行覆盖操作
            if os.path.exists(inventory_size_path):
                try:
                    # 打开 'inventory_xxx.png'
                    overlay_img = Image.open(inventory_size_path).convert("RGBA")
                    
                    # 检查尺寸是否匹配
                    if overlay_img.size != img.size:
                        log(f"Resizing '{inventory_size_file}' from {overlay_img.size} to {img.size}")
                        overlay_img = overlay_img.resize(img.size, Image.ANTIALIAS)
                    
                    # 将 overlay_img 叠加到 img 上
                    img = Image.alpha_composite(img, overlay_img)
                    log(f"Overlayed '{inventory_size_file}' onto 'inventory.png'")
                    
                    # 保存叠加后的图片
                    img.save(inventory_path)
                    log(f"Saved the updated 'inventory.png' with overlay in {gui_path}")
                except Exception as e:
                    log(f"Failed to overlay '{inventory_size_file}' onto 'inventory.png': {e}")
            else:
                log(f"Expected '{inventory_size_file}' not found in '{inventory_folder}'")

        else:
            log(f"No 'inventory.png' found in {gui_path}")

    except Exception as e:
        log(f"Error processing image '{inventory_path}': {e}")
        traceback.print_exc()