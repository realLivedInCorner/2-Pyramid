import os
from PIL import Image
import traceback

# 注意：此函数依赖于外部的log函数

def reverse_fix_ui_creative(temp_dir):
    creative_inventory_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container', 'creative_inventory', 'tab_inventory.png')
    log(f"Performing reverse operation on creative inventory image: {creative_inventory_path}")
    try:
        img = Image.open(creative_inventory_path).convert("RGBA")
        width, height = img.size

        if width == 256 and height == 256:
            scale_factor = 1
        elif width == 512 and height == 512:
            scale_factor = 2
        elif width == 1024 and height == 1024:
            scale_factor = 4
        elif width == 2048 and height == 2048:
            scale_factor = 8
        else:
            log(f"Unsupported image size for 'tab_inventory.png': {width}x{height}")
            return

        def scaled_coords(x1, y1, x2, y2):
            return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

        def scaled_point(x, y):
            return (x * scale_factor, y * scale_factor)

        # 1. Fill (34, 19, 52, 37) with the color at (164, 27)
        fill_color = img.getpixel(scaled_point(164, 27))
        for x in range(scaled_coords(34, 19, 52, 37)[0], scaled_coords(34, 19, 52, 37)[2]):
            for y in range(scaled_coords(34, 19, 52, 37)[1], scaled_coords(34, 19, 52, 37)[3]):
                img.putpixel((x, y), fill_color)

        # 2. Crop the region (51, 0, 129, 53) and paste it at (6, 0, 84, 53)
        source_box = scaled_coords(51, 0, 129, 53)
        dest_box = scaled_coords(6, 0, 84, 53)
        region = img.crop(source_box)
        img.paste(region, dest_box)

        # 3. Fill the region (84, 0, 129, 53) with the color at (164, 27)
        fill_box = scaled_coords(84, 0, 129, 53)
        for x in range(fill_box[0], fill_box[2]):
            for y in range(fill_box[1], fill_box[3]):
                img.putpixel((x, y), fill_color)

        # Save the modified image
        img.save(creative_inventory_path)
        log(f"Reverse operation completed and 'tab_inventory.png' has been saved.")
    except Exception as e:
        log(f"Error performing reverse operation on image '{creative_inventory_path}': {e}")
        traceback.print_exc()