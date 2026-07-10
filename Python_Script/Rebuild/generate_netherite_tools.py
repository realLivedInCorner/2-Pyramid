import os
import shutil
from PIL import Image
import traceback

# 注意：log函数、process_image函数需要从原文件中提取
def generate_netherite_tools(temp_dir):
    log(f"Processing and copying items in: {temp_dir}")
    items_path_new = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'item')
    items_to_copy_and_process = [
        'diamond_sword', 'diamond_helmet', 'diamond_chestplate', 'diamond_leggings', 'diamond_boots',
        'diamond_axe', 'diamond_pickaxe', 'diamond_shovel', 'diamond_hoe'
    ]
    for item in items_to_copy_and_process:
        try:
            original_path = os.path.join(items_path_new, f'{item}.png')
            new_path = os.path.join(items_path_new, f'netherite_{item[8:]}.png')
            if os.path.exists(original_path):
                shutil.copy(original_path, new_path)
                log(f"Copied and renamed '{item}.png' to 'netherite_{item[8:]}.png'")
                process_image(new_path)
                log(f"Processed image 'netherite_{item[8:]}.png'")

                original_meta_path = original_path + '.mcmeta'
                new_meta_path = new_path + '.mcmeta'
                if os.path.exists(original_meta_path):
                    shutil.copy(original_meta_path, new_meta_path)
                    log(f"Copied and renamed '{item}.png.mcmeta' to 'netherite_{item[8:]}.png.mcmeta'")

        except Exception as e:
            log(f"Error processing and copying item '{item}': {e}")
            traceback.print_exc()

    # 处理 arrow.png 文件
    try:
        arrow_path = os.path.join(items_path_new, 'arrow.png')
        spectral_arrow_path = os.path.join(items_path_new, 'spectral_arrow.png')

        if os.path.exists(arrow_path):
            # 复制 arrow.png 并处理
            shutil.copy(arrow_path, spectral_arrow_path)
            img = Image.open(spectral_arrow_path).convert("RGBA")
            
            new_image_data = []
            for item in img.getdata():
                if item[3] == 0:
                    new_image_data.append(item)
                else:
                    # 这里可以添加具体的处理逻辑
                    new_image_data.append(item)
            new_image = Image.new("RGBA", img.size)
            new_image.putdata(new_image_data)
            new_image.save(spectral_arrow_path)
            log(f"Processed 'arrow.png' to 'spectral_arrow.png'")

    except Exception as e:
        log(f"Error processing arrow.png: {e}")
        traceback.print_exc()