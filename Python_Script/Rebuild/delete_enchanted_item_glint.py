import os

# 注意：log函数需要从原文件中提取
def delete_enchanted_item_glint(temp_dir):
    log(f"Deleting enchanted item glint in: {temp_dir}")
    enchanted_item_glint_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'misc', 'enchanted_item_glint.png')
    if os.path.exists(enchanted_item_glint_path):
        os.remove(enchanted_item_glint_path)
        log(f"Deleted 'enchanted_item_glint.png' from {temp_dir}")
    else:
        log(f"No 'enchanted_item_glint.png' found in {temp_dir}")