import os
from PIL import Image
import traceback

# 注意：此函数依赖于外部的log函数、color_fill_region函数和move_region函数

def reverse_fix_ui_survival(temp_dir):
    log(f"Reversing inventory image in: {temp_dir}")
    inventory_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container', 'inventory.png')
    mob_effect_path= os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'mob_effect' )
    if os.path.exists(inventory_path):
        img = Image.open(inventory_path).convert("RGBA")
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
            log(f"Unsupported image size for 'inventory.png': {width}x{height}")
            return

        def scaled_coords(x1, y1, x2, y2):
            return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

        def scaled_point(x, y):
            return (x * scale_factor, y * scale_factor)

        # Step 1: Set (0, 198) to (144, 254) region to transparent
        transparent_box = scaled_coords(0, 198, 144, 254)
        log(f"Setting region {transparent_box} to transparent")
        for x in range(transparent_box[0], transparent_box[2]):
            for y in range(transparent_box[1], transparent_box[3]):
                img.putpixel((x, y), (0, 0, 0, 0))  # Set pixel to fully transparent

        # Step 2: Load images from mob_effect_path if provided and folder exists
        if mob_effect_path:
            mob_effect_folder = mob_effect_path
            if os.path.exists(mob_effect_folder):
                log(f"Mob effect folder found: {mob_effect_folder}. Proceeding with steps 2 and 3.")
                
                mob_effect_images = [
                    "speed.png", "slowness.png", "haste.png", "mining_fatigue.png", "strength.png",
                    "weakness.png", "poison.png", "regeneration.png",
                    "invisibility.png", "hunger.png", "jump_boost.png", "nausea.png", "night_vision.png",
                    "blindness.png", "resistance.png", "fire_resistance.png",
                    "water_breathing.png", "wither.png", "absorption.png"
                ]

                # Step 3: Place each mob effect image in the corresponding position
                image_width, image_height = 18 * scale_factor, 18 * scale_factor  # Adjusted based on scale_factor
                log(f"Pasting mob effect images onto 'inventory.png'")

                for i, effect_image in enumerate(mob_effect_images):
                    effect_image_path = os.path.join(mob_effect_folder, effect_image)
                    if os.path.exists(effect_image_path):
                        try:
                            effect_img = Image.open(effect_image_path).convert("RGBA")
                            # Resize the effect image if necessary
                            if effect_img.size != (image_width, image_height):
                                log(f"Resizing '{effect_image}' from {effect_img.size} to ({image_width}, {image_height})")
                                effect_img = effect_img.resize((image_width, image_height), Image.ANTIALIAS)

                            row = i // 8  # Determine the row (0, 1, or 2)
                            col = i % 8   # Determine the column (0 to 7)
                            
                            # Calculate position for each image in the scaled region
                            x_offset = col * image_width
                            y_offset = row * image_height
                            position = (transparent_box[0] + x_offset, transparent_box[1] + y_offset)

                            img.paste(effect_img, position, effect_img)  # Paste with transparency mask
                            log(f"Pasted '{effect_image}' at position {position}")
                        except Exception as e:
                            log(f"Failed to process '{effect_image}': {e}")
                    else:
                        log(f"Image '{effect_image}' not found in '{mob_effect_folder}'")
            else:
                log(f"Mob effect folder '{mob_effect_folder}' does not exist. Skipping steps 2 and 3.")
        else:
            log("No 'mob_effect_path' provided. Skipping steps 2 and 3.")

        # Step 4: Process custom color fill and region adjustments
        # Fill regions with (90, 10) color
        log("Filling specified regions with color")
        color_fill_region(img, *scaled_coords(76, 61, 94, 79), *scaled_point(90, 10))

        # Reverse translation (scale back the regions)
        log("Moving specified regions")
        move_region(img, *scaled_coords(96, 16, 172, 54), *scaled_point(-10, 8))

        # Additional color fill for specific regions
        log("Filling additional specified regions with color")
        color_fill_region(img, *scaled_coords(96, 16, 172, 25), *scaled_point(90, 10))
        color_fill_region(img, *scaled_coords(161, 25, 172, 54), *scaled_point(90, 10))

        # Save the processed image
        img.save(inventory_path)
        log(f"Reversed 'inventory.png' in {temp_dir}")
    else:
        log(f"No 'inventory.png' found in {temp_dir}")