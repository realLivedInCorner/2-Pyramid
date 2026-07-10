import os

# 注意：delete_folder函数需要从原文件中提取
def delete_font_folder(temp_dir):
    delete_folder(os.path.join(temp_dir, "assets/minecraft/font"))