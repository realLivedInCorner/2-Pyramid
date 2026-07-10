import os
from PIL import Image
import traceback

# 注意：log函数、adjust_hue_brightness函数需要从原文件中提取
def fix_sign(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/item/sign.png 文件，
    """
    try:
        # 定义相关路径
        item_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "item")
        sign_path = os.path.join(item_path, "oak_sign.png")
        
        if not os.path.exists(sign_path):
            log("No 'sign.png' found, skipping sign processing.")
            return

        log(f"Processing sign images in: {item_path}")

        # 将 oak_sign.png 重命名为 spruce_sign.png
        spruce_path = os.path.join(item_path, "spruce_sign.png")
        if os.path.exists(spruce_path):
            os.remove(spruce_path)
            log(f"Removed existing 'spruce_sign.png'")
        os.rename(sign_path, spruce_path)
        log("Renamed sign.png to spruce_sign.png")
                
        base_img = Image.open(spruce_path).convert("RGBA")

        # 定义需要生成的签名类型及其调整参数
        sign_variants = [
            {'filename': 'oak_sign.png', 'hue_shift': 0, 'brightness_shift': 15, 'saturation_shift': 0},
            {'filename': 'birch_sign.png', 'hue_shift': 0, 'brightness_shift': 40, 'saturation_shift': 0},
            {'filename': 'acacia_sign.png', 'hue_shift': -23, 'brightness_shift': 10, 'saturation_shift': 0},
            {'filename': 'dark_oak_sign.png', 'hue_shift': 0, 'brightness_shift': -15, 'saturation_shift': 0},
            {'filename': 'jungle_sign.png', 'hue_shift': -10, 'brightness_shift': 4.6, 'saturation_shift': 0},
            {'filename': 'crimson_sign.png', 'hue_shift': -59, 'brightness_shift': -30, 'saturation_shift': 0},
            {'filename': 'warped_sign.png', 'hue_shift': 130, 'brightness_shift': -33, 'saturation_shift': 0},
            {'filename': 'mangrove_sign.png', 'hue_shift': -59, 'brightness_shift': -10, 'saturation_shift': 0},
            {'filename': 'pale_oak_sign.png', 'hue_shift': 0, 'brightness_shift': 30, 'saturation_shift': -100},
            {'filename': 'bamboo_sign.png', 'hue_shift': 25, 'brightness_shift': 20, 'saturation_shift': 0},
            {'filename': 'cherry_sign.png', 'hue_shift': -80, 'brightness_shift': 20, 'saturation_shift': 0}
        ]
        
        # 生成各类签名图像
        for variant in sign_variants:
            filename = variant['filename']
            hue_shift = variant['hue_shift']
            brightness_shift = variant['brightness_shift']
            saturation_shift = variant['saturation_shift']
            
            # 调整图像
            adjusted_img = adjust_hue_brightness(
                base_img, 
                hue_shift=hue_shift, 
                brightness_shift=brightness_shift,
                saturation_shift=saturation_shift
            )
            
            # 保存调整后的图像
            output_path = os.path.join(item_path, filename)
            adjusted_img.save(output_path)
            log(f"Generated '{filename}' with hue_shift={hue_shift}, brightness_shift={brightness_shift}, saturation_shift={saturation_shift}")

    except Exception as e:
        log(f"Error processing sign images: {e}")
        traceback.print_exc()