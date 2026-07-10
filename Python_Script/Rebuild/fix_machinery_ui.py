import os
import traceback

# 注意：log函数、process_grindstone_image函数、process_cartography_table_image函数、process_stonecutter_image函数、process_loom_image函数、process_villager_image函数需要从原文件中提取
def fix_machinery_ui(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/gui/container 文件夹中的多个 UI 图像。
    """
    try:
        # 定义 container_path
        container_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "gui", "container")
        log(f"Processing machinery UI images in: {container_path}")
        
        # 调用各个处理函数
        process_grindstone_image(container_path)
        process_cartography_table_image(container_path)
        process_stonecutter_image(container_path)
        process_loom_image(container_path)
        process_villager_image(container_path)
        
        log("Completed processing machinery UI images.")
        
    except Exception as e:
        log(f"Error processing machinery UI images in '{container_path}': {e}")
        traceback.print_exc()