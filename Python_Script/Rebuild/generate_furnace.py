import os
import shutil
import traceback

# 注意：log函数需要从原文件中提取
def generate_furnace(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/gui/container/furnace.png 文件，
    """
    try:
        # 定义相关路径
        container_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "gui", "container")
        furnace_path = os.path.join(container_path, 'furnace.png')
        blast_furnace_path = os.path.join(container_path, 'blast_furnace.png')
        smoker_path = os.path.join(container_path, 'smoker.png')
        
        log(f"Processing furnace images in: {container_path}")
        
        if os.path.exists(furnace_path):
            shutil.copy(furnace_path, blast_furnace_path)
            shutil.copy(furnace_path, smoker_path)
            log(f"Copied 'furnace.png' to 'blast_furnace.png' and 'smoker.png'")
        else:
            log(f"No 'furnace.png' found in {container_path}")

    except Exception as e:
        log(f"Error processing furnace images in '{container_path}': {e}")
        traceback.print_exc()