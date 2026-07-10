import os

# 注意：此函数依赖于外部的merge_images函数和create_mcmeta_file函数

def reverse_fix_clock_compass(temp_dir):
    items_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'items')
    compass_images = [os.path.join(items_path, f"compass_{i:02d}.png") for i in range(0, 32)]  # 从 0 到 31
    merge_images(compass_images, os.path.join(items_path, "compass.png"))        
    clock_images = [os.path.join(items_path, f"clock_{i:02d}.png") for i in range(0, 64)]
    merge_images(clock_images, os.path.join(items_path, "clock.png"))
    create_mcmeta_file(os.path.join(items_path, "compass.png.mcmeta"))
    create_mcmeta_file(os.path.join(items_path, "clock.png.mcmeta"))