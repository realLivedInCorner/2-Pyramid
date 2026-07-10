import os
import traceback

# 注意：log函数、split_image函数需要从原文件中提取
def fix_clock_compass(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/items/clock.png 和 compass.png 文件，
    """
    try:
        assets_path = os.path.join(temp_dir, "assets", "minecraft", "textures")
        items_path_old = os.path.join(assets_path, 'items')
        clock_path = os.path.join(items_path_old, 'clock.png')
        compass_path = os.path.join(items_path_old, 'compass.png')
        
        log(f"Processing clock and compass images in: {items_path_old}")
        
        # 处理 clock.png
        if os.path.exists(clock_path):
            log(f"Found 'clock.png' at {clock_path}, processing...")
            split_image(clock_path, items_path_old, 'clock', 64)
            log(f"Successfully split 'clock.png' into clock images.")
        else:
            log(f"No 'clock.png' found in {items_path_old}. Skipping clock processing.")
        
        # 处理 compass.png
        if os.path.exists(compass_path):
            log(f"Found 'compass.png' at {compass_path}, processing...")
            split_image(compass_path, items_path_old, 'compass', 64)
            log(f"Successfully split 'compass.png' into compass images.")
        else:
            log(f"No 'compass.png' found in {items_path_old}. Skipping compass processing.")

    except Exception as e:
        log(f"Error processing clock and compass images: {e}")
        traceback.print_exc()