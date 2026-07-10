import os
import shutil
from PIL import Image
import traceback

# 注意：log函数需要从原文件中提取
def generate_potion_lingering(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/items/potion.png 和
    """
    try:
        items_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "items")
        log(f"Processing potion images in: {items_path}")

        # 定义需要处理的图片及其对应的输出名称
        images_to_process = [
            ('potion.png', 'lingering_potion.png'),
            ('potion_bottle_drinkable.png', 'potion_bottle_lingering.png')
        ]

        for original_name, new_name in images_to_process:
            original_path = os.path.join(items_path, original_name)
            new_path = os.path.join(items_path, new_name)
            mcmeta_original = original_path + '.mcmeta'
            mcmeta_new = new_path + '.mcmeta'

            if os.path.exists(original_path):
                log(f"Found '{original_name}', processing...")
                # 复制原始图片到新的文件名
                shutil.copy(original_path, new_path)
                img = Image.open(new_path).convert("RGBA")

                width, height = img.size
                log(f"Image size: {width}x{height}")

                # 判断图像是否为正方形
                if width == height:
                    log(f"'{original_name}' is a square image. Processing entire image.")
                    # 将上三分之一部分设为透明
                    new_image_data = []
                    for y in range(height):
                        for x in range(width):
                            pixel = img.getpixel((x, y))
                            if y < height // 3:
                                new_image_data.append((0, 0, 0, 0))  # 上三分之一透明
                            else:
                                new_image_data.append(pixel)

                    new_image = Image.new("RGBA", img.size)
                    new_image.putdata(new_image_data)
                    new_image.save(new_path)
                    log(f"Processed '{original_name}' to '{new_name}' by making the top third transparent.")

                else:
                    # 检查高度是否是宽度的整数倍，判断是否为多个正方形垂直拼接
                    if height % width == 0:
                        num_squares = height // width
                        log(f"'{original_name}' is a vertically stacked image with {num_squares} squares.")
                        new_image = Image.new("RGBA", (width, height))

                        for i in range(num_squares):
                            top = i * width
                            bottom = top + width
                            box = (0, top, width, bottom)
                            square = img.crop(box)

                            # 处理单个正方形：将上三分之一设为透明
                            square_data = []
                            for y in range(width):
                                for x in range(width):
                                    pixel = square.getpixel((x, y))
                                    if y < width // 3:
                                        square_data.append((0, 0, 0, 0))  # 上三分之一透明
                                    else:
                                        square_data.append(pixel)

                            processed_square = Image.new("RGBA", (width, width))
                            processed_square.putdata(square_data)
                            new_image.paste(processed_square, (0, top))
                            log(f"Processed square {i+1}/{num_squares} in '{original_name}'.")

                        new_image.save(new_path)
                        log(f"Processed '{original_name}' to '{new_name}' with {num_squares} squares by making the top third of each square transparent.")

                    else:
                        log(f"'{original_name}' is neither a square nor a vertically stacked image of squares. Skipping processing.")
                        continue  # 跳过不符合条件的图像

                # 处理对应的 .mcmeta 文件
                if os.path.exists(mcmeta_original):
                    shutil.copy(mcmeta_original, mcmeta_new)
                    log(f"Copied and renamed '{original_name}.mcmeta' to '{new_name}.mcmeta'.")
                else:
                    log(f"No '{original_name}.mcmeta' found in '{items_path}'. Skipping .mcmeta processing.")

            else:
                log(f"No '{original_name}' found in {items_path}. Skipping this image.")

    except Exception as e:
        log(f"Error processing potion images: {e}")
        traceback.print_exc()