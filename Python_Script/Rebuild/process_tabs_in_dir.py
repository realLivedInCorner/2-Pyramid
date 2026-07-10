import os
from PIL import Image
import traceback

# 注意：此函数依赖于外部的log函数

def process_tabs_in_dir(temp_dir):
    """
    处理解压后的目录中的assets/minecraft/textures/gui/container/creative_inventory/tabs.png，
    """
    tabs_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container', 'creative_inventory', 'tabs.png')
    
    if not os.path.exists(tabs_path):
        log(f"tabs.png 未在 {tabs_path} 找到。")
        return
    
    # 打开图像
    try:
        img = Image.open(tabs_path).convert("RGBA")
    except Exception as e:
        log(f"无法打开 tabs.png: {e}")
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
        log(f"不支持的 tabs.png 尺寸: {width}x{height}")
        return
    
    log(f"处理 tabs.png，尺寸: {width}x{height}, scale_factor: {scale_factor}")

    def scaled_box(x1, y1, x2, y2):
        return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

    # 定义所有裁剪操作
    operations = [
        # Step 1: (0,2)-(168,32) -> 6 slices 28x30, duplicate 6th as 7th
        {
            'crop': (0, 2, 168, 32),
            'split': 'horizontal',
            'slice_size': (28, 30),
            'save_names': [
                'tab_top_unselected_1.png', 'tab_top_unselected_2.png', 'tab_top_unselected_3.png',
                'tab_top_unselected_4.png', 'tab_top_unselected_5.png', 'tab_top_unselected_6.png'
            ],
            'duplicate_save_name': 'tab_top_unselected_7.png',
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'creative_inventory')
        },
        # Step 2: (0,32)-(168,64) -> 6 slices 28x32, duplicate 6th as 7th
        {
            'crop': (0, 32, 168, 64),
            'split': 'horizontal',
            'slice_size': (28, 32),
            'save_names': [
                'tab_top_selected_1.png', 'tab_top_selected_2.png', 'tab_top_selected_3.png',
                'tab_top_selected_4.png', 'tab_top_selected_5.png', 'tab_top_selected_6.png'
            ],
            'duplicate_save_name': 'tab_top_selected_7.png',
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'creative_inventory')
        },
        # Step 3: (0,64)-(168,94) -> 6 slices 28x30, duplicate 6th as 7th
        {
            'crop': (0, 64, 168, 94),
            'split': 'horizontal',
            'slice_size': (28, 30),
            'save_names': [
                'tab_bottom_unselected_1.png', 'tab_bottom_unselected_2.png', 'tab_bottom_unselected_3.png',
                'tab_bottom_unselected_4.png', 'tab_bottom_unselected_5.png', 'tab_bottom_unselected_6.png'
            ],
            'duplicate_save_name': 'tab_bottom_unselected_7.png',
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'creative_inventory')
        },
        # Step 4: (0,96)-(168,128) -> 6 slices 28x32, duplicate 6th as 7th
        {
            'crop': (0, 96, 168, 128),
            'split': 'horizontal',
            'slice_size': (28, 32),
            'save_names': [
                'tab_bottom_selected_1.png', 'tab_bottom_selected_2.png', 'tab_bottom_selected_3.png',
                'tab_bottom_selected_4.png', 'tab_bottom_selected_5.png', 'tab_bottom_selected_6.png'
            ],
            'duplicate_save_name': 'tab_bottom_selected_7.png',
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'creative_inventory')
        },
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
        try:
            cropped_img = img.crop(crop_region)
            log(f"[操作 {op_index}] 裁剪区域: {op['crop']} -> {crop_region}")
        except Exception as e:
            log(f"[操作 {op_index}] 裁剪区域 {op['crop']} 时出错: {e}")
            continue

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
                try:
                    slice_img = cropped_img.crop((left, upper, right, lower))
                    slices.append(slice_img)
                    log(f"[操作 {op_index}] 切片 {i+1}: ({left}, {upper}, {right}, {lower}) -> {slice_img.size}")
                except Exception as e:
                    log(f"[操作 {op_index}] 拆分切片 {i+1} 时出错: {e}")
        elif op['split'] == 'vertical':
            base_slice_width, base_slice_height = op['slice_size']
            slice_width, slice_height = base_slice_width * scale_factor, base_slice_height * scale_factor
            num_slices = len(op['save_names'])
            for i in range(num_slices):
                left = 0
                upper = i * slice_height
                right = slice_width
                lower = upper + slice_height
                try:
                    slice_img = cropped_img.crop((left, upper, right, lower))
                    slices.append(slice_img)
                    log(f"[操作 {op_index}] 切片 {i+1}: ({left}, {upper}, {right}, {lower}) -> {slice_img.size}")
                except Exception as e:
                    log(f"[操作 {op_index}] 拆分切片 {i+1} 时出错: {e}")
        else:
            slices = [cropped_img]
            log(f"[操作 {op_index}] 无需拆分，单一图像尺寸: {slices[0].size}")

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

        # 处理重复切片（如果有）
        if op.get('duplicate_save_name') and len(slices) >= len(op['save_names']):
            duplicate_slice = slices[-1]
            duplicate_save_name = op['duplicate_save_name']
            duplicate_target_path = os.path.join(op['target_dir'], duplicate_save_name).replace("\\", "/")
            try:
                duplicate_slice.save(duplicate_target_path, format='PNG')
                log(f"[操作 {op_index}] 已保存重复切片 {duplicate_target_path}，尺寸: {duplicate_slice.size}")
            except Exception as e:
                log(f"[操作 {op_index}] 保存重复切片 {duplicate_target_path} 时出错: {e}")

    log("所有 tabs.png 相关的图像已成功处理并保存。")