import os

# 注意：process_block_image函数需要从原文件中提取
def generate_redwood_cherry_bamboo_planks(temp_dir):
    blocks_path_new = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'block')
    process_block_image(blocks_path_new, 'oak_planks.png', 'mangrove_planks.png', hue_shift=-59, brightness_adjust=-15, saturation_adjust=0)
    process_block_image(blocks_path_new, 'oak_planks.png', 'cherry_planks.png', hue_shift=-80, brightness_adjust=40, saturation_adjust=0)
    process_block_image(blocks_path_new, 'oak_planks.png', 'bamboo_planks.png', hue_shift=25, brightness_adjust=20, saturation_adjust=0)