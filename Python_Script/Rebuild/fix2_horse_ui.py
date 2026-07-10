import os
import shutil
import traceback

# 注意：此函数依赖于外部的log函数

def fix2_horse_ui(temp_dir):
    """
    处理 temp_dir 目录中的 horse 图片文件，
    """
    log(f"Processing horse images in: {temp_dir}")
    try:
        # 定义源文件和目标文件的对应关系
        files_to_copy = {
            'armor_slot.png': 'horse_armor.png',
            'llama_armor_slot.png': 'llama_armor.png',
            'saddle_slot.png': 'saddle.png'
        }

        # 定义源目录和目标目录
        source_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'horse')
        target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites','container', 'slot')

        # 确保目标目录存在，如果不存在则创建
        os.makedirs(target_dir, exist_ok=True)
        log(f"Ensured target directory exists: {target_dir}")

        # 遍历需要复制的文件
        for source_file, target_file in files_to_copy.items():
            source_path = os.path.join(source_dir, source_file)
            target_path = os.path.join(target_dir, target_file)

            # 检查源文件是否存在
            if os.path.exists(source_path):
                try:
                    shutil.copyfile(source_path, target_path)
                    log(f"Copied '{source_path}' to '{target_path}'")
                except Exception as e:
                    log(f"Error copying '{source_path}' to '{target_path}': {e}")
                    traceback.print_exc()
            else:
                log(f"Source file '{source_path}' does not exist. Skipping.")

    except Exception as e:
        log(f"Error processing horse images in '{temp_dir}': {e}")
        traceback.print_exc()