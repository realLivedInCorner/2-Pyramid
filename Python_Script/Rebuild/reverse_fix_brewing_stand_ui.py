import os
from PIL import Image
import traceback

# 注意：此函数依赖于外部的log函数

def reverse_fix_brewing_stand_ui(temp_dir):
    container_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container')
    log(f"Reversing brewing stand image in: {container_path}")
    try:
        brewing_stand_path = os.path.join(container_path, 'brewing_stand.png')

        if os.path.exists(brewing_stand_path):
            img = Image.open(brewing_stand_path).convert("RGBA")
            width, height = img.size

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
                log(f"Unsupported image size for 'brewing_stand.png': {width}x{height}")
                return

            # Step 1: Get the color from (7, 4) to fill regions
            fill_color = img.getpixel((7 * scale_factor, 4 * scale_factor))

            # Step 2: Fill (41, 43) to (79, 49) region with the color
            fill_box1 = (41 * scale_factor, 43 * scale_factor, 79 * scale_factor, 49 * scale_factor)
            for x in range(fill_box1[0], fill_box1[2]):
                for y in range(fill_box1[1], fill_box1[3]):
                    img.putpixel((x, y), fill_color)

            # Step 3: Fill (14, 14) to (55, 43) region with the color
            fill_box2 = (14 * scale_factor, 14 * scale_factor, 55 * scale_factor, 43 * scale_factor)
            for x in range(fill_box2[0], fill_box2[2]):
                for y in range(fill_box2[1], fill_box2[3]):
                    img.putpixel((x, y), fill_color)

            # Step 4: Move the region (55, 50) to (119, 75) upwards by 5 pixels
            move_box = (55 * scale_factor, 50 * scale_factor, 119 * scale_factor, 75 * scale_factor)
            region_to_move = img.crop(move_box)
            img.paste(region_to_move, (55 * scale_factor, 45 * scale_factor))

            # Step 5: Fill (55, 70) to (119, 75) region with the color
            fill_box3 = (55 * scale_factor, 70 * scale_factor, 119 * scale_factor, 75 * scale_factor)
            for x in range(fill_box3[0], fill_box3[2]):
                for y in range(fill_box3[1], fill_box3[3]):
                    img.putpixel((x, y), fill_color)

            # Save the processed image
            img.save(brewing_stand_path)
            log(f"Reversed 'brewing_stand.png' in {container_path}")
        else:
            log(f"No 'brewing_stand.png' found in {container_path}")
    except Exception as e:
        log(f"Error reversing brewing stand image in '{container_path}': {e}")
        traceback.print_exc()