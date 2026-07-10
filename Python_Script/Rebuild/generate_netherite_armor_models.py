import os
import shutil
import traceback

# 注意：log函数、process_image函数需要从原文件中提取
def generate_netherite_armor_models(temp_dir):
    log(f"Processing armor layers in: {temp_dir}")
    armor_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'models', 'armor')
    armor_files = ['diamond_layer_1.png', 'diamond_layer_2.png']
    for armor_file in armor_files:
        try:
            original_path = os.path.join(armor_path, armor_file)
            new_path = os.path.join(armor_path, armor_file.replace('diamond', 'netherite'))
            log(f"Checking if {original_path} exists.")
            if os.path.exists(original_path):
                shutil.copy(original_path, new_path)
                log(f"Copied and renamed '{armor_file}' to '{armor_file.replace('diamond', 'netherite')}'")
                process_image(new_path)
                log(f"Processed image '{armor_file.replace('diamond', 'netherite')}'")
            else:
                log(f"'{original_path}' does not exist.")
        except Exception as e:
            log(f"Error processing and copying armor layer '{armor_file}': {e}")
            traceback.print_exc()