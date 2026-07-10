import os

# 注意：此函数依赖于外部的delete_folder函数

def delete_shaders_folder(temp_dir):
    delete_folder(os.path.join(temp_dir, "assets/minecraft/shaders"))