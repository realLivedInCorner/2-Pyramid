import os
import shutil
from PIL import Image
import traceback

# 注意：log函数需要从原文件中提取
def generate_snow_bucket(temp_dir):
    try:
        milk_bucket_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'item', 'milk_bucket.png')
        if os.path.exists(milk_bucket_path):
            # Open milk_bucket.png and get its size
            milk_bucket_img = Image.open(milk_bucket_path).convert("RGBA")
            width, height = milk_bucket_img.size
            if width != height:
                log(f"'milk_bucket.png' is not a square image, skipping powder_snow_bucket processing.")
                return

            # Copy milk_bucket.png to powder_snow_bucket.png
            powder_snow_bucket_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'item', 'powder_snow_bucket.png')
            shutil.copy(milk_bucket_path, powder_snow_bucket_path)

            # Find the overlay image
            size = width  # since width == height
            overlay_filename = f"powder_snow_bucket_{size}.png"
            overlay_dir = os.path.join(os.getcwd(), 'powder_snow_bucket')
            overlay_path = os.path.join(overlay_dir, overlay_filename)

            if os.path.exists(overlay_path):
                # Open the overlay image
                overlay_img = Image.open(overlay_path).convert("RGBA")
                # Open the powder_snow_bucket.png image
                bucket_img = Image.open(powder_snow_bucket_path).convert("RGBA")

                # Overlay the images
                combined_img = Image.alpha_composite(bucket_img, overlay_img)
                combined_img.save(powder_snow_bucket_path)
                log(f"Processed 'powder_snow_bucket.png' with overlay '{overlay_filename}'")
            else:
                log(f"No overlay image found for size {size}")
        else:
            log(f"No 'milk_bucket.png' found in {temp_dir}")

    except Exception as e:
        log(f"Error processing snow bucket image: {e}")
        traceback.print_exc()