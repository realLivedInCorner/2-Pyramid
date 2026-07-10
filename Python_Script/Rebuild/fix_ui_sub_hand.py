import os
from PIL import Image
import traceback

# 注意：log函数、determine_scale_factor函数需要从原文件中提取
def fix_ui_sub_hand(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/gui/widgets.png 文件。
    """
    widgets_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "gui", "widgets.png")
    log(f"Processing widgets image: {widgets_path}")
    try:
        if os.path.exists(widgets_path):
            log(f"Found 'widgets.png' at {widgets_path}")
            img = Image.open(widgets_path).convert("RGBA")
            width, height = img.size
            log(f"Image size: {width}x{height}")

            # 确定 scale_factor
            scale_factor, is_exact = determine_scale_factor(width, height)
            log(f"Determined scale_factor: {scale_factor} (Exact match: {is_exact})")

            def scaled_coords(x1, y1, x2, y2):
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            def scaled_point(x, y):
                return (x * scale_factor, y * scale_factor)

            # 1. 复制 (1,23)-(23,45) 到 (24,23)-(46,45) 和 (60,23)-(82,45)
            source_box = scaled_coords(1, 23, 23, 45)
            dest_box_1 = scaled_coords(24, 23, 46, 45)
            dest_box_2 = scaled_coords(60, 23, 82, 45)

            region = img.crop(source_box)
            img.paste(region, dest_box_1)
            log(f"Copied region {source_box} to {dest_box_1}")
            img.paste(region, dest_box_2)
            log(f"Copied region {source_box} to {dest_box_2}")

            # 保存修改后的 widgets.png
            img.save(widgets_path)
            log(f"Processed 'widgets.png' and saved the modified image")
        else:
            log(f"No 'widgets.png' found in {os.path.dirname(widgets_path)}")
    
    except Exception as e:
        log(f"Error processing image '{widgets_path}': {e}")
        traceback.print_exc()