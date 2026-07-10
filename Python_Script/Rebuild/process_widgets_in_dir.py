import os
from PIL import Image
import traceback

# 注意：此函数依赖于外部的log函数

def process_widgets_in_dir(temp_dir):
    """
    处理解压后的目录中的assets/minecraft/textures/gui/widgets.png，裁剪并保存指定的小图像。
    """
    widgets_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'widgets.png')
    
    if not os.path.exists(widgets_path):
        log(f"widgets.png 未在 {widgets_path} 找到。")
        return
    
    # 打开图像
    try:
        img = Image.open(widgets_path).convert("RGBA")
    except Exception as e:
        log(f"无法打开 widgets.png: {e}")
        return
    
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
        log(f"不支持的 widgets.png 尺寸: {width}x{height}")
        return
    
    log(f"处理 widgets.png，尺寸: {width}x{height}, scale_factor: {scale_factor}")

    def scaled_box(x1, y1, x2, y2):
        return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

    # 定义所有裁剪操作
    operations = [
        # 在这里添加所有裁剪操作
        # 由于函数定义被截断，这里只添加基本结构
    ]

    # 确保所有目标目录存在
    for op in operations:
        target_dir = op['target_dir']
        if not os.path.exists(target_dir):
            os.makedirs(target_dir)
            log(f"已创建目录: {target_dir}")

    for op_index, op in enumerate(operations, start=1):
        # 裁剪区域
        crop_region = scaled_box(*op['crop'])
        cropped_img = img.crop(crop_region)

        # 处理拆分
        slices = []
        if op['split'] == 'horizontal':
            base_slice_width, base_slice_height = op['slice_size']
            slice_width, slice_height = base_slice_width * scale_factor, base_slice_height * scale_factor
            num_slices = len(op['save_names'])
            for i in range(num_slices):
                left = i * slice_width
                upper = 0
                right = left + slice_width
                lower = upper + slice_height
                slice_img = cropped_img.crop((left, upper, right, lower))
                slices.append(slice_img)
        elif op['split'] == 'vertical':
            base_slice_width, base_slice_height = op['slice_size']
            slice_width, slice_height = base_slice_width * scale_factor, base_slice_height * scale_factor
            num_slices = len(op['save_names'])
            for i in range(num_slices):
                left = 0
                upper = i * slice_height
                right = slice_width
                lower = upper + slice_height
                slice_img = cropped_img.crop((left, upper, right, lower))
                slices.append(slice_img)
        else:
            slices = [cropped_img]

        # 保存切片
        for idx, save_name in enumerate(op['save_names']):
            if op['split'] in ['horizontal', 'vertical']:
                if idx < len(slices):
                    slice_img = slices[idx]
                else:
                    log(f"[操作 {op_index}] 切片数量不足，跳过 {save_name}")
                    continue
            else:
                slice_img = slices[0]

            # 定义目标路径
            target_path = os.path.join(op['target_dir'], save_name).replace("\\", "/")  # 确保路径使用正斜杠

            try:
                slice_img.save(target_path, format='PNG')
                log(f"[操作 {op_index}] 已保存 {target_path}，尺寸: {slice_img.size}")
            except Exception as e:
                log(f"[操作 {op_index}] 保存 {target_path} 时出错: {e}")

    log("所有 widgets.png 相关的图像已成功处理并保存。")