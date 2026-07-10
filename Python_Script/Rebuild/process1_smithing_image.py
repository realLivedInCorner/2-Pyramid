import os
import traceback
from PIL import Image

def process1_smithing_image(temp_dir):
    try:
        smithing_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','smithing.png')

        if os.path.exists(smithing_path):
            img = Image.open(smithing_path).convert("RGBA")
            width, height = img.size

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
                log(f"Unsupported image size for 'smithing.png': {width}x{height}")
                return

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (176, 0, 204, 21),
                    'save_name': 'error.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'smithing')

            # 确保目标目录存在
            if not os.path.exists(target_dir):
                try:
                    os.makedirs(target_dir)
                    log(f"Created directory: {target_dir}")
                except Exception as e:
                    log(f"Error creating directory {target_dir}: {e}")
                    return

            # 执行裁剪和保存操作
            for op_index, region in enumerate(crop_regions, start=1):
                scaled_region = scaled_box(*region['crop'])
                try:
                    cropped_img = img.crop(scaled_region)
                    save_path = os.path.join(target_dir, region['save_name']).replace("\\", "/")  # 确保路径使用正斜杠
                    cropped_img.save(save_path, format='PNG')
                    log(f"[Operation {op_index}] Saved {save_path}, size: {cropped_img.size}")
                except Exception as e:
                    log(f"[Operation {op_index}] Error saving {region['save_name']}: {e}")

    except Exception as e:
        traceback.print_exc()
