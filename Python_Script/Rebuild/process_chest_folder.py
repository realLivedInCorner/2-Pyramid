import os
from PIL import Image
import traceback

# 注意：log函数、swap_and_mirror函数、generate_double_chest_images函数需要从原文件中提取
def process_chest_folder(temp_dir):
    chest_path= os.path.join(temp_dir, "assets", "minecraft", "textures", "entity","chest")
    chest_files = ['ender.png', 'normal.png', 'trapped.png', 'christmas.png','normal_double.png', 'christmas_double.png','trapped_double.png']
    
    for chest_file in chest_files:
        chest_file_path = os.path.join(chest_path, chest_file)
        if os.path.exists(chest_file_path):
            try:
                img = Image.open(chest_file_path).convert("RGBA")
                width, height = img.size
                log(f"Processing '{chest_file}' with size: {width}x{height}")

                # Determine if the chest is single or double
                if chest_file in ['ender.png', 'normal.png', 'trapped.png', 'christmas.png']:
                    # Single chest image size determination
                    if width == 64 and height == 64:
                        scale_factor = 1
                    elif width == 128 and height == 128:
                        scale_factor = 2
                    elif width == 256 and height == 256:
                        scale_factor = 4
                    elif width == 512 and height == 512:
                        scale_factor = 8
                    elif width == 1024 and height == 1024:
                        scale_factor = 16
                    else:
                        log(f"Unsupported image size for '{chest_file}': {width}x{height}")
                        continue

                    def scaled_box(x1, y1, x2, y2):
                        return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

                    # Process single chests
                    swap_and_mirror(img, scaled_box(14, 0, 28, 14), scaled_box(28, 0, 42, 14))
                    swap_and_mirror(img, scaled_box(14, 14, 28, 19), scaled_box(42, 14, 56, 19))
                    swap_and_mirror(img, scaled_box(14, 19, 28, 33), scaled_box(28, 19, 42, 33))
                    swap_and_mirror(img, scaled_box(14, 33, 28, 43), scaled_box(42, 33, 56, 43))

                    mirror_boxes = [
                        scaled_box(14, 0, 28, 14), scaled_box(28, 0, 42, 14),
                        scaled_box(0, 14, 14, 19), scaled_box(28, 14, 42, 19), 
                        scaled_box(14, 19, 28, 33), scaled_box(28, 19, 42, 33),
                        scaled_box(0, 33, 14, 43), scaled_box(28, 33, 42, 43)
                    ]
                    for box in mirror_boxes:
                        region = img.crop(box).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                        img.paste(region, box)

                    img.save(chest_file_path)
                    log(f"Processed '{chest_file}' and swapped and mirrored specified regions.")

                elif chest_file in ['normal_double.png', 'trapped_double.png', 'christmas_double.png']:
                    # Double chest image size determination
                    if width == 128 and height == 64:
                        scale_factor = 1
                    elif width == 256 and height == 128:
                        scale_factor = 2
                    elif width == 512 and height == 256:
                        scale_factor = 4
                    elif width == 1024 and height == 512:
                        scale_factor = 8
                    else:
                        log(f"Unsupported image size for '{chest_file}': {width}x{height}")
                        continue
                    
                    def scaled_box(x1, y1, x2, y2):
                        return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

                    # Create transparent images for left and right parts based on scale_factor
                    left_img_size = (64 * scale_factor, 64 * scale_factor)
                    right_img_size = (64 * scale_factor, 64 * scale_factor)
                    left_img = Image.new("RGBA", left_img_size, (0, 0, 0, 0))
                    right_img = Image.new("RGBA", right_img_size, (0, 0, 0, 0))

                    # Determine the prefix based on the chest type
                    prefix = 'christmas' if 'christmas' in chest_file else ('normal' if 'normal' in chest_file else 'trapped')

                    # Generate double chest images
                    generate_double_chest_images(left_img, right_img, prefix, img, scaled_box, scale_factor)

                    # Save the processed left and right images with the correct prefix
                    left_img.save(os.path.join(chest_path, f"{prefix}_left.png"))
                    right_img.save(os.path.join(chest_path, f"{prefix}_right.png"))
                    log(f"Processed '{chest_file}' and saved '{prefix}_left.png' and '{prefix}_right.png'.")

            except Exception as e:
                log(f"Error processing '{chest_file}': {e}")
                continue

    log("Chest images processing completed.")