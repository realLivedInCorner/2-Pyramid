import os
import shutil
import traceback

# 注意：此函数依赖于外部的log函数

def reverse_fix_armor_models(temp_dir):
    """
    将 armor 模型文件从 humanoid 和 humanoid_leggings 文件夹移动并重命名回 armor 文件夹。
    """
    # 定义源目录和目标目录
    armor_target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'models', 'armor')
    humanoid_source_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'entity', 'equipment', 'humanoid')
    humanoid_leggings_source_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'entity', 'equipment', 'humanoid_leggings')

    # 定义需要移动和重命名的文件映射
    layer_mappings = {
        'humanoid': {
            'source_dir': humanoid_source_dir,
            'files': {
                'chainmail.png': 'chainmail_layer_1.png',
                'diamond.png': 'diamond_layer_1.png',
                'iron.png': 'iron_layer_1.png',
                'gold.png': 'gold_layer_1.png',
                'leather.png': 'leather_layer_1.png',
                'leather_overlay.png': 'leather_layer_1_overlay.png',
                'netherite.png': 'netherite_layer_1.png',
                'copper.png': 'copper_layer_1.png',  # 添加铜盔甲层1
            }
        },
        'humanoid_leggings': {
            'source_dir': humanoid_leggings_source_dir,
            'files': {
                'chainmail.png': 'chainmail_layer_2.png',
                'diamond.png': 'diamond_layer_2.png',
                'iron.png': 'iron_layer_2.png',
                'gold.png': 'gold_layer_2.png',
                'leather.png': 'leather_layer_2.png',
                'leather_overlay.png': 'leather_layer_2_overlay.png',
                'netherite.png': 'netherite_layer_2.png',
                'copper.png': 'copper_layer_2.png',  # 添加铜盔甲层2
            }
        }
    }

    # 确保目标目录存在
    os.makedirs(armor_target_dir, exist_ok=True)

    # 遍历每个源文件夹的文件映射
    for layer, config in layer_mappings.items():
        source_dir = config['source_dir']
        files = config['files']

        for src_name, dest_name in files.items():
            src_path = os.path.join(source_dir, src_name)
            dest_path = os.path.join(armor_target_dir, dest_name)

            if os.path.exists(src_path):
                try:
                    shutil.move(src_path, dest_path)
                    log(f"已移动并重命名 '{src_name}' 为 '{dest_name}' 到 '{armor_target_dir}'")
                except Exception as e:
                    log(f"移动 '{src_name}' 到 '{armor_target_dir}' 时出错: {e}")
                    traceback.print_exc()
            else:
                log(f"未找到文件: {src_path}")