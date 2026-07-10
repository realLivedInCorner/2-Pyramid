import os
from PIL import Image
import traceback

# 注意：此函数依赖于外部的log函数、determine_scale_factor函数、scaled_coords函数和scaled_point函数

def process1_slider_image(gui_path):
    """
    处理 gui_path 目录中的 slider.png，生成 slider.png, slider_handle.png, slider_handle_highlighted.png
    """
    log(f"Processing slider image in: {gui_path}")
    try:
        slider_path = os.path.join(gui_path, 'slider.png')
        if not os.path.exists(slider_path):
            log(f"'slider.png' not found in {gui_path}")
            return

        img = Image.open(slider_path).convert("RGBA")
        width, height = img.size

        # 确定 scale_factor
        scale_factor, is_exact = determine_scale_factor(width, height)
        log(f"Determined scale_factor: {scale_factor} (Exact match: {is_exact})")

        # 定义缩放坐标函数
        def get_scaled_coords(x1, y1, x2, y2):
            return scaled_coords(x1, y1, x2, y2, scale_factor)

        def get_scaled_point(x, y):
            return scaled_point(x, y, scale_factor)

        # 创建目标目录
        target_dir = os.path.join(gui_path, 'sprites', 'widget')
        os.makedirs(target_dir, exist_ok=True)
        log(f"Ensured target directory exists: {target_dir}")

        # Step 1: 生成 slider.png
        try:
            source_box_slider = get_scaled_coords(0, 0, 200, 20)
            slider_img = img.crop(source_box_slider)
            log(f"Copied region {source_box_slider} for slider.png")

            # 保存 slider.png 到目标目录
            slider_output_path = os.path.join(target_dir, 'slider.png')
            slider_img.save(slider_output_path)
            log(f"Saved 'slider.png' to {slider_output_path}")
        except Exception as e:
            log(f"Error processing 'slider.png': {e}")
            traceback.print_exc()

        # Step 2: 生成 slider_handle.png
        try:
            # 左侧区域 (0,40)-(4,60)
            source_box_handle_left = get_scaled_coords(0, 40, 4, 60)
            handle_left = img.crop(source_box_handle_left)
            log(f"Copied left handle region {source_box_handle_left}")

            # 右侧区域 (196,40)-(200,60)
            source_box_handle_right = get_scaled_coords(196, 40, 200, 60)
            handle_right = img.crop(source_box_handle_right)
            log(f"Copied right handle region {source_box_handle_right}")

            # 拼接左右区域
            handle_width, handle_height = handle_left.size
            slider_handle_img = Image.new("RGBA", (handle_width * 2, handle_height))
            slider_handle_img.paste(handle_left, (0, 0))
            slider_handle_img.paste(handle_right, (handle_width, 0))
            log(f"Created 'slider_handle.png' by concatenating left and right handle regions")

            # 保存 slider_handle.png 到目标目录
            slider_handle_output_path = os.path.join(target_dir, 'slider_handle.png')
            slider_handle_img.save(slider_handle_output_path)
            log(f"Saved 'slider_handle.png' to {slider_handle_output_path}")
        except Exception as e:
            log(f"Error processing 'slider_handle.png': {e}")
            traceback.print_exc()

        # Step 3: 生成 slider_handle_highlighted.png
        try:
            # 左侧区域 (0,60)-(4,80)
            source_box_highlight_left = get_scaled_coords(0, 60, 4, 80)
            highlight_left = img.crop(source_box_highlight_left)
            log(f"Copied left highlighted handle region {source_box_highlight_left}")

            # 右侧区域 (196,60)-(200,80)
            source_box_highlight_right = get_scaled_coords(196, 60, 200, 80)
            highlight_right = img.crop(source_box_highlight_right)
            log(f"Copied right highlighted handle region {source_box_highlight_right}")

            # 拼接左右区域
            highlight_width, highlight_height = highlight_left.size
            slider_handle_highlighted_img = Image.new("RGBA", (highlight_width * 2, highlight_height))
            slider_handle_highlighted_img.paste(highlight_left, (0, 0))
            slider_handle_highlighted_img.paste(highlight_right, (highlight_width, 0))
            log(f"Created 'slider_handle_highlighted.png' by concatenating left and right highlighted handle regions")

            # 保存 slider_handle_highlighted.png 到目标目录
            slider_handle_highlighted_output_path = os.path.join(target_dir, 'slider_handle_highlighted.png')
            slider_handle_highlighted_img.save(slider_handle_highlighted_output_path)
            log(f"Saved 'slider_handle_highlighted.png' to {slider_handle_highlighted_output_path}")
        except Exception as e:
            log(f"Error processing 'slider_handle_highlighted.png': {e}")
            traceback.print_exc()

    except Exception as e:
        log(f"Error processing slider image in '{gui_path}': {e}")
        traceback.print_exc()