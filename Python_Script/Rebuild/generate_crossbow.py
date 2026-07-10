import os
import sys
import shutil
from PIL import Image
import traceback

# 注意：log函数、overlay_images函数需要从原文件中提取
def generate_crossbow(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/item 目录下的 bow.png,
    """
    try:
        # 定义 items_path_new
        items_path_new = os.path.join(temp_dir, "assets", "minecraft", "textures", "item")
        log(f"Processing crossbow images in: {items_path_new}")
        
        # 获取 crossbow_dir，假设 crossbow 文件夹在 .exe 同目录下
        if getattr(sys, 'frozen', False):
            # 如果是打包后的 .exe
            application_path = os.path.dirname(sys.executable)
        else:
            # 如果是未打包的脚本
            application_path = os.path.dirname(os.path.abspath(__file__))
        
        crossbow_dir = os.path.join(application_path, 'crossbow')
        log(f"Using crossbow directory: {crossbow_dir}")
        
        # 定义 crossbow_base_path 映射
        size_to_path = {
            (16, 16): os.path.join(crossbow_dir, 'crossbow_16.png'),
            (32, 32): os.path.join(crossbow_dir, 'crossbow_32.png'),
            (64, 64): os.path.join(crossbow_dir, 'crossbow_64.png'),
            (128, 128): os.path.join(crossbow_dir, 'crossbow_128.png'),
            (256, 256): os.path.join(crossbow_dir, 'crossbow_256.png'),
        }
        
        # 处理 crossbow_standby.png 基于 bow.png
        bow_path = os.path.join(items_path_new, 'bow.png')
        if os.path.exists(bow_path):
            log(f"Found 'bow.png' at {bow_path}, processing...")
            bow_img = Image.open(bow_path).convert("RGBA")
            bow_size = bow_img.size
            
            if bow_size in size_to_path:
                crossbow_base_path = size_to_path[bow_size]
                if os.path.exists(crossbow_base_path):
                    log(f"Found crossbow base image at {crossbow_base_path}")
                    crossbow_base_img = Image.open(crossbow_base_path).convert("RGBA")
                    crossbow_standby_img = overlay_images(crossbow_base_img, bow_img, (0, 0))
                    crossbow_standby_output_path = os.path.join(items_path_new, 'crossbow_standby.png')
                    crossbow_standby_img.save(crossbow_standby_output_path)
                    log(f"Created 'crossbow_standby.png' in {items_path_new}")
                else:
                    log(f"No '{crossbow_base_path}' found in {crossbow_dir}")
            else:
                log(f"'bow.png' size is not supported: {bow_size}")
        else:
            log(f"No 'bow.png' found in {items_path_new}")

    except Exception as e:
        log(f"Error processing crossbow images: {e}")
        traceback.print_exc()