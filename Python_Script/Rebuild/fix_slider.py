import os
from PIL import Image
import traceback

# 注意：此函数依赖于外部的log函数和determine_scale_factor函数

def fix_slider(temp_dir):
    """
    处理 gui_path 目录中的 widgets.png，生成 slider.png。
    """
    log(f"Processing slider image in: {temp_dir}")
    try:
        gui_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui')
        widgets_path = os.path.join(gui_path, 'widgets.png')
        slider_path = os.path.join(gui_path, 'slider.png')

        if not os.path.exists(widgets_path):
            log(f"'widgets.png' not found in {gui_path}")
            return

        # 打开 widgets.png 并获取尺寸
        img = Image.open(widgets_path).convert("RGBA")
        width, height = img.size

        # 确定 scale_factor
        scale_factor, is_exact = determine_scale_factor(width, height)
        log(f"Determined scale_factor: {scale_factor} (Exact match: {is_exact})")

        def scaled_coords(x1, y1, x2, y2):
            """
    根据 scale_factor 缩放坐标。
    """
            return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

        def scaled_point(x, y):
            """
    根据 scale_factor 缩放单点坐标。
    """
            return (x * scale_factor, y * scale_factor)

        # 创建一个全透明的新图像
        slider_img = Image.new("RGBA", (width, height), (0, 0, 0, 0))

        # 步骤1: 将 (0,46)-(200,66) 复制到 (0,0)-(200,20)
        source_box1 = scaled_coords(0, 46, 200, 66)
        dest_box1 = scaled_coords(0, 0, 200, 20)
        region1 = img.crop(source_box1)
        slider_img.paste(region1, (dest_box1[0], dest_box1[1]))
        log(f"Copied region {source_box1} to {dest_box1}")

        # 步骤2: 将 (0,46)-(200,106) 复制到 (0,20)-(200,80)
        source_box2 = scaled_coords(0, 46, 200, 106)
        dest_box2 = scaled_coords(0, 20, 200, 80)
        region2 = img.crop(source_box2)
        slider_img.paste(region2, (dest_box2[0], dest_box2[1]))
        log(f"Copied region {source_box2} to {dest_box2}")

        # 保存 slider.png 到 gui_path
        slider_img.save(slider_path)
        log(f"Saved 'slider.png' in {gui_path}")

    except Exception as e:
        log(f"Error processing slider image in '{gui_path}': {e}")
        traceback.print_exc()