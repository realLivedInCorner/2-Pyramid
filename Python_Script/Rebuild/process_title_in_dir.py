import os
import shutil
from PIL import Image

def process_title_in_dir(temp_dir):
    """
    处理解压后的目录中的assets/minecraft/textures/gui/title/minecraft.png，
    """
    # 定义原始的 minecraft.png 路径
    title_minecraft_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'title', 'minecraft.png')
    
    if not os.path.exists(title_minecraft_path):
        log(f"minecraft.png 未在 {title_minecraft_path} 找到。")
        return
    
    # 打开图像
    try:
        img = Image.open(title_minecraft_path).convert("RGBA")
    except Exception as e:
        log(f"无法打开 minecraft.png: {e}")
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
        log(f"不支持的 minecraft.png 尺寸: {width}x{height}")
        return
    
    log(f"处理 minecraft.png，尺寸: {width}x{height}, scale_factor: {scale_factor}")

    def scaled_box(x1, y1, x2, y2):
        return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

    # 定义裁剪操作
    operations = [
        # 步骤 1: 裁剪 (0,94)-(200,194)，保存为 realms.png
        {
            'action': 'crop_and_save',
            'crop': (0, 94, 200, 194),
            'save_name': 'realms.png',
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'title')
        },
        # 步骤 2: 裁剪 (0,0)-(155,44) 和 (0,45)-(119,89)，拼接并添加透明区域，保存为 minecraft.png
        {
            'action': 'crop_concatenate_append',
            'crop1': (0, 0, 155, 44),
            'crop2': (0, 45, 119, 89),
            'save_name': 'minecraft.png',
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'title'),
            'concat_direction': 'horizontal',  # 左右拼接
            'transparent_append': {
                'width': 274 * scale_factor,
                'height': 25 * scale_factor  # 根据 scale_factor 调整高度
            }
        }
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
        if op['action'] == 'crop_and_save':
            # 裁剪区域并保存
            crop_region = scaled_box(*op['crop'])
            try:
                cropped_img = img.crop(crop_region)
                log(f"[操作 {op_index}] 裁剪区域: {op['crop']} -> {crop_region}")
            except Exception as e:
                log(f"[操作 {op_index}] 裁剪区域 {op['crop']} 时出错: {e}")
                continue

            # 定义目标路径
            target_path = os.path.join(op['target_dir'], op['save_name']).replace("\\", "/")
            try:
                cropped_img.save(target_path, format='PNG')
                log(f"[操作 {op_index}] 已保存 {target_path}，尺寸: {cropped_img.size}")
            except Exception as e:
                log(f"[操作 {op_index}] 保存 {target_path} 时出错: {e}")
        
        elif op['action'] == 'crop_concatenate_append':
            # 裁剪两个区域
            crop_region1 = scaled_box(*op['crop1'])
            crop_region2 = scaled_box(*op['crop2'])
            try:
                cropped_img1 = img.crop(crop_region1)
                cropped_img2 = img.crop(crop_region2)
                log(f"[操作 {op_index}] 裁剪区域1: {op['crop1']} -> {crop_region1}")
                log(f"[操作 {op_index}] 裁剪区域2: {op['crop2']} -> {crop_region2}")
            except Exception as e:
                log(f"[操作 {op_index}] 裁剪区域时出错: {e}")
                continue

            # 拼接图像
            if op['concat_direction'] == 'horizontal':
                new_width = cropped_img1.width + cropped_img2.width
                new_height = max(cropped_img1.height, cropped_img2.height)
            else:
                new_width = max(cropped_img1.width, cropped_img2.width)
                new_height = cropped_img1.height + cropped_img2.height

            try:
                concatenated_img = Image.new('RGBA', (new_width, new_height))
                concatenated_img.paste(cropped_img1, (0, 0))
                concatenated_img.paste(cropped_img2, (cropped_img1.width, 0))
                log(f"[操作 {op_index}] 拼接后的图像尺寸: {concatenated_img.size}")
            except Exception as e:
                log(f"[操作 {op_index}] 拼接图像时出错: {e}")
                continue

            # 创建透明区域并拼接
            transparent_width = op['transparent_append']['width']
            transparent_height = op['transparent_append']['height']
            try:
                transparent_img = Image.new('RGBA', (transparent_width, transparent_height), (0, 0, 0, 0))
                final_img = Image.new('RGBA', (transparent_width, concatenated_img.height + transparent_height))
                
                # 计算位置以确保拼接图像居中或按照需求对齐
                final_img.paste(concatenated_img, (0, 0))
                final_img.paste(transparent_img, (0, concatenated_img.height))
                log(f"[操作 {op_index}] 添加透明区域后的图像尺寸: {final_img.size}")
            except Exception as e:
                log(f"[操作 {op_index}] 添加透明区域时出错: {e}")
                continue

            # 定义目标路径
            target_path = os.path.join(op['target_dir'], op['save_name']).replace("\\", "/")
            try:
                final_img.save(target_path, format='PNG')
                log(f"[操作 {op_index}] 已保存拼接并添加透明区域的图像 {target_path}，尺寸: {final_img.size}")
            except Exception as e:
                log(f"[操作 {op_index}] 保存拼接图像 {target_path} 时出错: {e}")
                continue

            # 替换原始的 minecraft.png
            try:
                shutil.copy(target_path, title_minecraft_path)
                log(f"[操作 {op_index}] 已将新 minecraft.png 替换原始文件 {title_minecraft_path}")
            except Exception as e:
                log(f"[操作 {op_index}] 替换原始 minecraft.png 时出错: {e}")
                continue

    log("所有 minecraft.png 相关的图像已成功处理并保存。")
