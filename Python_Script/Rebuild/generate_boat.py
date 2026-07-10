import os
from PIL import Image
import traceback

# 注意：log函数、adjust_hue_brightness函数需要从原文件中提取
def generate_boat(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/items/boat.png 文件，
    """
    items_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "items")
    boat_path = os.path.join(items_path, "boat.png")
    
    if not os.path.exists(boat_path):
        log("No 'boat.png' found, skipping boat processing.")
        return

    try:
        base_img = Image.open(boat_path).convert("RGBA")
        log(f"Opened 'boat.png' with size {base_img.size}")

        # oak_boat.png（亮度+15）
        oak_img = adjust_hue_brightness(base_img, brightness_shift=15)
        oak_boat_path = os.path.join(items_path, "oak_boat.png")
        oak_img.save(oak_boat_path)
        log(f"Generated 'oak_boat.png' with brightness +15")

        # birch_boat.png（亮度+40）
        birch_img = adjust_hue_brightness(base_img, brightness_shift=40)
        birch_boat_path = os.path.join(items_path, "birch_boat.png")
        birch_img.save(birch_boat_path)
        log(f"Generated 'birch_boat.png' with brightness +40")

        # acacia_boat.png（色相-23度, 亮度+10）
        acacia_img = adjust_hue_brightness(base_img, hue_shift=-23, brightness_shift=10)
        acacia_boat_path = os.path.join(items_path, "acacia_boat.png")
        acacia_img.save(acacia_boat_path)
        log(f"Generated 'acacia_boat.png' with hue -23 and brightness +10")

        # dark_oak_boat.png（亮度-15）
        dark_oak_img = adjust_hue_brightness(base_img, brightness_shift=-15)
        dark_oak_boat_path = os.path.join(items_path, "dark_oak_boat.png")
        dark_oak_img.save(dark_oak_boat_path)
        log(f"Generated 'dark_oak_boat.png' with brightness -15")

        # jungle_boat.png（色相-10度, 亮度+4.6）
        jungle_img = adjust_hue_brightness(base_img, hue_shift=-10, brightness_shift=4.6)
        jungle_boat_path = os.path.join(items_path, "jungle_boat.png")
        jungle_img.save(jungle_boat_path)
        log(f"Generated 'jungle_boat.png' with hue -10 and brightness +4.6")

        # 将 boat.png 重命名为 spruce_boat.png
        spruce_path = os.path.join(items_path, "spruce_boat.png")
        if os.path.exists(spruce_path):
            os.remove(spruce_path)
            log(f"Removed existing 'spruce_boat.png'")
        os.rename(boat_path, spruce_path)
        log("Renamed 'boat.png' to 'spruce_boat.png'")

    except Exception as e:
        log(f"Error processing boat images: {e}")
        traceback.print_exc()