import os
import shutil
from PIL import Image
import traceback

# 注意：log函数需要从原文件中提取
def generate_fish_bucket(temp_dir):
    """
    处理解压后的目录中的 water_bucket.png，
    """
    log(f"Processing water bucket image in: {temp_dir}")
    try:
        items_path_new = os.path.join(temp_dir,'assets','minecraft','textures','item')
        water_bucket_path = os.path.join(temp_dir,'assets','minecraft','textures','item', 'water_bucket.png')
        if os.path.exists(water_bucket_path):
            # 打开 water_bucket.png 并获取其尺寸
            water_bucket_img = Image.open(water_bucket_path).convert("RGBA")
            width, height = water_bucket_img.size
            if width != height:
                log(f"'water_bucket.png' is not a square image ({width}x{height}), skipping water_bucket processing.")
                return

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                # 处理非标准尺寸，通过近似方法确定 scale_factor
                scale_factors = [1, 2, 4, 8]
                closest_scale_factor = min(scale_factors, key=lambda x: abs(x * 256 - width))
                scale_factor = closest_scale_factor
                log(f"Warning: Unsupported image size for 'water_bucket.png': {width}x{height}. Using scale_factor={scale_factor}")

            # 创建鱼桶图像
            fish_types = ['cod', 'salmon', 'pufferfish', 'tropical_fish']
            for fish_type in fish_types:
                fish_bucket_path = os.path.join(items_path_new, f'{fish_type}_bucket.png')
                shutil.copy(water_bucket_path, fish_bucket_path)
                log(f"Generated '{fish_type}_bucket.png' from 'water_bucket.png'")

    except Exception as e:
        log(f"Error processing water bucket image: {e}")
        traceback.print_exc()