import os
from PIL import Image
import traceback

# 注意：此函数依赖于外部的log函数

def process_icons_in_dir(temp_dir):
    """
    处理解压后的目录中的assets/minecraft/textures/gui/icons.png，裁剪并保存指定的小图像。
    """
    icons_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'icons.png')
    
    if not os.path.exists(icons_path):
        log(f"icons.png 未在 {icons_path} 找到。")
        return
    
    # 打开图像
    try:
        img = Image.open(icons_path).convert("RGBA")
    except Exception as e:
        log(f"无法打开 icons.png: {e}")
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
        log(f"不支持的 icons.png 尺寸: {width}x{height}")
        return
    
    log(f"处理 icons.png，尺寸: {width}x{height}, scale_factor: {scale_factor}")

    def scaled_box(x1, y1, x2, y2):
        return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

    # 定义所有裁剪操作
    operations = [
        # Step 1
        {
            'crop': (0, 0, 15, 15),
            'split': None,
            'save_names': ['crosshair.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 2
        {
            'crop': (16, 0, 196, 9),
            'split': 'horizontal',
            'slice_size': (9, 9),
            'save_names': [
                'container.png', 'container_blinking.png', 'wtf.png', 'wtf2.png', 'full.png', 'half.png',
                'full_blinking.png', 'half_blinking.png', 'poisoned_full.png', 'poisoned_half.png',
                'poisoned_full_blinking.png', 'poisoned_half_blinking.png',
                'withered_full.png', 'withered_half.png',
                'withered_full_blinking.png', 'withered_half_blinking.png',
                'absorbing_full.png', 'absorbing_half.png',
                'frozen_full.png', 'frozen_half.png'
            ],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud', 'heart')
        },
        # Step 3a
        {
            'crop': (16, 9, 124, 18),
            'split': 'horizontal',
            'slice_size': (9, 9),
            'save_names': ['armor_empty.png', 'armor_half.png', 'armor_full.png', 'wtf3.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 3b
        {
            'crop': (52, 9, 124, 18),
            'split': 'horizontal',
            'slice_size': (9, 9),
            'save_names': [
                'vehicle_container.png', 'wtf4.png', 'wtf5.png', 'wtf6.png',
                'vehicle_full.png', 'vehicle_half.png', 'wtf7.png', 'wtf8.png'
            ],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud', 'heart')
        },
        # Step 4
        {
            'crop': (16, 18, 52, 27),
            'split': 'horizontal',
            'slice_size': (9, 9),
            'save_names': ['air.png', 'air_bursting.png', 'wtf9.png', 'wtf10.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 5
        {
            'crop': (16, 27, 142, 36),
            'split': 'horizontal',
            'slice_size': (9, 9),
            'save_names': [
                'food_empty.png', 'wtf11.png', 'wtf123.png', 'wtf13.png',
                'food_full.png', 'food_half.png', 'wtf14.png', 'wtf15.png',
                'food_full_hunger.png', 'food_half_hunger.png',
                'wtf16.png', 'wtf17.png', 'wtf18.png', 'food_empty_hunger.png'
            ],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 6
        {
            'crop': (16, 45, 196, 54),
            'split': 'horizontal',
            'slice_size': (9, 9),
            'save_names': [
                'container_hardcore.png', 'container_hardcore_blinking.png', 'wtf19.png', 'wtf20.png',
                'hardcore_full.png', 'hardcore_half.png',
                'hardcore_full_blinking.png', 'hardcore_half_blinking.png',
                'poisoned_hardcore_full.png', 'poisoned_hardcore_half.png',
                'poisoned_hardcore_full_blinking.png', 'poisoned_hardcore_half_blinking.png',
                'withered_hardcore_full.png', 'withered_hardcore_half.png',
                'withered_hardcore_full_blinking.png', 'withered_hardcore_half_blinking.png',
                'absorbing_hardcore_full.png', 'absorbing_hardcore_half.png',
                'frozen_hardcore_full.png', 'frozen_hardcore_half.png'
            ],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud', 'heart')
        },
        # Step 7
        {
            'crop': (0, 15, 10, 63),
            'split': 'vertical',
            'slice_size': (10, 8),
            'save_names': ['ping_5.png', 'ping_4.png', 'ping_3.png', 'ping_2.png', 'ping_1.png', 'ping_unknown.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'icon')
        },
        # Step 8
        {
            'crop': (0, 64, 182, 94),
            'split': 'vertical',
            'slice_size': (182, 5),
            'save_names': [
                'experience_bar_background.png',
                'experience_bar_progress.png',
                'jump_bar_cooldown.png',
                'wtf21.png',
                'jump_bar_background.png',
                'jump_bar_progress.png'
            ],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 9
        {
            'crop': (0, 94, 18, 112),
            'split': None,
            'save_names': ['hotbar_attack_indicator_background.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 10
        {
            'crop': (18, 94, 36, 112),
            'split': None,
            'save_names': ['hotbar_attack_indicator_progress.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 11
        {
            'crop': (36, 94, 52, 98),
            'split': None,
            'save_names': ['crosshair_attack_indicator_background.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 12
        {
            'crop': (52, 94, 68, 98),
            'split': None,
            'save_names': ['crosshair_attack_indicator_progress.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 13
        {
            'crop': (68, 94, 84, 110),
            'split': None,
            'save_names': ['crosshair_attack_indicator_full.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 14
        {
            'crop': (0, 176, 10, 224),
            'split': 'vertical',
            'slice_size': (10, 8),
            'save_names': ['ping_5.png', 'ping_4.png', 'ping_3.png', 'ping_2.png', 'ping_1.png', 'unreachable.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'server_list')
        },
        # Step 15
        {
            'crop': (10, 176, 20, 216),
            'split': 'vertical',
            'slice_size': (10, 8),
            'save_names': ['pinging_5.png', 'pinging_4.png', 'pinging_3.png', 'pinging_2.png', 'pinging_1.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'server_list')
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

    log("所有 icons.png 相关的图像已成功处理并保存。")