import os
from PIL import Image
import traceback

# 注意：此函数依赖于外部的log函数和swap_and_mirror函数

def reverse_process_chest_folder(temp_dir):
    """
    逆向处理单个箱子图像，将它们恢复到原始状态。
    """
    chest_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'entity', 'chest')
    log(f"Reversing processing of chest images in: {chest_path}")
    single_chest_files = ['ender.png', 'normal.png', 'trapped.png', 'christmas.png']
    
    for chest_file in single_chest_files:
        chest_file_path = os.path.join(chest_path, chest_file)
        if os.path.exists(chest_file_path):
            try:
                img = Image.open(chest_file_path).convert("RGBA")
                width, height = img.size
                log(f"Reversing '{chest_file}' with size: {width}x{height}")

                # Determine scale_factor based on image size
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

                # 定义需要交换和镜像的区域
                swap_and_mirror_boxes = [
                    (scaled_box(14, 0, 28, 14), scaled_box(28, 0, 42, 14)),
                    (scaled_box(14, 14, 28, 19), scaled_box(42, 14, 56, 19)),
                    (scaled_box(14, 19, 28, 33), scaled_box(28, 19, 42, 33)),
                    (scaled_box(14, 33, 28, 43), scaled_box(42, 33, 56, 43))
                ]

                # 逆向交换和镜像操作
                for box1, box2 in swap_and_mirror_boxes:
                    swap_and_mirror(img, box1, box2)

                # 定义需要镜像的区域
                mirror_boxes = [
                    scaled_box(14, 0, 28, 14), scaled_box(28, 0, 42, 14),
                    scaled_box(0, 14, 14, 19), scaled_box(28, 14, 42, 19), 
                    scaled_box(14, 19, 28, 33), scaled_box(28, 19, 42, 33),
                    scaled_box(0, 33, 14, 43), scaled_box(28, 33, 42, 43)
                ]

                # 逆向镜像操作（再次应用镜像以恢复原始）
                for box in mirror_boxes:
                    region = img.crop(box).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                    img.paste(region, box)

                img.save(chest_file_path)
                log(f"Reversed processing of '{chest_file}' successfully.")

            except Exception as e:
                log(f"Error reversing '{chest_file}': {e}")
                continue
        else:
            log(f"File '{chest_file}' does not exist in the path '{chest_path}'. Skipping.")

    log("Reversing chest images processing completed.")