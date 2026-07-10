import os
import shutil

def delete_blockstates_models(temp_dir):
    delete_folder(os.path.join(temp_dir, "assets/minecraft/blockstates"))
    delete_folder(os.path.join(temp_dir, "assets/minecraft/models"))

# 注意：delete_folder函数需要从原文件中提取