import os
from PIL import Image

def process_server_selection_in_dir(temp_dir):
    """
    处理解压后的目录中的assets/minecraft/textures/gui/server_selection.png，
    """
    # 定义原始的 server_selection.png 路径
    server_selection_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'server_selection.png')
    
    if not os.path.exists(server_selection_path):
        log(f"server_selection.png 未在 {server_selection_path} 找到。")
        return
    
    # 打开图像
    try:
        img = Image.open(server_selection_path).convert("RGBA")
    except Exception as e:
        log(f"无法打开 server_selection.png: {e}")
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
        log(f"不支持的 server_selection.png 尺寸: {width}x{height}")
        return
    
    log(f"处理 server_selection.png，尺寸: {width}x{height}, scale_factor: {scale_factor}")

    def scaled_box(x1, y1, x2, y2):
        """
    根据 scale_factor 缩放裁剪坐标
    """
        return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

    # 定义裁剪操作
    operations = [
        # 步骤 1: 裁剪 (0,0)-(128,32)，分割成4个32x32的图片，保存为 join.png, emm.png, move_down.png, move_up.png
        {
            'action': 'crop_and_split',
            'crop': (0, 0, 128, 32),
            'split': 'horizontal',
            'slice_size': (32, 32),
            'save_names': ['join.png', 'emm.png', 'move_down.png', 'move_up.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'server_list')
        },
        # 步骤 2: 裁剪 (0,32)-(128,64)，分割成4个32x32的图片，保存为 join_highlighted.png, emmm.png, move_down_highlighted.png, move_up_highlighted.png
        {
            'action': 'crop_and_split',
            'crop': (0, 32, 128, 64),
            'split': 'horizontal',
            'slice_size': (32, 32),
            'save_names': [
                'join_highlighted.png', 
                'emmm.png', 
                'move_down_highlighted.png', 
                'move_up_highlighted.png'
            ],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'server_list')
        },
    ]

    # 确保目标目录存在
    for op in operations:
        target_dir = op['target_dir']
        if not os.path.exists(target_dir):
            try:
                os.makedirs(target_dir)
                log(f"已创建目录: {target_dir}")
            except Exception as e:
                log(f"创建目录 {target_dir} 时出错: {e}")
                return

    for op_index, op in enumerate(operations, start=1):
        if op['action'] == 'crop_and_split':
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
            
            # 保存切片
            for idx, save_name in enumerate(op['save_names']):
                if op['split'] == 'horizontal':
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

    log("所有 server_selection.png 相关的图像已成功处理并保存。")
