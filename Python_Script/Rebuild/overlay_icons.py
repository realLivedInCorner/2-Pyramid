import os
import sys
from PIL import Image
import traceback

# 注意：log函数需要从原文件中提取
def overlay_icons(temp_dir):
    log(f"Overlaying icons image in: {temp_dir}")
    try:
        icons_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'icons.png')
        log(f"Checking for icons.png at: {icons_path}")
        # 获取脚本所在目录（假设覆盖图像在与exe同目录的villager2文件夹中）
        if getattr(sys, 'frozen', False):
            # 如果是打包后的exe
            script_dir = os.path.dirname(sys.executable)
        else:
            # 如果是脚本运行
            script_dir = os.path.dirname(os.path.abspath(__file__))
        icons_dir = os.path.join(script_dir, 'icons')

        if os.path.exists(icons_path):
            base_img = Image.open(icons_path).convert("RGBA")
            overlay_img_path = None
            if base_img.size == (256, 256):
                overlay_img_path = os.path.join(icons_dir, 'icons_256.png')
            elif base_img.size == (512, 512):
                overlay_img_path = os.path.join(icons_dir, 'icons_512.png')
            elif base_img.size == (1024, 1024):
                overlay_img_path = os.path.join(icons_dir, 'icons_1024.png')
            elif base_img.size == (2048, 2048):
                overlay_img_path = os.path.join(icons_dir, 'icons_2048.png')
            else:
                log(f"Unsupported icons.png size: {base_img.size}")
                return temp_dir

            log(f"Overlay image path: {overlay_img_path}")
            if overlay_img_path and os.path.exists(overlay_img_path):
                overlay_img = Image.open(overlay_img_path).convert("RGBA")
                base_img.paste(overlay_img, (0, 0), overlay_img)
                base_img.save(icons_path)
                log(f"Overlayed '{os.path.basename(overlay_img_path)}' onto '{icons_path}'")
            else:
                log(f"No overlay image found for size {base_img.size}")
        else:
            log(f"No 'icons.png' found in {temp_dir}")

    except Exception as e:
        log(f"Error overlaying icons image in '{temp_dir}': {e}")
        traceback.print_exc()