import os
from PIL import Image
import traceback

# 注意：此函数依赖于外部的log函数

def reverse_fix_particles(temp_dir):
    particle_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'particle')
    entity_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'entity')
    log(f"合并粒子图片来自: {particle_dir} 和实体图片来自: {entity_dir}")
    try:
        # 定义文件名到其在网格中的位置 (row, col) 的映射
        filename_to_position = {
            'generic_0.png': (0, 0),
            'generic_1.png': (0, 1),
            'generic_2.png': (0, 2),
            'generic_3.png': (0, 3),
            'generic_4.png': (0, 4),
            'generic_5.png': (0, 5),
            'generic_6.png': (0, 6),
            'generic_7.png': (0, 7),
            'splash_0.png': (1, 3),
            'splash_1.png': (1, 4),
            'splash_2.png': (1, 5),
            'splash_3.png': (1, 6),
            'bubble.png': (2, 0),
            'fishing_hook.png': (2, 1),
            'flame.png': (3, 0),
            'lava.png': (3, 1),
            'note.png': (4, 0),
            'critical_hit.png': (4, 1),
            'enchanted_hit.png': (4, 2),
            'heart.png': (5, 0),
            'angry.png': (5, 1),
            'glint.png': (5, 2),
            'drip_hang.png': (7, 0),
            'drip_fall.png': (7, 1),
            'drip_land.png': (7, 2),
            'effect_0.png': (8, 0),
            'effect_1.png': (8, 1),
            'effect_2.png': (8, 2),
            'effect_3.png': (8, 3),
            'effect_4.png': (8, 4),
            'effect_5.png': (8, 5),
            'effect_6.png': (8, 6),
            'effect_7.png': (8, 7),
            'spell_0.png': (9, 0),
            'spell_1.png': (9, 1),
            'spell_2.png': (9, 2),
            'spell_3.png': (9, 3),
            'spell_4.png': (9, 4),
            'spell_5.png': (9, 5),
            'spell_6.png': (9, 6),
            'spell_7.png': (9, 7),
            'spark_0.png': (10, 0),
            'spark_1.png': (10, 1),
            'spark_2.png': (10, 2),
            'spark_3.png': (10, 3),
            'spark_4.png': (10, 4),
            'spark_5.png': (10, 5),
            'spark_6.png': (10, 6),
            'spark_7.png': (10, 7)
        }

        # 确定每个小图的大小 (假设所有小图大小相同)
        split_size = None
        for filename in filename_to_position.keys():
            # 尝试从 particle_dir 和 entity_dir 获取图片
            file_path_particle = os.path.join(particle_dir, filename)
            if os.path.exists(file_path_particle):
                img = Image.open(file_path_particle)
                split_size = img.width  # 假设小图为正方形
                break

            # 如果 particle_dir 没找到，则从 entity_dir 寻找
            file_path_entity = os.path.join(entity_dir, filename)
            if os.path.exists(file_path_entity):
                img = Image.open(file_path_entity)
                split_size = img.width
                break

        if split_size is None:
            log("未找到粒子图片或实体图片以确定分割大小。")
            return

        # 创建一个新的空白图片 (16行 x 16列)
        rows = 16
        cols = 16
        merged_size = (cols * split_size, rows * split_size)
        merged_image = Image.new("RGBA", merged_size, (0, 0, 0, 0))  # 透明背景

        # 遍历映射并粘贴每个小图片
        for filename, (row, col) in filename_to_position.items():
            file_path_particle = os.path.join(particle_dir, filename)
            file_path_entity = os.path.join(entity_dir, filename)
            
            if os.path.exists(file_path_particle):
                img = Image.open(file_path_particle).convert("RGBA")
                merged_image.paste(img, (col * split_size, row * split_size), img)
                log(f"粘贴 '{filename}' 从粒子目录，在行 {row}, 列 {col}")
            elif os.path.exists(file_path_entity):
                img = Image.open(file_path_entity).convert("RGBA")
                merged_image.paste(img, (col * split_size, row * split_size), img)
                log(f"粘贴 '{filename}' 从实体目录，在行 {row}, 列 {col}")
            else:
                log(f"缺少图片: '{filename}'")

        # 保存合并后的 particles.png
        particles_output_path = os.path.join(particle_dir, 'particles.png')
        merged_image.save(particles_output_path)
        log(f"已保存合并后的 'particles.png' 到 {particles_output_path}")

        # 合并后删除小图片
        for filename in filename_to_position.keys():
            file_path_particle = os.path.join(particle_dir, filename)
            file_path_entity = os.path.join(entity_dir, filename)
            if os.path.exists(file_path_particle):
                os.remove(file_path_particle)
                log(f"已删除粒子小图片: '{file_path_particle}'")
            if os.path.exists(file_path_entity):
                os.remove(file_path_entity)
                log(f"已删除实体小图片: '{file_path_entity}'")

    except Exception as e:
        log(f"合并粒子图片时出错: {e}")
        traceback.print_exc()