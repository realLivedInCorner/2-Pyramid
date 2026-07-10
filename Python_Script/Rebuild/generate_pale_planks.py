import os

# 注意：此函数依赖于外部的process_block_image函数

def generate_pale_planks(temp_dir):
    blocks_path_new = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'block')
    process_block_image(blocks_path_new, 'oak_planks.png', 'pale_oak_planks.png', hue_shift=0, brightness_adjust=30, saturation_adjust=-100)