import os
import shutil
from PIL import Image
import traceback

# 注意：log函数需要从原文件中提取
def generate_tipped_arrow_images(temp_dir):
    """
    生成新的 tipped_arrow_base.png 和 tipped_arrow_head.png 文件。
    """
    try:
        # 定义相关路径
        items_path_old = os.path.join(temp_dir, "assets/minecraft/textures/items")
        arrow_path = os.path.join(items_path_old, 'arrow.png')
        tipped_arrow_base_path = os.path.join(items_path_old, 'tipped_arrow_base.png')
        tipped_arrow_head_dir = os.path.join(os.getcwd(), 'tipped_arrow_head')  # 假设 tipped_arrow_head 文件夹在当前工作目录下

        # 检查 arrow.png 是否存在
        if os.path.exists(arrow_path):
            # 打开 arrow.png 并获取大小
            img = Image.open(arrow_path).convert("RGBA")
            size = img.size[0]  # 假设图片为正方形

            # 定义 tipped_arrow_head_X.png 的路径
            tipped_arrow_head_file = f'tipped_arrow_head_{size}.png'
            tipped_arrow_head_path = os.path.join(tipped_arrow_head_dir, tipped_arrow_head_file)

            # 检查 tipped_arrow_head_X.png 是否存在
            if os.path.exists(tipped_arrow_head_path):
                # 复制 arrow.png 为 tipped_arrow_base.png
                shutil.copy(arrow_path, tipped_arrow_base_path)
                base_img = Image.open(tipped_arrow_base_path).convert("RGBA")
                head_img = Image.open(tipped_arrow_head_path).convert("RGBA")

                # 获取像素数据
                base_data = base_img.getdata()
                head_data = head_img.getdata()
                new_base_data = []

                # 遍历每个像素，处理重叠部分
                for base_pixel, head_pixel in zip(base_data, head_data):
                    if head_pixel[3] > 0:  # 只处理非透明像素
                        # 将该像素改为透明
                        new_base_data.append((base_pixel[0], base_pixel[1], base_pixel[2], 0))
                    else:
                        new_base_data.append(base_pixel)

                # 更新 base_img 的像素数据
                base_img.putdata(new_base_data)
                base_img.save(tipped_arrow_base_path)
                log(f"已处理 'tipped_arrow_base.png' 通过使 '{tipped_arrow_head_file}' 重叠的像素透明")

                # 将 tipped_arrow_head_X.png 复制并重命名为 tipped_arrow_head.png
                new_tipped_arrow_head_path = os.path.join(items_path_old, 'tipped_arrow_head.png')
                shutil.copy(tipped_arrow_head_path, new_tipped_arrow_head_path)
                log(f"已复制并重命名 '{tipped_arrow_head_file}' 为 'tipped_arrow_head.png'")
            else:
                log(f"未找到 {tipped_arrow_head_file}，跳过生成 'tipped_arrow_head.png'")
        else:
            log(f"未找到 'arrow.png'，无法生成箭矢图像")

    except Exception as e:
        log(f"处理 'tipped_arrow_base.png' 时出错: {e}")
        traceback.print_exc()