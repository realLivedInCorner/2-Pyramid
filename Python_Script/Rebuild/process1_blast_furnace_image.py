import os
import traceback
from PIL import Image

def process1_blast_furnace_image(temp_dir):
    log(f"Processing blast_furnace image in: {temp_dir}")
    try:
        blast_furnace_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','blast_furnace.png')

        if os.path.exists(blast_furnace_path):
            img = Image.open(blast_furnace_path).convert("RGBA")
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
                log(f"Unsupported image size for 'blast_furnace.png': {width}x{height}")
                return

            log(f"Processing blast_furnace.png, size: {width}x{height}, scale_factor: {scale_factor}")

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (176, 0, 190, 14),
                    'save_name': 'lit_progress.png',
                },
                {
                    'crop': (176, 14, 200, 31),
                    'save_name': 'burn_progress.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'blast_furnace')

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

        else:
            log(f"No 'blast_furnace.png' found in {temp_dir}")
    except Exception as e:
        log(f"Error processing blast_furnace image in '{temp_dir}': {e}")
        traceback.print_exc()
