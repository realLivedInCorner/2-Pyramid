import os
from PIL import Image
import traceback

# 注意：log函数、determine_scale_factor函数需要从原文件中提取
def fix_ui_creative(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/gui/container/creative_inventory/tab_inventory.png 文件。
    """
    try:
        # 定义相关路径
        creative_inventory_path = os.path.join(
            temp_dir, 
            "assets", 
            "minecraft", 
            "textures", 
            "gui", 
            "container", 
            "creative_inventory", 
            "tab_inventory.png"
        )
        
        log(f"Processing creative inventory image: {creative_inventory_path}")
        
        if os.path.exists(creative_inventory_path):
            log(f"Found 'tab_inventory.png' at {creative_inventory_path}")
            img = Image.open(creative_inventory_path).convert("RGBA")
            width, height = img.size
            log(f"Image size: {width}x{height}")

            # 确定 scale_factor
            scale_factor, is_exact = determine_scale_factor(width, height)
            log(f"Determined scale_factor: {scale_factor} (Exact match: {is_exact})")

            def scaled_coords(x1, y1, x2, y2):
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            def scaled_point(x, y):
                return (x * scale_factor, y * scale_factor)

            # 1. 复制 (6,0)-(84,53) 到 (51,0)-(129,53)
            source_box = scaled_coords(6, 0, 84, 53)
            dest_box = scaled_coords(51, 0, 129, 53)
            region = img.crop(source_box)
            img.paste(region, dest_box)
            log(f"Copied region {source_box} to {dest_box}")

            # 2. 填充 (6,0)-(53,53) 区域的颜色
            fill_box = scaled_coords(6, 0, 53, 53)
            fill_color = img.getpixel(scaled_point(164, 27))
            log(f"Filling region {fill_box} with color {fill_color}")
            for x in range(fill_box[0], fill_box[2]):
                for y in range(fill_box[1], fill_box[3]):
                    img.putpixel((x, y), fill_color)

            # 3. 复制并粘贴 (53,5)-(71,23) 到 (34,19)-(52,37)
            source_box_18x18 = scaled_coords(53, 5, 71, 23)
            dest_position = scaled_coords(34, 19, 52, 37)[:2]  # (34,19)
            region_18x18 = img.crop(source_box_18x18)
            img.paste(region_18x18, dest_position)
            log(f"Pasted region {source_box_18x18} to {dest_position}")

            # 保存生成的 tab_inventory.png
            img.save(creative_inventory_path)
            log(f"Processed 'tab_inventory.png' and saved the modified image")
        else:
            log(f"No 'tab_inventory.png' found in {os.path.dirname(creative_inventory_path)}")
    
    except Exception as e:
        log(f"Error processing image '{creative_inventory_path}': {e}")
        traceback.print_exc()