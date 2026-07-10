import os
from PIL import Image
import traceback

# 注意：log函数、adjust_hue_brightness函数需要从原文件中提取
def fix_sign_entities(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/entity/sign.png 文件，
    """
    try:
        # 定义相关路径
        entity_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "entity")
        sign_path = os.path.join(entity_path, "sign.png")
        
        if not os.path.exists(sign_path):
            log("No 'sign.png' found in entity textures, skipping entity sign processing.")
            return

        # 创建 signs 文件夹
        signs_folder = os.path.join(entity_path, "signs")
        os.makedirs(signs_folder, exist_ok=True)
        log(f"Ensured 'signs' folder exists at: {signs_folder}")
        
        log(f"Processing sign images in: {entity_path}")
        
        base_img = Image.open(sign_path).convert("RGBA")
        
        # 定义需要生成的签名类型及其调整参数
        sign_variants = [
            {'filename': 'oak.png', 'hue_shift': 0, 'brightness_shift': 15, 'saturation_shift': 0},
            {'filename': 'birch.png', 'hue_shift': 0, 'brightness_shift': 40, 'saturation_shift': 0},
            {'filename': 'acacia.png', 'hue_shift': -23, 'brightness_shift': 10, 'saturation_shift': 0},
            {'filename': 'dark_oak.png', 'hue_shift': 0, 'brightness_shift': -15, 'saturation_shift': 0},
            {'filename': 'jungle.png', 'hue_shift': -10, 'brightness_shift': 4.6, 'saturation_shift': 0},
            {'filename': 'crimson.png', 'hue_shift': -59, 'brightness_shift': -30, 'saturation_shift': 0},
            {'filename': 'warped.png', 'hue_shift': 130, 'brightness_shift': -33, 'saturation_shift': 0},
            {'filename': 'mangrove.png', 'hue_shift': -59, 'brightness_shift': -10, 'saturation_shift': 0},
            {'filename': 'pale_oak.png', 'hue_shift': 0, 'brightness_shift': 30, 'saturation_shift': -100},
            {'filename': 'bamboo.png', 'hue_shift': 25, 'brightness_shift': 20, 'saturation_shift': 0},
            {'filename': 'cherry.png', 'hue_shift': -80, 'brightness_shift': 20, 'saturation_shift': 0}
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
            output_path = os.path.join(signs_folder, filename)
            adjusted_img.save(output_path)
            log(f"Generated '{filename}' in signs folder with hue_shift={hue_shift}, brightness_shift={brightness_shift}, saturation_shift={saturation_shift}")

        # 删除原始的 sign.png 文件
        os.remove(sign_path)
        log("Removed original 'sign.png' from entity textures")

    except Exception as e:
        log(f"Error processing entity sign images: {e}")
        traceback.print_exc()