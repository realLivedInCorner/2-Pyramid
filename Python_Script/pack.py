"""
注释由AI生成（未审查）
"""

import colorsys
import io
import os
import tempfile
import threading
import sys
import zipfile
import shutil
import math
from PIL import Image, ImageEnhance, ImageDraw, ImageTk, ImageOps, ImageFont
import json
import re
import traceback
import tkinter as tk
from tkinter import ttk
from tkinter import filedialog, messagebox, simpledialog, colorchooser
from tkinterdnd2 import TkinterDnD, DND_FILES, DND_TEXT
import pyperclip
from tkinter import font as tkfont
import random
import string
import subprocess
import logging
import datetime
import struct
import tarfile
import gzip
import bz2


# 设置日志级别为 DEBUG，记录所有级别的日志
logging.basicConfig(level=logging.DEBUG,  # 设置日志级别为DEBUG
                    format='%(asctime)s - %(levelname)s - %(message)s')  # 设置日志输出格式

current_scale = 1.0 
selected_zip_path = None
selected_color = (0, 0, 0)  # 初始化为黑色，可根据需要修改默认值
current_hue = 0  # 当前色相，用于色相条的选择
pen_color = (0, 0, 0)  # 全局变量铅笔颜色
img_copy = None
zoom_value = 1# 放大倍数滑动条值
axe_images = ["diamond_axe.png", "golden_axe.png", "iron_axe.png", "stone_axe.png", "wooden_axe.png"]  # 斧子图片
current_image_index = 0  # 初始化当前显示的图片索引
golden_axe_saved = False
saved_images = {}  # 保存已保存图片路径的字典

# 在代码的全局区域增加两个独立的工具栏容器与状态：
left_tool_lines_container = None
left_tool_lines_container_visible = [False]

right_tool_lines_container = None
right_tool_lines_container_visible = [False]

result_label_exists = False
new_button_exists = False
result_label = None
select_button = None
new_button = None

# 初始化全局变量
export_path = None
app = None

# 定义为全局变量
overlay_frame = None 

# 定义 log 函数
def log(message):
    print(message)
    # 同时将日志写入文件
    log_file_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'conversion.log')
    try:
        with open(log_file_path, 'a', encoding='utf-8') as log_file:
            timestamp = datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S')
            log_file.write(f'[{timestamp}] {message}\n')
    except Exception as e:
        print(f"Error writing to log file: {e}")


def detect_file_format(file_path):
    """
    检测文件格式
    """
    try:
        with open(file_path, 'rb') as f:
            header = f.read(512)  # 读取完整的TAR头部（512字节）
            
        # 检测文件格式
        if header.startswith(b'PK'):
            return 'ZIP'
        elif header.startswith(b'Rar!'):
            return 'RAR'
        elif header.startswith(b'7z\xbc\xaf\x27\x1c'):
            return '7Z'
        elif header.startswith(b'\x1f\x8b'):
            return 'GZ'
        elif header.startswith(b'BZh'):
            return 'BZ2'
        else:
            # 尝试通过其他方式检测TAR格式
            # TAR文件通常以文件头部开始，每个头部512字节
            # 检查是否看起来像TAR格式
            if len(header) >= 512:
                # TAR头部格式：前100字节是文件名，257-262字节是magic "ustar"
                try:
                    # 检查magic字符串位置 (ustar\0)
                    if header[257:263] == b'ustar\x00':
                        return 'TAR'
                    # 另一种检查方式：检查标准的ustar magic
                    elif header[257:265] == b'ustar\x00  ':
                        return 'TAR'
                except:
                    pass
            
            return 'UNKNOWN'
            
    except Exception as e:
        log(f"检测文件格式失败: {str(e)}")
        return 'UNKNOWN'


def convert_to_zip(input_file_path):
    """
    将非ZIP格式的文件转换为ZIP格式
    支持: RAR, 7Z, TAR, GZ, BZ2等格式
    """
    import tempfile
    import shutil
    import subprocess
    import os
    import tarfile
    import gzip
    import bz2
    import zipfile
    
    try:
        log(f"检测到非ZIP格式文件，开始转换: {input_file_path}")
        
        # 首先使用文件格式检测来判断文件真实格式
        detected_format = detect_file_format(input_file_path)
        log(f"检测到的文件格式: {detected_format}")
        
        # 如果检测结果是ZIP格式，检查扩展名
        if detected_format == 'ZIP':
            file_ext = os.path.splitext(input_file_path)[1].lower()
            if file_ext == '.zip':
                log(f"文件已经是ZIP格式，无需转换")
                return input_file_path, False
            else:
                # 检测为ZIP但扩展名不是.zip，可能是误检，继续处理
                log(f"文件检测为ZIP但扩展名不符，继续转换过程")
        
        # 如果检测为未知格式，也继续转换过程
        file_ext = os.path.splitext(input_file_path)[1].lower()
        
        # 创建临时目录用于转换
        temp_dir = tempfile.mkdtemp(prefix='mcpack_convert_', suffix='_temp')
        output_zip_path = os.path.join(temp_dir, 'converted_pack.zip')
        
        log(f"创建临时转换目录: {temp_dir}")
        log(f"输出ZIP路径: {output_zip_path}")
        
        # 尝试不同的解压方法
        extraction_success = False
        
        # 方法1: 尝试使用7zip (如果有的话)
        try:
            # 常见7z可执行文件路径
            seven_zip_paths = [
                '7z', '7za',  # 命令行版本
                r'C:\Program Files\7-Zip\7z.exe',
                r'C:\Program Files (x86)\7-Zip\7z.exe',
                r'C:\Tools\7-Zip\7z.exe',
                r'C:\7-Zip\7z.exe'
            ]
            
            for seven_zip_path in seven_zip_paths:
                try:
                    # 检查7z是否可用
                    subprocess.run([seven_zip_path, '--help'], 
                                 capture_output=True, check=True, timeout=5)
                    
                    log(f"找到7z工具: {seven_zip_path}")
                    
                    # 使用7z解压到临时目录
                    result = subprocess.run([
                        seven_zip_path, 'x', input_file_path, 
                        f'-o{temp_dir}', '-y'
                    ], capture_output=True, text=True, timeout=60)
                    
                    if result.returncode == 0:
                        log("7z解压成功")
                        extraction_success = True
                        break
                    else:
                        log(f"7z解压失败: {result.stderr}")
                        
                except (subprocess.CalledProcessError, FileNotFoundError, subprocess.TimeoutExpired):
                    continue
                    
        except Exception as e:
            log(f"7z解压尝试失败: {str(e)}")
        
        # 方法2: 尝试使用Python内置解压功能 (扩展版)
        if not extraction_success:
            try:
                log("尝试使用Python内置解压功能")
                
                # 检测文件格式并使用相应的Python库
                with open(input_file_path, 'rb') as f:
                    header = f.read(16)
                
                if file_ext in ['.gz', '.tgz']:
                    log("处理GZ/TGZ格式...")
                    # 解压gz文件
                    temp_gz_dir = tempfile.mkdtemp(prefix='mcpack_gz_', suffix='_temp')
                    try:
                        with gzip.open(input_file_path, 'rb') as f_in:
                            with open(os.path.join(temp_gz_dir, 'extracted'), 'wb') as f_out:
                                shutil.copyfileobj(f_in, f_out)
                        
                        # 如果是.tar.gz，提取tar文件
                        if os.path.splitext(input_file_path)[1] == '.gz' and not os.path.basename(input_file_path).endswith('.tar.gz'):
                            # 可能是单独的gz文件，移动到temp_dir
                            extracted_path = os.path.join(temp_gz_dir, 'extracted')
                            if os.path.exists(extracted_path):
                                shutil.move(extracted_path, os.path.join(temp_dir, 'extracted.gz'))
                            shutil.rmtree(temp_gz_dir)
                        else:
                            # 是tar.gz文件，提取tar
                            extracted_path = os.path.join(temp_gz_dir, 'extracted')
                            if os.path.exists(extracted_path):
                                with tarfile.open(extracted_path, 'r') as tar_ref:
                                    tar_ref.extractall(temp_dir)
                                log("tar.gz解压成功")
                                extraction_success = True
                            shutil.rmtree(temp_gz_dir)
                    except Exception as e:
                        log(f"GZ解压失败: {str(e)}")
                        shutil.rmtree(temp_gz_dir, ignore_errors=True)
                        
                elif file_ext in ['.bz2', '.tbz2']:
                    log("处理BZ2/TBZ2格式...")
                    # 解压bz2文件
                    temp_bz2_dir = tempfile.mkdtemp(prefix='mcpack_bz2_', suffix='_temp')
                    try:
                        with bz2.open(input_file_path, 'rb') as f_in:
                            with open(os.path.join(temp_bz2_dir, 'extracted'), 'wb') as f_out:
                                shutil.copyfileobj(f_in, f_out)
                        
                        # 如果是.tar.bz2，提取tar文件
                        if os.path.splitext(input_file_path)[1] == '.bz2' and not os.path.basename(input_file_path).endswith('.tar.bz2'):
                            extracted_path = os.path.join(temp_bz2_dir, 'extracted')
                            if os.path.exists(extracted_path):
                                shutil.move(extracted_path, os.path.join(temp_dir, 'extracted.bz2'))
                            shutil.rmtree(temp_bz2_dir)
                        else:
                            extracted_path = os.path.join(temp_bz2_dir, 'extracted')
                            if os.path.exists(extracted_path):
                                with tarfile.open(extracted_path, 'r') as tar_ref:
                                    tar_ref.extractall(temp_dir)
                                log("tar.bz2解压成功")
                                extraction_success = True
                            shutil.rmtree(temp_bz2_dir)
                    except Exception as e:
                        log(f"BZ2解压失败: {str(e)}")
                        shutil.rmtree(temp_bz2_dir, ignore_errors=True)
            except Exception as e:
                log(f"Python内置解压功能失败: {str(e)}")
        
        # 如果解压成功，创建ZIP文件
        if extraction_success:
            log("开始创建ZIP文件...")
            
            # 确定ZIP文件的最终输出路径
            base_name = os.path.splitext(input_file_path)[0]
            final_zip_path = f"{base_name}.zip"
            
            # 遍历临时目录，将所有文件添加到ZIP中
            with zipfile.ZipFile(final_zip_path, 'w', zipfile.ZIP_DEFLATED) as zipf:
                for root, dirs, files in os.walk(temp_dir):
                    for file in files:
                        file_path = os.path.join(root, file)
                        # 获取文件在ZIP中的相对路径
                        arcname = os.path.relpath(file_path, temp_dir)
                        zipf.write(file_path, arcname)
            
            log(f"ZIP文件创建成功: {final_zip_path}")
            
            # 清理临时目录
            shutil.rmtree(temp_dir, ignore_errors=True)
            
            return final_zip_path, True
        else:
            log(f"无法解压文件: {input_file_path}")
            shutil.rmtree(temp_dir, ignore_errors=True)
            return None, False
            
    except Exception as e:
        log(f"转换文件格式时出错: {str(e)}")
        traceback.print_exc()
        return None, False


def set_default_theme(root):
    default_font = ("微软雅黑", 14)
    root.option_add("*Font", default_font)

def show_main_menu():
    clear_frame()
    option_frame = tk.Frame(frame, bg="#f0f0f0")
    option_frame.pack(pady=10)  # 减少外部间距

    # 转换材质包按钮
    convert_button = tk.Button(option_frame, text="转换材质包版本", command=show_conversion_options, bg="#4CAF50", fg="white", font=("微软雅黑", 14), padx=20, pady=8)
    convert_button.pack(pady=5)  # 减少按钮间距

    # 新增制作覆盖包按钮
    overlay_button = tk.Button(option_frame, text="制作你的覆盖包", command=show_overlay_options, bg="#4CAF50", fg="white", font=("微软雅黑", 14), padx=20, pady=8)
    overlay_button.pack(pady=5)  # 减少按钮间距

def open_file_location(path):
    if os.path.isfile(path):
        path = os.path.dirname(path)
    if sys.platform == "win32":
        # 跨平台打开文件或文件夹
        import platform, subprocess
        system_type = platform.system()
        
        try:
            if system_type == 'Windows':
                os.startfile(path)
            elif system_type == 'Darwin':  # macOS
                subprocess.run(['open', path], check=True)
            else:  # Linux和其他Unix-like系统
                subprocess.run(['xdg-open', path], check=True)
        except Exception as e:
            print(f"打开失败: {str(e)}")
    elif sys.platform == "darwin":
        subprocess.call(["open", path])
    else:
        subprocess.call(["xdg-open", path])

def start_processing_conversion(pack_format2, progress_callback=None, file_paths=None, fix_alpha_layers=False):
    """
    开始处理材质包转换
    """
    import time
    start_time = time.time()
    
    # 自动获取源版本的 pack_format
    if file_paths is None:
        file_paths = selected_files.get()
    processed_files = []
    all_files_to_process = []
    # 存储文件与原始拖放路径的映射关系
    file_to_parent_folder = {}
    # 存储文件的结构修复状态
    file_structure_fixed = {}
    
    # 收集所有要处理的文件（包括文件夹中的ZIP和RAR文件）
    for path in file_paths:
        if os.path.isfile(path) and path.lower().endswith(('.zip', '.rar')):
            # 单个压缩文件
            all_files_to_process.append(path)
            file_to_parent_folder[path] = None  # 单个文件没有父文件夹
        elif os.path.isdir(path):
            # 文件夹，添加其中所有的压缩文件
            for root, dirs, files in os.walk(path):
                for file in files:
                    if file.lower().endswith(('.zip', '.rar')):
                        zip_path = os.path.join(root, file)
                        all_files_to_process.append(zip_path)
                        file_to_parent_folder[zip_path] = path  # 记录文件来自哪个拖放的文件夹
    
    if not all_files_to_process:
        raise ValueError("没有找到可处理的材质包文件（.zip或.rar格式）")
    
    log(f"开始批量处理 {len(all_files_to_process)} 个材质包文件")
    
    # 计算每个文件的进度权重
    file_count = len(all_files_to_process)
    file_weight = 1.0 / file_count if file_count > 0 else 0
    
    current_file_index = 0
    
    # 逐个处理每个文件
    for file_path in all_files_to_process:
        try:
            current_file_index += 1
            log(f"开始处理文件 {current_file_index}/{file_count}: {file_path}")
            
            # 解压文件，如有错误会在extract_zip中抛出
            temp_dir, structure_fixed = extract_zip(file_path)
            if not temp_dir:
                log(f"文件解压失败：{os.path.basename(file_path)}")
                # 更新进度，跳过当前文件
                if progress_callback:
                    base_progress = (current_file_index - 1) * file_weight * 100
                    progress_callback(int(base_progress + file_weight * 100))
                continue
            
            # 记录文件的结构修复状态
            file_structure_fixed[file_path] = structure_fixed
            
            # 自动读取 pack.mcmeta 获取 pack_format1
            pack_format1 = get_pack_format(temp_dir)
            if pack_format1 == "未知版本":
                log(f"版本识别失败：{os.path.basename(file_path)}")
                # 更新进度，跳过当前文件
                if progress_callback:
                    base_progress = (current_file_index - 1) * file_weight * 100
                    progress_callback(int(base_progress + file_weight * 100))
                continue

            # 如果启用了全物品贴图图层修复，执行修复操作
            if fix_alpha_layers:
                log(f"开始执行全物品贴图图层修复: {os.path.basename(file_path)}")
                fix_alpha_layers_in_textures(temp_dir)
                log(f"全物品贴图图层修复完成: {os.path.basename(file_path)}")

            # 为当前文件创建进度回调函数，考虑文件在总进度中的位置
            base_progress = (current_file_index - 1) * file_weight * 100
            
            def file_progress_callback(file_progress, base_progress=base_progress, file_weight=file_weight):
                # 计算整体进度：基础进度 + 文件内部进度 * 文件权重
                overall_progress = base_progress + (file_progress * file_weight)
                if progress_callback:
                    progress_callback(int(overall_progress))
            
            # 获取当前文件对应的父文件夹路径
            parent_folder_path = file_to_parent_folder.get(file_path)
            
            # 调用 process_zip 处理文件，传递父文件夹路径
            processed_zip_path = process_zip(temp_dir, file_path, pack_format1, pack_format2, file_progress_callback, parent_folder_path=parent_folder_path)
            if not processed_zip_path:
                log(f"转换处理失败：{os.path.basename(file_path)}")
                # 更新进度，跳过当前文件
                if progress_callback:
                    progress_callback(int(base_progress + file_weight * 100))
                continue
            
            processed_files.append(processed_zip_path)
            log(f"成功处理: {processed_zip_path}")
            
        except Exception as e:
            # 记录单个文件处理失败的异常，继续处理下一个文件
            log(f"处理文件 {os.path.basename(file_path)} 时出错: {str(e)}")
            # 出错时也更新进度，避免进度条卡住
            if progress_callback:
                base_progress = (current_file_index - 1) * file_weight * 100
                progress_callback(int(base_progress + file_weight * 100))
            continue
    
    if not processed_files:
        raise RuntimeError("所有文件处理失败，请检查日志获取详细信息")
    
    # 计算转换时间
    end_time = time.time()
    elapsed_time = end_time - start_time
    
    # 设置全局变量以便UI可以访问处理结果
    global global_last_processed_files
    global_last_processed_files = processed_files
    
    log(f"批量处理完成，成功转换 {len(processed_files)} 个文件，耗时: {elapsed_time:.2f} 秒")
    # 不调用display_multiple_results，返回处理结果给调用者
    return processed_files, file_structure_fixed

def get_pack_format(temp_dir):
    """
    从 pack.mcmeta 中自动读取 pack_format，使用字符串搜索方式以兼容不规范的JSON格式，支持递归搜索子目录
    """
    import re
    import os
    
    # 首先在根目录查找
    pack_meta_path = os.path.join(temp_dir, 'pack.mcmeta')
    
    # 如果根目录没有找到，递归搜索整个temp_extract目录
    if not os.path.exists(pack_meta_path):
        log(f"根目录未找到 pack.mcmeta 文件，开始递归搜索整个目录: {temp_dir}")
        for root, dirs, files in os.walk(temp_dir):
            if 'pack.mcmeta' in files:
                pack_meta_path = os.path.join(root, 'pack.mcmeta')
                log(f"在子目录中找到 pack.mcmeta 文件: {pack_meta_path}")
                break
    
    if os.path.exists(pack_meta_path):
        try:
            # 尝试直接读取文件内容，不依赖完整的JSON解析
            with open(pack_meta_path, 'r', encoding='utf-8') as f:
                content = f.read()
        except UnicodeDecodeError:
            log("UTF-8 解码失败，尝试使用 ISO-8859-1")
            try:
                with open(pack_meta_path, 'r', encoding='iso-8859-1') as f:
                    content = f.read()
            except Exception as e:
                log(f"读取文件失败: {e}")
                return "未知版本"
        except Exception as e:
            log(f"读取文件失败: {e}")
            return "未知版本"

        # 使用正则表达式搜索 pack_format 后面的数字值
        # 匹配 "pack_format": 数字 或 'pack_format': 数字 格式
        match = re.search(r'["\']pack_format["\']\s*:\s*(\d+)', content)
        if match:
            try:
                pack_format = int(match.group(1))
                log(f"成功提取 pack_format: {pack_format}")
                return pack_format
            except ValueError:
                log("提取的 pack_format 不是有效数字")
                return "未知版本"
        else:
            log(f"在文件中未找到 pack_format 字段: {pack_meta_path}")
            return "未知版本"
    else:
        log(f"未找到 pack.mcmeta 文件: {temp_dir}")
        return "未知版本"

PACK_FORMAT_MAP = {
    1: "Java 1.6-1.8",
    2: "Java 1.9-1.10",
    3: "Java 1.11-1.12",
    4: "Java 1.13-1.14",
    5: "Java 1.15-1.16.1",
    6: "Java 1.16.2-1.16.5",
    7: "Java 1.17",
    8: "Java 1.18",
    9: "Java 1.19-1.19.2",
    12: "Java 1.19.3",
    13: "Java 1.19.4",
    15: "Java 1.20-1.20.1",
    18: "Java 1.20.2",
    22: "Java 1.20.3-1.20.4",
    32: "Java 1.20.5-1.20.6",
    34: "Java 1.21-1.21.1",
    42: "Java 1.21.2-1.21.3",
    46: "Java 1.21.4",
    55: "Java 1.21.5",
    63: "Java 1.21.6",
    64: "Java 1.21.7-1.21.8",
    69: "Java 1.21.9-1.21.10",
    75: "Java 1.21.11",
    1000: "Bedrock Latest",
    
}

PACK_FORMAT_MAP2 = {
    1: "Java 1.8",
    2: "Java 1.9-1.10",
    3: "Java 1.11-1.12",
    4: "Java 1.13-1.14",
    5: "Java 1.15-1.16.1",
    6: "Java 1.16.2-1.16.5",
    7: "Java 1.17",
    8: "Java 1.18",
    9: "Java 1.19-1.19.2",
    12: "Java 1.19.3",
    13: "Java 1.19.4",
    15: "Java 1.20-1.20.1",
    18: "Java 1.20.2",
    22: "Java 1.20.3-1.20.4",
    32: "Java 1.20.5-1.20.6",
    34: "Java 1.21-1.21.1",
    42: "Java 1.21.2-1.21.3",
    46: "Java 1.21.4",
    55: "Java 1.21.5",
    63: "Java 1.21.6",
    64: "Java 1.21.7-1.21.8",
    69: "Java 1.21.9-1.21.10",
    75: "Java 1.21.11",
    1000: "Bedrock Latest",
}

# 创建版本字符串到 pack_format 的反向映射
VERSION_TO_PACK_FORMAT_MAP = {v: k for k, v in PACK_FORMAT_MAP.items()}

def show_conversion_options():

    clear_frame()

    global overlay_frame  # 声明使用全局变量
    overlay_frame = tk.Frame(frame, bg="#f0f0f0")
    overlay_frame.pack(pady=10)  # 减少外部间距

    # 材质包选择按钮
    select_button = tk.Button(frame, text="请将材质包拖拽到此处或点击此处选择材质包", 
                              command=select_files, bg="#008CBA", fg="white", 
                              font=("微软雅黑", 14), padx=20, pady=10)
    select_button.pack(pady=20)

    # 目标版本下拉菜单选项
    target_versions = [
        "1.6-1.8", "1.9-1.10", "1.11-1.12", "1.13-1.14", "1.15-1.16.1", "1.16.2-1.16.5", 
        "1.17", "1.18", "1.19-1.19.2", "1.19.3", "1.19.4", "1.20-1.20.1", 
        "1.20.2", "1.20.3-1.20.4", "1.20.5-1.20.6", "1.21-1.21.1", 
        "1.21.2-1.21.3", "1.21.4", "1.21.5", "1.21.6", "1.21.7-1.21.8"
    ]
    target_var = tk.StringVar(frame)
    target_var.set(target_versions[0])  # 默认选择

    target_label = tk.Label(frame, text="选择目标版本:", font=("微软雅黑", 14))
    target_label.pack(pady=(10, 0))
    target_menu = tk.OptionMenu(frame, target_var, *target_versions)
    target_menu.pack(pady=5)

    # 全物品贴图图层修复复选框
    global fix_alpha_layers_var
    fix_alpha_layers_var = tk.BooleanVar(frame)
    fix_alpha_layers_var.set(False)  # 默认不启用

    fix_alpha_layers_checkbox = tk.Checkbutton(
        frame, 
        text="启用全物品贴图图层修复 (Beta)", 
        variable=fix_alpha_layers_var, 
        font=("微软雅黑", 12),
        bg="#f0f0f0"
    )
    fix_alpha_layers_checkbox.pack(pady=(10, 5))

    # 性能警告提示
    warning_label = tk.Label(
        frame, 
        text="⚠️ 注意：此功能可能会导致严重的性能问题，延长转换时间", 
        font=("微软雅黑", 10), 
        fg="#ff6b6b",
        bg="#f0f0f0"
    )
    warning_label.pack(pady=(0, 10))

    # 开始制作按钮
    start_button = tk.Button(frame, text="开始制作", command=lambda: start_conversion(target_var.get()), 
                             bg="#008CBA", fg="white", font=("微软雅黑", 14), padx=20, pady=10)
    start_button.pack(pady=20)

    # 返回主页按钮
    back_button = tk.Button(frame, text="返回主页", command=show_main_menu, 
                             bg="#4CAF50", fg="white", font=("微软雅黑", 14), padx=20, pady=10)
    back_button.pack(pady=10)


def start_bedrock_conversion(file_paths, progress_callback=None):
    """
    执行基岩版转换流程
    """
    import time
    start_time = time.time()
    
    try:
        from bedrock_converter import BedrockConverterInterface as BedrockConverter
    except ImportError as e:
        raise ImportError(f"无法导入Python基岩版转换器: {e}")
    
    converter = BedrockConverter()
    processed_files = []
    all_files_to_process = []
    
    # 收集所有要处理的文件（包括文件夹中的ZIP文件）
    for path in file_paths:
        if os.path.isfile(path) and path.lower().endswith('.zip'):
            all_files_to_process.append(path)
        elif os.path.isdir(path):
            # 文件夹，添加其中所有的ZIP文件
            for root, dirs, files in os.walk(path):
                for file in files:
                    if file.lower().endswith('.zip'):
                        zip_path = os.path.join(root, file)
                        all_files_to_process.append(zip_path)
    
    if not all_files_to_process:
        raise ValueError("没有找到可处理的材质包文件（.zip或.rar格式）")
    
    log(f"开始基岩版批量转换 {len(all_files_to_process)} 个材质包文件")
    
    # 计算每个文件的进度权重
    file_count = len(all_files_to_process)
    file_weight = 1.0 / file_count if file_count > 0 else 0
    
    current_file_index = 0
    
    # 逐个处理每个文件
    for file_path in all_files_to_process:
        try:
            current_file_index += 1
            log(f"开始处理文件 {current_file_index}/{file_count}: {file_path}")
            
            # 为当前文件创建进度回调函数
            base_progress = (current_file_index - 1) * file_weight * 100
            
            def file_progress_callback(file_progress, base_progress=base_progress, file_weight=file_weight):
                # 计算Java转换到1.21.11的进度（0-50%的文件权重部分）
                java_progress = base_progress + (file_progress * 0.5 * file_weight)
                if progress_callback:
                    progress_callback(int(java_progress))
            
            def bedrock_progress_callback(bedrock_progress, base_progress=base_progress, file_weight=file_weight):
                # 计算基岩版转换的进度（50-100%的文件权重部分）
                overall_progress = base_progress + 50 * file_weight + (bedrock_progress * 0.5 * file_weight)
                if progress_callback:
                    progress_callback(int(overall_progress))
            
            # 1. 先将材质包转换到Java 1.21.11（pack_format=75）
            log(f"开始将材质包转换到Java 1.21.11: {file_path}")
            java_1_21_11_format = 75
            # 使用临时列表传递单个文件
            temp_files = [file_path]
            # 执行Java 1.21.11转换
            java_converted_files, java_structure_fixed = start_processing_conversion(java_1_21_11_format, file_progress_callback, temp_files)
            
            if not java_converted_files:
                log(f"Java 1.21.11转换失败: {file_path}")
                if progress_callback:
                    progress_callback(int(base_progress + file_weight * 100))
                continue
            
            # 获取转换后的Java 1.21.11文件路径
            converted_java_file = java_converted_files[0]
            log(f"Java 1.21.11转换成功: {converted_java_file}")
            
            # 2. 再将转换后的Java 1.21.11文件转换为基岩版
            log(f"开始将Java 1.21.11材质包转换为基岩版: {converted_java_file}")
            success, result = converter.convert_to_bedrock(converted_java_file, progress_callback=bedrock_progress_callback)
            
            # 无论转换成功与否，都删除中间的Java 1.21.11文件
            try:
                if os.path.exists(converted_java_file):
                    os.remove(converted_java_file)
                    log(f"已删除中间Java 1.21.11文件: {converted_java_file}")
            except Exception as e:
                log(f"删除中间文件失败: {e}")
            
            if success:
                # 重命名输出文件，去掉[Java 1.21.11]前缀
                import re
                
                # 获取原始输入文件的名称
                original_name = os.path.basename(file_path)
                original_name = os.path.splitext(original_name)[0]  # 去掉扩展名
                
                # 构建新的输出文件名
                output_dir = os.path.dirname(result)
                new_output_name = f"[Bedrock-Latest]{original_name}.mcpack"
                new_output_path = os.path.join(output_dir, new_output_name)
                
                # 重命名文件
                os.rename(result, new_output_path)
                
                processed_files.append(new_output_path)
                log(f"基岩版转换成功: {new_output_path}")
            else:
                log(f"基岩版转换失败: {result}")
                # 更新进度，跳过当前文件
                if progress_callback:
                    progress_callback(int(base_progress + file_weight * 100))
                continue
            
        except Exception as e:
            # 记录单个文件处理失败的异常，继续处理下一个文件
            log(f"处理文件 {os.path.basename(file_path)} 时出错: {str(e)}")
            # 出错时也更新进度，避免进度条卡住
            if progress_callback:
                base_progress = (current_file_index - 1) * file_weight * 100
                progress_callback(int(base_progress + file_weight * 100))
            continue
    
    if not processed_files:
        raise RuntimeError("所有文件基岩版转换失败，请检查日志获取详细信息")
    
    # 计算转换时间
    end_time = time.time()
    elapsed_time = end_time - start_time
    
    # 设置全局变量以便UI可以访问处理结果
    global global_last_processed_files
    global_last_processed_files = processed_files
    
    log(f"基岩版批量转换完成，成功转换 {len(processed_files)} 个文件，耗时: {elapsed_time:.2f} 秒")
    return processed_files

def start_conversion(target_version, progress_callback=None):
    """
    开始材质包转换流程。
    """
    pack_format2 = VERSION_TO_PACK_FORMAT_MAP.get(target_version)
    
    if pack_format2 is None:
        # 更友好的错误消息
        supported_versions = "、".join(VERSION_TO_PACK_FORMAT_MAP.keys())
        raise ValueError(f"不支持的目标版本：'{target_version}'\n\n请选择以下支持的版本之一：\n{supported_versions}")
    
    # 获取已选择的材质包文件
    file_paths = selected_files.get()
    if not file_paths:
        # 更友好的错误消息
        raise ValueError("操作失败：请先选择至少一个材质包文件再进行转换")
    
    # 获取是否启用全物品贴图图层修复
    global fix_alpha_layers_var
    fix_alpha_layers = fix_alpha_layers_var.get() if fix_alpha_layers_var else False
    
    # 执行转换，不捕获异常，让异常向上传播
    processed_files, file_structure_fixed = start_processing_conversion(pack_format2, progress_callback, file_paths, fix_alpha_layers)
    
    # 设置全局变量以便UI可以访问处理结果
    global global_last_processed_files
    global_last_processed_files = processed_files
    
    log(f"Java版批量转换完成，成功转换 {len(processed_files)} 个文件")
    return processed_files

def select_export_path(label):
    """
    全局函数：选择导出路径并更新相应的标签。
    """
    global export_path
    selected_path = filedialog.askdirectory(title="选择导出路径")
    if selected_path:
        export_path = selected_path
        label.config(text=export_path)
        logging.info(f"选择了导出路径: {export_path}")  # 调试信息
    else:
        label.config(text="未选择导出路径")
        logging.info("未选择导出路径")  # 调试信息

def update_overlay_options_ui():
    global overlay_frame
    if os.path.exists(output_zip_path):
        # 跨平台打开文件夹函数
        def open_folder_cross_platform(folder_path):
            import platform, subprocess
            system_type = platform.system()
            
            try:
                if system_type == 'Windows':
                    os.startfile(folder_path)
                elif system_type == 'Darwin':  # macOS
                    subprocess.run(['open', folder_path], check=True)
                else:  # Linux和其他Unix-like系统
                    subprocess.run(['xdg-open', folder_path], check=True)
            except Exception as e:
                print(f"打开文件夹失败: {str(e)}")
        
        open_folder_button = tk.Button(overlay_frame, text="你的覆盖包放在了这里", command=lambda: open_folder_cross_platform(export_path), bg="#2196F3", fg="white", font=("微软雅黑", 14), padx=20, pady=8)
        open_folder_button.pack(pady=5)

def show_overlay_options():
    """
    显示覆盖包制作选项界面。
    """
    clear_frame()

    global overlay_frame  # 声明使用全局变量
    overlay_frame = tk.Frame(frame, bg="#f0f0f0")
    overlay_frame.pack(pady=10)  # 减少外部间距

    # 标题
    title_label = tk.Label(overlay_frame, text="制作你的覆盖包", font=("微软雅黑", 24), bg="#f0f0f0", fg="#333333")
    title_label.pack(pady=10)  # 减少标题的间距

    # 版本选择标题
    version_title_label = tk.Label(overlay_frame, text="请选择你要制作的覆盖包版本", font=("微软雅黑", 14), bg="#f0f0f0", fg="#333333")
    version_title_label.pack(pady=(10, 5))  # 调整间距

    # 目标版本下拉菜单选项
    target_versions = [
        "1.8", "1.9-1.10", "1.11-1.12", "1.13-1.14", "1.15-1.16.1", "1.16.2-1.16.5", 
        "1.17", "1.18", "1.19-1.19.2", "1.19.3", "1.19.4", "1.20-1.20.1", 
        "1.20.2", "1.20.3-1.20.4", "1.20.5-1.20.6", "1.21-1.21.1", 
        "1.21.2-1.21.3", "1.21.4", "1.21.5", "1.21.6", "1.21.7-1.21.8"
    ]
    target_var = tk.StringVar(overlay_frame)
    target_var.set(target_versions[0])  # 默认选择

    target_menu = tk.OptionMenu(overlay_frame, target_var, *target_versions)
    target_menu.pack(pady=5)

    # 改变物品大小按钮
    change_size_button = tk.Button(overlay_frame, text="改变物品大小", command=show_change_item_size, bg="#4CAF50",
                                   fg="white", font=("微软雅黑", 14), padx=20, pady=8)
    change_size_button.pack(pady=5)  # 减少按钮间距

    # 显示当前选择的导出路径
    overlay_export_path_label = tk.Label(overlay_frame, text=export_path if export_path else "未选择导出路径", font=("微软雅黑", 12), bg="#f0f0f0", fg="#333333")
    overlay_export_path_label.pack(pady=5)  # 显示路径的标签

    # 选择导出路径按钮
    export_path_button = tk.Button(overlay_frame, text="选择导出路径", command=lambda: select_export_path(overlay_export_path_label), bg="#2196F3", fg="white", font=("微软雅黑", 14), padx=20, pady=8)
    export_path_button.pack(pady=5)  # 减少按钮间距

    # 开始制作按钮
    start_button = tk.Button(overlay_frame, text="开始制作", command=lambda: start_processing_overlay(target_var.get(), export_path, sliders), bg="#008CBA",
                             fg="white", font=("微软雅黑", 14), padx=20, pady=8)
    start_button.pack(pady=10)  # 调整为适中的按钮间距

    # 返回主页按钮
    back_button = tk.Button(overlay_frame, text="返回主页", command=show_main_menu, bg="#4CAF50",
                            fg="white", font=("微软雅黑", 14), padx=20, pady=8)
    back_button.pack(pady=5)  # 减少按钮间距

def start_processing_overlay(target_version, export_path, sliders):
    global output_zip_path  # 声明 output_zip_path 为全局变量
    
    # 检查导出路径是否有效
    if not export_path:
        messagebox.showerror("错误", "请先选择导出路径！")
        return

    # 读取 mapping 文件
    try:
        with open("change_item_size/mapping.txt", "r", encoding="utf-8") as f:
            mapping = json.load(f)  # 使用 json.load 而不是 eval
    except Exception as e:
        logging.error(f"读取 mapping.txt 文件时出错: {e}")
        messagebox.showerror("错误", f"读取配置文件失败: {e}")
        return

    # 找到目标版本对应的 pack_format 值
    pack_format = None
    for key, version in PACK_FORMAT_MAP2.items():
        if version == target_version:
            pack_format = key
            break
    
    if pack_format is None:
        logging.error(f"未找到对应的pack_format: {target_version}")
        return
    
    # 创建 pack.mcmeta 文件内容
    mcmeta_content = {
        "pack": {
            "pack_format": pack_format,
            "description": "这是你的覆盖包"
        }
    }

    # 构造目标压缩包路径
    output_zip_path = os.path.join(export_path, f"[{target_version}]覆盖包.zip")

    try:
        # 创建压缩包并添加 pack.mcmeta 文件
        with zipfile.ZipFile(output_zip_path, "w") as zf:
            zf.writestr("pack.mcmeta", json.dumps(mcmeta_content, indent=4, ensure_ascii=False))
        
        logging.info(f"成功生成覆盖包: {output_zip_path}")

        # 获取滑动条的值
        for item_name, slider_values in sliders.items():
            # 使用保存的值而不是滑动条对象
            dropped_size = slider_values['dropped_size']
            third_person_size = slider_values['third_person_size']
            first_person_size = slider_values['first_person_size']
            
            # 如果任意一个值不是默认值1.00，就需要处理JSON
            if dropped_size != 1.00 or third_person_size != 1.00 or first_person_size != 1.00:
                # 在 mapping 中查找对应的 JSON 文件名
                json_file_name = None
                category_name = None
                
                # 遍历 mapping 查找物品名对应的 JSON 文件名
                for cat_name, items in mapping.items():
                    if item_name in items:
                        json_file_name = items[item_name]  # 获取对应的 JSON 文件名
                        category_name = cat_name
                        break
                
                if json_file_name is None:
                    logging.error(f"在 mapping 中未找到物品 {item_name} 的对应 JSON 文件名")
                    continue

                logging.info(f"处理物品: {item_name}, JSON文件: {json_file_name}, 类别: {category_name}")
                
                # 预定义所有可能的文件夹
                size_folders = ['e', '1', '0.5', '0.25', '0.55', '0.85', '0.375']

                # 遍历所有文件夹查找文件
                original_json_path = None
                for size_folder in size_folders:
                    temp_path = os.path.join("change_item_size", "vanilla", "1.8", size_folder, json_file_name)
                    if os.path.exists(temp_path):
                        original_json_path = temp_path
                        break
                
                if original_json_path is None:
                    logging.warning(f"{json_file_name} not found in any size folder, skipping...")
                    continue

                # 检查原始 JSON 文件是否存在
                logging.info(f"检查 {original_json_path} 是否存在...")

                if os.path.exists(original_json_path):
                    logging.info(f"找到 {original_json_path}，开始复制到目标目录。")
                    
                    # 创建目标路径的文件夹（如果不存在）
                    target_json_folder = os.path.join(export_path, "assets", "minecraft", "models", "item")
                    logging.info(f"准备创建目标文件夹: {target_json_folder}")

                    try:
                        os.makedirs(target_json_folder, exist_ok=True)
                        logging.info(f"目标文件夹 {target_json_folder} 创建成功。")
                    except Exception as e:
                        logging.error(f"创建目标文件夹时发生错误: {e}")
                        return
                    
                    target_json_path = os.path.join(target_json_folder, json_file_name)
                    shutil.copy(original_json_path, target_json_path)

                    # 读取并修改 JSON 内容
                    with open(target_json_path, "r", encoding="utf-8") as json_file:
                        json_data = json.load(json_file)

                    # 添加 ground 配置
                    if dropped_size != 1.00:
                        json_data["display"]["ground"] = {
                            "rotation": [0, 0, 0],
                            "translation": [0, 0, 0],
                            "scale": [dropped_size, dropped_size, dropped_size]
                        }
                    
                    # 处理第三人称视角的配置
                    if third_person_size != 1.00:
                        # 获取原始文件所在的文件夹名
                        folder_name = os.path.basename(os.path.dirname(original_json_path))
                        
                        # 根据文件夹确定基础比例
                        base_scale = {
                            '0.5': 0.5,
                            '0.25': 0.25,
                            '0.55': 0.55,
                            '0.85': 0.85,
                            '0.375': 0.375,
                            '1': 1.0,
                            'e': 1.0
                        }.get(folder_name, 1.0)  # 默认为1.0
                        
                        # 计算最终的缩放值
                        final_scale = base_scale * third_person_size
                        
                        # 确保display字段存在
                        if "display" not in json_data:
                            json_data["display"] = {}
                        
                        # 只更新scale值，保留原有的rotation和translation
                        if "thirdperson" in json_data["display"]:
                            json_data["display"]["thirdperson"]["scale"] = [final_scale, final_scale, final_scale]
                        else:
                            json_data["display"]["thirdperson"] = {"scale": [final_scale, final_scale, final_scale]}
                        
                        logging.info(f"已更新 {json_file_name} 的第三人称视角配置，基础比例: {base_scale}，最终比例: {final_scale}")

                    # 处理第一人称视角的配置
                    if first_person_size != 1.00:
                        # 获取原始文件所在的文件夹名
                        folder_name = os.path.basename(os.path.dirname(original_json_path))
                        
                        # 根据文件夹确定基础比例
                        base_scale = {
                            '0.5': 1.0,          # 直接使用滑动条的值
                            '0.25': 0.55,        # 乘以0.55
                            '0.55': 1.7,         # 乘以1.7
                            '0.85': 1.7,         # 乘以1.7
                            '0.375': 1.0,        # 直接使用滑动条的值
                            '1': 1.7,            # 乘以1.7
                            'e': 1.0             # 直接使用滑动条的值
                        }.get(folder_name, 1.0)  # 默认为1.0
                        
                        # 计算最终的缩放值
                        final_scale = base_scale * first_person_size
                        
                        # 确保display字段存在
                        if "display" not in json_data:
                            json_data["display"] = {}
                        
                        # 只更新scale值，保留原有的rotation和translation
                        if "firstperson" in json_data["display"]:
                            json_data["display"]["firstperson"]["scale"] = [final_scale, final_scale, final_scale]
                        else:
                            json_data["display"]["firstperson"] = {"scale": [final_scale, final_scale, final_scale]}
                        
                        logging.info(f"已更新 {json_file_name} 的第一人称视角配置，基础比例: {base_scale}，最终比例: {final_scale}")

                    # 保存修改后的 JSON 文件
                    with open(target_json_path, "w", encoding="utf-8") as json_file:
                        json.dump(json_data, json_file, indent=4, ensure_ascii=False)

                    # 添加文件到压缩包
                    with zipfile.ZipFile(output_zip_path, "a") as zf:
                        arcname = os.path.join("assets", "minecraft", "models", "item", json_file_name)
                        zf.write(target_json_path, arcname=arcname)
                        logging.info(f"添加文件到压缩包: {arcname}")
                else:
                    logging.error(f"未找到原始 JSON 文件: {original_json_path}")

        # 删除 assets 文件夹
        try:
            assets_folder_path = os.path.join(export_path, "assets")
            if os.path.exists(assets_folder_path):
                shutil.rmtree(assets_folder_path)  # 删除整个 assets 文件夹及其内容
                logging.info(f"成功删除 assets 文件夹: {assets_folder_path}")
            else:
                logging.warning(f"assets 文件夹不存在，跳过删除操作.")
        except Exception as e:
            logging.error(f"删除 assets 文件夹时发生错误: {e}")
            logging.exception(e)

        update_overlay_options_ui()

    except Exception as e:
        logging.error(f"生成覆盖包时发生错误: {e}")
        logging.exception(e)  # 记录完整的堆栈信息

def show_change_item_size():
    """
    显示掉落物的物品名字列表界面。
    """
    global sliders, item_widgets  # 添加 item_widgets 作为全局变量
    clear_frame()

    global overlay_frame
    overlay_frame = tk.Frame(frame, bg="#f0f0f0")
    overlay_frame.pack(pady=10, fill=tk.BOTH, expand=True)
    
    # 创建主容器
    main_frame = tk.Frame(overlay_frame, bg="#f0f0f0")
    main_frame.pack(fill=tk.BOTH, expand=True)

    # 搜索框放在最上方
    search_frame = tk.Frame(main_frame, bg="#f0f0f0")
    search_frame.pack(fill=tk.X, pady=(5, 0))
    
    search_label = tk.Label(search_frame, text="搜索物品", font=("微软雅黑", 12), bg="#f0f0f0", fg="#333333")
    search_label.pack(side=tk.LEFT, padx=10, pady=5)

    search_var = tk.StringVar()
    search_entry = tk.Entry(search_frame, textvariable=search_var, font=("微软雅黑", 12))
    search_entry.pack(side=tk.LEFT, padx=10, pady=5)

    # 固定表头
    header_frame = tk.Frame(main_frame, bg="#f0f0f0")
    header_frame.pack(fill=tk.X, pady=5)

    # 表头内容
    headers = ["物品", "掉落物大小", "手持物大小(第三人称)", "手持物大小(第一人称)"]

    for i, header in enumerate(headers):
        header_label = tk.Label(header_frame, text=header, font=("微软雅黑", 14), bg="#f0f0f0", fg="#333333")
        header_label.grid(row=0, column=i, padx=(60, 0), pady=5, sticky="w")

    # 创建带滚动条的画布
    canvas = tk.Canvas(main_frame, bg="#f0f0f0", highlightthickness=0)
    scrollbar = ttk.Scrollbar(main_frame, orient=tk.VERTICAL, command=canvas.yview)
    
    # 内容框架
    content_frame = tk.Frame(canvas, bg="#f0f0f0")
    content_frame.bind("<Configure>", lambda e: canvas.configure(scrollregion=canvas.bbox("all")))
    
    # 在画布上创建窗口
    canvas_window = canvas.create_window((0, 0), window=content_frame, anchor="nw")
    
    # 配置画布
    canvas.configure(yscrollcommand=scrollbar.set)
    
    # 绑定画布大小调整
    def on_canvas_configure(event):
        canvas.itemconfig(canvas_window, width=event.width)
    canvas.bind("<Configure>", on_canvas_configure)

    # 优化滚动体验
    def on_mousewheel(event):
        canvas.yview_scroll(int(-1 * (event.delta / 120)), "units")

    # 绑定滚轮事件
    canvas.bind_all("<MouseWheel>", on_mousewheel)
    
    # 布局画布和滚动条
    canvas.pack(side=tk.LEFT, fill=tk.BOTH, expand=True, padx=(5, 0))
    scrollbar.pack(side=tk.RIGHT, fill=tk.Y)

    sliders = {}
    item_widgets = {}
    category_frames = {}  # 存储每个类别的框架
    category_states = {}  # 存储每个类别的展开/折叠状态
    category_buttons = {}  # 存储类别按钮的引用

    # 加载数据
    try:
        with open("change_item_size/mapping.txt", "r", encoding="utf-8") as f:
            mapping = json.load(f)  # 使用 json.load() 读取 JSON 格式的数据
    except Exception as e:
        messagebox.showerror("错误", f"无法读取 mapping.txt 文件: {e}")
        return

    def show_modified_items():
        """
    显示所有修改过的物品
    """
        modified_items = []
        # 收集所有修改过的物品
        for category_name, category_data in mapping.items():
            for item_name in category_data:
                widgets = item_widgets[item_name]
                if (widgets["dropped_size_slider"].get() != 1.00 or
                    widgets["third_person_size_slider"].get() != 1.00 or
                    widgets["first_person_size_slider"].get() != 1.00):
                    modified_items.append((category_name, item_name))
        
        if modified_items:
            # 创建修改过的物品的容器
            modified_container = tk.Frame(content_frame, bg="#f0f0f0")
            modified_container.grid(row=0, column=0, columnspan=4, sticky="ew")
            
            # 添加标题
            title_frame = tk.Frame(modified_container, bg="#e0e0e0")
            title_frame.pack(fill=tk.X, pady=(10, 5))
            title_label = tk.Label(title_frame, 
                                 text="已修改的物品", 
                                 font=("微软雅黑", 12, "bold"),
                                 bg="#e0e0e0",
                                 fg="#4CAF50")
            title_label.pack(fill=tk.X, pady=5, padx=20)
            
            # 显示修改过的物品
            items_frame = tk.Frame(modified_container, bg="#f0f0f0")
            items_frame.pack(fill=tk.X)
            
            for i, (category_name, item_name) in enumerate(modified_items):
                widgets = item_widgets[item_name]
                # 创建新的行框架
                row_frame = tk.Frame(items_frame, bg="#f0f0f0")
                row_frame.grid(row=i, column=0, columnspan=4, sticky="ew")
                
                # 显示类别和物品名称
                label_frame = tk.Frame(row_frame, bg="#f0f0f0", width=250)
                label_frame.grid(row=0, column=0, padx=(20,0), pady=5, sticky="w")
                
                item_text = f"[{category_name}] {item_name}"
                item_label = tk.Label(label_frame, 
                                    text=item_text,
                                    font=("微软雅黑", 12, "bold"),
                                    bg="#f0f0f0",
                                    fg="#4CAF50")
                item_label.grid(row=0, column=0, sticky="w")
                
                # 显示滑动条
                for j, slider_key in enumerate(["dropped_size_slider", 
                                             "third_person_size_slider",
                                             "first_person_size_slider"]):
                    slider = widgets[slider_key]
                    slider_container = tk.Frame(row_frame, bg="#f0f0f0")
                    slider_container.grid(row=0, column=j+1, padx=5, pady=5, sticky="w")
                    
                    # 创建新的滑动条并设置相同的值
                    new_slider = tk.Scale(slider_container,
                                        from_=0.0, to=4.0,
                                        resolution=0.01,
                                        orient=tk.HORIZONTAL,
                                        length=200,
                                        font=("微软雅黑", 12),
                                        showvalue=True,
                                        sliderlength=20)
                    new_slider.set(slider.get())
                    new_slider.pack(side=tk.LEFT)
                    
                    # 同步原始滑动条和新滑动条的值
                    def sync_sliders(value, original_slider=slider, new_s=new_slider):
                        original_slider.set(float(value))
                    new_slider.config(command=sync_sliders)
            
            return len(modified_items)
        return 0

    def update_category_items(category_name, search_term=""):
        """
    更新类别中的物品显示
    """
        category_frame = category_frames[category_name]
        if category_states.get(category_name, False):  # 如果类别是展开状态
            # 获取匹配搜索词的物品
            matched_items = [item for item in mapping[category_name] 
                           if search_term.lower() in item.lower()]
            
            # 先隐藏所有物品
            for item_name in mapping[category_name]:
                widgets = item_widgets[item_name]
                row_frame = widgets["item_label"].master.master
                row_frame.grid_remove()
            
            # 只显示未修改的物品
            current_row = 0
            for item_name in matched_items:
                widgets = item_widgets[item_name]
                if (widgets["dropped_size_slider"].get() == 1.00 and
                    widgets["third_person_size_slider"].get() == 1.00 and
                    widgets["first_person_size_slider"].get() == 1.00):
                    row_frame = widgets["item_label"].master.master
                    row_frame.grid(row=current_row, column=0, columnspan=4, sticky="ew")
                    current_row += 1

    def create_category_header(category_name, row_index):
        """
    创建类别标题和内容区域
    """
        # 创建一个总容器来包含标题和内容
        category_container = tk.Frame(content_frame, bg="#f0f0f0")
        category_container.grid(row=row_index, column=0, columnspan=4, sticky="ew")
        
        # 创建类别标题框架
        category_header = tk.Frame(category_container, bg="#e0e0e0")
        category_header.pack(fill=tk.X, pady=(10, 5))
        
        # 创建类别按钮（作为折叠控制器）
        category_button = tk.Button(category_header, 
                                  text=f"▶ {category_name}", 
                                  font=("微软雅黑", 12, "bold"),
                                  bg="#e0e0e0", 
                                  relief="flat",
                                  anchor="w",
                                  padx=20)
        category_button.pack(fill=tk.X, pady=5)
        
        # 创建存放该类别所有物品的框架
        items_frame = tk.Frame(category_container, bg="#f0f0f0")
        
        def toggle_this_category():
            """
    切换当前类别的显示/隐藏状态
    """
            current_state = category_states.get(category_name, False)
            category_states[category_name] = not current_state
            
            # 更新箭头和显示状态
            arrow = "▼" if not current_state else "▶"
            category_button.config(text=f"{arrow} {category_name}")
            
            if current_state:  # 如果当前是显示状态，则隐藏
                items_frame.pack_forget()
            else:  # 如果当前是隐藏状态，则显示
                items_frame.pack(fill=tk.X)
                # 使用当前搜索词更新显示
                update_category_items(category_name, search_var.get())
            
            # 更新画布
            content_frame.update_idletasks()
            canvas.config(scrollregion=canvas.bbox("all"))
        
        # 设置按钮命令
        category_button.config(command=toggle_this_category)
        
        category_frames[category_name] = items_frame
        category_states[category_name] = False  # 初始状态设为False
        return category_button, items_frame

    def create_item_row(item_name, parent_frame, row_index):
        # 创建一个框架来包含这一行的所有元素
        row_frame = tk.Frame(parent_frame, bg="#f0f0f0")
        row_frame.grid(row=row_index, column=0, columnspan=4, sticky="ew")
        
        # 创建固定宽度的标签容器，使用grid布局
        label_frame = tk.Frame(row_frame, bg="#f0f0f0", width=250)
        label_frame.grid(row=0, column=0, padx=(20,0), pady=5, sticky="w")
        label_frame.grid_columnconfigure(0, minsize=10)
        
        # 创建标签
        item_label = tk.Label(label_frame, text=item_name, font=("微软雅黑", 12), 
                            bg="#f0f0f0", fg="#333333")
        item_label.grid(row=0, column=0, sticky="w")
        def create_slider_with_reset(column):
            # 创建容器来包含滑动条和重置按钮
            container = tk.Frame(row_frame, bg="#f0f0f0")
            container.grid(row=0, column=column, padx=5, pady=5, sticky="w")

            # 创建滑动条
            slider = tk.Scale(container, from_=0.0, to=4.0, resolution=0.01,
                            orient=tk.HORIZONTAL, length=200, font=("微软雅黑", 12),
                            showvalue=True, sliderlength=20)
            slider.set(1.00)
            slider.pack(side=tk.LEFT)

            # 创建重置按钮
            reset_button = tk.Button(container, text="重置", 
                                   font=("微软雅黑", 10),
                                   bg="#4CAF50", fg="white",
                                   padx=5, pady=0)
            reset_button.pack(side=tk.LEFT, padx=(5,0))

            # 重置按钮的命令
            def reset_value():
                slider.set(1.00)
            reset_button.config(command=reset_value)

            return slider

        # 创建三个滑动条和重置按钮
        dropped_slider = create_slider_with_reset(1)
        third_person_slider = create_slider_with_reset(2)
        first_person_slider = create_slider_with_reset(3)

        # 保存部件引用
        sliders[item_name] = {'dropped_size': dropped_slider}
        item_widgets[item_name] = {
            "item_label": item_label,
            "dropped_size_slider": dropped_slider,
            "third_person_size_slider": third_person_slider,
            "first_person_size_slider": first_person_slider
        }

    def save_and_return():
        """
    保存当前所有滑动条的值并返回
    """
        # 保存所有滑动条的当前值
        for item_name, widgets in item_widgets.items():
            sliders[item_name] = {
                'dropped_size': widgets['dropped_size_slider'].get(),
                'third_person_size': widgets['third_person_size_slider'].get(),
                'first_person_size': widgets['first_person_size_slider'].get()
            }
        show_overlay_options()

    # 返回按钮
    back_button = tk.Button(overlay_frame, text="保存并返回", 
                           command=save_and_return, 
                           bg="#FF6347", fg="white", 
                           font=("微软雅黑", 14), 
                           padx=20, pady=8)
    back_button.pack(pady=10)

    def filter_items(search_term):
        search_term = search_term.lower()
        
        # 先清除所有已有的修改物品显示
        for widget in content_frame.winfo_children():
            if isinstance(widget, tk.Frame):
                widget.grid_remove()
        
        # 显示修改过的物品
        offset_rows = show_modified_items()
        
        # 显示其他类别
        for category_name, category_data in mapping.items():
            matched_items = [item for item in category_data if search_term in item.lower()]
            category_frame = category_frames[category_name]
            category_button = category_buttons[category_name]
            
            if matched_items or search_term == "":
                # 调整类别容器的位置
                category_button.master.master.grid(row=offset_rows, column=0, columnspan=4, sticky="ew")
                offset_rows += 1
                
                # 更新当前展开的类别中的物品
                update_category_items(category_name, search_term)
                
                if not category_states.get(category_name, False):
                    category_frame.pack_forget()
            else:
                category_button.master.master.grid_remove()
        
        # 更新画布
        content_frame.update_idletasks()
        canvas.config(scrollregion=canvas.bbox("all"))

    # 防抖搜索
    search_after_id = None
    def delayed_filter(event=None):
        nonlocal search_after_id
        if search_after_id:
            root.after_cancel(search_after_id)
        search_term = search_var.get()
        search_after_id = root.after(150, lambda: filter_items(search_term))

    # 绑定搜索事件
    search_entry.bind("<KeyRelease>", delayed_filter)

    # 遍历所有类别及其物品
    current_row = 0
    for category_name, category_data in mapping.items():
        # 创建类别标题
        category_button, items_frame = create_category_header(category_name, current_row)
        category_buttons[category_name] = category_button
        current_row += 1
        
        # 创建该类别下的所有物品行
        for i, item_name in enumerate(category_data):
            create_item_row(item_name, items_frame, i)
        
        # 确保初始状态是隐藏的
        items_frame.pack_forget()

def clear_frame():
    global frame, result_label_exists, new_button_exists
    for widget in frame.winfo_children():
        widget.destroy()
    
    # 只销毁不再需要的控件
    result_label_exists = False
    new_button_exists = False

def select_export_folder():
    global export_path  # 声明 export_path 为全局变量
    export_path = filedialog.askdirectory()  # 使用文件夹选择对话框
    if export_path:
        select_button.config(text=f"已选择导出路径: {export_path}")
    else:
        select_button.config(text="未选择导出路径")

# Function to split particles image
def split_particles_image(particles_path):
    log(f"Splitting particles image: {particles_path}")
    try:
        img = Image.open(particles_path).convert("RGBA")
        img_width, img_height = img.size
        if img_width != img_height or img_width % 16 != 0:
            log(f"'particles.png' is not a square image with a size multiple of 16: {img_width}x{img_height}")
            return

        split_size = img_width // 16
        output_dir_particle = os.path.dirname(particles_path)
        output_dir_entity = os.path.join(os.path.dirname(os.path.dirname(particles_path)), 'entity')
        os.makedirs(output_dir_particle, exist_ok=True)
        os.makedirs(output_dir_entity, exist_ok=True)

        for row in range(16):
            for col in range(16):
                split_img = img.crop((col * split_size, row * split_size, (col + 1) * split_size, (row + 1) * split_size))
                if row == 0 and col < 8:
                    split_img.save(os.path.join(output_dir_particle, f'generic_{col}.png'))
                elif row == 1 and 3 <= col <= 6:
                    split_img.save(os.path.join(output_dir_particle, f'splash_{col - 3}.png'))
                elif row == 2 and col == 0:
                    split_img.save(os.path.join(output_dir_particle, 'bubble.png'))
                elif row == 2 and col == 1:
                    split_img.save(os.path.join(output_dir_entity, 'fishing_hook.png'))
                elif row == 3 and col == 0:
                    split_img.save(os.path.join(output_dir_particle, 'flame.png'))
                elif row == 3 and col == 1:
                    split_img.save(os.path.join(output_dir_particle, 'lava.png'))
                elif row == 4 and col < 3:
                    split_img.save(os.path.join(output_dir_particle, ['note.png', 'critical_hit.png', 'enchanted_hit.png'][col]))
                elif row == 5 and col < 3:
                    split_img.save(os.path.join(output_dir_particle, ['heart.png', 'angry.png', 'glint.png'][col]))
                elif row == 7 and col < 3:
                    split_img.save(os.path.join(output_dir_particle, ['drip_hang.png', 'drip_fall.png', 'drip_land.png'][col]))
                elif row == 8 and col < 8:
                    split_img.save(os.path.join(output_dir_particle, f'effect_{col}.png'))
                elif row == 9 and col < 8:
                    split_img.save(os.path.join(output_dir_particle, f'spell_{col}.png'))
                elif row == 10 and col < 8:
                    split_img.save(os.path.join(output_dir_particle, f'spark_{col}.png'))

        os.remove(particles_path)
        log(f"Split and processed 'particles.png', and removed the original image.")
    except Exception as e:
        log(f"Error splitting 'particles.png': {e}")
        traceback.print_exc()

# Function to split image
def split_image(image_path, output_dir, prefix, retain_num):
    log(f"Splitting image: {image_path}")
    try:
        img = Image.open(image_path).convert("RGBA")
        img_width, img_height = img.size
        num_splits = img_height // img_width
        split_height = img_height // num_splits

        if num_splits > retain_num:
            # 计算要保留的索引，尽量均匀分布
            step = num_splits / retain_num
            indices = [int(i * step) for i in range(retain_num)]
            # 确保索引不超过范围
            indices = [min(idx, num_splits - 1) for idx in indices]
        else:
            # 如果分割数不超过保留数，则保留所有分割
            indices = list(range(num_splits))

        for j, i in enumerate(indices):
            split_img = img.crop((0, i * split_height, img_width, (i + 1) * split_height))
            split_img.save(os.path.join(output_dir, f"{prefix}_{j:02d}.png"))

        # 删除原始图片
        os.remove(image_path)

        # 删除对应的 .mcmeta 文件（如果存在）
        mcmeta_path = image_path + '.mcmeta'
        if os.path.exists(mcmeta_path):
            os.remove(mcmeta_path)

        log(f"Split '{image_path}' into {len(indices)} parts and removed the original image and its .mcmeta")
    except Exception as e:
        log(f"Error splitting image '{image_path}': {e}")
        traceback.print_exc()

def merge_images(image_paths, output_path):
    try:
        images = [Image.open(img_path) for img_path in image_paths]
        widths, heights = zip(*(img.size for img in images))
        
        # 计算合并后的图片尺寸
        total_height = sum(heights)
        max_width = max(widths)
        
        # 创建一个新的图像
        merged_image = Image.new("RGBA", (max_width, total_height))
        
        current_height = 0
        for img in images:
            merged_image.paste(img, (0, current_height))
            current_height += img.height
        
        # 保存合并后的图像
        merged_image.save(output_path)
        log(f"Successfully merged {len(images)} images into '{output_path}'")

        # 删除原始小图片
        for img_path in image_paths:
            if os.path.exists(img_path):
                os.remove(img_path)
                log(f"Deleted {img_path}")
    except Exception as e:
        log(f"Error merging images: {e}")

def create_mcmeta_file(file_path):
    """
    创建包含空动画对象的.mcmeta文件
    """
    mcmeta_data = {
        "animation": {}
    }
    
    # 创建目录（如果不存在）
    os.makedirs(os.path.dirname(file_path), exist_ok=True)
    
    # 保存.mcmeta文件
    with open(file_path, 'w', encoding='utf-8') as f:
        json.dump(mcmeta_data, f, ensure_ascii=False, indent=4)
    log(f"已创建 .mcmeta 文件: {file_path}")

# Helper function to swap rectangles in an image
def swap_rectangles(image, box1, box2):
    region1 = image.crop(box1)
    region2 = image.crop(box2)
    image.paste(region2, box1)
    image.paste(region1, box2)

# Helper function to swap and mirror rectangles in an image
def swap_and_mirror(image, box1, box2):
    region1 = image.crop(box1)
    region2 = image.crop(box2)
    image.paste(region2, box1)
    image.paste(region1, box2)
    region1 = image.crop(box1).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
    region2 = image.crop(box2).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
    image.paste(region1, box1)
    image.paste(region2, box2)

# Function to generate double chest images
def generate_double_chest_images(left_image, right_image, prefix, img, scaled_box, scale_factor):
    # Processing for the left image (normal_left.png or trapped_left.png)
    left_image.paste(img.crop(scaled_box(29, 0, 44, 14)).transpose(Image.FLIP_TOP_BOTTOM), (29 * scale_factor, 0))
    left_image.paste(img.crop(scaled_box(59, 0, 74, 14)).transpose(Image.FLIP_TOP_BOTTOM), (14 * scale_factor, 0))
    left_image.paste(
        img.crop(scaled_box(29, 14, 44, 19)).transpose(Image.FLIP_TOP_BOTTOM).transpose(Image.FLIP_LEFT_RIGHT),
        (43 * scale_factor, 14 * scale_factor)
    )
    left_image.paste(
        img.crop(scaled_box(44, 14, 58, 19)).transpose(Image.FLIP_TOP_BOTTOM).transpose(Image.FLIP_LEFT_RIGHT),
        (29 * scale_factor, 14 * scale_factor)
    )
    left_image.paste(
        img.crop(scaled_box(58, 14, 73, 19)).transpose(Image.FLIP_TOP_BOTTOM).transpose(Image.FLIP_LEFT_RIGHT),
        (14 * scale_factor, 14 * scale_factor)
    )
    left_image.paste(img.crop(scaled_box(29, 19, 44, 33)).transpose(Image.FLIP_TOP_BOTTOM), (29 * scale_factor, 19 * scale_factor))
    left_image.paste(img.crop(scaled_box(59, 19, 74, 33)).transpose(Image.FLIP_TOP_BOTTOM), (14 * scale_factor, 19 * scale_factor))
    left_image.paste(
        img.crop(scaled_box(29, 33, 44, 43)).transpose(Image.FLIP_TOP_BOTTOM).transpose(Image.FLIP_LEFT_RIGHT),
        (43 * scale_factor, 33 * scale_factor)
    )
    left_image.paste(
        img.crop(scaled_box(44, 33, 58, 43)).transpose(Image.FLIP_TOP_BOTTOM).transpose(Image.FLIP_LEFT_RIGHT),
        (29 * scale_factor, 33 * scale_factor)
    )
    left_image.paste(
        img.crop(scaled_box(58, 33, 73, 43)).transpose(Image.FLIP_TOP_BOTTOM).transpose(Image.FLIP_LEFT_RIGHT),
        (14 * scale_factor, 33 * scale_factor)
    )

    # Additional transformations for the left image
    # 将(2,1)到(5,5)形成的区域复制粘贴到(1,1)到(4,5)，然后上下翻转再左右翻转
    left_image.paste(
        img.crop(scaled_box(2, 1, 5, 5)).transpose(Image.FLIP_TOP_BOTTOM).transpose(Image.FLIP_LEFT_RIGHT),
        (1 * scale_factor, 1 * scale_factor)
    )

    # 将(2,0)到(3,1)形成的区域复制粘贴到(2,0)到(3,1)
    left_image.paste(img.crop(scaled_box(2, 0, 3, 1)), (2 * scale_factor, 0))

    # 将(4,0)到(5,1)形成的区域复制粘贴到(1,0)到(2,1)
    left_image.paste(img.crop(scaled_box(4, 0, 5, 1)), (1 * scale_factor, 0))

    # 将(5,1)到(6,5)形成的区域复制粘贴到(1,1)到(2,5)，然后上下翻转
    left_image.paste(
        img.crop(scaled_box(5, 1, 6, 5)).transpose(Image.FLIP_TOP_BOTTOM),
        (1 * scale_factor, 1 * scale_factor)
    )

    # 将(1,0)到(2,1)形成的区域复制粘贴到(2,0)到(3,1)
    left_image.paste(img.crop(scaled_box(1, 0, 2, 1)), (2 * scale_factor, 0))

    # 将(3,0)到(4,1)形成的区域复制粘贴到(1,0)到(2,1)
    left_image.paste(img.crop(scaled_box(3, 0, 4, 1)), (1 * scale_factor, 0))

    # Processing for the right image (normal_right.png or trapped_right.png)
    right_image.paste(img.crop(scaled_box(44, 0, 59, 14)).transpose(Image.FLIP_TOP_BOTTOM), (14 * scale_factor, 0))
    right_image.paste(img.crop(scaled_box(14, 0, 29, 14)).transpose(Image.FLIP_TOP_BOTTOM), (29 * scale_factor, 0))
    right_image.paste(
        img.crop(scaled_box(0, 14, 14, 19)).transpose(Image.FLIP_TOP_BOTTOM).transpose(Image.FLIP_LEFT_RIGHT),
        (0, 14 * scale_factor)
    )
    right_image.paste(
        img.crop(scaled_box(73, 14, 88, 19)).transpose(Image.FLIP_TOP_BOTTOM).transpose(Image.FLIP_LEFT_RIGHT),
        (14 * scale_factor, 14 * scale_factor)
    )
    right_image.paste(
        img.crop(scaled_box(14, 14, 29, 19)).transpose(Image.FLIP_TOP_BOTTOM).transpose(Image.FLIP_LEFT_RIGHT),
        (43 * scale_factor, 14 * scale_factor)
    )
    right_image.paste(img.crop(scaled_box(14, 19, 29, 33)).transpose(Image.FLIP_TOP_BOTTOM), (29 * scale_factor, 19 * scale_factor))
    right_image.paste(img.crop(scaled_box(44, 19, 59, 33)).transpose(Image.FLIP_TOP_BOTTOM), (14 * scale_factor, 19 * scale_factor))
    right_image.paste(
        img.crop(scaled_box(14, 33, 29, 43)).transpose(Image.FLIP_TOP_BOTTOM).transpose(Image.FLIP_LEFT_RIGHT),
        (43 * scale_factor, 33 * scale_factor)
    )
    right_image.paste(
        img.crop(scaled_box(0, 33, 14, 43)).transpose(Image.FLIP_TOP_BOTTOM).transpose(Image.FLIP_LEFT_RIGHT),
        (0, 33 * scale_factor)
    )
    right_image.paste(
        img.crop(scaled_box(73, 33, 88, 43)).transpose(Image.FLIP_TOP_BOTTOM).transpose(Image.FLIP_LEFT_RIGHT),
        (14 * scale_factor, 33 * scale_factor)
    )

    # Additional transformations for the right image
    # 将(0,1)到(1,5)形成的区域复制粘贴到(0,1)到(1,5)，然后上下翻转
    right_image.paste(
        img.crop(scaled_box(0, 1, 1, 5)).transpose(Image.FLIP_TOP_BOTTOM),
        (0 * scale_factor, 1 * scale_factor)
    )

    # 将(1,1)到(2,5)形成的区域复制粘贴到(3,1)到(4,5)，然后上下翻转
    right_image.paste(
        img.crop(scaled_box(1, 1, 2, 5)).transpose(Image.FLIP_TOP_BOTTOM),
        (3 * scale_factor, 1 * scale_factor)
    )

    # 将(5,1)到(6,5)形成的区域复制粘贴到(1,1)到(2,5)，然后上下翻转
    right_image.paste(
        img.crop(scaled_box(5, 1, 6, 5)).transpose(Image.FLIP_TOP_BOTTOM),
        (1 * scale_factor, 1 * scale_factor)
    )

    # 将(1,0)到(2,1)形成的区域复制粘贴到(2,0)到(3,1)
    right_image.paste(img.crop(scaled_box(1, 0, 2, 1)), (2 * scale_factor, 0))

    # 将(3,0)到(4,1)形成的区域复制粘贴到(1,0)到(2,1)
    right_image.paste(img.crop(scaled_box(3, 0, 4, 1)), (1 * scale_factor, 0))

def combine_double_chest_images(chest_path):
    """
    将左右双箱子图像合并回原始的双箱子图像。
    """
    log(f"Combining double chest images in: {chest_path}")
    double_chest_prefixes = ['normal', 'trapped', 'christmas']
    
    for prefix in double_chest_prefixes:
        left_file = os.path.join(chest_path, f"{prefix}_left.png")
        right_file = os.path.join(chest_path, f"{prefix}_right.png")
        
        if os.path.exists(left_file) and os.path.exists(right_file):
            try:
                left_img = Image.open(left_file).convert("RGBA")
                right_img = Image.open(right_file).convert("RGBA")
                width_left, height_left = left_img.size
                width_right, height_right = right_img.size

                if width_left != width_right or height_left != height_right:
                    log(f"Left and Right images for '{prefix}' have different sizes.")
                    continue

                # Determine scale_factor based on image size
                # 单个箱子图像大小为 64*scale_factor x 64*scale_factor
                if width_left == 64 and height_left == 64:
                    scale_factor = 1
                elif width_left == 128 and height_left == 128:
                    scale_factor = 2
                elif width_left == 256 and height_left == 256:
                    scale_factor = 4
                elif width_left == 512 and height_left == 512:
                    scale_factor = 8
                elif width_left == 1024 and height_left == 1024:
                    scale_factor = 16
                else:
                    log(f"Unsupported left/right image size for '{prefix}': {width_left}x{height_left}")
                    continue

                # 创建一个新的双箱子图像
                double_chest_width = 128 * scale_factor
                double_chest_height = 64 * scale_factor
                double_chest_img = Image.new("RGBA", (double_chest_width, double_chest_height), (0,0,0,0))

                def scaled_box(x1, y1, x2, y2):
                    return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

                # 逆向应用 generate_double_chest_images 中的所有 paste 操作

                # 处理左图像逆向还原到双箱子图像
                # 对应 generate_double_chest_images 中的左图像粘贴操作
                # 需要逆向裁剪并粘贴回双箱子图像

                # 1. 左图像粘贴的第一部分
                region = left_img.crop(scaled_box(29, 0, 44, 14)).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(29, 0, 44, 14))

                # 2. 左图像粘贴的第二部分
                region = left_img.crop(scaled_box(59, 0, 74, 14)).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(59, 0, 74, 14))

                # 3. 左图像粘贴的第三部分
                region = left_img.crop(scaled_box(29, 14, 44, 19)).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(29, 14, 44, 19))

                # 4. 左图像粘贴的第四部分
                region = left_img.crop(scaled_box(44, 14, 58, 19)).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(44, 14, 58, 19))

                # 5. 左图像粘贴的第五部分
                region = left_img.crop(scaled_box(58, 14, 73, 19)).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(58, 14, 73, 19))

                # 6. 左图像粘贴的第六部分
                region = left_img.crop(scaled_box(29, 19, 44, 33)).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(29, 19, 44, 33))

                # 7. 左图像粘贴的第七部分
                region = left_img.crop(scaled_box(59, 19, 74, 33)).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(59, 19, 74, 33))

                # 8. 左图像粘贴的第八部分
                region = left_img.crop(scaled_box(29, 33, 44, 43)).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(29, 33, 44, 43))

                # 9. 左图像粘贴的第九部分
                region = left_img.crop(scaled_box(44, 33, 58, 43)).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(44, 33, 58, 43))

                # 10. 左图像粘贴的第十部分
                region = left_img.crop(scaled_box(58, 33, 73, 43)).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(58, 33, 73, 43))

                # 处理左图像的附加转换
                # 1. 将(2,1)-(5,5)逆向粘贴到(2,1)-(5,5)，并进行翻转
                region = left_img.crop(scaled_box(2, 1, 5, 5)).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(2, 1, 5, 5))

                # 2. 将(2,0)-(3,1)逆向粘贴回(2,0)-(3,1)
                region = left_img.crop(scaled_box(2, 0, 3, 1))
                double_chest_img.paste(region, scaled_box(2, 0, 3, 1))

                # 3. 将(4,0)-(5,1)逆向粘贴回(4,0)-(5,1)
                region = left_img.crop(scaled_box(4, 0, 5, 1))
                double_chest_img.paste(region, scaled_box(4, 0, 5, 1))

                # 4. 将(5,1)-(6,5)逆向粘贴回(5,1)-(6,5)，并进行翻转
                region = left_img.crop(scaled_box(5, 1, 6, 5)).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(5, 1, 6, 5))

                # 5. 将(1,0)-(2,1)逆向粘贴回(1,0)-(2,1)
                region = left_img.crop(scaled_box(1, 0, 2, 1))
                double_chest_img.paste(region, scaled_box(1, 0, 2, 1))

                # 6. 将(3,0)-(4,1)逆向粘贴回(3,0)-(4,1)
                region = left_img.crop(scaled_box(3, 0, 4, 1))
                double_chest_img.paste(region, scaled_box(3, 0, 4, 1))

                # 处理右图像逆向还原到双箱子图像
                # 对应 generate_double_chest_images 中的右图像粘贴操作

                # 1. 右图像粘贴的第一部分
                region = right_img.crop(scaled_box(44, 0, 59, 14)).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(44, 0, 59, 14))

                # 2. 右图像粘贴的第二部分
                region = right_img.crop(scaled_box(14, 0, 29, 14)).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(14, 0, 29, 14))

                # 3. 右图像粘贴的第三部分
                region = right_img.crop(scaled_box(0, 14, 14, 19)).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(0, 14, 14, 19))

                # 4. 右图像粘贴的第四部分
                region = right_img.crop(scaled_box(73, 14, 88, 19)).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(73, 14, 88, 19))

                # 5. 右图像粘贴的第五部分
                region = right_img.crop(scaled_box(14, 14, 29, 19)).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(14, 14, 29, 19))

                # 6. 右图像粘贴的第六部分
                region = right_img.crop(scaled_box(14, 19, 29, 33)).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(14, 19, 29, 33))

                # 7. 右图像粘贴的第七部分
                region = right_img.crop(scaled_box(44, 19, 59, 33)).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(44, 19, 59, 33))

                # 8. 右图像粘贴的第八部分
                region = right_img.crop(scaled_box(14, 33, 29, 43)).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(14, 33, 29, 43))

                # 9. 右图像粘贴的第九部分
                region = right_img.crop(scaled_box(0, 33, 14, 43)).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(0, 33, 14, 43))

                # 10. 右图像粘贴的第十部分
                region = right_img.crop(scaled_box(73, 33, 88, 43)).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(73, 33, 88, 43))

                # 处理右图像的附加转换
                # 1. 将(0,1)-(1,5)逆向粘贴回(0,1)-(1,5)，并进行翻转
                region = right_img.crop(scaled_box(0, 1, 1, 5)).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(0, 1, 1, 5))

                # 2. 将(1,1)-(2,5)逆向粘贴回(1,1)-(2,5)，并进行翻转
                region = right_img.crop(scaled_box(1, 1, 2, 5)).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(1, 1, 2, 5))

                # 3. 将(5,1)-(6,5)逆向粘贴回(5,1)-(6,5)，并进行翻转
                region = right_img.crop(scaled_box(5, 1, 6, 5)).transpose(Image.FLIP_TOP_BOTTOM)
                double_chest_img.paste(region, scaled_box(5, 1, 6, 5))

                # 4. 将(1,0)-(2,1)逆向粘贴回(1,0)-(2,1)
                region = right_img.crop(scaled_box(1, 0, 2, 1))
                double_chest_img.paste(region, scaled_box(1, 0, 2, 1))

                # 5. 将(3,0)-(4,1)逆向粘贴回(3,0)-(4,1)
                region = right_img.crop(scaled_box(3, 0, 4, 1))
                double_chest_img.paste(region, scaled_box(3, 0, 4, 1))

                # 保存合并后的双箱子图像
                double_chest_filename = f"{prefix}_double.png"
                double_chest_path = os.path.join(chest_path, double_chest_filename)
                double_chest_img.save(double_chest_path)
                log(f"Combined '{prefix}_left.png' and '{prefix}_right.png' into '{double_chest_filename}' successfully.")

            except Exception as e:
                log(f"Error combining '{prefix}_left.png' and '{prefix}_right.png': {e}")
                continue
        else:
            log(f"Left or Right image for '{prefix}' does not exist in the path '{chest_path}'. Skipping.")

    log("Combining double chest images completed.")

# Function to process redstone dust cross image
def process_redstone_dust_cross_image(blocks_path_new):
    log(f"Processing redstone dust cross image in: {blocks_path_new}")
    redstone_dust_cross_path = os.path.join(blocks_path_new, 'redstone_dust_cross.png')
    if os.path.exists(redstone_dust_cross_path):
        img = Image.open(redstone_dust_cross_path).convert("RGBA")
        if img.size == (16, 16):
            pixels = img.load()
            for x in range(16):
                for y in range(16):
                    if not ((x == y and 5 <= x <= 11) or (x + y == 16 and 5 <= x <= 11)):
                        pixels[x, y] = (0, 0, 0, 0)
            new_path = os.path.join(blocks_path_new, 'red_dust_dot.png')
            img.save(new_path)
            log(f"Processed and renamed 'redstone_dust_cross.png' to 'red_dust_dot.png'")
        else:
            log(f"'redstone_dust_cross.png' is not a 16x16 image")
    else:
        log(f"'redstone_dust_cross.png' not found in {blocks_path_new}")

# Function to process redstone dust line image
def process_redstone_dust_line_image(blocks_path_new):
    log(f"Processing redstone dust line image in: {blocks_path_new}")
    redstone_dust_line_path = os.path.join(blocks_path_new, 'redstone_dust_line.png')
    if os.path.exists(redstone_dust_line_path):
        img = Image.open(redstone_dust_line_path).convert("RGBA")
        rotated_90_cw = img.rotate(-90, expand=True)
        rotated_90_ccw = img.rotate(90, expand=True)
        line_0_path = os.path.join(blocks_path_new, 'redstone_dust_line0.png')
        line_1_path = os.path.join(blocks_path_new, 'redstone_dust_line1.png')
        rotated_90_cw.save(line_0_path)
        rotated_90_ccw.save(line_1_path)
        log(f"Processed 'redstone_dust_line.png' to 'redstone_dust_line0.png' and 'redstone_dust_line1.png'")
    else:
        log(f"'redstone_dust_line.png' not found in {blocks_path_new}")

def scaled_coords(x1, y1, x2, y2, scale_factor):
    """
    根据 scale_factor 缩放坐标。
    """
    return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

def scaled_point(x, y, scale_factor):
    """
    根据 scale_factor 缩放单点坐标。
    """
    return (x * scale_factor, y * scale_factor)

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

def determine_scale_factor(width, height, standard_scale_factors=[1, 2, 4, 8]):
    """
    确定最接近图像尺寸的 scale_factor。
    """
    if width != height:
        log(f"Warning: Image is not square (width={width}, height={height}). Using width to determine scale_factor.")
    
    image_size = width  # 假设图像为方形，使用宽度作为基准
    closest_scale_factor = min(standard_scale_factors, key=lambda x: abs(x * 256 - image_size))
    is_exact = (closest_scale_factor * 256 == image_size)
    
    if not is_exact:
        log(f"Warning: Image size {width}x{height} is not a standard 256x256 multiple. Using scale_factor={closest_scale_factor} based on closest match.")
    
    return closest_scale_factor, is_exact

def average_color(image, box):
    """
    计算指定区域内所有像素的平均颜色。
    """
    region = image.crop(box)
    pixels = list(region.getdata())
    if not pixels:
        return (0, 0, 0, 0)
    avg = tuple(sum(p[i] for p in pixels) // len(pixels) for i in range(4))  # RGBA
    return avg

def process_villager_image(container_path):
    """
    处理解压后的目录中的 assets/minecraft/textures/gui/container/villager.png，
    """
    log(f"Processing villager image in: {container_path}")
    try:
        villager_path = os.path.join(container_path, 'villager.png')
        if os.path.exists(villager_path):
            img = Image.open(villager_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'villager.png': {width}x{height}")
                return

            log(f"Processing villager.png, size: {width}x{height}, scale_factor: {scale_factor}")

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义新图像的尺寸
            new_width = width * 2
            new_height = height
            villager2_path = os.path.join(container_path, 'villager2.png')

            # 创建一个新的透明图像
            villager2_img = Image.new('RGBA', (new_width, new_height), (0, 0, 0, 0))
            log(f"Created new transparent image: {villager2_path}, size: {villager2_img.size}")

            # 裁剪原图并粘贴到新图像
            crop_region = scaled_box(0, 0, 240, 166)
            paste_position = (100 * scale_factor, 0)  # (100, 0) scaled
            try:
                cropped_img = img.crop(crop_region)
                villager2_img.paste(cropped_img, paste_position)
                log(f"Pasted cropped region {crop_region} to {paste_position}")
            except Exception as e:
                log(f"Error cropping and pasting regions: {e}")
                return

            # 获取脚本所在目录（假设覆盖图像在与exe同目录的villager2文件夹中）
            if getattr(sys, 'frozen', False):
                # 如果是打包后的exe
                script_dir = os.path.dirname(sys.executable)
            else:
                # 如果是脚本运行
                script_dir = os.path.dirname(os.path.abspath(__file__))

            overlay_folder = os.path.join(script_dir, 'villager2')
            overlay_image_name = f'villager2_{256 * scale_factor}.png'
            overlay_img_path = os.path.join(overlay_folder, overlay_image_name)

            if os.path.exists(overlay_img_path):
                try:
                    overlay_img = Image.open(overlay_img_path).convert("RGBA")
                    villager2_img.paste(overlay_img, (0, 0), overlay_img)
                    log(f"Overlayed {overlay_image_name} onto villager2.png")
                except Exception as e:
                    log(f"Error overlaying image {overlay_image_name}: {e}")
                    return
            else:
                log(f"No overlay image found: {overlay_img_path}")

            # 新步骤1：覆盖颜色区域 (185,17)-(186,18) -> (186,24)-(208,39)
            try:
                # 定义源颜色区域 (185,17)-(186,18) scaled
                source_box = scaled_box(185, 17, 186, 18)  # (185,17)-(186,18), 1x1 像素

                # 获取源颜色
                source_color = villager2_img.getpixel((185 * scale_factor, 17 * scale_factor))
                log(f"Color at (185,17) scaled: {source_color}")

                # 定义覆盖区域 (186,24)-(208,39) scaled
                cover_box = scaled_box(186, 24, 208, 39)
                cover_x1, cover_y1, cover_x2, cover_y2 = cover_box

                cover_width = cover_x2 - cover_x1
                cover_height = cover_y2 - cover_y1

                # 创建一个单色图像用于覆盖
                cover_img = Image.new('RGBA', (cover_width, cover_height), source_color)

                # 粘贴覆盖图像到 villager2_img
                villager2_img.paste(cover_img, (cover_x1, cover_y1))
                log(f"Covered region {cover_box} in villager2.png with color {source_color}")
            except Exception as e:
                log(f"Error covering region (186,24)-(208,39) in 'villager2.png': {e}")
                traceback.print_exc()

            # 新步骤2：移动矩形区域 (133,48)-(242,76) 向上平移16个像素
            try:
                # 定义原始区域 (133,48)-(242,76) scaled
                original_region = scaled_box(133, 48, 242, 76)
                orig_x1, orig_y1, orig_x2, orig_y2 = original_region

                # 定义目标区域，向上平移16个像素 scaled
                shift_y = 16 * scale_factor
                target_region = (orig_x1, orig_y1 - shift_y, orig_x2, orig_y2 - shift_y)

                # 裁剪原始区域
                cropped_move = villager2_img.crop(original_region)

                # 粘贴到目标区域
                villager2_img.paste(cropped_move, (target_region[0], target_region[1]))
                log(f"Moved region {original_region} to {target_region} in villager2.png")
            except Exception as e:
                log(f"Error moving region (133,48)-(242,76) in 'villager2.png': {e}")
                traceback.print_exc()

            # 新步骤3：覆盖颜色区域 (132,60)-(133,61) -> (133,60)-(242,76)
            try:
                # 定义源颜色区域 (132,60)-(133,61) scaled
                source_box_2 = scaled_box(132, 60, 133, 61)  # (132,60)-(133,61), 1x1 像素

                # 获取源颜色
                source_color_2 = villager2_img.getpixel((132 * scale_factor, 60 * scale_factor))
                log(f"Color at (132,60) scaled: {source_color_2}")

                # 定义覆盖区域 (133,60)-(242,76) scaled
                cover_box_2 = scaled_box(133, 60, 242, 76)
                cover_x1_2, cover_y1_2, cover_x2_2, cover_y2_2 = cover_box_2

                cover_width_2 = cover_x2_2 - cover_x1_2
                cover_height_2 = cover_y2_2 - cover_y1_2

                # 创建一个单色图像用于覆盖
                cover_img_2 = Image.new('RGBA', (cover_width_2, cover_height_2), source_color_2)

                # 粘贴覆盖图像到 villager2_img
                villager2_img.paste(cover_img_2, (cover_x1_2, cover_y1_2))
                log(f"Covered region {cover_box_2} in villager2.png with color {source_color_2}")
            except Exception as e:
                log(f"Error covering region (133,60)-(242,76) in 'villager2.png': {e}")
                traceback.print_exc()

            # 保存生成的 villager2.png
            try:
                villager2_img.save(villager2_path)
                log(f"Saved 'villager2.png' at {villager2_path}")
            except Exception as e:
                log(f"Error saving 'villager2.png': {e}")
                return
        else:
            log(f"No 'villager.png' found in {container_path}")

    except Exception as e:
        log(f"Error processing villager image in '{container_path}': {e}")
        traceback.print_exc()

# Function to process grindstone image
def process_grindstone_image(container_path):
    log(f"Processing grindstone image in: {container_path}")
    try:
        shulker_box_path = os.path.join(container_path, 'shulker_box.png')
        grindstone_path = os.path.join(container_path, 'grindstone.png')
        anvil_path = os.path.join(container_path, 'anvil.png')  # 添加 anvil.png 路径

        if os.path.exists(shulker_box_path):
            img = Image.open(shulker_box_path).convert("RGBA")
            width, height = img.size

            if width == 256 and height == 256:
                scale_factor = 1
                grindstone_image_path = os.path.join('grindstone', 'grindstone_256.png')
            elif width == 512 and height == 512:
                scale_factor = 2
                grindstone_image_path = os.path.join('grindstone', 'grindstone_512.png')
            elif width == 1024 and height == 1024:
                scale_factor = 4
                grindstone_image_path = os.path.join('grindstone', 'grindstone_1024.png')
            elif width == 2048 and height == 2048:
                scale_factor = 8
                grindstone_image_path = os.path.join('grindstone', 'grindstone_2048.png')
            else:
                log(f"Unsupported image size for 'shulker_box.png': {width}x{height}")
                return

            img_copy = img.copy()
            cover_box = (6 * scale_factor, 16 * scale_factor, 170 * scale_factor, 72 * scale_factor)
            fill_color = img.getpixel((5 * scale_factor, 4 * scale_factor))
            
            log(f"Filling region {cover_box} with color {fill_color}")
            for x in range(cover_box[0], cover_box[2]):
                for y in range(cover_box[1], cover_box[3]):
                    img_copy.putpixel((x, y), fill_color)
                    
            # 移动区域
            region = img_copy.crop((7 * scale_factor, 83 * scale_factor, 25 * scale_factor, 101 * scale_factor))
            img_copy.paste(region, (48 * scale_factor, 18 * scale_factor))
            img_copy.paste(region, (128 * scale_factor, 33 * scale_factor))
            img_copy.paste(region, (48 * scale_factor, 39 * scale_factor))
            
            # 覆盖图像（如果存在）
            if os.path.exists(grindstone_image_path):
                overlay_img = Image.open(grindstone_image_path).convert("RGBA")
                img_copy.paste(overlay_img, (0, 0), overlay_img)
                log(f"Overlayed {grindstone_image_path} onto grindstone.png")
            else:
                log(f"No overlay image found for size {width}x{height}")

            # 新步骤2：在保存前处理 anvil.png 并粘贴到 grindstone.png（根据 scale_factor 缩放）
            try:
                if os.path.exists(anvil_path):
                    # 打开 anvil.png 并等比缩放到与 grindstone.png 相同的尺寸
                    anvil_img = Image.open(anvil_path).convert("RGBA")
                    if anvil_img.size != img_copy.size:
                        anvil_img = anvil_img.resize(img_copy.size, Image.Resampling.NEAREST)
                        log(f"Resized 'anvil.png' from {anvil_img.size} to {img_copy.size} using nearest neighbor")

                    # 定义源区域 (176,0)-(204,21) scaled
                    source_box = (
                        176 * scale_factor,
                        0 * scale_factor,
                        204 * scale_factor,
                        21 * scale_factor
                    )
                    cropped_region = anvil_img.crop(source_box)
                    log(f"Cropped region {source_box} from 'anvil.png'")

                    # 定义目标位置 (176,0) scaled
                    target_position = (176 * scale_factor, 0 * scale_factor)

                    # 粘贴裁剪后的区域到 grindstone.png
                    img_copy.paste(cropped_region, target_position, cropped_region)
                    log(f"Pasted cropped region {source_box} from 'anvil.png' to {target_position} in 'grindstone.png'")
                else:
                    log(f"No 'anvil.png' found in {container_path} for paste step")
            except Exception as e:
                log(f"Error processing and pasting from 'anvil.png': {e}")
                traceback.print_exc()

            # 保存生成的 grindstone.png
            try:
                img_copy.save(grindstone_path)
                log(f"Processed 'shulker_box.png' and saved as 'grindstone.png'")
            except Exception as e:
                log(f"Error saving 'grindstone.png': {e}")
                return
        else:
            log(f"No 'shulker_box.png' found in {container_path}")
    except Exception as e:
        log(f"Error processing grindstone image in '{container_path}': {e}")
        traceback.print_exc()

# Function to process cartography table image
def process_cartography_table_image(container_path):
    log(f"Processing cartography table image in: {container_path}")
    try:
        shulker_box_path = os.path.join(container_path, 'shulker_box.png')
        cartography_table_path = os.path.join(container_path, 'cartography_table.png')

        if os.path.exists(shulker_box_path):
            img = Image.open(shulker_box_path).convert("RGBA")
            width, height = img.size

            if width == 256 and height == 256:
                scale_factor = 1
                cartography_table_image_path = os.path.join('cartography_table', 'cartography_table_256.png')
            elif width == 512 and height == 512:
                scale_factor = 2
                cartography_table_image_path = os.path.join('cartography_table', 'cartography_table_512.png')
            elif width == 1024 and height == 1024:
                scale_factor = 4
                cartography_table_image_path = os.path.join('cartography_table', 'cartography_table_1024.png')
            elif width == 2048 and height == 2048:
                scale_factor = 8
                cartography_table_image_path = os.path.join('cartography_table', 'cartography_table_2048.png')
            else:
                log(f"Unsupported image size for 'shulker_box.png': {width}x{height}")
                return

            img_copy = img.copy()
            cover_box = (6 * scale_factor, 16 * scale_factor, 170 * scale_factor, 72 * scale_factor)
            fill_color = img.getpixel((5 * scale_factor, 4 * scale_factor))
            
            for x in range(cover_box[0], cover_box[2]):
                for y in range(cover_box[1], cover_box[3]):
                    img_copy.putpixel((x, y), fill_color)
                    
            region = img_copy.crop((7 * scale_factor, 83 * scale_factor, 25 * scale_factor, 101 * scale_factor))
            img_copy.paste(region, (14 * scale_factor, 51 * scale_factor))
            img_copy.paste(region, (144 * scale_factor, 38 * scale_factor))
            img_copy.paste(region, (14 * scale_factor, 14 * scale_factor))
            
            if os.path.exists(cartography_table_image_path):
                overlay_img = Image.open(cartography_table_image_path).convert("RGBA")
                img_copy.paste(overlay_img, (0, 0), overlay_img)
                img_copy.save(cartography_table_path)
                log(f"Processed 'shulker_box.png' and saved as 'cartography_table.png'")
            else:
                log(f"No overlay image found for size {width}x{height}")
        else:
            log(f"No 'shulker_box.png' found in {container_path}")
    except Exception as e:
        log(f"Error processing cartography table image in '{container_path}': {e}")
        traceback.print_exc()

# Function to process stonecutter image
def process_stonecutter_image(container_path):
    log(f"Processing stonecutter image in: {container_path}")
    try:
        shulker_box_path = os.path.join(container_path, 'shulker_box.png')
        stonecutter_path = os.path.join(container_path, 'stonecutter.png')

        if os.path.exists(shulker_box_path):
            img = Image.open(shulker_box_path).convert("RGBA")
            width, height = img.size

            if width == 256 and height == 256:
                scale_factor = 1
                stonecutter_image_path = os.path.join('stonecutter', 'stonecutter_256.png')
            elif width == 512 and height == 512:
                scale_factor = 2
                stonecutter_image_path = os.path.join('stonecutter', 'stonecutter_512.png')
            elif width == 1024 and height == 1024:
                scale_factor = 4
                stonecutter_image_path = os.path.join('stonecutter', 'stonecutter_1024.png')
            elif width == 2048 and height == 2048:
                scale_factor = 8
                stonecutter_image_path = os.path.join('stonecutter', 'stonecutter_2048.png')
            else:
                log(f"Unsupported image size for 'shulker_box.png': {width}x{height}")
                return

            img_copy = img.copy()
            cover_box = (6 * scale_factor, 16 * scale_factor, 170 * scale_factor, 72 * scale_factor)
            fill_color = img.getpixel((5 * scale_factor, 4 * scale_factor))
            
            for x in range(cover_box[0], cover_box[2]):
                for y in range(cover_box[1], cover_box[3]):
                    img_copy.putpixel((x, y), fill_color)
                    
            region = img_copy.crop((7 * scale_factor, 83 * scale_factor, 25 * scale_factor, 101 * scale_factor))
            img_copy.paste(region, (19 * scale_factor, 32 * scale_factor))
            img_copy.paste(region, (142 * scale_factor, 32 * scale_factor))
            
            if os.path.exists(stonecutter_image_path):
                overlay_img = Image.open(stonecutter_image_path).convert("RGBA")
                img_copy.paste(overlay_img, (0, 0), overlay_img)
                img_copy.save(stonecutter_path)
                log(f"Processed 'shulker_box.png' and saved as 'stonecutter.png'")
            else:
                log(f"No overlay image found for size {width}x{height}")
        else:
            log(f"No 'shulker_box.png' found in {container_path}")
    except Exception as e:
        log(f"Error processing stonecutter image in '{container_path}': {e}")
        traceback.print_exc()

# Function to process loom image
def process_loom_image(container_path):
    log(f"Processing loom image in: {container_path}")
    try:
        shulker_box_path = os.path.join(container_path, 'shulker_box.png')
        loom_path = os.path.join(container_path, 'loom.png')

        if os.path.exists(shulker_box_path):
            img = Image.open(shulker_box_path).convert("RGBA")
            width, height = img.size

            if width == 256 and height == 256:
                scale_factor = 1
                loom_image_path = os.path.join('loom', 'loom_256.png')
            elif width == 512 and height == 512:
                scale_factor = 2
                loom_image_path = os.path.join('loom', 'loom_512.png')
            elif width == 1024 and height == 1024:
                scale_factor = 4
                loom_image_path = os.path.join('loom', 'loom_1024.png')
            elif width == 2048 and height == 2048:
                scale_factor = 8
                loom_image_path = os.path.join('loom', 'loom_2048.png')
            else:
                log(f"Unsupported image size for 'shulker_box.png': {width}x{height}")
                return

            img_copy = img.copy()
            cover_box = (6 * scale_factor, 16 * scale_factor, 170 * scale_factor, 72 * scale_factor)
            fill_color = img.getpixel((5 * scale_factor, 4 * scale_factor))
            
            for x in range(cover_box[0], cover_box[2]):
                for y in range(cover_box[1], cover_box[3]):
                    img_copy.putpixel((x, y), fill_color)
                    
            region = img_copy.crop((7 * scale_factor, 83 * scale_factor, 25 * scale_factor, 101 * scale_factor))
            img_copy.paste(region, (12 * scale_factor, 25 * scale_factor))
            img_copy.paste(region, (32 * scale_factor, 25 * scale_factor))
            img_copy.paste(region, (22 * scale_factor, 44 * scale_factor))
            img_copy.paste(region, (142 * scale_factor, 56 * scale_factor))
            
            if os.path.exists(loom_image_path):
                overlay_img = Image.open(loom_image_path).convert("RGBA")
                img_copy.paste(overlay_img, (0, 0), overlay_img)
                img_copy.save(loom_path)
                log(f"Processed 'shulker_box.png' and saved as 'loom.png'")
            else:
                log(f"No overlay image found for size {width}x{height}")
        else:
            log(f"No 'shulker_box.png' found in {container_path}")
    except Exception as e:
        log(f"Error processing loom image in '{container_path}': {e}")
        traceback.print_exc()

# Helper function to convert RGBA to HSV
def rgba_to_hsv(rgba):
    r, g, b, a = rgba
    r, g, b = r / 255.0, g / 255.0, b / 255.0
    max_rgb = max(r, g, b)
    min_rgb = min(r, g, b)
    v = max_rgb
    delta = max_rgb - min_rgb
    if max_rgb == 0:
        s = 0
    else:
        s = delta / max_rgb
    if delta == 0:
        h = 0
    else:
        if max_rgb == r:
            h = (g - b) / delta
        elif max_rgb == g:
            h = 2 + (b - r) / delta
        else:
            h = 4 + (r - g) / delta
    h = (h / 6.0) % 1.0  # 转换为 0-1 范围
    return (h, s, v, a / 255.0)

# Helper function to convert HSV to RGBA
def hsv_to_rgba(hsv):
    h, s, v, a = hsv
    h = h * 6  # h 已经是 0-1 范围，现在转换为 0-6
    i = int(h)
    f = h - i
    p = v * (1 - s)
    q = v * (1 - s * f)
    t = v * (1 - s * (1 - f))
    if i == 0:
        r, g, b = v, t, p
    elif i == 1:
        r, g, b = q, v, p
    elif i == 2:
        r, g, b = p, v, t
    elif i == 3:
        r, g, b = p, q, v
    elif i == 4:
        r, g, b = t, p, v
    else:  # i == 5
        r, g, b = v, p, q
    return (int(r * 255), int(g * 255), int(b * 255), int(a * 255))

def adjust_hue_brightness(img, hue_shift=0, brightness_shift=0, saturation_shift=0):
    """
    调整图像的色相、亮度和饱和度。
    """
    log(f"执行色相调整: hue_shift={hue_shift}, brightness_shift={brightness_shift}, saturation_shift={saturation_shift}")
    img = img.convert("RGBA")
    pixels = img.load()
    width, height = img.size

    # 将偏移值转换到合适的范围
    hue_shift_normalized = hue_shift / 360.0  # 将度数转换为0-1之间的偏移量
    brightness_factor = brightness_shift / 100.0  # 将亮度偏移映射到0-1的增量
    saturation_factor = saturation_shift / 100.0  # 将饱和度偏移映射到0-1的增量

    # 用于验证转换是否正常工作的计数器
    changed_pixels = 0
    total_opaque_pixels = 0
    # 用于记录一些代表性像素的转换前后值（调试用）
    sample_pixels_logged = False
    
    for y in range(height):
        for x in range(width):
            r, g, b, a = pixels[x, y]
            if a > 0:  # 只处理不透明的像素
                total_opaque_pixels += 1
                # 转换为HSV
                h, s, v, alpha = rgba_to_hsv((r, g, b, a))

                # 记录一些代表性像素的原始值（仅记录一次）
                if not sample_pixels_logged and (x == width//4 and y == height//4 or x == width//2 and y == height//2):
                    log(f"样本像素 ({x},{y}) 原始值 - RGBA: ({r},{g},{b},{a}), HSV: ({h:.4f},{s:.4f},{v:.4f})")
                    sample_pixels_logged = True

                # 调整H、S 和 V
                h = (h + hue_shift_normalized) % 1.0  # 色相偏移
                s = s + saturation_factor  # 饱和度偏移
                if s > 1.0: s = 1.0
                if s < 0.0: s = 0.0
                v = v + brightness_factor  # 亮度偏移
                if v > 1.0: v = 1.0
                elif v < 0.0: v = 0.0

                # 转回RGBA
                nr, ng, nb, na = hsv_to_rgba((h, s, v, alpha))
                
                # 使用更宽松的像素变化检测，允许微小的浮点误差
                if not all(abs(nc - c) < 1 for nc, c in zip([nr, ng, nb, na], [r, g, b, a])):
                    changed_pixels += 1
                    
                pixels[x, y] = (nr, ng, nb, na)

    log(f"色相调整完成: 共处理 {total_opaque_pixels} 个不透明像素，改变了 {changed_pixels} 个像素 ({changed_pixels/max(1, total_opaque_pixels)*100:.2f}%)")
    return img

def adjust_copper_color(img):
    """
    专门为铜材质设计的颜色调整函数，确保效果明显（更偏红棕色）
    """
    log("执行铜材质专用颜色调整")
    img = img.convert("RGBA")
    pixels = img.load()
    width, height = img.size
    
    changed_pixels = 0
    for y in range(height):
        for x in range(width):
            r, g, b, a = pixels[x, y]
            if a > 0:  # 只处理不透明的像素
                # 修改RGB值，创建更自然的铜色（稍微偏红棕色）
                # 适度增加红色分量，适度降低绿色分量，适度降低蓝色分量
                new_r = min(255, int(r * 1.15 + 25))  # 适度增加红色，使铜稍微红一点
                new_g = max(0, int(g * 0.8 - 10))     # 适度减少绿色，避免金色
                new_b = max(0, int(b * 0.5 - 15))     # 适度减少蓝色，增强铜色
                
                if (new_r, new_g, new_b) != (r, g, b):
                    changed_pixels += 1
                    
                pixels[x, y] = (new_r, new_g, new_b, a)
    
    log(f"铜材质专用颜色调整完成: 共改变了 {changed_pixels} 个像素 ({changed_pixels/(width*height)*100:.2f}%)")
    return img

# Function to process image
def process_image(image_path):
    log(f"Processing image: {image_path}")
    try:
        img = Image.open(image_path).convert("RGBA")
        new_image_data = []
        for item in img.getdata():
            if item[3] == 0:
                new_image_data.append(item)
            else:
                hsva = rgba_to_hsv(item)
                new_hue = 310 / 360.0
                new_saturation = hsva[1] / 3
                new_value = hsva[2] / 3
                new_image_data.append(hsv_to_rgba((new_hue, new_saturation, new_value, hsva[3])))
        new_image = Image.new("RGBA", img.size)
        new_image.putdata(new_image_data)
        new_image.save(image_path)
    except Exception as e:
        log(f"Error processing image '{image_path}': {e}")
        traceback.print_exc()

# Helper function to change white to yellow in an image
def change_white_to_yellow(image):
    data = image.getdata()
    new_data = []
    for item in data:
        if item[3] > 0 and item[0] in range(180, 256) and item[1] in range(180, 256) and item[2] in range(180, 256):
            new_data.append((255, 255, 0, item[3]))
        else:
            new_data.append(item)
    image.putdata(new_data)
    return image

# Function to rename items
def rename_items(items_path_new, rename_pairs):
    log(f"Renaming items in: {items_path_new}")

    # 先处理 .png 文件
    for old_name, new_name in rename_pairs.items():
        try:
            old_png_path = os.path.join(items_path_new, old_name)
            new_png_path = os.path.join(items_path_new, new_name)
            if os.path.exists(old_png_path):
                os.rename(old_png_path, new_png_path)
                log(f"Renamed '{old_name}' to '{new_name}'")
                
                
                # 检查并重命名对应的 .png.mcmeta 文件
                old_meta_path = old_png_path + '.mcmeta'
                new_meta_path = new_png_path + '.mcmeta'
                if os.path.exists(old_meta_path):
                    os.rename(old_meta_path, new_meta_path)
                    log(f"Renamed '{old_meta_path}' to '{new_meta_path}'")
        
        except Exception as e:
            log(f"Error renaming item '{old_name}' to '{new_name}': {e}")
            traceback.print_exc()

    # 然后独立处理 .png.mcmeta 文件
    for old_name, new_name in rename_pairs.items():
        try:
            # 独立处理 .png.mcmeta 文件
            old_meta_path = os.path.join(items_path_new, old_name + '.mcmeta')
            new_meta_path = os.path.join(items_path_new, new_name + '.mcmeta')
            if os.path.exists(old_meta_path) and not os.path.exists(new_meta_path):  # 防止已重命名过的文件再次重命名
                os.rename(old_meta_path, new_meta_path)
                log(f"Renamed '{old_meta_path}' to '{new_meta_path}'")
        
        except Exception as e:
            log(f"Error renaming item '{old_name}.mcmeta' to '{new_name}.mcmeta': {e}")
            traceback.print_exc()

# Function to copy and adjust block images
def copy_and_adjust(blocks_path_new, file_name, new_name, hue_shift, saturation_adjust, brightness_adjust):
    log(f"Copying and adjusting block '{file_name}' to '{new_name}' with hue_shift={hue_shift}, saturation_adjust={saturation_adjust}, brightness_adjust={brightness_adjust}")
    try:
        original_path = os.path.join(blocks_path_new, file_name)
        new_path = os.path.join(blocks_path_new, new_name)
        if os.path.exists(original_path):
            shutil.copy(original_path, new_path)
            img = Image.open(new_path).convert("RGBA")
            img_hsv = img.convert('HSV')
            h, s, v = img_hsv.split()
            h = h.point(lambda p: (p + hue_shift) % 256)
            img_hsv = Image.merge('HSV', (h, s, v))
            img = img_hsv.convert('RGBA')
            enhancer = ImageEnhance.Color(img)
            img = enhancer.enhance(saturation_adjust)
            enhancer = ImageEnhance.Brightness(img)
            img = enhancer.enhance(brightness_adjust)
            img.save(new_path)
            log(f"Copied and adjusted '{file_name}' to '{new_name}' with hue_shift={hue_shift}, saturation_adjust={saturation_adjust}, brightness_adjust={brightness_adjust}")
    except Exception as e:
        log(f"Error copying and adjusting block '{file_name}' to '{new_name}': {e}")
        traceback.print_exc()

def adjust_saturation(img, saturation_adjust):
    """
    调整图像的饱和度，范围为 -100 到 100
    """
    if saturation_adjust == 0:
        return img

    # 将饱和度调整的值映射到适合 ImageEnhance 的系数
    factor = 1 + saturation_adjust / 100.0
    enhancer = ImageEnhance.Color(img)
    return enhancer.enhance(factor)

# Function to process block images
def process_block_image(blocks_path_new, file_name, new_name, hue_shift=0, brightness_adjust=0, saturation_adjust=0):
    log(f"Processing block image '{file_name}' to '{new_name}' with hue_shift={hue_shift}, brightness_adjust={brightness_adjust}, saturation_adjust={saturation_adjust}")
    try:
        original_path = os.path.join(blocks_path_new, file_name)
        new_path = os.path.join(blocks_path_new, new_name)
        if os.path.exists(original_path):
            shutil.copy(original_path, new_path)
            img = Image.open(new_path).convert("RGBA")
            
            # 将偏移值转换到合适的范围
            hue_shift_normalized = hue_shift / 360.0  # 将度数转换为0-1之间的偏移量
            brightness_factor = brightness_adjust / 100.0  # 将亮度偏移映射到0-1的增量
            saturation_factor = saturation_adjust / 100.0  # 将饱和度偏移映射到0-1的增量

            # 获取图片的像素数据
            pixels = img.load()
            width, height = img.size

            # 遍历每个像素进行色相、饱和度和亮度的调整
            for y in range(height):
                for x in range(width):
                    r, g, b, a = pixels[x, y]
                    # 转换为HSV
                    h, s, v, alpha = rgba_to_hsv((r, g, b, a))

                    # 调整H、S 和 V
                    h = (h + hue_shift_normalized) % 1.0  # 色相偏移
                    s = s + saturation_factor  # 饱和度偏移
                    if s > 1.0: s = 1.0
                    if s < 0.0: s = 0.0
                    v = v + brightness_factor  # 亮度偏移
                    if v > 1.0: v = 1.0
                    elif v < 0.0: v = 0.0

                    # 转回RGBA
                    nr, ng, nb, na = hsv_to_rgba((h, s, v, alpha))
                    pixels[x, y] = (nr, ng, nb, na)

            # 保存处理后的图像
            img.save(new_path)
            log(f"Processed '{file_name}' to '{new_name}' with hue_shift={hue_shift}, brightness_adjust={brightness_adjust}, saturation_adjust={saturation_adjust}")

            # 复制并重命名 .mcmeta 文件
            original_mcmeta_path = original_path + '.mcmeta'
            new_mcmeta_path = new_path + '.mcmeta'
            if os.path.exists(original_mcmeta_path):
                shutil.copy(original_mcmeta_path, new_mcmeta_path)
                log(f"Copied and renamed '{original_mcmeta_path}' to '{new_mcmeta_path}'")

    except Exception as e:
        log(f"Error processing block image '{file_name}' to '{new_name}': {e}")
        traceback.print_exc()

def fix_description(description):
    """
    修复包含换行符的 description 字段
    """
    if isinstance(description, list):
        # 如果 description 是列表，递归地修复每个元素
        description = ' '.join([str(item).replace('\n', ' ').replace('\r', '') for item in description])
    elif isinstance(description, str):
        # 如果是字符串，直接处理
        description = description.replace('\n', ' ').replace('\r', '')
    return description

# Helper function to adjust brightness for grayscale images
def adjust_brightness_for_grayscale(image, brightness_adjust):
    img = image.convert("RGB")
    pixels = img.load()
    for i in range(img.width):
        for j in range(img.height):
            r, g, b = pixels[i, j]
            if abs(r - g) <= 2 and abs(r - b) <= 2 and abs(g - b) <= 2:
                new_value = int(r * (1 + (brightness_adjust / 100)))
                new_value = max(0, min(255, new_value))
                pixels[i, j] = (new_value, new_value, new_value)
    return img

# Helper function to adjust hue
def adjust_hue(image, hue_shift):
    img_hsv = image.convert('HSV')
    h, s, v = img_hsv.split()
    h = h.point(lambda p: (p + int(hue_shift * 255 / 360)) % 256)
    img_hsv = Image.merge('HSV', (h, s, v))
    return img_hsv.convert('RGBA')

# Function to rename and process blocks with optional reversal
def rename_and_process_blocks(blocks_path_new, reverse=False):
    log(f"Renaming and processing blocks in: {blocks_path_new}")
    rename_pairs = {
        'stone_granite.png': 'granite.png',
        'stone_granite_smooth.png': 'polished_granite.png',
        'stone_diorite.png': 'diorite.png',
        'stone_diorite_smooth.png': 'polished_diorite.png',
        'stone_andesite.png': 'andesite.png',
        'stone_andesite_smooth.png': 'polished_andesite.png',
        'grass_side.png': 'grass_block_side.png',
        'grass_top.png': 'grass_block_top.png',
        'dirt_podzol_side.png': 'podzol_side.png',
        'dirt_podzol_top.png': 'podzol_top.png',
        'planks_acacia.png': 'acacia_planks.png',
        'planks_big_oak.png': 'dark_oak_planks.png',
        'planks_birch.png': 'birch_planks.png',
        'planks_jungle.png': 'jungle_planks.png',
        'planks_spruce.png': 'spruce_planks.png',
        'planks_oak.png': 'oak_planks.png',
        'quartz_ore.png': 'nether_quartz_ore.png',
        'sponge_wet.png': 'wet_sponge.png',
        'sandstone_normal.png': 'sandstone.png',
        'sandstone_carved.png': 'chiseled_sandstone.png',
        'sandstone_smooth.png': 'cut_sandstone.png',
        'red_sandstone_normal.png': 'red_sandstone.png',
        'red_sandstone_carved.png': 'chiseled_red_sandstone.png',
        'red_sandstone_smooth.png': 'cut_red_sandstone.png',
        'wool_colored_black.png': 'black_wool.png',
        'wool_colored_blue.png': 'blue_wool.png',
        'wool_colored_brown.png': 'brown_wool.png',
        'wool_colored_cyan.png': 'cyan_wool.png',
        'wool_colored_gray.png': 'gray_wool.png',
        'wool_colored_green.png': 'green_wool.png',
        'wool_colored_light_blue.png': 'light_blue_wool.png',
        'wool_colored_lime.png': 'lime_wool.png',
        'wool_colored_magenta.png': 'magenta_wool.png',
        'wool_colored_orange.png': 'orange_wool.png',
        'wool_colored_pink.png': 'pink_wool.png',
        'wool_colored_purple.png': 'purple_wool.png',
        'wool_colored_red.png': 'red_wool.png',
        'wool_colored_silver.png': 'light_gray_wool.png',
        'wool_colored_white.png': 'white_wool.png',
        'wool_colored_yellow.png': 'yellow_wool.png',
        'stone_slab_side.png': 'smooth_stone_slab_side.png',
        'stone_slab_top.png': 'smooth_stone.png',
        'brick.png': 'bricks.png',
        'nether_brick.png': 'nether_bricks.png',
        'stonebrick.png': 'stone_bricks.png',
        'stonebrick_carved.png': 'chiseled_stone_bricks.png',
        'stonebrick_mossy.png': 'mossy_stone_bricks.png',
        'quartz_block_chiseled.png': 'chiseled_quartz_block.png',
        'quartz_block_lines.png': 'quartz_pillar.png',
        'quartz_block_lines_top.png': 'quartz_pillar_top.png',
        'prismarine_dark.png': 'dark_prismarine.png',
        'prismarine_rough.png': 'prismarine.png',
        'prismarine_rough.png.mcmeta': 'prismarine.png.mcmeta',
        'anvil_base.png': 'anvil.png',
        'anvil_top_damaged_0.png': 'anvil_top.png',
        'anvil_top_damaged_1.png': 'chipped_anvil_top.png',
        'anvil_top_damaged_2.png': 'damaged_anvil_top.png',
        'carrots_stage_0.png': 'carrots_stage0.png',
        'carrots_stage_1.png': 'carrots_stage1.png',
        'carrots_stage_2.png': 'carrots_stage2.png',
        'carrots_stage_3.png': 'carrots_stage3.png',
        'cobblestone_mossy.png': 'mossy_cobblestone.png',
        'cocoa_stage_0.png': 'cocoa_stage0.png',
        'cocoa_stage_1.png': 'cocoa_stage1.png',
        'cocoa_stage_2.png': 'cocoa_stage2.png',
        'comparator_off.png': 'comparator.png',
        'deadbush.png': 'dead_bush.png',
        'dispenser_front_horizontal.png': 'dispenser_horizontal.png',
        'door_acacia_lower.png': 'acacia_door_bottom.png',
        'door_acacia_upper.png': 'acacia_door_top.png',
        'door_birch_lower.png': 'birch_door_bottom.png',
        'door_birch_upper.png': 'birch_door_top.png',
        'door_dark_oak_lower.png': 'dark_oak_door_bottom.png',
        'door_dark_oak_upper.png': 'dark_oak_door_top.png',
        'door_iron_lower.png': 'iron_door_bottom.png',
        'door_iron_upper.png': 'iron_door_top.png',
        'door_jungle_lower.png': 'jungle_door_bottom.png',
        'door_jungle_upper.png': 'jungle_door_top.png',
        'door_spruce_lower.png': 'spruce_door_bottom.png',
        'door_spruce_upper.png': 'spruce_door_top.png',
        'door_wood_lower.png': 'oak_door_bottom.png',
        'door_wood_upper.png': 'oak_door_top.png',
        'double_plant_fern_bottom.png': 'large_fern_bottom.png',
        'double_plant_fern_top.png': 'large_fern_top.png',
        'double_plant_grass_bottom.png': 'tall_grass_bottom.png',
        'double_plant_grass_top.png': 'tall_grass_top.png',
        'double_plant_paeonia_bottom.png': 'peony_bottom.png',
        'double_plant_paeonia_top.png': 'peony_top.png',
        'double_plant_rose_bottom.png': 'rose_bush_bottom.png',
        'double_plant_rose_top.png': 'rose_bush_top.png',
        'double_plant_sunflower_back.png': 'sunflower_back.png',
        'double_plant_sunflower_bottom.png': 'sunflower_bottom.png',
        'double_plant_sunflower_top.png': 'sunflower_top.png',
        'double_plant_sunflower_front.png': 'sunflower_front.png',
        'double_plant_syringa_bottom.png': 'lilac_bottom.png',
        'double_plant_syringa_top.png': 'lilac_top.png',
        'dropper_front_horizontal.png': 'dropper_front.png',
        'endframe_eye.png': 'end_portal_frame_eye.png',
        'endframe_side.png': 'end_portal_frame_side.png',
        'endframe_top.png': 'end_portal_frame_top.png',
        'farmland_dry.png': 'farmland.png',
        'farmland_wet.png': 'farmland_moist.png',
        'fire_layer_0.png': 'fire_0.png',
        'fire_layer_1.png': 'fire_1.png',
        'flower_allium.png': 'allium.png',
        'flower_blue_orchid.png': 'blue_orchid.png',
        'flower_dandelion.png': 'dandelion.png',
        'flower_houstonia.png': 'azure_bluet.png',
        'flower_oxeye_daisy.png': 'oxeye_daisy.png',
        'flower_rose.png': 'poppy.png',
        'flower_tulip_orange.png': 'orange_tulip.png',
        'flower_tulip_pink.png': 'pink_tulip.png',
        'flower_tulip_red.png': 'red_tulip.png',
        'flower_tulip_white.png': 'white_tulip.png',
        'furnace_front_off.png': 'furnace_front.png',
        'glass_black.png': 'black_stained_glass.png',
        'glass_blue.png': 'blue_stained_glass.png',
        'glass_brown.png': 'brown_stained_glass.png',
        'glass_cyan.png': 'cyan_stained_glass.png',
        'glass_gray.png': 'gray_stained_glass.png',
        'glass_green.png': 'green_stained_glass.png',
        'glass_light_blue.png': 'light_blue_stained_glass.png',
        'glass_lime.png': 'lime_stained_glass.png',
        'glass_magenta.png': 'magenta_stained_glass.png',
        'glass_orange.png': 'orange_stained_glass.png',
        'glass_pink.png': 'pink_stained_glass.png',
        'glass_purple.png': 'purple_stained_glass.png',
        'glass_red.png': 'red_stained_glass.png',
        'glass_silver.png': 'light_gray_stained_glass.png',
        'glass_white.png': 'white_stained_glass.png',
        'glass_yellow.png': 'yellow_stained_glass.png',
        'glass_pane_top_black.png': 'black_stained_glass_pane_top.png',
        'glass_pane_top_blue.png': 'blue_stained_glass_pane_top.png',
        'glass_pane_top_brown.png': 'brown_stained_glass_pane_top.png',
        'glass_pane_top_cyan.png': 'cyan_stained_glass_pane_top.png',
        'glass_pane_top_gray.png': 'gray_stained_glass_pane_top.png',
        'glass_pane_top_green.png': 'green_stained_glass_pane_top.png',
        'glass_pane_top_light_blue.png': 'light_blue_stained_glass_pane_top.png',
        'glass_pane_top_lime.png': 'lime_stained_glass_pane_top.png',
        'glass_pane_top_magenta.png': 'magenta_stained_glass_pane_top.png',
        'glass_pane_top_orange.png': 'orange_stained_glass_pane_top.png',
        'glass_pane_top_pink.png': 'pink_stained_glass_pane_top.png',
        'glass_pane_top_purple.png': 'purple_stained_glass_pane_top.png',
        'glass_pane_top_red.png': 'red_stained_glass_pane_top.png',
        'glass_pane_top_silver.png': 'light_gray_stained_glass_pane_top.png',
        'glass_pane_top_white.png': 'white_stained_glass_pane_top.png',
        'glass_pane_top_yellow.png': 'yellow_stained_glass_pane_top.png',
        'grass_side_overlay.png': 'grass_block_side_overlay.png',
        'grass_side_snowed.png': 'grass_block_snow.png',
        'hardened_clay.png': 'terracotta.png',
        'hardened_clay_stained_black.png': 'black_terracotta.png',
        'hardened_clay_stained_blue.png': 'blue_terracotta.png',
        'hardened_clay_stained_brown.png': 'brown_terracotta.png',
        'hardened_clay_stained_cyan.png': 'cyan_terracotta.png',
        'hardened_clay_stained_gray.png': 'gray_terracotta.png',
        'hardened_clay_stained_green.png': 'green_terracotta.png',
        'hardened_clay_stained_light_blue.png': 'light_blue_terracotta.png',
        'hardened_clay_stained_lime.png': 'lime_terracotta.png',
        'hardened_clay_stained_magenta.png': 'magenta_terracotta.png',
        'hardened_clay_stained_orange.png': 'orange_terracotta.png',
        'hardened_clay_stained_pink.png': 'pink_terracotta.png',
        'hardened_clay_stained_purple.png': 'purple_terracotta.png',
        'hardened_clay_stained_red.png': 'red_terracotta.png',
        'hardened_clay_stained_silver.png': 'light_gray_terracotta.png',
        'hardened_clay_stained_white.png': 'white_terracotta.png',
        'hardened_clay_stained_yellow.png': 'yellow_terracotta.png',
        'ice_packed.png': 'packed_ice.png',
        'itemframe_background.png': 'item_frame.png',
        'leaves_acacia.png': 'acacia_leaves.png',
        'leaves_big_oak.png': 'dark_oak_leaves.png',
        'leaves_birch.png': 'birch_leaves.png',
        'leaves_jungle.png': 'jungle_leaves.png',
        'leaves_oak.png': 'oak_leaves.png',
        'leaves_spruce.png': 'spruce_leaves.png',
        'log_acacia.png': 'acacia_log.png',
        'log_acacia_top.png': 'acacia_log_top.png',
        'log_big_oak.png': 'dark_oak_log.png',
        'log_big_oak_top.png': 'dark_oak_log_top.png',
        'log_spruce.png': 'spruce_log.png',
        'log_spruce_top.png': 'spruce_log_top.png',
        'log_birch.png': 'birch_log.png',
        'log_birch_top.png': 'birch_log_top.png',
        'log_jungle.png': 'jungle_log.png',
        'log_jungle_top.png': 'jungle_log_top.png',   
        'log_oak.png': 'oak_log.png',
        'log_oak_top.png': 'oak_log_top.png', 
        'melon_stem_connected.png': 'attached_melon_stem.png',
        'melon_stem_disconnected.png': 'melon_stem.png',
        'mob_spawner.png': 'spawner.png',
        'mushroom_block_skin_brown.png': 'brown_mushroom_block.png',
        'mushroom_block_skin_red.png': 'red_mushroom_block.png',
        'mushroom_block_skin_stem.png': 'mushroom_stem.png',
        'mushroom_brown.png': 'brown_mushroom.png',
        'mushroom_red.png': 'red_mushroom.png',
        'nether_wart_stage_0.png': 'nether_wart_stage0.png',
        'nether_wart_stage_1.png': 'nether_wart_stage1.png',
        'nether_wart_stage_2.png': 'nether_wart_stage2.png',
        'noteblock.png': 'note_block.png',
        'piston_top_normal.png': 'piston_top.png',
        'portal.png': 'nether_portal.png',
        'potatoes_stage_0.png': 'potatoes_stage0.png',
        'potatoes_stage_1.png': 'potatoes_stage1.png',
        'potatoes_stage_2.png': 'potatoes_stage2.png',
        'potatoes_stage_3.png': 'potatoes_stage3.png',
        'pumpkin_face_off.png': 'carved_pumpkin.png',
        'pumpkin_face_on.png': 'jack_o_lantern.png',
        'pumpkin_stem_connected.png': 'attached_pumpkin_stem.png',
        'pumpkin_stem_disconnected.png': 'pumpkin_stem.png',
        'rail_activator.png': 'activator_rail.png',
        'rail_activator_powered.png': 'activator_rail_on.png',
        'rail_detector.png': 'detector_rail.png',
        'rail_detector_powered.png': 'detector_rail_on.png',
        'rail_golden.png': 'powered_rail.png',
        'rail_golden_powered.png': 'powered_rail_on.png',
        'rail_normal.png': 'rail.png',
        'rail_normal_turned.png': 'rail_corner.png',
        'redstone_dust_cross_overlay.png': 'redstone_dust_overlay.png',
        'redstone_lamp_off.png': 'redstone_lamp.png',
        'redstone_torch_on.png': 'redstone_torch.png',
        'reeds.png': 'sugar_cane.png',
        'repeater_off.png': 'repeater.png',
        'sapling_acacia.png': 'acacia_sapling.png',
        'sapling_birch.png': 'birch_sapling.png',
        'sapling_jungle.png': 'jungle_sapling.png',
        'sapling_oak.png': 'oak_sapling.png',
        'sapling_roofed_oak.png': 'dark_oak_sapling.png',
        'sapling_spruce.png': 'spruce_sapling.png',
        'stonebrick_cracked.png': 'cracked_stone_bricks.png',
        'slime.png': 'slime_block.png',
        'tallgrass.png': 'grass.png',
        'torch_on.png': 'torch.png',
        'trapdoor.png': 'oak_trapdoor.png',
        'trip_wire_source.png': 'tripwire_hook.png',
        'waterlily.png': 'lily_pad.png',
        'web.png': 'cobweb.png',
        'wheat_stage_0.png': 'wheat_stage0.png',
        'wheat_stage_1.png': 'wheat_stage1.png',
        'wheat_stage_2.png': 'wheat_stage2.png',
        'wheat_stage_3.png': 'wheat_stage3.png',
        'wheat_stage_4.png': 'wheat_stage4.png',
        'wheat_stage_5.png': 'wheat_stage5.png',
        'wheat_stage_6.png': 'wheat_stage6.png',
        'wheat_stage_7.png': 'wheat_stage7.png'
    }

    if reverse:
        # 反转 rename_pairs 字典
        rename_pairs = {v: k for k, v in rename_pairs.items()}
        log("Reversed rename_pairs for 1.18 to 1.8 conversion")

    rename_items(blocks_path_new, rename_pairs)
    process_redstone_dust_cross_image(blocks_path_new)
    process_redstone_dust_line_image(blocks_path_new)
    process_block_image(blocks_path_new, 'oak_planks.png', 'warped_planks.png', hue_shift=130, brightness_adjust=-33, saturation_adjust=0)
    process_block_image(blocks_path_new, 'oak_planks.png', 'crimson_planks.png', hue_shift=-59, brightness_adjust=-30, saturation_adjust=0)
    ores = ['coal_ore', 'iron_ore', 'gold_ore', 'diamond_ore', 'emerald_ore', 'redstone_ore', 'lapis_ore']
    for ore in ores:
        process_block_image(blocks_path_new, f'{ore}.png', f'deepslate_{ore}.png', hue_shift=0, brightness_adjust=-20, saturation_adjust=0)
        if ore == 'redstone_ore':
            process_block_image(blocks_path_new, 'redstone_ore.png', 'copper_ore.png', hue_shift=26, saturation_adjust=0)
            process_block_image(blocks_path_new, 'copper_ore.png', 'deepslate_copper_ore.png', hue_shift=0, brightness_adjust=-20, saturation_adjust=0)

    quartz_path = os.path.join(blocks_path_new, 'nether_quartz_ore.png')
    gold_path = os.path.join(blocks_path_new, 'nether_gold_ore.png')
    if os.path.exists(quartz_path):
        shutil.copy(quartz_path, gold_path)
        log(f"Copied and renamed 'nether_quartz_ore.png' to 'nether_gold_ore.png'")
        img = Image.open(gold_path).convert("RGBA")
        img = change_white_to_yellow(img)
        img.save(gold_path)
        log(f"Processed image 'nether_gold_ore.png'")

# Helper function to mirror image horizontally
def mirror_image_horizontally(image):
    return image.transpose(Image.FLIP_LEFT_RIGHT)

# Helper function to overlay images
def overlay_images(background_image, overlay_image, position):
    new_image = background_image.copy()
    new_image.paste(overlay_image, position, overlay_image)
    return new_image

def process_tipped_arrow(items_path, tipped_arrow_head_dir):
    try:
        arrow_path = os.path.join(items_path, 'arrow.png')
        tipped_arrow_base_path = os.path.join(items_path, 'tipped_arrow_base.png')

        if os.path.exists(arrow_path):
            img = Image.open(arrow_path).convert("RGBA")
            size = img.size[0]  # 假设图片为正方形

            tipped_arrow_head_file = f'tipped_arrow_head_{size}.png'
            tipped_arrow_head_path = os.path.join(tipped_arrow_head_dir, tipped_arrow_head_file)

            if os.path.exists(tipped_arrow_head_path):
                shutil.copy(arrow_path, tipped_arrow_base_path)
                base_img = Image.open(tipped_arrow_base_path).convert("RGBA")
                head_img = Image.open(tipped_arrow_head_path).convert("RGBA")

                # 将 tipped_arrow_head_X.png 与 arrow.png 重合的像素改为透明像素
                base_data = base_img.getdata()
                head_data = head_img.getdata()
                new_base_data = [
                    (base_pixel[0], base_pixel[1], base_pixel[2], 0) if head_pixel[3] > 0 else base_pixel
                    for base_pixel, head_pixel in zip(base_data, head_data)
                ]

                base_img.putdata(new_base_data)
                base_img.save(tipped_arrow_base_path)
                log(f"已处理 'tipped_arrow_base.png' 通过使 '{tipped_arrow_head_file}' 重叠的像素透明")

                # 将 tipped_arrow_head_X.png 文件复制并重命名为 tipped_arrow_head.png
                new_tipped_arrow_head_path = os.path.join(items_path, 'tipped_arrow_head.png')
                shutil.copy(tipped_arrow_head_path, new_tipped_arrow_head_path)
                log(f"已复制并重命名 '{tipped_arrow_head_file}' 为 'tipped_arrow_head.png'")
    except Exception as e:
        log(f"处理 'tipped_arrow_base.png' 时出错: {e}")
        traceback.print_exc()

def process_icons_in_dir(temp_dir):
    """
    处理解压后的目录中的assets/minecraft/textures/gui/icons.png，裁剪并保存指定的小图像。
    """
    icons_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'icons.png')
    
    if not os.path.exists(icons_path):
        log(f"icons.png 未在 {icons_path} 找到。")
        return
    
    # 打开图像
    try:
        img = Image.open(icons_path).convert("RGBA")
    except Exception as e:
        log(f"无法打开 icons.png: {e}")
        return
    
    width, height = img.size

    # 确定 scale_factor
    if width == 256 and height == 256:
        scale_factor = 1
    elif width == 512 and height == 512:
        scale_factor = 2
    elif width == 1024 and height == 1024:
        scale_factor = 4
    elif width == 2048 and height == 2048:
        scale_factor = 8
    else:
        log(f"不支持的 icons.png 尺寸: {width}x{height}")
        return
    
    log(f"处理 icons.png，尺寸: {width}x{height}, scale_factor: {scale_factor}")

    def scaled_box(x1, y1, x2, y2):
        return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

    # 定义所有裁剪操作
    operations = [
        # Step 1
        {
            'crop': (0, 0, 15, 15),
            'split': None,
            'save_names': ['crosshair.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 2
        {
            'crop': (16, 0, 196, 9),
            'split': 'horizontal',
            'slice_size': (9, 9),
            'save_names': [
                'container.png', 'container_blinking.png', 'wtf.png', 'wtf2.png', 'full.png', 'half.png',
                'full_blinking.png', 'half_blinking.png', 'poisoned_full.png', 'poisoned_half.png',
                'poisoned_full_blinking.png', 'poisoned_half_blinking.png',
                'withered_full.png', 'withered_half.png',
                'withered_full_blinking.png', 'withered_half_blinking.png',
                'absorbing_full.png', 'absorbing_half.png',
                'frozen_full.png', 'frozen_half.png'
            ],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud', 'heart')
        },
        # Step 3a
        {
            'crop': (16, 9, 124, 18),
            'split': 'horizontal',
            'slice_size': (9, 9),
            'save_names': ['armor_empty.png', 'armor_half.png', 'armor_full.png', 'wtf3.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 3b
        {
            'crop': (52, 9, 124, 18),
            'split': 'horizontal',
            'slice_size': (9, 9),
            'save_names': [
                'vehicle_container.png', 'wtf4.png', 'wtf5.png', 'wtf6.png',
                'vehicle_full.png', 'vehicle_half.png', 'wtf7.png', 'wtf8.png'
            ],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud', 'heart')
        },
        # Step 4
        {
            'crop': (16, 18, 52, 27),
            'split': 'horizontal',
            'slice_size': (9, 9),
            'save_names': ['air.png', 'air_bursting.png', 'wtf9.png', 'wtf10.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 5
        {
            'crop': (16, 27, 142, 36),
            'split': 'horizontal',
            'slice_size': (9, 9),
            'save_names': [
                'food_empty.png', 'wtf11.png', 'wtf123.png', 'wtf13.png',
                'food_full.png', 'food_half.png', 'wtf14.png', 'wtf15.png',
                'food_full_hunger.png', 'food_half_hunger.png',
                'wtf16.png', 'wtf17.png', 'wtf18.png', 'food_empty_hunger.png'
            ],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 6
        {
            'crop': (16, 45, 196, 54),
            'split': 'horizontal',
            'slice_size': (9, 9),
            'save_names': [
                'container_hardcore.png', 'container_hardcore_blinking.png', 'wtf19.png', 'wtf20.png',
                'hardcore_full.png', 'hardcore_half.png',
                'hardcore_full_blinking.png', 'hardcore_half_blinking.png',
                'poisoned_hardcore_full.png', 'poisoned_hardcore_half.png',
                'poisoned_hardcore_full_blinking.png', 'poisoned_hardcore_half_blinking.png',
                'withered_hardcore_full.png', 'withered_hardcore_half.png',
                'withered_hardcore_full_blinking.png', 'withered_hardcore_half_blinking.png',
                'absorbing_hardcore_full.png', 'absorbing_hardcore_half.png',
                'frozen_hardcore_full.png', 'frozen_hardcore_half.png'
            ],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud', 'heart')
        },
        # Step 7
        {
            'crop': (0, 15, 10, 63),
            'split': 'vertical',
            'slice_size': (10, 8),
            'save_names': ['ping_5.png', 'ping_4.png', 'ping_3.png', 'ping_2.png', 'ping_1.png', 'ping_unknown.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'icon')
        },
        # Step 8
        {
            'crop': (0, 64, 182, 94),
            'split': 'vertical',
            'slice_size': (182, 5),
            'save_names': [
                'experience_bar_background.png',
                'experience_bar_progress.png',
                'jump_bar_cooldown.png',
                'wtf21.png',
                'jump_bar_background.png',
                'jump_bar_progress.png'
            ],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 9
        {
            'crop': (0, 94, 18, 112),
            'split': None,
            'save_names': ['hotbar_attack_indicator_background.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 10
        {
            'crop': (18, 94, 36, 112),
            'split': None,
            'save_names': ['hotbar_attack_indicator_progress.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 11
        {
            'crop': (36, 94, 52, 98),
            'split': None,
            'save_names': ['crosshair_attack_indicator_background.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 12
        {
            'crop': (52, 94, 68, 98),
            'split': None,
            'save_names': ['crosshair_attack_indicator_progress.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 13
        {
            'crop': (68, 94, 84, 110),
            'split': None,
            'save_names': ['crosshair_attack_indicator_full.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 14
        {
            'crop': (0, 176, 10, 224),
            'split': 'vertical',
            'slice_size': (10, 8),
            'save_names': ['ping_5.png', 'ping_4.png', 'ping_3.png', 'ping_2.png', 'ping_1.png', 'unreachable.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'server_list')
        },
        # Step 15
        {
            'crop': (10, 176, 20, 216),
            'split': 'vertical',
            'slice_size': (10, 8),
            'save_names': ['pinging_5.png', 'pinging_4.png', 'pinging_3.png', 'pinging_2.png', 'pinging_1.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'server_list')
        },
    ]

    # 确保所有目标目录存在
    for op in operations:
        target_dir = op['target_dir']
        if not os.path.exists(target_dir):
            os.makedirs(target_dir)
            log(f"已创建目录: {target_dir}")

    for op_index, op in enumerate(operations, start=1):
        # 裁剪区域
        crop_region = scaled_box(*op['crop'])
        cropped_img = img.crop(crop_region)

        # 处理拆分
        slices = []
        if op['split'] == 'horizontal':
            base_slice_width, base_slice_height = op['slice_size']
            slice_width, slice_height = base_slice_width * scale_factor, base_slice_height * scale_factor
            num_slices = len(op['save_names'])
            for i in range(num_slices):
                left = i * slice_width
                upper = 0
                right = left + slice_width
                lower = upper + slice_height
                slice_img = cropped_img.crop((left, upper, right, lower))
                slices.append(slice_img)
        elif op['split'] == 'vertical':
            base_slice_width, base_slice_height = op['slice_size']
            slice_width, slice_height = base_slice_width * scale_factor, base_slice_height * scale_factor
            num_slices = len(op['save_names'])
            for i in range(num_slices):
                left = 0
                upper = i * slice_height
                right = slice_width
                lower = upper + slice_height
                slice_img = cropped_img.crop((left, upper, right, lower))
                slices.append(slice_img)
        else:
            slices = [cropped_img]

        # 保存切片
        for idx, save_name in enumerate(op['save_names']):
            if op['split'] in ['horizontal', 'vertical']:
                if idx < len(slices):
                    slice_img = slices[idx]
                else:
                    log(f"[操作 {op_index}] 切片数量不足，跳过 {save_name}")
                    continue
            else:
                slice_img = slices[0]

            # 定义目标路径
            target_path = os.path.join(op['target_dir'], save_name).replace("\\", "/")  # 确保路径使用正斜杠

            try:
                slice_img.save(target_path, format='PNG')
                log(f"[操作 {op_index}] 已保存 {target_path}，尺寸: {slice_img.size}")
            except Exception as e:
                log(f"[操作 {op_index}] 保存 {target_path} 时出错: {e}")

    log("所有 icons.png 相关的图像已成功处理并保存。")

def process_widgets_in_dir(temp_dir):
    """
    处理解压后的目录中的assets/minecraft/textures/gui/widgets.png，裁剪并保存指定的小图像。
    """
    widgets_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'widgets.png')
    
    if not os.path.exists(widgets_path):
        log(f"widgets.png 未在 {widgets_path} 找到。")
        return
    
    # 打开图像
    try:
        img = Image.open(widgets_path).convert("RGBA")
    except Exception as e:
        log(f"无法打开 widgets.png: {e}")
        return
    
    width, height = img.size

    # 确定 scale_factor
    if width == 256 and height == 256:
        scale_factor = 1
    elif width == 512 and height == 512:
        scale_factor = 2
    elif width == 1024 and height == 1024:
        scale_factor = 4
    elif width == 2048 and height == 2048:
        scale_factor = 8
    else:
        log(f"不支持的 widgets.png 尺寸: {width}x{height}")
        return
    
    log(f"处理 widgets.png，尺寸: {width}x{height}, scale_factor: {scale_factor}")

    def scaled_box(x1, y1, x2, y2):
        return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

    # 定义所有裁剪操作
    operations = [
        # Step 1
        {
            'crop': (0, 0, 182, 22),
            'split': None,
            'save_names': ['hotbar.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 2
        {
            'crop': (0, 22, 24, 45),
            'split': None,
            'save_names': ['hotbar_selection.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 3
        {
            'crop': (24, 22, 53, 46),
            'split': None,
            'save_names': ['hotbar_offhand_left.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 4
        {
            'crop': (53, 22, 82, 46),
            'split': None,
            'save_names': ['hotbar_offhand_right.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'hud')
        },
        # Step 5
        {
            'crop': (0, 46, 200, 66),
            'split': None,
            'save_names': ['button_disabled.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'widget')
        },
        # Step 6
        {
            'crop': (0, 66, 200, 86),
            'split': None,
            'save_names': ['button.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'widget')
        },
        # Step 7
        {
            'crop': (0, 86, 200, 106),
            'split': None,
            'save_names': ['button_highlighted.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'widget')
        },
        # Step 8
        {
            'crop': (3, 109, 18, 124),
            'split': None,
            'save_names': ['language.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'icon')
        },
        # Step 9
        {
            'crop': (0, 146, 20, 206),
            'split': 'vertical',
            'slice_size': (20, 20),
            'save_names': ['locked_button.png', 'locked_button_highlighted.png', 'locked_button_disabled.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'widget')
        },
        # Step 10
        {
            'crop': (20, 146, 40, 206),
            'split': 'vertical',
            'slice_size': (20, 20),
            'save_names': ['unlocked_button.png', 'unlocked_button_highlighted.png', 'unlocked_button_disabled.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'widget')
        },
    ]

    # 确保所有目标目录存在
    for op in operations:
        target_dir = op['target_dir']
        if not os.path.exists(target_dir):
            os.makedirs(target_dir)
            log(f"已创建目录: {target_dir}")

    for op_index, op in enumerate(operations, start=1):
        # 裁剪区域
        crop_region = scaled_box(*op['crop'])
        try:
            cropped_img = img.crop(crop_region)
        except Exception as e:
            log(f"[操作 {op_index}] 裁剪区域 {op['crop']} 时出错: {e}")
            continue

        # 处理拆分
        slices = []
        if op['split'] == 'horizontal':
            base_slice_width, base_slice_height = op['slice_size']
            slice_width, slice_height = base_slice_width * scale_factor, base_slice_height * scale_factor
            num_slices = len(op['save_names'])
            for i in range(num_slices):
                left = i * slice_width
                upper = 0
                right = left + slice_width
                lower = upper + slice_height
                try:
                    slice_img = cropped_img.crop((left, upper, right, lower))
                    slices.append(slice_img)
                except Exception as e:
                    log(f"[操作 {op_index}] 拆分切片 {i} 时出错: {e}")
        elif op['split'] == 'vertical':
            base_slice_width, base_slice_height = op['slice_size']
            slice_width, slice_height = base_slice_width * scale_factor, base_slice_height * scale_factor
            num_slices = len(op['save_names'])
            for i in range(num_slices):
                left = 0
                upper = i * slice_height
                right = slice_width
                lower = upper + slice_height
                try:
                    slice_img = cropped_img.crop((left, upper, right, lower))
                    slices.append(slice_img)
                except Exception as e:
                    log(f"[操作 {op_index}] 拆分切片 {i} 时出错: {e}")
        else:
            slices = [cropped_img]

        # 保存切片
        for idx, save_name in enumerate(op['save_names']):
            if op['split'] in ['horizontal', 'vertical']:
                if idx < len(slices):
                    slice_img = slices[idx]
                else:
                    log(f"[操作 {op_index}] 切片数量不足，跳过 {save_name}")
                    continue
            else:
                slice_img = slices[0]

            # 定义目标路径
            target_path = os.path.join(op['target_dir'], save_name).replace("\\", "/")  # 确保路径使用正斜杠

            try:
                slice_img.save(target_path, format='PNG')
                log(f"[操作 {op_index}] 已保存 {target_path}，尺寸: {slice_img.size}")
            except Exception as e:
                log(f"[操作 {op_index}] 保存 {target_path} 时出错: {e}")

    log("所有 widgets.png 相关的图像已成功处理并保存。")

def process_tabs_in_dir(temp_dir):
    """
    处理解压后的目录中的assets/minecraft/textures/gui/container/creative_inventory/tabs.png，
    """
    tabs_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container', 'creative_inventory', 'tabs.png')
    
    if not os.path.exists(tabs_path):
        log(f"tabs.png 未在 {tabs_path} 找到。")
        return
    
    # 打开图像
    try:
        img = Image.open(tabs_path).convert("RGBA")
    except Exception as e:
        log(f"无法打开 tabs.png: {e}")
        return
    
    width, height = img.size

    # 确定 scale_factor
    if width == 256 and height == 256:
        scale_factor = 1
    elif width == 512 and height == 512:
        scale_factor = 2
    elif width == 1024 and height == 1024:
        scale_factor = 4
    elif width == 2048 and height == 2048:
        scale_factor = 8
    else:
        log(f"不支持的 tabs.png 尺寸: {width}x{height}")
        return
    
    log(f"处理 tabs.png，尺寸: {width}x{height}, scale_factor: {scale_factor}")

    def scaled_box(x1, y1, x2, y2):
        return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

    # 定义所有裁剪操作
    operations = [
        # Step 1: (0,2)-(168,32) -> 6 slices 28x30, duplicate 6th as 7th
        {
            'crop': (0, 2, 168, 32),
            'split': 'horizontal',
            'slice_size': (28, 30),
            'save_names': [
                'tab_top_unselected_1.png', 'tab_top_unselected_2.png', 'tab_top_unselected_3.png',
                'tab_top_unselected_4.png', 'tab_top_unselected_5.png', 'tab_top_unselected_6.png'
            ],
            'duplicate_save_name': 'tab_top_unselected_7.png',
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'creative_inventory')
        },
        # Step 2: (0,32)-(168,64) -> 6 slices 28x32, duplicate 6th as 7th
        {
            'crop': (0, 32, 168, 64),
            'split': 'horizontal',
            'slice_size': (28, 32),
            'save_names': [
                'tab_top_selected_1.png', 'tab_top_selected_2.png', 'tab_top_selected_3.png',
                'tab_top_selected_4.png', 'tab_top_selected_5.png', 'tab_top_selected_6.png'
            ],
            'duplicate_save_name': 'tab_top_selected_7.png',
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'creative_inventory')
        },
        # Step 3: (0,64)-(168,94) -> 6 slices 28x30, duplicate 6th as 7th
        {
            'crop': (0, 64, 168, 94),
            'split': 'horizontal',
            'slice_size': (28, 30),
            'save_names': [
                'tab_bottom_unselected_1.png', 'tab_bottom_unselected_2.png', 'tab_bottom_unselected_3.png',
                'tab_bottom_unselected_4.png', 'tab_bottom_unselected_5.png', 'tab_bottom_unselected_6.png'
            ],
            'duplicate_save_name': 'tab_bottom_unselected_7.png',
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'creative_inventory')
        },
        # Step 4: (0,96)-(168,128) -> 6 slices 28x32, duplicate 6th as 7th
        {
            'crop': (0, 96, 168, 128),
            'split': 'horizontal',
            'slice_size': (28, 32),
            'save_names': [
                'tab_bottom_selected_1.png', 'tab_bottom_selected_2.png', 'tab_bottom_selected_3.png',
                'tab_bottom_selected_4.png', 'tab_bottom_selected_5.png', 'tab_bottom_selected_6.png'
            ],
            'duplicate_save_name': 'tab_bottom_selected_7.png',
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'creative_inventory')
        },
    ]

    # 确保所有目标目录存在
    for op in operations:
        target_dir = op['target_dir']
        if not os.path.exists(target_dir):
            os.makedirs(target_dir)
            log(f"已创建目录: {target_dir}")

    for op_index, op in enumerate(operations, start=1):
        # 裁剪区域
        crop_region = scaled_box(*op['crop'])
        try:
            cropped_img = img.crop(crop_region)
            log(f"[操作 {op_index}] 裁剪区域: {op['crop']} -> {crop_region}")
        except Exception as e:
            log(f"[操作 {op_index}] 裁剪区域 {op['crop']} 时出错: {e}")
            continue

        # 处理拆分
        slices = []
        if op['split'] == 'horizontal':
            base_slice_width, base_slice_height = op['slice_size']
            slice_width, slice_height = base_slice_width * scale_factor, base_slice_height * scale_factor
            num_slices = len(op['save_names'])
            for i in range(num_slices):
                left = i * slice_width
                upper = 0
                right = left + slice_width
                lower = upper + slice_height
                try:
                    slice_img = cropped_img.crop((left, upper, right, lower))
                    slices.append(slice_img)
                    log(f"[操作 {op_index}] 切片 {i+1}: ({left}, {upper}, {right}, {lower}) -> {slice_img.size}")
                except Exception as e:
                    log(f"[操作 {op_index}] 拆分切片 {i+1} 时出错: {e}")
        elif op['split'] == 'vertical':
            base_slice_width, base_slice_height = op['slice_size']
            slice_width, slice_height = base_slice_width * scale_factor, base_slice_height * scale_factor
            num_slices = len(op['save_names'])
            for i in range(num_slices):
                left = 0
                upper = i * slice_height
                right = slice_width
                lower = upper + slice_height
                try:
                    slice_img = cropped_img.crop((left, upper, right, lower))
                    slices.append(slice_img)
                    log(f"[操作 {op_index}] 切片 {i+1}: ({left}, {upper}, {right}, {lower}) -> {slice_img.size}")
                except Exception as e:
                    log(f"[操作 {op_index}] 拆分切片 {i+1} 时出错: {e}")
        else:
            slices = [cropped_img]
            log(f"[操作 {op_index}] 无需拆分，单一图像尺寸: {slices[0].size}")

        # 保存切片
        for idx, save_name in enumerate(op['save_names']):
            if op['split'] in ['horizontal', 'vertical']:
                if idx < len(slices):
                    slice_img = slices[idx]
                else:
                    log(f"[操作 {op_index}] 切片数量不足，跳过 {save_name}")
                    continue
            else:
                slice_img = slices[0]

            # 定义目标路径
            target_path = os.path.join(op['target_dir'], save_name).replace("\\", "/")  # 确保路径使用正斜杠

            try:
                slice_img.save(target_path, format='PNG')
                log(f"[操作 {op_index}] 已保存 {target_path}，尺寸: {slice_img.size}")
            except Exception as e:
                log(f"[操作 {op_index}] 保存 {target_path} 时出错: {e}")

        # 处理重复切片（如果有）
        if op.get('duplicate_save_name') and len(slices) >= len(op['save_names']):
            duplicate_slice = slices[-1]
            duplicate_save_name = op['duplicate_save_name']
            duplicate_target_path = os.path.join(op['target_dir'], duplicate_save_name).replace("\\", "/")
            try:
                duplicate_slice.save(duplicate_target_path, format='PNG')
                log(f"[操作 {op_index}] 已保存重复切片 {duplicate_target_path}，尺寸: {duplicate_slice.size}")
            except Exception as e:
                log(f"[操作 {op_index}] 保存重复切片 {duplicate_target_path} 时出错: {e}")

    log("所有 tabs.png 相关的图像已成功处理并保存。")

def process_resource_packs_in_dir(temp_dir):
    """
    处理解压后的目录中的assets/minecraft/textures/gui/resource_packs.png，
    """
    resource_packs_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'resource_packs.png')
    
    if not os.path.exists(resource_packs_path):
        log(f"resource_packs.png 未在 {resource_packs_path} 找到。")
        return
    
    # 打开图像
    try:
        img = Image.open(resource_packs_path).convert("RGBA")
    except Exception as e:
        log(f"无法打开 resource_packs.png: {e}")
        return
    
    width, height = img.size

    # 确定 scale_factor
    if width == 256 and height == 256:
        scale_factor = 1
    elif width == 512 and height == 512:
        scale_factor = 2
    elif width == 1024 and height == 1024:
        scale_factor = 4
    elif width == 2048 and height == 2048:
        scale_factor = 8
    else:
        log(f"不支持的 resource_packs.png 尺寸: {width}x{height}")
        return
    
    log(f"处理 resource_packs.png，尺寸: {width}x{height}, scale_factor: {scale_factor}")

    def scaled_box(x1, y1, x2, y2):
        return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

    # 定义所有裁剪操作
    operations = [
        # Step 1: (0,0)-(128,32) -> 4 slices 32x32
        {
            'crop': (0, 0, 128, 32),
            'split': 'horizontal',
            'slice_size': (32, 32),
            'save_names': ['select.png', 'unselect.png', 'move_down.png', 'move_up.png'],
            'duplicate_save_name': None,
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'transferable_list')
        },
        # Step 2: (0,32)-(128,64) -> 4 slices 32x32
        {
            'crop': (0, 32, 128, 64),
            'split': 'horizontal',
            'slice_size': (32, 32),
            'save_names': [
                'select_highlighted.png', 'unselect_highlighted.png', 'move_down_highlighted.png','move_up_highlighted.png'],
            'duplicate_save_name': None,
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'transferable_list')
        },
    ]

    # 确保所有目标目录存在
    for op in operations:
        target_dir = op['target_dir']
        if not os.path.exists(target_dir):
            os.makedirs(target_dir)
            log(f"已创建目录: {target_dir}")

    for op_index, op in enumerate(operations, start=1):
        # 裁剪区域
        crop_region = scaled_box(*op['crop'])
        try:
            cropped_img = img.crop(crop_region)
            log(f"[操作 {op_index}] 裁剪区域: {op['crop']} -> {crop_region}")
        except Exception as e:
            log(f"[操作 {op_index}] 裁剪区域 {op['crop']} 时出错: {e}")
            continue

        # 处理拆分
        slices = []
        if op['split'] == 'horizontal':
            base_slice_width, base_slice_height = op['slice_size']
            slice_width, slice_height = base_slice_width * scale_factor, base_slice_height * scale_factor
            num_slices = len(op['save_names'])
            for i in range(num_slices):
                left = i * slice_width
                upper = 0
                right = left + slice_width
                lower = upper + slice_height
                try:
                    slice_img = cropped_img.crop((left, upper, right, lower))
                    slices.append(slice_img)
                    log(f"[操作 {op_index}] 切片 {i+1}: ({left}, {upper}, {right}, {lower}) -> {slice_img.size}")
                except Exception as e:
                    log(f"[操作 {op_index}] 拆分切片 {i+1} 时出错: {e}")
        elif op['split'] == 'vertical':
            base_slice_width, base_slice_height = op['slice_size']
            slice_width, slice_height = base_slice_width * scale_factor, base_slice_height * scale_factor
            num_slices = len(op['save_names'])
            for i in range(num_slices):
                left = 0
                upper = i * slice_height
                right = slice_width
                lower = upper + slice_height
                try:
                    slice_img = cropped_img.crop((left, upper, right, lower))
                    slices.append(slice_img)
                    log(f"[操作 {op_index}] 切片 {i+1}: ({left}, {upper}, {right}, {lower}) -> {slice_img.size}")
                except Exception as e:
                    log(f"[操作 {op_index}] 拆分切片 {i+1} 时出错: {e}")
        else:
            slices = [cropped_img]
            log(f"[操作 {op_index}] 无需拆分，单一图像尺寸: {slices[0].size}")

        # 保存切片
        for idx, save_name in enumerate(op['save_names']):
            if op['split'] in ['horizontal', 'vertical']:
                if idx < len(slices):
                    slice_img = slices[idx]
                else:
                    log(f"[操作 {op_index}] 切片数量不足，跳过 {save_name}")
                    continue
            else:
                slice_img = slices[0]

            # 定义目标路径
            target_path = os.path.join(op['target_dir'], save_name).replace("\\", "/")  # 确保路径使用正斜杠

            try:
                slice_img.save(target_path, format='PNG')
                log(f"[操作 {op_index}] 已保存 {target_path}，尺寸: {slice_img.size}")
            except Exception as e:
                log(f"[操作 {op_index}] 保存 {target_path} 时出错: {e}")

        # 如果有需要重复保存的切片，可以在这里处理（本例中没有）

    log("所有 resource_packs.png 相关的图像已成功处理并保存。")

def process_title_in_dir(temp_dir):
    """
    处理解压后的目录中的assets/minecraft/textures/gui/title/minecraft.png，
    """
    # 定义原始的 minecraft.png 路径
    title_minecraft_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'title', 'minecraft.png')
    
    if not os.path.exists(title_minecraft_path):
        log(f"minecraft.png 未在 {title_minecraft_path} 找到。")
        return
    
    # 打开图像
    try:
        img = Image.open(title_minecraft_path).convert("RGBA")
    except Exception as e:
        log(f"无法打开 minecraft.png: {e}")
        return
    
    width, height = img.size

    # 确定 scale_factor
    if width == 256 and height == 256:
        scale_factor = 1
    elif width == 512 and height == 512:
        scale_factor = 2
    elif width == 1024 and height == 1024:
        scale_factor = 4
    elif width == 2048 and height == 2048:
        scale_factor = 8
    else:
        log(f"不支持的 minecraft.png 尺寸: {width}x{height}")
        return
    
    log(f"处理 minecraft.png，尺寸: {width}x{height}, scale_factor: {scale_factor}")

    def scaled_box(x1, y1, x2, y2):
        return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

    # 定义裁剪操作
    operations = [
        # 步骤 1: 裁剪 (0,94)-(200,194)，保存为 realms.png
        {
            'action': 'crop_and_save',
            'crop': (0, 94, 200, 194),
            'save_name': 'realms.png',
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'title')
        },
        # 步骤 2: 裁剪 (0,0)-(155,44) 和 (0,45)-(119,89)，拼接并添加透明区域，保存为 minecraft.png
        {
            'action': 'crop_concatenate_append',
            'crop1': (0, 0, 155, 44),
            'crop2': (0, 45, 119, 89),
            'save_name': 'minecraft.png',
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'title'),
            'concat_direction': 'horizontal',  # 左右拼接
            'transparent_append': {
                'width': 274 * scale_factor,
                'height': 25 * scale_factor  # 根据 scale_factor 调整高度
            }
        }
    ]

    # 确保目标目录存在
    for op in operations:
        target_dir = op['target_dir']
        if not os.path.exists(target_dir):
            try:
                os.makedirs(target_dir)
                log(f"已创建目录: {target_dir}")
            except Exception as e:
                log(f"创建目录 {target_dir} 时出错: {e}")
                return

    for op_index, op in enumerate(operations, start=1):
        if op['action'] == 'crop_and_save':
            # 裁剪区域并保存
            crop_region = scaled_box(*op['crop'])
            try:
                cropped_img = img.crop(crop_region)
                log(f"[操作 {op_index}] 裁剪区域: {op['crop']} -> {crop_region}")
            except Exception as e:
                log(f"[操作 {op_index}] 裁剪区域 {op['crop']} 时出错: {e}")
                continue

            # 定义目标路径
            target_path = os.path.join(op['target_dir'], op['save_name']).replace("\\", "/")
            try:
                cropped_img.save(target_path, format='PNG')
                log(f"[操作 {op_index}] 已保存 {target_path}，尺寸: {cropped_img.size}")
            except Exception as e:
                log(f"[操作 {op_index}] 保存 {target_path} 时出错: {e}")
        
        elif op['action'] == 'crop_concatenate_append':
            # 裁剪两个区域
            crop_region1 = scaled_box(*op['crop1'])
            crop_region2 = scaled_box(*op['crop2'])
            try:
                cropped_img1 = img.crop(crop_region1)
                cropped_img2 = img.crop(crop_region2)
                log(f"[操作 {op_index}] 裁剪区域1: {op['crop1']} -> {crop_region1}")
                log(f"[操作 {op_index}] 裁剪区域2: {op['crop2']} -> {crop_region2}")
            except Exception as e:
                log(f"[操作 {op_index}] 裁剪区域时出错: {e}")
                continue

            # 拼接图像
            if op['concat_direction'] == 'horizontal':
                new_width = cropped_img1.width + cropped_img2.width
                new_height = max(cropped_img1.height, cropped_img2.height)
            else:
                new_width = max(cropped_img1.width, cropped_img2.width)
                new_height = cropped_img1.height + cropped_img2.height

            try:
                concatenated_img = Image.new('RGBA', (new_width, new_height))
                concatenated_img.paste(cropped_img1, (0, 0))
                concatenated_img.paste(cropped_img2, (cropped_img1.width, 0))
                log(f"[操作 {op_index}] 拼接后的图像尺寸: {concatenated_img.size}")
            except Exception as e:
                log(f"[操作 {op_index}] 拼接图像时出错: {e}")
                continue

            # 创建透明区域并拼接
            transparent_width = op['transparent_append']['width']
            transparent_height = op['transparent_append']['height']
            try:
                transparent_img = Image.new('RGBA', (transparent_width, transparent_height), (0, 0, 0, 0))
                final_img = Image.new('RGBA', (transparent_width, concatenated_img.height + transparent_height))
                
                # 计算位置以确保拼接图像居中或按照需求对齐
                final_img.paste(concatenated_img, (0, 0))
                final_img.paste(transparent_img, (0, concatenated_img.height))
                log(f"[操作 {op_index}] 添加透明区域后的图像尺寸: {final_img.size}")
            except Exception as e:
                log(f"[操作 {op_index}] 添加透明区域时出错: {e}")
                continue

            # 定义目标路径
            target_path = os.path.join(op['target_dir'], op['save_name']).replace("\\", "/")
            try:
                final_img.save(target_path, format='PNG')
                log(f"[操作 {op_index}] 已保存拼接并添加透明区域的图像 {target_path}，尺寸: {final_img.size}")
            except Exception as e:
                log(f"[操作 {op_index}] 保存拼接图像 {target_path} 时出错: {e}")
                continue

            # 替换原始的 minecraft.png
            try:
                shutil.copy(target_path, title_minecraft_path)
                log(f"[操作 {op_index}] 已将新 minecraft.png 替换原始文件 {title_minecraft_path}")
            except Exception as e:
                log(f"[操作 {op_index}] 替换原始 minecraft.png 时出错: {e}")
                continue

    log("所有 minecraft.png 相关的图像已成功处理并保存。")

def process_server_selection_in_dir(temp_dir):
    """
    处理解压后的目录中的assets/minecraft/textures/gui/server_selection.png，
    """
    # 定义原始的 server_selection.png 路径
    server_selection_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'server_selection.png')
    
    if not os.path.exists(server_selection_path):
        log(f"server_selection.png 未在 {server_selection_path} 找到。")
        return
    
    # 打开图像
    try:
        img = Image.open(server_selection_path).convert("RGBA")
    except Exception as e:
        log(f"无法打开 server_selection.png: {e}")
        return
    
    width, height = img.size

    # 确定 scale_factor
    if width == 256 and height == 256:
        scale_factor = 1
    elif width == 512 and height == 512:
        scale_factor = 2
    elif width == 1024 and height == 1024:
        scale_factor = 4
    elif width == 2048 and height == 2048:
        scale_factor = 8
    else:
        log(f"不支持的 server_selection.png 尺寸: {width}x{height}")
        return
    
    log(f"处理 server_selection.png，尺寸: {width}x{height}, scale_factor: {scale_factor}")

    def scaled_box(x1, y1, x2, y2):
        """
    根据 scale_factor 缩放裁剪坐标
    """
        return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

    # 定义裁剪操作
    operations = [
        # 步骤 1: 裁剪 (0,0)-(128,32)，分割成4个32x32的图片，保存为 join.png, emm.png, move_down.png, move_up.png
        {
            'action': 'crop_and_split',
            'crop': (0, 0, 128, 32),
            'split': 'horizontal',
            'slice_size': (32, 32),
            'save_names': ['join.png', 'emm.png', 'move_down.png', 'move_up.png'],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'server_list')
        },
        # 步骤 2: 裁剪 (0,32)-(128,64)，分割成4个32x32的图片，保存为 join_highlighted.png, emmm.png, move_down_highlighted.png, move_up_highlighted.png
        {
            'action': 'crop_and_split',
            'crop': (0, 32, 128, 64),
            'split': 'horizontal',
            'slice_size': (32, 32),
            'save_names': [
                'join_highlighted.png', 
                'emmm.png', 
                'move_down_highlighted.png', 
                'move_up_highlighted.png'
            ],
            'target_dir': os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'server_list')
        },
    ]

    # 确保目标目录存在
    for op in operations:
        target_dir = op['target_dir']
        if not os.path.exists(target_dir):
            try:
                os.makedirs(target_dir)
                log(f"已创建目录: {target_dir}")
            except Exception as e:
                log(f"创建目录 {target_dir} 时出错: {e}")
                return

    for op_index, op in enumerate(operations, start=1):
        if op['action'] == 'crop_and_split':
            # 裁剪区域
            crop_region = scaled_box(*op['crop'])
            try:
                cropped_img = img.crop(crop_region)
                log(f"[操作 {op_index}] 裁剪区域: {op['crop']} -> {crop_region}")
            except Exception as e:
                log(f"[操作 {op_index}] 裁剪区域 {op['crop']} 时出错: {e}")
                continue

            # 处理拆分
            slices = []
            if op['split'] == 'horizontal':
                base_slice_width, base_slice_height = op['slice_size']
                slice_width, slice_height = base_slice_width * scale_factor, base_slice_height * scale_factor
                num_slices = len(op['save_names'])
                for i in range(num_slices):
                    left = i * slice_width
                    upper = 0
                    right = left + slice_width
                    lower = upper + slice_height
                    try:
                        slice_img = cropped_img.crop((left, upper, right, lower))
                        slices.append(slice_img)
                        log(f"[操作 {op_index}] 切片 {i+1}: ({left}, {upper}, {right}, {lower}) -> {slice_img.size}")
                    except Exception as e:
                        log(f"[操作 {op_index}] 拆分切片 {i+1} 时出错: {e}")
            
            # 保存切片
            for idx, save_name in enumerate(op['save_names']):
                if op['split'] == 'horizontal':
                    if idx < len(slices):
                        slice_img = slices[idx]
                    else:
                        log(f"[操作 {op_index}] 切片数量不足，跳过 {save_name}")
                        continue
                else:
                    slice_img = slices[0]
                
                # 定义目标路径
                target_path = os.path.join(op['target_dir'], save_name).replace("\\", "/")  # 确保路径使用正斜杠
                
                try:
                    slice_img.save(target_path, format='PNG')
                    log(f"[操作 {op_index}] 已保存 {target_path}，尺寸: {slice_img.size}")
                except Exception as e:
                    log(f"[操作 {op_index}] 保存 {target_path} 时出错: {e}")

    log("所有 server_selection.png 相关的图像已成功处理并保存。")

def process1_anvil_image(temp_dir):
    """
    处理解压后的目录中的assets/minecraft/textures/gui/container/anvil.png，
    """
    log(f"Processing anvil image in: {temp_dir}")
    try:
        anvil_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','anvil.png')

        if os.path.exists(anvil_path):
            img = Image.open(anvil_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'anvil.png': {width}x{height}")
                return

            log(f"Processing anvil.png, size: {width}x{height}, scale_factor: {scale_factor}")

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (176, 0, 204, 21),
                    'save_name': 'error.png',
                },
                {
                    'crop': (0, 166, 110, 182),
                    'save_name': 'text_field.png',
                },
                {
                    'crop': (0, 182, 110, 198),
                    'save_name': 'text_field_disabled.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'anvil')

            # 确保目标目录存在
            if not os.path.exists(target_dir):
                try:
                    os.makedirs(target_dir)
                    log(f"Created directory: {target_dir}")
                except Exception as e:
                    log(f"Error creating directory {target_dir}: {e}")
                    return

            # 执行裁剪和保存操作
            for op_index, region in enumerate(crop_regions, start=1):
                scaled_region = scaled_box(*region['crop'])
                try:
                    cropped_img = img.crop(scaled_region)
                    save_path = os.path.join(target_dir, region['save_name']).replace("\\", "/")  # 确保路径使用正斜杠
                    cropped_img.save(save_path, format='PNG')
                    log(f"[Operation {op_index}] Saved {save_path}, size: {cropped_img.size}")
                except Exception as e:
                    log(f"[Operation {op_index}] Error saving {region['save_name']}: {e}")

        else:
            log(f"No 'anvil.png' found in {container_path}")
    except Exception as e:
        log(f"Error processing anvil image in '{temp_dir}': {e}")
        traceback.print_exc()

def process1_beacon_image(temp_dir):
    """
    处理解压后的目录中的assets/minecraft/textures/gui/container/beacon.png，
    """
    log(f"Processing beacon image in: {temp_dir}")
    try:
        beacon_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','beacon.png')

        if os.path.exists(beacon_path):
            img = Image.open(beacon_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'beacon.png': {width}x{height}")
                return

            log(f"Processing beacon.png, size: {width}x{height}, scale_factor: {scale_factor}")

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (0, 219, 22, 241),
                    'save_name': 'button.png',
                },
                {
                    'crop': (22, 219, 44, 241),
                    'save_name': 'button_selected.png',
                },
                {
                    'crop': (44, 219, 66, 241),
                    'save_name': 'button_disabled.png',
                },
                {
                    'crop': (66, 219, 88, 241),
                    'save_name': 'button_highlighted.png',
                },
                {
                    'crop': (90, 220, 108, 238),
                    'save_name': 'confirm.png',
                },
                {
                    'crop': (112, 220, 130, 238),
                    'save_name': 'cancel.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'beacon')

            # 确保目标目录存在
            if not os.path.exists(target_dir):
                try:
                    os.makedirs(target_dir)
                    log(f"Created directory: {target_dir}")
                except Exception as e:
                    log(f"Error creating directory {target_dir}: {e}")
                    return

            # 执行裁剪和保存操作
            for op_index, region in enumerate(crop_regions, start=1):
                scaled_region = scaled_box(*region['crop'])
                try:
                    cropped_img = img.crop(scaled_region)
                    save_path = os.path.join(target_dir, region['save_name']).replace("\\", "/")  # 确保路径使用正斜杠
                    cropped_img.save(save_path, format='PNG')
                    log(f"[Operation {op_index}] Saved {save_path}, size: {cropped_img.size}")
                except Exception as e:
                    log(f"[Operation {op_index}] Error saving {region['save_name']}: {e}")

        else:
            log(f"No 'beacon.png' found in {container_path}")
    except Exception as e:
        log(f"Error processing anvil image in '{temp_dir}': {e}")
        traceback.print_exc()

def process1_blast_furnace_image(temp_dir):
    log(f"Processing blast_furnace image in: {temp_dir}")
    try:
        blast_furnace_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','blast_furnace.png')

        if os.path.exists(blast_furnace_path):
            img = Image.open(blast_furnace_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'blast_furnace.png': {width}x{height}")
                return

            log(f"Processing blast_furnace.png, size: {width}x{height}, scale_factor: {scale_factor}")

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (176, 0, 190, 14),
                    'save_name': 'lit_progress.png',
                },
                {
                    'crop': (176, 14, 200, 31),
                    'save_name': 'burn_progress.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'blast_furnace')

            # 确保目标目录存在
            if not os.path.exists(target_dir):
                try:
                    os.makedirs(target_dir)
                    log(f"Created directory: {target_dir}")
                except Exception as e:
                    log(f"Error creating directory {target_dir}: {e}")
                    return

            # 执行裁剪和保存操作
            for op_index, region in enumerate(crop_regions, start=1):
                scaled_region = scaled_box(*region['crop'])
                try:
                    cropped_img = img.crop(scaled_region)
                    save_path = os.path.join(target_dir, region['save_name']).replace("\\", "/")  # 确保路径使用正斜杠
                    cropped_img.save(save_path, format='PNG')
                    log(f"[Operation {op_index}] Saved {save_path}, size: {cropped_img.size}")
                except Exception as e:
                    log(f"[Operation {op_index}] Error saving {region['save_name']}: {e}")

        else:
            log(f"No 'blast_furnace.png' found in {container_path}")
    except Exception as e:
        log(f"Error processing anvil image in '{temp_dir}': {e}")
        traceback.print_exc()

def process1_brewing_stand_image(temp_dir):
    try:
        brewing_stand_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','brewing_stand.png')

        if os.path.exists(brewing_stand_path):
            img = Image.open(brewing_stand_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'brewing_stand.png': {width}x{height}")
                return

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (176, 0, 185, 28),
                    'save_name': 'brew_progress.png',
                },
                {
                    'crop': (185, 14, 197, 29),
                    'save_name': 'bubbles.png',
                },
                {
                    'crop': (176, 29, 194, 33),
                    'save_name': 'fuel_length.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'brewing_stand')

            # 确保目标目录存在
            if not os.path.exists(target_dir):
                try:
                    os.makedirs(target_dir)
                    log(f"Created directory: {target_dir}")
                except Exception as e:
                    log(f"Error creating directory {target_dir}: {e}")
                    return

            # 执行裁剪和保存操作
            for op_index, region in enumerate(crop_regions, start=1):
                scaled_region = scaled_box(*region['crop'])
                try:
                    cropped_img = img.crop(scaled_region)
                    save_path = os.path.join(target_dir, region['save_name']).replace("\\", "/")  # 确保路径使用正斜杠
                    cropped_img.save(save_path, format='PNG')
                    log(f"[Operation {op_index}] Saved {save_path}, size: {cropped_img.size}")
                except Exception as e:
                    log(f"[Operation {op_index}] Error saving {region['save_name']}: {e}")

    except Exception as e:
        traceback.print_exc()

def process1_cartography_table_image(temp_dir):
    try:
        cartography_table_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','cartography_table.png')

        if os.path.exists(cartography_table_path):
            img = Image.open(cartography_table_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'cartography_table.png': {width}x{height}")
                return

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (176, 132, 226, 198),
                    'save_name': 'duplicated_map.png',
                },
                {
                    'crop': (176, 66, 242, 132),
                    'save_name': 'scaled_map.png',
                },
                {
                    'crop': (176, 0, 242, 66),
                    'save_name': 'map.png',
                },
                {
                    'crop': (52, 214, 62, 228),
                    'save_name': 'locked.png',
                },
                {
                    'crop': (226, 132, 254, 153),
                    'save_name': 'error.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'cartography_table')

            # 确保目标目录存在
            if not os.path.exists(target_dir):
                try:
                    os.makedirs(target_dir)
                    log(f"Created directory: {target_dir}")
                except Exception as e:
                    log(f"Error creating directory {target_dir}: {e}")
                    return

            # 执行裁剪和保存操作
            for op_index, region in enumerate(crop_regions, start=1):
                scaled_region = scaled_box(*region['crop'])
                try:
                    cropped_img = img.crop(scaled_region)
                    save_path = os.path.join(target_dir, region['save_name']).replace("\\", "/")  # 确保路径使用正斜杠
                    cropped_img.save(save_path, format='PNG')
                    log(f"[Operation {op_index}] Saved {save_path}, size: {cropped_img.size}")
                except Exception as e:
                    log(f"[Operation {op_index}] Error saving {region['save_name']}: {e}")

    except Exception as e:
        traceback.print_exc()

def process1_enchanting_table_image(temp_dir):
    try:
        enchanting_table_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','enchanting_table.png')

        if os.path.exists(enchanting_table_path):
            img = Image.open(enchating_table_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'enchanting_table.png': {width}x{height}")
                return

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (0, 166, 108, 185),
                    'save_name': 'enchantment_slot.png',
                },
                {
                    'crop': (0, 185, 108, 204),
                    'save_name': 'enchantment_slot_disabled.png',
                },
                {
                    'crop': (0, 204, 108, 223),
                    'save_name': 'enchantment_slot_highlighted.png',
                },
                {
                    'crop': (0,223,16,239),
                    'save_name': 'level_1.png',
                },
                {
                    'crop': (16,223,32,239),
                    'save_name': 'level_2.png',
                },
                {
                    'crop': (32,223,48,239),
                    'save_name': 'level_3.png',
                },
                {
                    'crop': (0,239,16,255),
                    'save_name': 'level_1_disabled.png',
                },
                {
                    'crop': (16,239,32,255),
                    'save_name': 'level_2_disabled.png',
                },
                {
                    'crop': (32,239,48,255),
                    'save_name': 'level_3_disabled.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'enchanting_table')

            # 确保目标目录存在
            if not os.path.exists(target_dir):
                try:
                    os.makedirs(target_dir)
                    log(f"Created directory: {target_dir}")
                except Exception as e:
                    log(f"Error creating directory {target_dir}: {e}")
                    return

            # 执行裁剪和保存操作
            for op_index, region in enumerate(crop_regions, start=1):
                scaled_region = scaled_box(*region['crop'])
                try:
                    cropped_img = img.crop(scaled_region)
                    save_path = os.path.join(target_dir, region['save_name']).replace("\\", "/")  # 确保路径使用正斜杠
                    cropped_img.save(save_path, format='PNG')
                    log(f"[Operation {op_index}] Saved {save_path}, size: {cropped_img.size}")
                except Exception as e:
                    log(f"[Operation {op_index}] Error saving {region['save_name']}: {e}")

    except Exception as e:
        traceback.print_exc()

def process1_furnace_image(temp_dir):
    log(f"Processing furnace image in: {temp_dir}")
    try:
        furnace_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','furnace.png')

        if os.path.exists(furnace_path):
            img = Image.open(furnace_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'furnace.png': {width}x{height}")
                return

            log(f"Processing furnace.png, size: {width}x{height}, scale_factor: {scale_factor}")

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (176, 0, 190, 14),
                    'save_name': 'lit_progress.png',
                },
                {
                    'crop': (176, 14, 200, 31),
                    'save_name': 'burn_progress.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'furnace')

            # 确保目标目录存在
            if not os.path.exists(target_dir):
                try:
                    os.makedirs(target_dir)
                    log(f"Created directory: {target_dir}")
                except Exception as e:
                    log(f"Error creating directory {target_dir}: {e}")
                    return

            # 执行裁剪和保存操作
            for op_index, region in enumerate(crop_regions, start=1):
                scaled_region = scaled_box(*region['crop'])
                try:
                    cropped_img = img.crop(scaled_region)
                    save_path = os.path.join(target_dir, region['save_name']).replace("\\", "/")  # 确保路径使用正斜杠
                    cropped_img.save(save_path, format='PNG')
                    log(f"[Operation {op_index}] Saved {save_path}, size: {cropped_img.size}")
                except Exception as e:
                    log(f"[Operation {op_index}] Error saving {region['save_name']}: {e}")

    except Exception as e:
        traceback.print_exc()

def process1_smoker_image(temp_dir):
    log(f"Processing smoker image in: {temp_dir}")
    try:
        smoker_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','smoker.png')

        if os.path.exists(smoker_path):
            img = Image.open(smoker_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'smoker.png': {width}x{height}")
                return

            log(f"Processing smoker.png, size: {width}x{height}, scale_factor: {scale_factor}")

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (176, 0, 190, 14),
                    'save_name': 'lit_progress.png',
                },
                {
                    'crop': (176, 14, 200, 31),
                    'save_name': 'burn_progress.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'smoker')

            # 确保目标目录存在
            if not os.path.exists(target_dir):
                try:
                    os.makedirs(target_dir)
                    log(f"Created directory: {target_dir}")
                except Exception as e:
                    log(f"Error creating directory {target_dir}: {e}")
                    return

            # 执行裁剪和保存操作
            for op_index, region in enumerate(crop_regions, start=1):
                scaled_region = scaled_box(*region['crop'])
                try:
                    cropped_img = img.crop(scaled_region)
                    save_path = os.path.join(target_dir, region['save_name']).replace("\\", "/")  # 确保路径使用正斜杠
                    cropped_img.save(save_path, format='PNG')
                    log(f"[Operation {op_index}] Saved {save_path}, size: {cropped_img.size}")
                except Exception as e:
                    log(f"[Operation {op_index}] Error saving {region['save_name']}: {e}")

        else:
            log(f"No 'smoker.png' found in {container_path}")
    except Exception as e:
        log(f"Error processing anvil image in '{temp_dir}': {e}")
        traceback.print_exc()

def process1_grindstone_image(temp_dir):
    try:
        grindstone_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','grindstone.png')

        if os.path.exists(grindstone_path):
            img = Image.open(grindstone_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'grindstone.png': {width}x{height}")
                return

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (30, 53, 48, 71),
                    'save_name': 'input_slot.png',
                },
                {
                    'crop': (66, 53, 84, 71),
                    'save_name': 'additional_slot.png',
                },
                {
                    'crop': (66, 53, 84, 71),
                    'save_name': 'output_slot.png',
                },
                {
                    'crop': (66, 53, 84, 71),
                    'save_name': 'result_slot.png',
                },
                {
                    'crop': (176, 0, 204, 21),
                    'save_name': 'error.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'grindstone')

            # 确保目标目录存在
            if not os.path.exists(target_dir):
                try:
                    os.makedirs(target_dir)
                    log(f"Created directory: {target_dir}")
                except Exception as e:
                    log(f"Error creating directory {target_dir}: {e}")
                    return

            # 执行裁剪和保存操作
            for op_index, region in enumerate(crop_regions, start=1):
                scaled_region = scaled_box(*region['crop'])
                try:
                    cropped_img = img.crop(scaled_region)
                    save_path = os.path.join(target_dir, region['save_name']).replace("\\", "/")  # 确保路径使用正斜杠
                    cropped_img.save(save_path, format='PNG')
                    log(f"[Operation {op_index}] Saved {save_path}, size: {cropped_img.size}")
                except Exception as e:
                    log(f"[Operation {op_index}] Error saving {region['save_name']}: {e}")

    except Exception as e:
        traceback.print_exc()

def process1_horse_image(temp_dir):
    try:
        horse_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','horse.png')

        if os.path.exists(horse_path):
            img = Image.open(horse_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'horse.png': {width}x{height}")
                return

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (0, 220, 18, 238),
                    'save_name': 'armor_slot.png',
                },
                {
                    'crop': (18, 220, 36, 238),
                    'save_name': 'saddle_slot.png',
                },
                {
                    'crop': (36, 220, 54, 238),
                    'save_name': 'llama_armor_slot.png',
                },
                {
                    'crop': (0, 166, 90, 220),
                    'save_name': 'chest_slots.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'horse')

            # 确保目标目录存在
            if not os.path.exists(target_dir):
                try:
                    os.makedirs(target_dir)
                    log(f"Created directory: {target_dir}")
                except Exception as e:
                    log(f"Error creating directory {target_dir}: {e}")
                    return

            # 执行裁剪和保存操作
            for op_index, region in enumerate(crop_regions, start=1):
                scaled_region = scaled_box(*region['crop'])
                try:
                    cropped_img = img.crop(scaled_region)
                    save_path = os.path.join(target_dir, region['save_name']).replace("\\", "/")  # 确保路径使用正斜杠
                    cropped_img.save(save_path, format='PNG')
                    log(f"[Operation {op_index}] Saved {save_path}, size: {cropped_img.size}")
                except Exception as e:
                    log(f"[Operation {op_index}] Error saving {region['save_name']}: {e}")

    except Exception as e:
        traceback.print_exc()

def process1_inventory_image(temp_dir):
    try:
        inventory_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','inventory.png')

        if os.path.exists(inventory_path):
            img = Image.open(inventory_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'inventory.png': {width}x{height}")
                return

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (0, 166, 120, 198),
                    'save_name': 'effect_background_large.png',
                },
                {
                    'crop': (0, 198, 32, 230),
                    'save_name': 'effect_background_small.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'inventory')

            # 确保目标目录存在
            if not os.path.exists(target_dir):
                try:
                    os.makedirs(target_dir)
                    log(f"Created directory: {target_dir}")
                except Exception as e:
                    log(f"Error creating directory {target_dir}: {e}")
                    return

            # 执行裁剪和保存操作
            for op_index, region in enumerate(crop_regions, start=1):
                scaled_region = scaled_box(*region['crop'])
                try:
                    cropped_img = img.crop(scaled_region)
                    save_path = os.path.join(target_dir, region['save_name']).replace("\\", "/")  # 确保路径使用正斜杠
                    cropped_img.save(save_path, format='PNG')
                    log(f"[Operation {op_index}] Saved {save_path}, size: {cropped_img.size}")
                except Exception as e:
                    log(f"[Operation {op_index}] Error saving {region['save_name']}: {e}")

    except Exception as e:
        traceback.print_exc()


def process1_smithing_image(temp_dir):
    try:
        smithing_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','smithing.png')

        if os.path.exists(smithing_path):
            img = Image.open(smithing_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'smithing.png': {width}x{height}")
                return

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (16, 0, 32, 16),
                    'save_name': 'template_slot.png',
                },
                {
                    'crop': (32, 0, 48, 16),
                    'save_name': 'base_slot.png',
                },
                {
                    'crop': (16, 16, 32, 32),
                    'save_name': 'addition_slot.png',
                },
                {
                    'crop': (32, 16, 48, 32),
                    'save_name': 'result_slot.png',
                },
                {
                    'crop': (176, 0, 204, 21),
                    'save_name': 'error.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'smithing')

            # 确保目标目录存在
            if not os.path.exists(target_dir):
                try:
                    os.makedirs(target_dir)
                    log(f"Created directory: {target_dir}")
                except Exception as e:
                    log(f"Error creating directory {target_dir}: {e}")
                    return

            # 执行裁剪和保存操作
            for op_index, region in enumerate(crop_regions, start=1):
                scaled_region = scaled_box(*region['crop'])
                try:
                    cropped_img = img.crop(scaled_region)
                    save_path = os.path.join(target_dir, region['save_name']).replace("\\", "/")  # 确保路径使用正斜杠
                    cropped_img.save(save_path, format='PNG')
                    log(f"[Operation {op_index}] Saved {save_path}, size: {cropped_img.size}")
                except Exception as e:
                    log(f"[Operation {op_index}] Error saving {region['save_name']}: {e}")

    except Exception as e:
        traceback.print_exc()

def process1_loom_image(temp_dir):
    try:
        loom_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','loom.png')

        if os.path.exists(loom_path):
            img = Image.open(loom_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'loom.png': {width}x{height}")
                return

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (176, 0, 192, 16),
                    'save_name': 'banner_slot.png',
                },
                {
                    'crop': (192, 0, 208, 16),
                    'save_name': 'dye_slot.png',
                },
                {
                    'crop': (208, 0, 224, 16),
                    'save_name': 'pattern_slot.png',
                },
                {
                    'crop': (0, 166, 14, 180),
                    'save_name': 'pattern.png',
                },
                {
                    'crop': (0, 180, 14, 194),
                    'save_name': 'pattern_selceted.png',
                },
                {
                    'crop': (0, 194, 14, 208),
                    'save_name': 'pattern_highlighted.png',
                },
                {
                    'crop': (232, 0, 244, 15),
                    'save_name': 'scroller.png',
                },
                {
                    'crop': (244, 0, 256, 15),
                    'save_name': 'scroller_disabled.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'loom')

            # 确保目标目录存在
            if not os.path.exists(target_dir):
                try:
                    os.makedirs(target_dir)
                    log(f"Created directory: {target_dir}")
                except Exception as e:
                    log(f"Error creating directory {target_dir}: {e}")
                    return

            # 执行裁剪和保存操作
            for op_index, region in enumerate(crop_regions, start=1):
                scaled_region = scaled_box(*region['crop'])
                try:
                    cropped_img = img.crop(scaled_region)
                    save_path = os.path.join(target_dir, region['save_name']).replace("\\", "/")  # 确保路径使用正斜杠
                    cropped_img.save(save_path, format='PNG')
                    log(f"[Operation {op_index}] Saved {save_path}, size: {cropped_img.size}")
                except Exception as e:
                    log(f"[Operation {op_index}] Error saving {region['save_name']}: {e}")

    except Exception as e:
        traceback.print_exc()

def process1_stonecutter_image(temp_dir):
    try:
        stonecutter_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','stonecutter.png')

        if os.path.exists(stonecutter_path):
            img = Image.open(stonecutter_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'stonecutter.png': {width}x{height}")
                return

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (0, 166, 16, 184),
                    'save_name': 'recipe.png',
                },
                {
                    'crop': (0,184,16,202),
                    'save_name': 'recipe_selected.png',
                },
                {
                    'crop': (0,202,16,220),
                    'save_name': 'recipe_highlighted.png',
                },
                {
                    'crop': (176,0,188,15),
                    'save_name': 'scroller.png',
                },
                {
                    'crop': (188,0,200,15),
                    'save_name': 'scroller_disabled.png',
                },
                {
                    'crop': (176, 0, 192, 16),
                    'save_name': 'input_slot.png',
                },
                {
                    'crop': (192, 0, 208, 16),
                    'save_name': 'output_slot.png',
                },
                {
                    'crop': (192, 0, 208, 16),
                    'save_name': 'result_slot.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'stonecutter')

            # 确保目标目录存在
            if not os.path.exists(target_dir):
                try:
                    os.makedirs(target_dir)
                    log(f"Created directory: {target_dir}")
                except Exception as e:
                    log(f"Error creating directory {target_dir}: {e}")
                    return

            # 执行裁剪和保存操作
            for op_index, region in enumerate(crop_regions, start=1):
                scaled_region = scaled_box(*region['crop'])
                try:
                    cropped_img = img.crop(scaled_region)
                    save_path = os.path.join(target_dir, region['save_name']).replace("\\", "/")  # 确保路径使用正斜杠
                    cropped_img.save(save_path, format='PNG')
                    log(f"[Operation {op_index}] Saved {save_path}, size: {cropped_img.size}")
                except Exception as e:
                    log(f"[Operation {op_index}] Error saving {region['save_name']}: {e}")

    except Exception as e:
        traceback.print_exc()

def process1_villager2_image(temp_dir):
    try:
        villager2_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container','villager2.png')

        if os.path.exists(villager2_path):
            img = Image.open(villager2_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            if width == 512 and height == 256:
                scale_factor = 1
            elif width == 1024 and height == 512:
                scale_factor = 2
            elif width == 2048 and height == 1024:
                scale_factor = 4
            elif width == 4096 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'villager2.png': {width}x{height}")
                return

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义裁剪区域及保存名称
            crop_regions = [
                {
                    'crop': (0, 176, 9, 178),
                    'save_name': 'discount_strikethrough.png',
                },
                {
                    'crop': (0, 181, 102, 186),
                    'save_name': 'experience_bar_result.png',
                },
                {
                    'crop': (0, 186, 102, 191),
                    'save_name': 'experience_bar_background.png',
                },
                {
                    'crop': (0, 191, 102, 196),
                    'save_name': 'experience_bar_current.png',
                },
                {
                    'crop': (15, 171, 25, 180),
                    'save_name': 'trade_arrow.png',
                },
                {
                    'crop': (25, 171, 35, 180),
                    'save_name': 'out_of_stuck.png',
                },
                {
                    'crop': (0, 199, 6, 226),
                    'save_name': 'scroller.png',
                },
                {
                    'crop': (6, 199, 12, 226),
                    'save_name': 'scroller.png',
                },
            ]

            # 定义目标目录
            target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'villager')

            # 确保目标目录存在
            if not os.path.exists(target_dir):
                try:
                    os.makedirs(target_dir)
                    log(f"Created directory: {target_dir}")
                except Exception as e:
                    log(f"Error creating directory {target_dir}: {e}")
                    return

            # 执行裁剪和保存操作
            for op_index, region in enumerate(crop_regions, start=1):
                scaled_region = scaled_box(*region['crop'])
                try:
                    cropped_img = img.crop(scaled_region)
                    save_path = os.path.join(target_dir, region['save_name']).replace("\\", "/")  # 确保路径使用正斜杠
                    cropped_img.save(save_path, format='PNG')
                    log(f"[Operation {op_index}] Saved {save_path}, size: {cropped_img.size}")
                except Exception as e:
                    log(f"[Operation {op_index}] Error saving {region['save_name']}: {e}")

    except Exception as e:
        traceback.print_exc()

# Helper function to move region in an image
def move_region(image, start_x, start_y, end_x, end_y, shift_x, shift_y):
    region = image.crop((start_x, start_y, end_x, end_y))
    image.paste(region, (start_x + shift_x, start_y + shift_y))

# Helper function to color fill region in an image
def color_fill_region(image, start_x, start_y, end_x, end_y, color_x, color_y):
    fill_color = image.getpixel((color_x, color_y))
    for x in range(start_x, end_x):
        for y in range(start_y, end_y):
            image.putpixel((x, y), fill_color)

# Helper function to copy and paste region in an image
def copy_and_paste_region(image, src_x1, src_y1, src_x2, src_y2, dst_x, dst_y):
    region = image.crop((src_x1, src_y1, src_x2, src_y2))
    image.paste(region, (dst_x, dst_y))

def on_file_drop(event):
    files = root.splitlist(event.data)
    zip_files = [f for f in files if f.lower().endswith(('.zip', '.rar'))]
    if zip_files:
        selected_files.set(zip_files)
        select_button.config(text=f"{len(zip_files)} 个文件已选择")
    else:
        messagebox.showerror("错误", "请拖拽ZIP或RAR文件")

def select_files():
    file_paths = filedialog.askopenfilenames(filetypes=[('压缩文件', '*.zip *.rar'), ('ZIP files', '*.zip'), ('RAR files', '*.rar')])
    if file_paths:
        selected_files.set(file_paths)
        select_button.config(text=f"{len(file_paths)} 个文件已选择")

def display_result(processed_zip_path, version="1.18", message_type="default"):
    # 调用 display_multiple_results 以支持单文件显示
    display_multiple_results([processed_zip_path], version, message_type)

    global result_label, new_button, result_label_exists, new_button_exists
    try:
        if result_label_exists:
            result_label.config(text="")
        else:
            result_label = tk.Label(frame, text="", wraplength=500, justify="left", bg="#f0f0f0", font=("微软雅黑", 14))
            result_label.pack(pady=20)
            result_label_exists = True

        if message_type == "conversion":
            result_text = f"你的{version}版本材质包放在了这里"
        elif message_type == "custom":
            result_text = f"你的自定义秒人斧材质包已生成，保存在这里"
        else:
            result_text = "你的自定义材质包放在了这里"

        if new_button_exists:
            new_button.config(text=result_text, bg="#FFFFE0", command=lambda: open_file_location(processed_zip_path))
        else:
            new_button = tk.Button(frame, text=result_text, bg="#FFFFE0", command=lambda: open_file_location(processed_zip_path), fg="black", font=("微软雅黑", 14), padx=20, pady=10)
            new_button.pack(pady=10)
            new_button_exists = True

        pyperclip.copy(processed_zip_path)
    except Exception as e:
        print(f"Error displaying result: {e}")
        messagebox.showerror("Error", f"Error displaying result: {e}")

# 全局变量，用于存储最后一次转换的文件路径
global_last_processed_files = []

def display_multiple_results(processed_zip_paths, version="1.18", message_type="default"):
    global global_last_processed_files
    try:
        # 存储最后一次转换的文件路径
        global_last_processed_files = processed_zip_paths
        
        # 提取所有处理后的文件所在的文件夹路径并去重
        output_dirs = set()
        for file_path in processed_zip_paths:
            # 获取文件所在的文件夹
            output_dir = os.path.dirname(file_path)
            output_dirs.add(output_dir)
        
        # 将去重后的文件夹路径复制到剪贴板
        output_dirs_list = list(output_dirs)
        pyperclip.copy('\n'.join(output_dirs_list))
        
        # 简化显示处理信息，只显示转换数量
        print(f"已成功转换 {len(processed_zip_paths)} 个材质包文件")
        print("文件夹路径已复制到剪贴板，您可以直接粘贴使用")
        
        # 如果是带有结构修复的转换，显示特殊的提示信息
        if message_type == "conversion_with_fix":
            messagebox.showinfo(
                "转换完成", 
                f"已成功转换 {len(processed_zip_paths)} 个材质包文件到 {version} 版本\n\n" 
                "注意：部分材质包的结构存在问题，已自动修复\n" 
                "修复内容包括：\n" 
                "- 将 pack.mcmeta 文件移动到根目录\n" 
                "- 将 assets 文件夹移动到根目录\n" 
                "- 将 pack.png 文件移动到根目录\n\n" 
                f"转换后的文件保存在：\n{output_dirs_list[0] if output_dirs_list else '未知位置'}\n\n" 
                "路径已复制到剪贴板"
            )
        elif message_type == "conversion":
            messagebox.showinfo(
                "转换完成", 
                f"已成功转换 {len(processed_zip_paths)} 个材质包文件到 {version} 版本\n\n" 
                f"转换后的文件保存在：\n{output_dirs_list[0] if output_dirs_list else '未知位置'}\n\n" 
                "路径已复制到剪贴板"
            )
    except Exception as e:
        print(f"显示结果时出错: {e}")

def clean_control_characters(content):
    """
    清理无效控制字符（例如：非法的 Unicode 控制字符）
    """
    # 使用正则表达式删除所有控制字符
    cleaned_content = re.sub(r'[\x00-\x1F\x7F\x80-\x9F]', '', content)  # 删除控制字符
    return cleaned_content

def clean_non_json_content(content):
    """
    清理非 JSON 内容，修复JSON格式问题
    """
    try:
        # 尝试直接解析JSON
        json.loads(content)
        return content
    except json.JSONDecodeError as e:
        log(f"JSON 解析失败: {e}")
        
        # 尝试1: 查找匹配的括号对来提取有效的JSON对象
        try:
            # 找到第一个 '{' 和匹配的 '}'
            start = content.find('{')
            if start == -1:
                log("未找到JSON开始标记 '{'")
                return None
            
            count = 1
            end = start + 1
            while count > 0 and end < len(content):
                if content[end] == '{':
                    count += 1
                elif content[end] == '}':
                    count -= 1
                end += 1
            
            if count == 0:
                json_content = content[start:end]
                # 验证提取的内容是否是有效的JSON
                json.loads(json_content)
                log("成功提取有效的JSON对象")
                return json_content
        except Exception as inner_e:
            log(f"尝试提取匹配括号对失败: {inner_e}")
        
        # 尝试2: 处理转义字符问题
        try:
            # 修复常见的转义字符问题
            fixed_content = content.replace('\\', '\\\\')  # 转义反斜杠
            json.loads(fixed_content)
            log("成功修复转义字符问题")
            return fixed_content
        except Exception as inner_e:
            log(f"尝试修复转义字符失败: {inner_e}")
        
        # 尝试3: 使用正则表达式清理
        try:
            # 去除JSON前后的非JSON内容
            json_pattern = r'(\{.*\})'  # 匹配最外层的JSON对象
            match = re.search(json_pattern, content, re.DOTALL)
            if match:
                json_content = match.group(1)
                # 修复可能的转义问题
                json_content = json_content.replace('\\', '\\\\')
                json.loads(json_content)
                log("成功通过正则表达式提取并修复JSON")
                return json_content
        except Exception as inner_e:
            log(f"尝试正则表达式提取失败: {inner_e}")
        
        log("未找到有效的 JSON 内容，所有修复尝试均失败")
        return None
    except Exception as e:
        log(f"清理非 JSON 内容时出错: {e}")
        return None

def extract_zip(zip_path):
    import tempfile
    log(f"Extracting zip: {zip_path}")
    temp_dir = None
    try:
        # 创建唯一的临时目录
        temp_dir = tempfile.mkdtemp(prefix='mcpack_', suffix='_temp')
        os.makedirs(temp_dir, exist_ok=True)

        # 检查文件是否存在
        if not os.path.exists(zip_path):
            raise FileNotFoundError(f"文件不存在: {zip_path}")
        
        # 检查文件大小
        file_size = os.path.getsize(zip_path)
        if file_size == 0:
            raise ValueError(f"文件为空: {zip_path}")
        
        # 检查文件的真实类型（通过文件头）
        with open(zip_path, 'rb') as f:
            header = f.read(8)
        
        # 根据文件类型选择解压方式
        if header[:4] == b'PK\x03\x04' or header[:4] == b'PK\x05\x06' or header[:4] == b'PK\x07\x08':
            # ZIP文件
            log(f"检测到ZIP文件，开始解压: {zip_path}")
            with zipfile.ZipFile(zip_path, 'r') as zip_ref:
                for item in zip_ref.infolist():
                    try:
                        filename = None
                        encodings = ['utf-8', 'gbk', 'latin-1']
                        
                        for encoding in encodings:
                            try:
                                if isinstance(item.filename, bytes):
                                    filename = item.filename.decode(encoding)
                                else:
                                    filename = item.filename
                                test_path = os.path.join(temp_dir, filename)
                                break
                            except (UnicodeDecodeError, UnicodeEncodeError):
                                continue
                        
                        if filename is None:
                            if isinstance(item.filename, bytes):
                                filename = item.filename.decode('latin-1')
                            else:
                                filename = item.filename
                        
                        target_path = os.path.join(temp_dir, filename.replace('/', os.sep))
                        
                        try:
                            os.makedirs(os.path.dirname(target_path), exist_ok=True)
                        except Exception as e:
                            log(f"创建目录时出错: {str(e)}")
                            target_dir = os.path.dirname(target_path)
                            if not os.path.exists(target_dir):
                                try:
                                    os.makedirs(target_dir)
                                except Exception as e2:
                                    log(f"再次创建目录时出错: {str(e2)}")
                                    continue
                        
                        try:
                            with zip_ref.open(item) as source:
                                if target_path.endswith(os.sep):
                                    if not os.path.exists(target_path):
                                        os.makedirs(target_path, exist_ok=True)
                                    continue
                                
                                with open(target_path, 'wb') as target:
                                    shutil.copyfileobj(source, target)
                        except PermissionError as e:
                            log(f"写入文件时权限被拒绝: {str(e)}")
                            try:
                                simple_filename = os.path.basename(target_path)
                                simple_target_path = os.path.join(temp_dir, simple_filename)
                                with zip_ref.open(item) as source, open(simple_target_path, 'wb') as target:
                                    shutil.copyfileobj(source, target)
                                log(f"使用简化路径成功写入文件: {simple_filename}")
                            except Exception as e2:
                                log(f"再次写入文件时出错: {str(e2)}")
                                continue
                        except Exception as e:
                            log(f"处理文件时出错: {str(e)}")
                            continue
                        
                        log(f"成功解压: {filename}")
                        
                    except Exception as e:
                        log(f"解压文件时出错: {str(e)}")
                        continue
                        
        elif header[:4] == b'Rar!':
            # RAR文件 - 使用unrar命令行工具
            log(f"检测到RAR文件，使用unrar命令行解压: {zip_path}")
            try:
                import subprocess
                
                # 使用unrar命令行工具
                cmd = [
                    'unrar',
                    'x',           # 解压并保留路径
                    '-y',          # 自动覆盖
                    '-p-',         # 无密码
                    '-o+',         # 覆盖现有文件
                    zip_path,
                    temp_dir + os.sep
                ]
                
                log(f"执行命令: {' '.join(cmd)}")
                
                # 执行unrar命令，设置超时
                result = subprocess.run(
                    cmd,
                    capture_output=True,
                    text=True,
                    timeout=300,  # 5分钟超时
                    encoding='utf-8',
                    errors='replace'
                )
                
                if result.returncode == 0:
                    log(f"成功解压RAR文件到: {temp_dir}")
                    # 统计解压的文件数量
                    file_count = 0
                    for root, dirs, files in os.walk(temp_dir):
                        file_count += len(files)
                    log(f"共解压 {file_count} 个文件")
                else:
                    # 检查是否是密码问题
                    if "password" in result.stderr.lower() or "密码" in result.stderr:
                        raise ValueError(f"RAR文件已加密，需要密码才能解压: {zip_path}")
                    else:
                        log(f"unrar输出: {result.stdout}")
                        log(f"unrar错误: {result.stderr}")
                        raise ValueError(f"解压RAR文件失败 (返回码: {result.returncode}): {zip_path}")
                        
            except subprocess.TimeoutExpired:
                raise ValueError(f"解压RAR文件超时: {zip_path}")
            except FileNotFoundError:
                raise ValueError(f"未找到unrar工具，请下载并安装UnRAR: https://www.rarlab.com/rar_add.htm")
            except Exception as e:
                raise ValueError(f"解压RAR文件时出错: {e}")
                
        else:
            raise ValueError(f"文件不是有效的ZIP或RAR格式: {zip_path}")
        
        # 处理pack.mcmeta文件
        pack_meta_path = os.path.join(temp_dir, 'pack.mcmeta')
        assets_folder_path = os.path.join(temp_dir, 'assets')
        pack_png_path = os.path.join(temp_dir, 'pack.png')
        
        # 标记是否需要修复结构
        structure_fixed = False
        
        if not os.path.exists(pack_meta_path) or not os.path.exists(assets_folder_path) or not os.path.exists(pack_png_path):
            log(f"根目录未找到必要文件，开始递归搜索整个目录: {temp_dir}")
            
            # 搜索必要文件
            found_pack_meta = None
            found_assets = None
            found_pack_png = None
            
            for root, dirs, files in os.walk(temp_dir):
                if not found_pack_meta and 'pack.mcmeta' in files:
                    found_pack_meta = os.path.join(root, 'pack.mcmeta')
                    log(f"在子目录中找到 pack.mcmeta 文件: {found_pack_meta}")
                
                if not found_assets and 'assets' in dirs:
                    found_assets = os.path.join(root, 'assets')
                    log(f"在子目录中找到 assets 文件夹: {found_assets}")
                
                if not found_pack_png and 'pack.png' in files:
                    found_pack_png = os.path.join(root, 'pack.png')
                    log(f"在子目录中找到 pack.png 文件: {found_pack_png}")
                
                # 如果所有文件都找到了，停止搜索
                if found_pack_meta and found_assets and found_pack_png:
                    break
            
            # 修复结构
            if found_pack_meta:
                if not os.path.exists(pack_meta_path):
                    shutil.copy(found_pack_meta, pack_meta_path)
                    log(f"已将 pack.mcmeta 复制到根目录")
                    structure_fixed = True
                pack_meta_path = pack_meta_path
            
            if found_assets:
                if not os.path.exists(assets_folder_path):
                    shutil.copytree(found_assets, assets_folder_path)
                    log(f"已将 assets 文件夹复制到根目录")
                    structure_fixed = True
            
            if found_pack_png:
                if not os.path.exists(pack_png_path):
                    shutil.copy(found_pack_png, pack_png_path)
                    log(f"已将 pack.png 复制到根目录")
                    structure_fixed = True
        
        # 检查是否找到pack.mcmeta文件
        if not os.path.exists(pack_meta_path):
            log("错误: 未找到 pack.mcmeta 文件")
            if temp_dir and os.path.exists(temp_dir):
                shutil.rmtree(temp_dir)
            raise ValueError(f"材质包缺少必要的 pack.mcmeta 文件: {zip_path}")
        
        # 检查是否找到assets文件夹
        if not os.path.exists(assets_folder_path):
            log("错误: 未找到 assets 文件夹")
            if temp_dir and os.path.exists(temp_dir):
                shutil.rmtree(temp_dir)
            raise ValueError(f"材质包缺少必要的 assets 文件夹: {zip_path}")
        
        if os.path.exists(pack_meta_path):
            data = None
            try:
                with open(pack_meta_path, 'r', encoding='utf-8-sig') as f:
                    data = json.load(f)
            except (json.JSONDecodeError, UnicodeDecodeError):
                log("编码或JSON解析失败，尝试其他编码格式")
                
                encodings_to_try = ['gbk', 'utf-16', 'latin-1']
                content = None
                
                for encoding in encodings_to_try:
                    try:
                        with open(pack_meta_path, 'r', encoding=encoding) as f:
                            content = f.read()
                        log(f"成功使用{encoding}编码读取文件")
                        break
                    except UnicodeDecodeError:
                        continue
                
                if content is None:
                    log("所有编码尝试失败，使用二进制模式读取并修复")
                    with open(pack_meta_path, 'rb') as f:
                        binary_content = f.read()
                    content = binary_content.decode('utf-8', errors='replace')
                
                cleaned_content = clean_control_characters(content)
                json_content = clean_non_json_content(cleaned_content)
                
                if json_content:
                    try:
                        data = json.loads(json_content)
                    except json.JSONDecodeError as e:
                        log(f"无法修复 JSON 解码错误: {e}")
                        return None
            
            if data is not None:
                with open(pack_meta_path, 'w', encoding='utf-8') as f:
                    json.dump(data, f, ensure_ascii=False, indent=4)
            else:
                log("警告: 无法解析pack.mcmeta文件，跳过保存操作")
        
        return temp_dir, structure_fixed
    except PermissionError as e:
        log(f"Error extracting zip '{zip_path}': Permission denied. {e}")
        traceback.print_exc()
        if temp_dir and os.path.exists(temp_dir):
            shutil.rmtree(temp_dir)
        return None, False
    except Exception as e:
        log(f"Error extracting zip '{zip_path}': {e}")
        if temp_dir and os.path.exists(temp_dir):
            shutil.rmtree(temp_dir)
        traceback.print_exc()
        return None, False

def delete_folder(path):
    """
    删除指定路径的文件夹及其内容。
    """
    if os.path.exists(path):
        shutil.rmtree(path)
        log(f"已删除文件夹: {path}")
    else:
        log(f"文件夹不存在，无法删除: {path}")

def delete_blockstates_models(temp_dir):
    delete_folder(os.path.join(temp_dir, "assets/minecraft/blockstates"))
    delete_folder(os.path.join(temp_dir, "assets/minecraft/models"))

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


def fix_alpha_layers_in_textures(temp_dir):
    """
    修复材质包中所有物品贴图的 alpha 图层问题
    尝试多种修复方式以处理不同类型的 alpha 图层问题
    """
    try:
        import os
        from PIL import Image
        
        # 定义需要搜索的目录
        search_dirs = [
            os.path.join(temp_dir, "assets", "minecraft", "textures", "items"),
            os.path.join(temp_dir, "assets", "minecraft", "textures", "blocks"),
            os.path.join(temp_dir, "assets", "minecraft", "textures", "entity"),
            os.path.join(temp_dir, "assets", "minecraft", "textures", "gui"),
            os.path.join(temp_dir, "assets", "minecraft", "textures", "misc")
        ]
        
        # 记录修复的文件数量
        fixed_count = 0
        total_count = 0
        
        # 递归搜索所有 PNG 文件
        for search_dir in search_dirs:
            if not os.path.exists(search_dir):
                continue
            
            for root, dirs, files in os.walk(search_dir):
                for file in files:
                    if file.lower().endswith('.png'):
                        total_count += 1
                        png_path = os.path.join(root, file)
                        
                        try:
                            # 打开图像文件
                            img = Image.open(png_path).convert('RGBA')
                            
                            # 获取图像的宽度和高度
                            width, height = img.size
                            
                            # 检查并修复 alpha 图层
                            has_alpha_issues = False
                            pixels = img.load()
                            
                            for x in range(width):
                                for y in range(height):
                                    r, g, b, a = pixels[x, y]
                                    # 修复方式 1: 处理 alpha 为 0 但 RGB 不为 0 的像素
                                    if a == 0 and (r != 0 or g != 0 or b != 0):
                                        # 修复：将 RGB 值设置为 0
                                        pixels[x, y] = (0, 0, 0, 0)
                                        has_alpha_issues = True
                                    # 修复方式 2: 处理 alpha 不为 0 但 RGB 为 0 的像素（可能是透明度过高）
                                    elif a != 0 and a < 255 and r == 0 and g == 0 and b == 0:
                                        # 修复：根据 alpha 值调整 RGB 值，使其更接近可见
                                        # 这里使用 alpha 值作为 RGB 值的基础，确保有一定的可见性
                                        gray_value = int(a * 0.8)  # 使用 alpha 值的 80% 作为灰度值
                                        pixels[x, y] = (gray_value, gray_value, gray_value, a)
                                        has_alpha_issues = True
                                    # 修复方式 3: 处理 alpha 值异常的像素（如负值或超过 255 的值）
                                    elif a < 0 or a > 255:
                                        # 修复：将 alpha 值限制在 0-255 范围内
                                        clamped_alpha = max(0, min(255, a))
                                        pixels[x, y] = (r, g, b, clamped_alpha)
                                        has_alpha_issues = True
                                    # 修复方式 4: 处理 RGB 值异常的像素（如负值或超过 255 的值）
                                    elif r < 0 or r > 255 or g < 0 or g > 255 or b < 0 or b > 255:
                                        # 修复：将 RGB 值限制在 0-255 范围内
                                        clamped_r = max(0, min(255, r))
                                        clamped_g = max(0, min(255, g))
                                        clamped_b = max(0, min(255, b))
                                        pixels[x, y] = (clamped_r, clamped_g, clamped_b, a)
                                        has_alpha_issues = True
                                    # 修复方式 5: 处理半透明像素的边缘问题（抗锯齿修复）
                                    elif a > 0 and a < 255:
                                        # 计算像素的实际亮度
                                        brightness = (r + g + b) / 3
                                        # 如果亮度与 alpha 值差异过大，可能存在边缘问题
                                        # 这里使用简单的方法：确保半透明像素的亮度与 alpha 值成比例
                                        expected_brightness = int(a * 0.8)
                                        if abs(brightness - expected_brightness) > 50:
                                            # 调整 RGB 值，使其亮度更接近预期值
                                            if brightness > expected_brightness:
                                                # 过亮，降低亮度
                                                scale_factor = expected_brightness / max(brightness, 1)
                                                new_r = int(r * scale_factor)
                                                new_g = int(g * scale_factor)
                                                new_b = int(b * scale_factor)
                                            else:
                                                # 过暗，增加亮度
                                                scale_factor = expected_brightness / max(brightness, 1)
                                                new_r = min(255, int(r * scale_factor))
                                                new_g = min(255, int(g * scale_factor))
                                                new_b = min(255, int(b * scale_factor))
                                            pixels[x, y] = (new_r, new_g, new_b, a)
                                            has_alpha_issues = True
                                    # 修复方式 6: 处理颜色通道不平衡的像素
                                    elif a == 255 and (r, g, b) != (0, 0, 0):
                                        # 计算 RGB 值的标准差，如果过大，说明颜色通道不平衡
                                        avg_color = (r + g + b) / 3
                                        std_dev = ((r - avg_color)**2 + (g - avg_color)**2 + (b - avg_color)**2) ** 0.5
                                        if std_dev > 80:  # 阈值可以根据需要调整
                                            # 修复：调整颜色通道，使其更加平衡
                                            # 这里使用简单的方法：向平均颜色值靠拢
                                            balance_factor = 0.3  # 平衡因子，0-1 之间
                                            new_r = int(r * (1 - balance_factor) + avg_color * balance_factor)
                                            new_g = int(g * (1 - balance_factor) + avg_color * balance_factor)
                                            new_b = int(b * (1 - balance_factor) + avg_color * balance_factor)
                                            pixels[x, y] = (new_r, new_g, new_b, a)
                                            has_alpha_issues = True
                                    # 修复方式 7: 处理边缘像素的 alpha 值，确保边缘过渡更加平滑
                                    elif a > 0 and a < 255:
                                        # 检查周围像素的 alpha 值，如果差异过大，可能存在边缘问题
                                        # 这里使用简单的方法：检查上下左右四个像素
                                        neighbor_alpha_values = []
                                        for dx, dy in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
                                            nx, ny = x + dx, y + dy
                                            if 0 <= nx < width and 0 <= ny < height:
                                                nr, ng, nb, na = pixels[nx, ny]
                                                neighbor_alpha_values.append(na)
                                        if neighbor_alpha_values:
                                            avg_neighbor_alpha = sum(neighbor_alpha_values) / len(neighbor_alpha_values)
                                            if abs(a - avg_neighbor_alpha) > 80:  # 阈值可以根据需要调整
                                                # 修复：向周围像素的平均 alpha 值靠拢
                                                smooth_factor = 0.5  # 平滑因子，0-1 之间
                                                new_alpha = int(a * (1 - smooth_factor) + avg_neighbor_alpha * smooth_factor)
                                                # 同时调整 RGB 值，确保颜色与新的 alpha 值匹配
                                                brightness = (r + g + b) / 3
                                                expected_brightness = int(new_alpha * 0.8)
                                                if brightness != 0:
                                                    scale_factor = expected_brightness / brightness
                                                    new_r = min(255, int(r * scale_factor))
                                                    new_g = min(255, int(g * scale_factor))
                                                    new_b = min(255, int(b * scale_factor))
                                                else:
                                                    new_r = new_g = new_b = expected_brightness
                                                pixels[x, y] = (new_r, new_g, new_b, new_alpha)
                                                has_alpha_issues = True
                                    # 修复方式 8: 处理完全不透明但颜色接近黑色的像素
                                    elif a == 255 and r < 30 and g < 30 and b < 30:
                                        # 检查周围像素的颜色，如果周围像素较亮，可能是由于 alpha 通道问题导致的
                                        neighbor_brightness = []
                                        for dx, dy in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
                                            nx, ny = x + dx, y + dy
                                            if 0 <= nx < width and 0 <= ny < height:
                                                nr, ng, nb, na = pixels[nx, ny]
                                                if na == 255:
                                                    neighbor_brightness.append((nr + ng + nb) / 3)
                                        if neighbor_brightness:
                                            avg_neighbor_brightness = sum(neighbor_brightness) / len(neighbor_brightness)
                                            if avg_neighbor_brightness > 100:  # 阈值可以根据需要调整
                                                # 修复：将像素颜色调整为周围像素的平均颜色
                                                new_r = int(avg_neighbor_brightness)
                                                new_g = int(avg_neighbor_brightness)
                                                new_b = int(avg_neighbor_brightness)
                                                pixels[x, y] = (new_r, new_g, new_b, a)
                                                has_alpha_issues = True
                                    # 修复方式 9: 处理半透明像素的颜色饱和度
                                    elif a > 0 and a < 255:
                                        # 计算颜色的饱和度
                                        max_rgb = max(r, g, b)
                                        min_rgb = min(r, g, b)
                                        if max_rgb > 0:
                                            saturation = (max_rgb - min_rgb) / max_rgb
                                            # 如果饱和度与 alpha 值不匹配，可能存在问题
                                            expected_saturation = a / 255
                                            if abs(saturation - expected_saturation) > 0.3:  # 阈值可以根据需要调整
                                                # 修复：调整颜色饱和度，使其与 alpha 值匹配
                                                # 这里使用简单的方法：向灰度值靠拢或远离
                                                if saturation > expected_saturation:
                                                    # 降低饱和度
                                                    gray_value = (r + g + b) / 3
                                                    desaturate_factor = expected_saturation / max(saturation, 0.1)
                                                    new_r = int(r * desaturate_factor + gray_value * (1 - desaturate_factor))
                                                    new_g = int(g * desaturate_factor + gray_value * (1 - desaturate_factor))
                                                    new_b = int(b * desaturate_factor + gray_value * (1 - desaturate_factor))
                                                else:
                                                    # 增加饱和度
                                                    gray_value = (r + g + b) / 3
                                                    saturate_factor = expected_saturation / max(saturation, 0.1)
                                                    new_r = min(255, int((r - gray_value) * saturate_factor + gray_value))
                                                    new_g = min(255, int((g - gray_value) * saturate_factor + gray_value))
                                                    new_b = min(255, int((b - gray_value) * saturate_factor + gray_value))
                                                pixels[x, y] = (new_r, new_g, new_b, a)
                                                has_alpha_issues = True
                                    # 修复方式 10: 处理图像整体的 alpha 通道问题
                                    # 这里可以添加更复杂的算法，如 alpha 通道的一致性检查
                                    # 暂时使用简单的方法：确保 alpha 值为 255 的像素不会有半透明的邻居
                                    elif a == 255:
                                        # 检查周围像素的 alpha 值
                                        for dx, dy in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
                                            nx, ny = x + dx, y + dy
                                            if 0 <= nx < width and 0 <= ny < height:
                                                nr, ng, nb, na = pixels[nx, ny]
                                                if na > 0 and na < 255:
                                                    # 计算边缘像素的颜色，确保与不透明像素的颜色匹配
                                                    brightness = (r + g + b) / 3
                                                    expected_brightness = int(na * 0.8)
                                                    if brightness != 0:
                                                        scale_factor = expected_brightness / brightness
                                                        new_r = min(255, int(r * scale_factor))
                                                        new_g = min(255, int(g * scale_factor))
                                                        new_b = min(255, int(b * scale_factor))
                                                    else:
                                                        new_r = new_g = new_b = expected_brightness
                                                    pixels[nx, ny] = (new_r, new_g, new_b, na)
                                                    has_alpha_issues = True
                            
                            # 如果有修复，保存文件
                            if has_alpha_issues:
                                img.save(png_path, 'PNG')
                                fixed_count += 1
                                log(f"修复了 alpha 图层问题: {os.path.relpath(png_path, temp_dir)}")
                            
                            # 关闭图像文件
                            img.close()
                            
                        except Exception as e:
                            log(f"处理文件 {png_path} 时出错: {e}")
                        
        log(f"全物品贴图图层修复完成，共处理 {total_count} 个文件，修复了 {fixed_count} 个文件")
        
    except Exception as e:
        log(f"执行全物品贴图图层修复时出错: {e}")


def fix_ui_survival(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/gui/inventory.png 文件，
    """
    try:
        # 定义相关路径
        gui_path = os.path.join(temp_dir, "assets/minecraft/textures/gui/container")
        inventory_path = os.path.join(gui_path, 'inventory.png')
        
        log(f"Processing inventory image in: {gui_path}")
        
        if os.path.exists(inventory_path):
            log(f"Found 'inventory.png' at {inventory_path}")
            img = Image.open(inventory_path).convert("RGBA")
            width, height = img.size
            log(f"Image size: {width}x{height}")

            # Determine scale factor based on image size
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'inventory.png': {width}x{height}")
                return

            # Adjust mob effect image size based on the scale_factor
            image_width = 18 * scale_factor  # Each mob effect image is adjusted based on the scale_factor
            image_height = 18 * scale_factor

            def scaled_coords(x1, y1, x2, y2):
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            def scaled_point(x, y):
                return (x * scale_factor, y * scale_factor)

            # Step 1: Extract the mob effect region from (0,198) to (144,254)
            effect_box = scaled_coords(0, 198, 144, 254)
            effect_region = img.crop(effect_box)
            log(f"Extracted mob effect region from {effect_box}")

            # Step 2: Define the effect image names for each row
            mob_effect_images = [
                # First row
                ["speed.png", "slowness.png", "haste.png", "mining_fatigue.png", "strength.png", "weakness.png", "poison.png", "regeneration.png"],
                # Second row
                ["invisibility.png", "hunger.png", "jump_boost.png", "nausea.png", "night_vision.png", "blindness.png", "resistance.png", "fire_resistance.png"],
                # Third row
                ["water_breathing.png", "wither.png", "absorption.png"]
            ]

            # Step 3: Set up mob effect path
            mob_effect_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'mob_effect')

            log(f"Saving mob effect images to: {mob_effect_path}")

            # Create the output folder for mob effect images if it doesn't exist
            if not os.path.exists(mob_effect_path):
                os.makedirs(mob_effect_path)
                log(f"Created folder for mob effect images: {mob_effect_path}")
            
            # Step 4: Loop through each effect and save it as individual images
            for row_idx, row in enumerate(mob_effect_images):
                for col_idx, effect_image_name in enumerate(row):
                    # Calculate position of each image in the effect region
                    x_offset = col_idx * image_width
                    y_offset = row_idx * image_height
                    effect_img = effect_region.crop((x_offset, y_offset, x_offset + image_width, y_offset + image_height))

                    # Save the image to the mob effect folder
                    effect_img.save(os.path.join(mob_effect_path, effect_image_name))
                    log(f"Saved {effect_image_name} in {mob_effect_path}")

            # Step 5: Process the rest of the inventory image as needed (moving, color filling, etc.)
            # Move regions
            move_region(img, *scaled_coords(86, 24, 162, 62), *scaled_point(10, -8))

            # Color fill regions
            color_fill_region(img, *scaled_coords(75, 6, 96, 80), *scaled_point(90, 10))
            color_fill_region(img, *scaled_coords(96, 54, 162, 62), *scaled_point(90, 10))

            # Copy and paste regions
            copy_and_paste_region(img, *scaled_coords(152, 26, 172, 46), *scaled_point(75, 60))


            # ===== 新增步骤：根据尺寸覆盖 'inventory.png' =====

            # 获取脚本或exe的路径
            if getattr(sys, 'frozen', False):
                exe_folder = os.path.dirname(sys.executable)
            else:
                exe_folder = os.path.dirname(os.path.abspath(__file__))
            inventory_folder = os.path.join(exe_folder, 'inventory')

            log(f"Looking for inventory size files in: {inventory_folder}")

            # 根据尺寸选择对应的 inventory_xxx.png 文件名
            if scale_factor == 1:
                inventory_size_file = 'inventory_256.png'
            elif scale_factor == 2:
                inventory_size_file = 'inventory_512.png'
            elif scale_factor == 4:
                inventory_size_file = 'inventory_1024.png'
            elif scale_factor == 8:
                inventory_size_file = 'inventory_2048.png'
            else:
                # 这一步实际上不会被执行，因为之前已经检查了尺寸
                log(f"No corresponding inventory_xxx.png for scale_factor: {scale_factor}")
                return

            # 构建要覆盖的文件路径
            inventory_size_path = os.path.join(inventory_folder, inventory_size_file)

            # 检查对应的 inventory_xxx.png 是否存在，并进行覆盖操作
            if os.path.exists(inventory_size_path):
                try:
                    # 打开 'inventory_xxx.png'
                    overlay_img = Image.open(inventory_size_path).convert("RGBA")
                    
                    # 检查尺寸是否匹配
                    if overlay_img.size != img.size:
                        log(f"Resizing '{inventory_size_file}' from {overlay_img.size} to {img.size}")
                        overlay_img = overlay_img.resize(img.size, Image.Resampling.NEAREST)
                    
                    # 将 overlay_img 叠加到 img 上
                    img = Image.alpha_composite(img, overlay_img)
                    log(f"Overlayed '{inventory_size_file}' onto 'inventory.png'")
                    
                    # 保存叠加后的图片
                    img.save(inventory_path)
                    log(f"Saved the updated 'inventory.png' with overlay in {gui_path}")
                except Exception as e:
                    log(f"Failed to overlay '{inventory_size_file}' onto 'inventory.png': {e}")
            else:
                log(f"Expected '{inventory_size_file}' not found in '{inventory_folder}'")

        else:
            log(f"No 'inventory.png' found in {gui_path}")

    except Exception as e:
        log(f"Error processing image '{inventory_path}': {e}")
        traceback.print_exc()

def fix_ui_creative(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/gui/container/creative_inventory/tab_inventory.png 文件。
    """
    try:
        # 定义相关路径
        creative_inventory_path = os.path.join(
            temp_dir, 
            "assets", 
            "minecraft", 
            "textures", 
            "gui", 
            "container", 
            "creative_inventory", 
            "tab_inventory.png"
        )
        
        log(f"Processing creative inventory image: {creative_inventory_path}")
        
        if os.path.exists(creative_inventory_path):
            log(f"Found 'tab_inventory.png' at {creative_inventory_path}")
            img = Image.open(creative_inventory_path).convert("RGBA")
            width, height = img.size
            log(f"Image size: {width}x{height}")

            # 确定 scale_factor
            scale_factor, is_exact = determine_scale_factor(width, height)
            log(f"Determined scale_factor: {scale_factor} (Exact match: {is_exact})")

            def scaled_coords(x1, y1, x2, y2):
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            def scaled_point(x, y):
                return (x * scale_factor, y * scale_factor)

            # 1. 复制 (6,0)-(84,53) 到 (51,0)-(129,53)
            source_box = scaled_coords(6, 0, 84, 53)
            dest_box = scaled_coords(51, 0, 129, 53)
            region = img.crop(source_box)
            img.paste(region, dest_box)
            log(f"Copied region {source_box} to {dest_box}")

            # 2. 填充 (6,0)-(53,53) 区域的颜色
            fill_box = scaled_coords(6, 0, 53, 53)
            fill_color = img.getpixel(scaled_point(164, 27))
            log(f"Filling region {fill_box} with color {fill_color}")
            for x in range(fill_box[0], fill_box[2]):
                for y in range(fill_box[1], fill_box[3]):
                    img.putpixel((x, y), fill_color)

            # 3. 复制并粘贴 (53,5)-(71,23) 到 (34,19)-(52,37)
            source_box_18x18 = scaled_coords(53, 5, 71, 23)
            dest_position = scaled_coords(34, 19, 52, 37)[:2]  # (34,19)
            region_18x18 = img.crop(source_box_18x18)
            img.paste(region_18x18, dest_position)
            log(f"Pasted region {source_box_18x18} to {dest_position}")

            # 保存生成的 tab_inventory.png
            img.save(creative_inventory_path)
            log(f"Processed 'tab_inventory.png' and saved the modified image")
        else:
            log(f"No 'tab_inventory.png' found in {os.path.dirname(creative_inventory_path)}")
    
    except Exception as e:
        log(f"Error processing image '{creative_inventory_path}': {e}")
        traceback.print_exc()

def fix_ui_sub_hand(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/gui/widgets.png 文件。
    """
    widgets_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "gui", "widgets.png")
    log(f"Processing widgets image: {widgets_path}")
    try:
        if os.path.exists(widgets_path):
            log(f"Found 'widgets.png' at {widgets_path}")
            img = Image.open(widgets_path).convert("RGBA")
            width, height = img.size
            log(f"Image size: {width}x{height}")

            # 确定 scale_factor
            scale_factor, is_exact = determine_scale_factor(width, height)
            log(f"Determined scale_factor: {scale_factor} (Exact match: {is_exact})")

            def scaled_coords(x1, y1, x2, y2):
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            def scaled_point(x, y):
                return (x * scale_factor, y * scale_factor)

            # 1. 复制 (1,23)-(23,45) 到 (24,23)-(46,45) 和 (60,23)-(82,45)
            source_box = scaled_coords(1, 23, 23, 45)
            dest_box_1 = scaled_coords(24, 23, 46, 45)
            dest_box_2 = scaled_coords(60, 23, 82, 45)

            region = img.crop(source_box)
            img.paste(region, dest_box_1)
            log(f"Copied region {source_box} to {dest_box_1}")
            img.paste(region, dest_box_2)
            log(f"Copied region {source_box} to {dest_box_2}")

            # 保存修改后的 widgets.png
            img.save(widgets_path)
            log(f"Processed 'widgets.png' and saved the modified image")
        else:
            log(f"No 'widgets.png' found in {os.path.dirname(widgets_path)}")
    
    except Exception as e:
        log(f"Error processing image '{widgets_path}': {e}")
        traceback.print_exc()

def generate_boat(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/items/boat.png 文件，
    """
    items_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "items")
    boat_path = os.path.join(items_path, "boat.png")
    
    if not os.path.exists(boat_path):
        log("No 'boat.png' found, skipping boat processing.")
        return

    try:
        base_img = Image.open(boat_path).convert("RGBA")
        log(f"Opened 'boat.png' with size {base_img.size}")

        # oak_boat.png（亮度+15）
        oak_img = adjust_hue_brightness(base_img, brightness_shift=15)
        oak_boat_path = os.path.join(items_path, "oak_boat.png")
        oak_img.save(oak_boat_path)
        log(f"Generated 'oak_boat.png' with brightness +15")

        # birch_boat.png（亮度+40）
        birch_img = adjust_hue_brightness(base_img, brightness_shift=40)
        birch_boat_path = os.path.join(items_path, "birch_boat.png")
        birch_img.save(birch_boat_path)
        log(f"Generated 'birch_boat.png' with brightness +40")

        # acacia_boat.png（色相-23度, 亮度+10）
        acacia_img = adjust_hue_brightness(base_img, hue_shift=-23, brightness_shift=10)
        acacia_boat_path = os.path.join(items_path, "acacia_boat.png")
        acacia_img.save(acacia_boat_path)
        log(f"Generated 'acacia_boat.png' with hue -23 and brightness +10")

        # dark_oak_boat.png（亮度-15）
        dark_oak_img = adjust_hue_brightness(base_img, brightness_shift=-15)
        dark_oak_boat_path = os.path.join(items_path, "dark_oak_boat.png")
        dark_oak_img.save(dark_oak_boat_path)
        log(f"Generated 'dark_oak_boat.png' with brightness -15")

        # jungle_boat.png（色相-10度, 亮度+4.6）
        jungle_img = adjust_hue_brightness(base_img, hue_shift=-10, brightness_shift=4.6)
        jungle_boat_path = os.path.join(items_path, "jungle_boat.png")
        jungle_img.save(jungle_boat_path)
        log(f"Generated 'jungle_boat.png' with hue -10 and brightness +4.6")

        # 将 boat.png 重命名为 spruce_boat.png
        spruce_path = os.path.join(items_path, "spruce_boat.png")
        if os.path.exists(spruce_path):
            os.remove(spruce_path)
            log(f"Removed existing 'spruce_boat.png'")
        os.rename(boat_path, spruce_path)
        log("Renamed 'boat.png' to 'spruce_boat.png'")

    except Exception as e:
        log(f"Error processing boat images: {e}")
        traceback.print_exc()

def generate_potion_lingering(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/items/potion.png 和
    """
    try:
        items_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "items")
        log(f"Processing potion images in: {items_path}")

        # 定义需要处理的图片及其对应的输出名称
        images_to_process = [
            ('potion.png', 'lingering_potion.png'),
            ('potion_bottle_drinkable.png', 'potion_bottle_lingering.png')
        ]

        for original_name, new_name in images_to_process:
            original_path = os.path.join(items_path, original_name)
            new_path = os.path.join(items_path, new_name)
            mcmeta_original = original_path + '.mcmeta'
            mcmeta_new = new_path + '.mcmeta'

            if os.path.exists(original_path):
                log(f"Found '{original_name}', processing...")
                # 复制原始图片到新的文件名
                shutil.copy(original_path, new_path)
                img = Image.open(new_path).convert("RGBA")

                width, height = img.size
                log(f"Image size: {width}x{height}")

                # 判断图像是否为正方形
                if width == height:
                    log(f"'{original_name}' is a square image. Processing entire image.")
                    # 将上三分之一部分设为透明
                    new_image_data = []
                    for y in range(height):
                        for x in range(width):
                            pixel = img.getpixel((x, y))
                            if y < height // 3:
                                new_image_data.append((0, 0, 0, 0))  # 上三分之一透明
                            else:
                                new_image_data.append(pixel)

                    new_image = Image.new("RGBA", img.size)
                    new_image.putdata(new_image_data)
                    new_image.save(new_path)
                    log(f"Processed '{original_name}' to '{new_name}' by making the top third transparent.")

                else:
                    # 检查高度是否是宽度的整数倍，判断是否为多个正方形垂直拼接
                    if height % width == 0:
                        num_squares = height // width
                        log(f"'{original_name}' is a vertically stacked image with {num_squares} squares.")
                        new_image = Image.new("RGBA", (width, height))

                        for i in range(num_squares):
                            top = i * width
                            bottom = top + width
                            box = (0, top, width, bottom)
                            square = img.crop(box)

                            # 处理单个正方形：将上三分之一设为透明
                            square_data = []
                            for y in range(width):
                                for x in range(width):
                                    pixel = square.getpixel((x, y))
                                    if y < width // 3:
                                        square_data.append((0, 0, 0, 0))  # 上三分之一透明
                                    else:
                                        square_data.append(pixel)

                            processed_square = Image.new("RGBA", (width, width))
                            processed_square.putdata(square_data)
                            new_image.paste(processed_square, (0, top))
                            log(f"Processed square {i+1}/{num_squares} in '{original_name}'.")

                        new_image.save(new_path)
                        log(f"Processed '{original_name}' to '{new_name}' with {num_squares} squares by making the top third of each square transparent.")

                    else:
                        log(f"'{original_name}' is neither a square nor a vertically stacked image of squares. Skipping processing.")
                        continue  # 跳过不符合条件的图像

                # 处理对应的 .mcmeta 文件
                if os.path.exists(mcmeta_original):
                    shutil.copy(mcmeta_original, mcmeta_new)
                    log(f"Copied and renamed '{original_name}.mcmeta' to '{new_name}.mcmeta'.")
                else:
                    log(f"No '{original_name}.mcmeta' found in '{items_path}'. Skipping .mcmeta processing.")

            else:
                log(f"No '{original_name}' found in {items_path}. Skipping this image.")

    except Exception as e:
        log(f"Error processing potion images: {e}")
        traceback.print_exc()

def generate_shulker_box_ui(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/gui/container/generic_54.png 文件，
    """
    try:
        # 定义相关路径
        container_path = os.path.join(
            temp_dir,
            "assets",
            "minecraft",
            "textures",
            "gui",
            "container"
        )
        generic_54_path = os.path.join(container_path, 'generic_54.png')
        shulker_box_path = os.path.join(container_path, 'shulker_box.png')
        
        log(f"Processing generic 54 image in: {container_path}")
        
        if os.path.exists(generic_54_path):
            log(f"Found 'generic_54.png' at {generic_54_path}")
            img = Image.open(generic_54_path).convert("RGBA")
            width, height = img.size
            log(f"Image size: {width}x{height}")

            # 确定 scale_factor
            scale_factor, is_exact = determine_scale_factor(width, height)
            log(f"Determined scale_factor: {scale_factor} (Exact match: {is_exact})")

            if not is_exact:
                log(f"Scale factor for 'generic_54.png' not exact. Proceeding with scale_factor: {scale_factor}")

            # 创建图像副本进行修改
            new_img = img.copy()

            # 1. 将 (0,71)-(176,127) 区域设为透明
            for x in range(0, 176 * scale_factor):
                for y in range(71 * scale_factor, 127 * scale_factor):
                    new_img.putpixel((x, y), (0, 0, 0, 0))
            log(f"Set pixels in (0, {71 * scale_factor}) to (176*{scale_factor}, {127 * scale_factor}) to transparent.")

            # 2. 将 (0,127)-(176,222) 区域向上移动56*scale_factor像素，并将原位置设为透明
            for x in range(0, 176 * scale_factor):
                for y in range(127 * scale_factor, 222 * scale_factor):
                    new_y = y - 56 * scale_factor
                    if new_y >= 0:
                        pixel = img.getpixel((x, y))
                        new_img.putpixel((x, new_y), pixel)
                    new_img.putpixel((x, y), (0, 0, 0, 0))
            log(f"Moved pixels in (0, {127 * scale_factor}) to (176*{scale_factor}, {222 * scale_factor}) up by {56 * scale_factor} pixels and set original area to transparent.")

            # 保存生成的 shulker_box.png
            new_img.save(shulker_box_path)
            log(f"Processed 'generic_54.png' and saved as 'shulker_box.png'")
        else:
            log(f"No 'generic_54.png' found in {container_path}")
    
    except Exception as e:
        log(f"Error processing 'generic_54.png' in '{container_path}': {e}")
        traceback.print_exc()

def fix_brewing_stand_ui(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/gui/container/shulker_box.png 文件，
    """
    try:
        # 定义相关路径
        container_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "gui", "container")
        shulker_box_path = os.path.join(container_path, 'shulker_box.png')
        brewing_stand_new_path = os.path.join(container_path, 'brewing_stand.png')
        # 获取脚本或exe的路径
        if getattr(sys, 'frozen', False):
            exe_folder = os.path.dirname(sys.executable)
        else:
            exe_folder = os.path.dirname(os.path.abspath(__file__))
        # 生成brewing_stand.png的路径
        brewing_stand_path = os.path.join(exe_folder, 'brewing_stand')
        
        log(f"Processing brewing stand image in: {container_path}")
        
        if os.path.exists(shulker_box_path):
            log(f"Found 'shulker_box.png' at {shulker_box_path}")
            img = Image.open(shulker_box_path).convert("RGBA")
            width, height = img.size
            log(f"Image size: {width}x{height}")

            if width == height:
                if width == 256:
                    scale_factor = 1
                    brewing_stand_image_path = os.path.join(brewing_stand_path, 'brewing_stand_256.png')
                elif width == 512:
                    scale_factor = 2
                    brewing_stand_image_path = os.path.join(brewing_stand_path, 'brewing_stand_512.png')
                elif width == 1024:
                    scale_factor = 4
                    brewing_stand_image_path = os.path.join(brewing_stand_path, 'brewing_stand_1024.png')
                elif width == 2048:
                    scale_factor = 8
                    brewing_stand_image_path = os.path.join(brewing_stand_path, 'brewing_stand_2048.png')
                else:
                    log(f"Unsupported image size for 'shulker_box.png': {width}x{height}")
                    return

                img_copy = img.copy()
                cover_box = (6 * scale_factor, 16 * scale_factor, 170 * scale_factor, 72 * scale_factor)
                fill_color = img.getpixel((5 * scale_factor, 4 * scale_factor))
                
                # 填充 cover_box 区域的颜色
                for x in range(cover_box[0], cover_box[2]):
                    for y in range(cover_box[1], cover_box[3]):
                        img_copy.putpixel((x, y), fill_color)
                log(f"Filled cover_box {cover_box} with color {fill_color}")
                
                # 复制并粘贴区域
                region = img_copy.crop((7 * scale_factor, 83 * scale_factor, 25 * scale_factor, 101 * scale_factor))
                img_copy.paste(region, (16 * scale_factor, 16 * scale_factor))
                img_copy.paste(region, (78 * scale_factor, 16 * scale_factor))
                img_copy.paste(region, (55 * scale_factor, 50 * scale_factor))
                img_copy.paste(region, (78 * scale_factor, 57 * scale_factor))
                img_copy.paste(region, (101 * scale_factor, 50 * scale_factor))
                log(f"Pasted region to multiple positions with scale_factor {scale_factor}")

                if os.path.exists(brewing_stand_image_path):
                    overlay_img = Image.open(brewing_stand_image_path).convert("RGBA")
                    img_copy.paste(overlay_img, (0, 0), overlay_img)
                    log(f"Overlayed '{brewing_stand_image_path}' onto brewing_stand.png")
                else:
                    log(f"No overlay image found at '{brewing_stand_image_path}'")

                # 保存 brewing_stand.png
                img_copy.save(brewing_stand_new_path)
                log(f"Processed 'shulker_box.png' and saved as 'brewing_stand.png'")
            else:
                log(f"'shulker_box.png' is not a square image: {width}x{height}. Skipping processing.")
        else:
            log(f"No 'shulker_box.png' found in {container_path}")

    except Exception as e:
        log(f"Error processing brewing stand image in '{container_path}': {e}")
        traceback.print_exc()

def fix_clock_compass(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/items/clock.png 和 compass.png 文件，
    """
    try:
        assets_path = os.path.join(temp_dir, "assets", "minecraft", "textures")
        items_path_old = os.path.join(assets_path, 'items')
        clock_path = os.path.join(items_path_old, 'clock.png')
        compass_path = os.path.join(items_path_old, 'compass.png')
        
        log(f"Processing clock and compass images in: {items_path_old}")
        
        # 处理 clock.png
        if os.path.exists(clock_path):
            log(f"Found 'clock.png' at {clock_path}, processing...")
            split_image(clock_path, items_path_old, 'clock', 64)
            log(f"Successfully split 'clock.png' into clock images.")
        else:
            log(f"No 'clock.png' found in {items_path_old}. Skipping clock processing.")
        
        # 处理 compass.png
        if os.path.exists(compass_path):
            log(f"Found 'compass.png' at {compass_path}, processing...")
            split_image(compass_path, items_path_old, 'compass', 32)
            log(f"Successfully split 'compass.png' into compass images.")
        else:
            log(f"No 'compass.png' found in {items_path_old}. Skipping compass processing.")
    
    except Exception as e:
        log(f"Error processing clock and compass images: {e}")
        traceback.print_exc()

def delete_horse_folder(temp_dir):
    delete_folder(os.path.join(temp_dir, "assets/minecraft/textures/entity/horse"))

def fix_horse_ui(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/gui/container/horse.png 文件，
    """
    try:
        # 定义相关路径
        container_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "gui", "container")
        horse_new_path = os.path.join(container_path, 'horse.png')
        
        # 获取脚本或exe的路径
        if getattr(sys, 'frozen', False):
            exe_folder = os.path.dirname(sys.executable)
        else:
            exe_folder = os.path.dirname(os.path.abspath(__file__))
        # 生成brewing_stand.png的路径
        horse_path = os.path.join(exe_folder, 'horse')

        log(f"Processing horse image in: {container_path}")
        
        if os.path.exists(horse_new_path):
            log(f"Found 'horse.png' at {horse_new_path}")
            img = Image.open(horse_new_path).convert("RGBA")
            width, height = img.size
            log(f"Image size: {width}x{height}")

            # 根据分辨率设置 scale_factor 和输出路径
            if width == 256 and height == 256:
                scale_factor = 1
                horse_image_path = os.path.join(horse_path, 'horse_256.png')
            elif width == 512 and height == 512:
                scale_factor = 2
                horse_image_path = os.path.join(horse_path, 'horse_512.png')
            elif width == 1024 and height == 1024:
                scale_factor = 4
                horse_image_path = os.path.join(horse_path, 'horse_1024.png')
            elif width == 2048 and height == 2048:
                scale_factor = 8
                horse_image_path = os.path.join(horse_path, 'horse_2048.png')
            else:
                log(f"Unsupported image size for 'horse.png': {width}x{height}")
                return

            img_copy = img.copy()

            # 步骤1: 将 (7,17) 到 (25,35) 的矩形区域平移到 (18,220) 到 (36,238)
            try:
                move_box = (7 * scale_factor, 17 * scale_factor, 25 * scale_factor, 35 * scale_factor)
                paste_box = (18 * scale_factor, 220 * scale_factor, 36 * scale_factor, 238 * scale_factor)
                region = img_copy.crop(move_box)
                img_copy.paste(region, paste_box)
                log(f"Moved region from {move_box} to {paste_box}")
            except Exception as e:
                log(f"Error moving region: {e}")
                traceback.print_exc()

            # 步骤2: 用 (7,16) 到 (8,17) 的一个像素块的颜色填充 (7,17) 到 (25,35) 之间的矩形区域
            try:
                fill_color = img_copy.getpixel((7 * scale_factor, 16 * scale_factor))
                fill_box = (7 * scale_factor, 17 * scale_factor, 25 * scale_factor, 35 * scale_factor)
                log(f"Filling region {fill_box} with color {fill_color}")
                for x in range(fill_box[0], fill_box[2]):
                    for y in range(fill_box[1], fill_box[3]):
                        img_copy.putpixel((x, y), fill_color)
            except Exception as e:
                log(f"Error filling region with color: {e}")
                traceback.print_exc()

            # 步骤3: 将 (36,202) 到 (54,220) 之间的矩形区域复制粘贴到 (36,220) 到 (54,238)
            try:
                copy_box = (36 * scale_factor, 202 * scale_factor, 54 * scale_factor, 220 * scale_factor)
                paste_box = (36 * scale_factor, 220 * scale_factor, 54 * scale_factor, 238 * scale_factor)
                region = img_copy.crop(copy_box)
                img_copy.paste(region, paste_box)
                log(f"Copied region from {copy_box} to {paste_box}")
            except Exception as e:
                log(f"Error copying region: {e}")
                traceback.print_exc()

            # 步骤4: 在 horse 文件夹中寻找对应分辨率的 horse 图片并覆盖
            try:
                if os.path.exists(horse_image_path):
                    overlay_img = Image.open(horse_image_path).convert("RGBA")
                    img_copy.paste(overlay_img, (0, 0), overlay_img)
                    log(f"Overlayed '{horse_image_path}' onto horse.png")
                else:
                    log(f"No overlay image found for size {width}x{height} at '{horse_image_path}'")
            except Exception as e:
                log(f"Error overlaying image: {e}")
                traceback.print_exc()

            # 保存处理后的 horse.png
            try:
                img_copy.save(horse_new_path)
                log(f"Processed 'horse.png' and saved it")
            except Exception as e:
                log(f"Error saving 'horse.png': {e}")
                return
        else:
            log(f"No 'horse.png' found in {container_path}")

    except Exception as e:
        log(f"Error processing horse images: {e}")
        traceback.print_exc()

def rename_blocks_items(temp_dir):
    """
    重命名并处理 temp_dir 中的 'blocks' 和 'items' 文件夹。
    """
    try:
        assets_path = os.path.join(temp_dir, "assets", "minecraft", "textures")
        blocks_path_old = os.path.join(assets_path, 'blocks')
        blocks_path_new = os.path.join(assets_path, 'block')
        items_path_old = os.path.join(assets_path, 'items')
        items_path_new = os.path.join(assets_path, 'item')
        
        # 重命名 'blocks' 文件夹为 'block'
        if os.path.exists(blocks_path_old):
            os.rename(blocks_path_old, blocks_path_new)
            log(f"已将 'blocks' 重命名为 'block' 在 {temp_dir}")
        else:
            log(f"未找到 {temp_dir} 中的 'blocks' 文件夹")
        
        # 重命名 'items' 文件夹为 'item'
        if os.path.exists(items_path_old):
            os.rename(items_path_old, items_path_new)
            log(f"已将 'items' 重命名为 'item' 在 {temp_dir}")
        else:
            log(f"未找到 {temp_dir} 中的 'items' 文件夹")
        
        # 定义重命名对
        rename_pairs = {
            'gold_sword.png': 'golden_sword.png',
            'wood_sword.png': 'wooden_sword.png',
            'gold_helmet.png': 'golden_helmet.png',
            'gold_chestplate.png': 'golden_chestplate.png',
            'gold_leggings.png': 'golden_leggings.png',
            'gold_boots.png': 'golden_boots.png',
            'apple_golden.png': 'golden_apple.png',
            'bow_standby.png': 'bow.png',
            'book_enchanted.png': 'enchanted_book.png',
            'wood_axe.png': 'wooden_axe.png',
            'wood_pickaxe.png': 'wooden_pickaxe.png',
            'wood_shovel.png': 'wooden_shovel.png',
            'wood_hoe.png': 'wooden_hoe.png',
            'gold_axe.png': 'golden_axe.png',
            'gold_pickaxe.png': 'golden_pickaxe.png',
            'gold_shovel.png': 'golden_shovel.png',
            'gold_hoe.png': 'golden_hoe.png',
            'fishing_rod_uncast.png': 'fishing_rod.png',
            'potion_bottle_empty.png': 'glass_bottle.png',
            'potion_bottle_drinkable.png': 'potion.png',
            'potion_bottle_splash.png': 'splash_potion.png',
            'potion_bottle_lingering.png': 'lingering_potion.png',
            'spider_eye_fermented.png': 'fermented_spider_eye.png',
            'melon_speckled.png': 'glistering_melon_slice.png',
            'melon.png': 'melon_slice.png',
            'carrot_golden.png': 'golden_carrot.png',
            'porkchop_raw.png': 'porkchop.png',
            'porkchop_cooked.png': 'cooked_porkchop.png',
            'chicken_raw.png': 'chicken.png',
            'chicken_cooked.png': 'cooked_chicken.png',
            'rabbit_raw.png': 'rabbit.png',
            'rabbit_cooked.png': 'cooked_rabbit.png',
            'beef_raw.png': 'beef.png',
            'beef_cooked.png': 'cooked_beef.png',
            'boat.png': 'oak_boat.png',
            'book_normal.png': 'book.png',
            'book_writable.png': 'writable_book.png',
            'book_written.png': 'written_book.png',
            'bucket_empty.png': 'bucket.png',
            'bucket_lava.png': 'lava_bucket.png',
            'bucket_water.png': 'water_bucket.png',
            'bucket_milk.png': 'milk_bucket.png',
            'door_acacia.png': 'acacia_door.png',
            'door_birch.png': 'birch_door.png',
            'door_dark_oak.png': 'dark_oak_door.png',
            'door_iron.png': 'iron_door.png',
            'door_jungle.png': 'jungle_door.png',
            'door_spruce.png': 'spruce_door.png',
            'door_wood.png': 'oak_door.png',
            'dye_powder_black.png': 'ink_sac.png',
            'dye_powder_blue.png': 'lapis_lazuli.png',
            'dye_powder_brown.png': 'cocoa_beans.png',
            'dye_powder_cyan.png': 'cyan_dye.png',
            'dye_powder_gray.png': 'gray_dye.png',
            'dye_powder_green.png': 'green_dye.png',
            'dye_powder_light_blue.png': 'light_blue_dye.png',
            'dye_powder_lime.png': 'lime_dye.png',
            'dye_powder_magenta.png': 'magenta_dye.png',
            'dye_powder_orange.png': 'orange_dye.png',
            'dye_powder_pink.png': 'pink_dye.png',
            'dye_powder_purple.png': 'purple_dye.png',
            'dye_powder_red.png': 'red_dye.png',
            'dye_powder_silver.png': 'light_gray_dye.png',
            'dye_powder_white.png': 'bone_meal.png',
            'dye_powder_yellow.png': 'yellow_dye.png',
            'fireball.png': 'fire_charge.png',
            'fireworks.png': 'firework_rocket.png',
            'fireworks_charge.png': 'firework_star.png',
            'firework_charge_overlay.png': 'firework_star_overlay.png',
            'fish_cod_raw.png': 'cod.png',
            'fish_cod_cooked.png': 'cooked_cod.png',
            'fish_salmon_raw.png': 'salmon.png',
            'fish_salmon_cooked.png': 'cooked_salmon.png',
            'fish_clownfish_raw.png': 'tropical_fish.png',
            'fish_pufferfish_raw.png': 'pufferfish.png',
            'map_empty.png': 'map.png',
            'map_filled.png': 'filled_map.png',
            'minecart_chest.png': 'chest_minecart.png',
            'minecart_command_block.png': 'command_block_minecart.png',
            'minecart_furnace.png': 'furnace_minecart.png',
            'minecart_hopper.png': 'hopper_minecart.png',
            'minecart_normal.png': 'minecart.png',
            'minecart_tnt.png': 'tnt_minecart.png',
            'mutton_cooked.png': 'cooked_mutton.png',
            'mutton_raw.png': 'mutton.png',
            'netherbrick.png': 'nether_brick.png',
            'potato_baked.png': 'baked_potato.png',
            'potato_poisonous.png': 'poisonous_potato.png',
            'record_11.png': 'music_disc_11.png',
            'record_13.png': 'music_disc_13.png',
            'record_blocks.png': 'music_disc_blocks.png',
            'record_cat.png': 'music_disc_cat.png',
            'record_chirp.png': 'music_disc_chirp.png',
            'record_far.png': 'music_disc_far.png',
            'record_mail.png': 'music_disc_mail.png',
            'record_mellohi.png': 'music_disc_mellohi.png',
            'record_stal.png': 'music_disc_stal.png',
            'record_strad.png': 'music_disc_strad.png',
            'record_wait.png': 'music_disc_wait.png',
            'record_ward.png': 'music_disc_ward.png',
            'record_mall.png': 'music_disc_mall.png',
            'redstone_dust.png': 'redstone.png',
            'reeds.png': 'sugar_cane.png',
            'seeds_melon.png': 'melon_seeds.png',
            'seeds_pumpkin.png': 'pumpkin_seeds.png',
            'seeds_wheat.png': 'wheat_seeds.png',
            'sign.png': 'oak_sign.png',
            'slimeball.png': 'slime_ball.png',
            'wooden_armorstand.png': 'armor_stand.png',
            'gold_horse_armor.png': 'golden_horse_armor.png'
        }
        
        # 调用重命名和处理函数
        rename_and_process_blocks(blocks_path_new)
        rename_items(items_path_new, rename_pairs)
        
    except Exception as e:
        log(f"Error renaming blocks and items: {e}")
        traceback.print_exc()

def fix_sign(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/item/sign.png 文件，
    """
    try:
        # 定义相关路径
        item_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "item")
        sign_path = os.path.join(item_path, "oak_sign.png")
        
        if not os.path.exists(sign_path):
            log("No 'sign.png' found, skipping sign processing.")
            return

        log(f"Processing sign images in: {item_path}")

        # 将 oak_sign.png 重命名为 spruce_sign.png
        spruce_path = os.path.join(item_path, "spruce_sign.png")
        if os.path.exists(spruce_path):
            os.remove(spruce_path)
            log(f"Removed existing 'spruce_sign.png'")
        os.rename(sign_path, spruce_path)
        log("Renamed sign.png to spruce_sign.png")
                
        base_img = Image.open(spruce_path).convert("RGBA")

        # 定义需要生成的签名类型及其调整参数
        sign_variants = [
            {'filename': 'oak_sign.png', 'hue_shift': 0, 'brightness_shift': 15, 'saturation_shift': 0},
            {'filename': 'birch_sign.png', 'hue_shift': 0, 'brightness_shift': 40, 'saturation_shift': 0},
            {'filename': 'acacia_sign.png', 'hue_shift': -23, 'brightness_shift': 10, 'saturation_shift': 0},
            {'filename': 'dark_oak_sign.png', 'hue_shift': 0, 'brightness_shift': -15, 'saturation_shift': 0},
            {'filename': 'jungle_sign.png', 'hue_shift': -10, 'brightness_shift': 4.6, 'saturation_shift': 0},
            {'filename': 'crimson_sign.png', 'hue_shift': -59, 'brightness_shift': -30, 'saturation_shift': 0},
            {'filename': 'warped_sign.png', 'hue_shift': 130, 'brightness_shift': -33, 'saturation_shift': 0},
            {'filename': 'mangrove_sign.png', 'hue_shift': -59, 'brightness_shift': -10, 'saturation_shift': 0},
            {'filename': 'pale_oak_sign.png', 'hue_shift': 0, 'brightness_shift': 30, 'saturation_shift': -100},
            {'filename': 'bamboo_sign.png', 'hue_shift': 25, 'brightness_shift': 20, 'saturation_shift': 0},
            {'filename': 'cherry_sign.png', 'hue_shift': -80, 'brightness_shift': 20, 'saturation_shift': 0}
        ]
        
        # 生成各类签名图像
        for variant in sign_variants:
            filename = variant['filename']
            hue_shift = variant['hue_shift']
            brightness_shift = variant['brightness_shift']
            saturation_shift = variant['saturation_shift']
            
            # 调整图像
            adjusted_img = adjust_hue_brightness(
                base_img, 
                hue_shift=hue_shift, 
                brightness_shift=brightness_shift, 
                saturation_shift=saturation_shift
            )
            
            # 保存新图像
            new_sign_path = os.path.join(item_path, filename)
            adjusted_img.save(new_sign_path)
            log(f"Generated {filename} with hue {hue_shift} and brightness {brightness_shift}")
            
    except Exception as e:
        log(f"Error processing sign images in '{item_path}': {e}")
        traceback.print_exc()

def fix_sign_entities(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/entity/sign.png 文件，
    """
    try:
        # 定义相关路径
        entity_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "entity")
        sign_path = os.path.join(entity_path, "sign.png")
        
        if not os.path.exists(sign_path):
            log("No 'sign.png' found in entity textures, skipping entity sign processing.")
            return

        # 创建 signs 文件夹
        signs_folder = os.path.join(entity_path, "signs")
        os.makedirs(signs_folder, exist_ok=True)
        log(f"Ensured 'signs' folder exists at: {signs_folder}")
        
        log(f"Processing sign images in: {entity_path}")
        
        base_img = Image.open(sign_path).convert("RGBA")
        
        # 定义需要生成的签名类型及其调整参数
        sign_variants = [
            {'filename': 'oak.png', 'hue_shift': 0, 'brightness_shift': 15, 'saturation_shift': 0},
            {'filename': 'birch.png', 'hue_shift': 0, 'brightness_shift': 40, 'saturation_shift': 0},
            {'filename': 'acacia.png', 'hue_shift': -23, 'brightness_shift': 10, 'saturation_shift': 0},
            {'filename': 'dark_oak.png', 'hue_shift': 0, 'brightness_shift': -15, 'saturation_shift': 0},
            {'filename': 'jungle.png', 'hue_shift': -10, 'brightness_shift': 4.6, 'saturation_shift': 0},
            {'filename': 'crimson.png', 'hue_shift': -59, 'brightness_shift': -30, 'saturation_shift': 0},
            {'filename': 'warped.png', 'hue_shift': 130, 'brightness_shift': -33, 'saturation_shift': 0},
            {'filename': 'mangrove.png', 'hue_shift': -59, 'brightness_shift': -10, 'saturation_shift': 0},
            {'filename': 'pale_oak.png', 'hue_shift': 0, 'brightness_shift': 30, 'saturation_shift': -100},
            {'filename': 'bamboo.png', 'hue_shift': 25, 'brightness_shift': 20, 'saturation_shift': 0},
            {'filename': 'cherry.png', 'hue_shift': -80, 'brightness_shift': 20, 'saturation_shift': 0}
        ]
        
        # 生成各类签名图像
        for variant in sign_variants:
            filename = variant['filename']
            hue_shift = variant['hue_shift']
            brightness_shift = variant['brightness_shift']
            saturation_shift = variant['saturation_shift']
            
            # 调整图像
            adjusted_img = adjust_hue_brightness(
                base_img, 
                hue_shift=hue_shift, 
                brightness_shift=brightness_shift, 
                saturation_shift=saturation_shift
            )
            
            # 保存新图像
            new_sign_path = os.path.join(signs_folder, filename)
            adjusted_img.save(new_sign_path)
            log(f"Generated {filename} with hue {hue_shift} and brightness {brightness_shift}")
        
        # 将 sign.png 重命名为 spruce.png 并移动到 signs 文件夹
        spruce_path = os.path.join(signs_folder, "spruce.png")
        if os.path.exists(spruce_path):
            os.remove(spruce_path)
            log(f"Removed existing 'spruce.png' at {spruce_path}")
        os.rename(sign_path, spruce_path)
        log("Renamed sign.png to spruce.png and moved to 'signs' folder.")

    except Exception as e:
        log(f"Error processing sign images in '{entity_path}': {e}")
        traceback.print_exc()

def generate_furnace(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/gui/container/furnace.png 文件，
    """
    try:
        # 定义相关路径
        container_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "gui", "container")
        furnace_path = os.path.join(container_path, 'furnace.png')
        blast_furnace_path = os.path.join(container_path, 'blast_furnace.png')
        smoker_path = os.path.join(container_path, 'smoker.png')
        
        log(f"Processing furnace images in: {container_path}")
        
        if os.path.exists(furnace_path):
            shutil.copy(furnace_path, blast_furnace_path)
            shutil.copy(furnace_path, smoker_path)
            log(f"Copied 'furnace.png' to 'blast_furnace.png' and 'smoker.png'")
        else:
            log(f"No 'furnace.png' found in {container_path}")

    except Exception as e:
        log(f"Error processing furnace images in '{container_path}': {e}")
        traceback.print_exc()

def fix_machinery_ui(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/gui/container 文件夹中的多个 UI 图像。
    """
    try:
        # 定义 container_path
        container_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "gui", "container")
        log(f"Processing machinery UI images in: {container_path}")
        
        # 调用各个处理函数
        process_grindstone_image(container_path)
        process_cartography_table_image(container_path)
        process_stonecutter_image(container_path)
        process_loom_image(container_path)
        process_villager_image(container_path)
        
        log("Completed processing machinery UI images.")
        
    except Exception as e:
        log(f"Error processing machinery UI images in '{container_path}': {e}")
        traceback.print_exc()

def fix_particles(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/particle/particles.png 文件，
    """
    try:
        # 定义 particles.png 的路径
        particles_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "particle", "particles.png")
        
        log(f"正在处理 particles.png 在: {particles_path}")
        
        if os.path.exists(particles_path):
            split_particles_image(particles_path)
            log(f"已分割 'particles.png' 在 {particles_path}")
        else:
            log(f"未找到 {temp_dir} 中的 'particles.png'")
    
    except Exception as e:
        log(f"处理 'particles.png' 时出错: {e}")
        traceback.print_exc()

def generate_fish_bucket(temp_dir):
    """
    处理解压后的目录中的 water_bucket.png，
    """
    log(f"Processing water bucket image in: {temp_dir}")
    try:
        items_path_new = os.path.join(temp_dir,'assets','minecraft','textures','item')
        water_bucket_path = os.path.join(temp_dir,'assets','minecraft','textures','item', 'water_bucket.png')
        if os.path.exists(water_bucket_path):
            # 打开 water_bucket.png 并获取其尺寸
            water_bucket_img = Image.open(water_bucket_path).convert("RGBA")
            width, height = water_bucket_img.size
            if width != height:
                log(f"'water_bucket.png' is not a square image ({width}x{height}), skipping water_bucket processing.")
                return

            # 确定 scale_factor
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                # 处理非标准尺寸，通过近似方法确定 scale_factor
                scale_factors = [1, 2, 4, 8]
                closest_scale_factor = min(scale_factors, key=lambda x: abs(x * 256 - width))
                scale_factor = closest_scale_factor
                log(f"Warning: Unsupported image size for 'water_bucket.png': {width}x{height}. Using scale_factor={scale_factor}")

            log(f"Processing 'water_bucket.png', size: {width}x{height}, scale_factor: {scale_factor}")

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            # 定义鱼类类型
            fish_types = ['axolotl', 'cod', 'pufferfish', 'salmon', 'tropical_fish','tadpole']

            # 获取脚本所在目录（假设覆盖图像在与exe同目录的water_bucket文件夹中）
            if getattr(sys, 'frozen', False):
                # 如果是打包后的exe
                script_dir = os.path.dirname(sys.executable)
            else:
                # 如果是脚本运行
                script_dir = os.path.dirname(os.path.abspath(__file__))

            overlay_folder = os.path.join(script_dir, 'water_bucket')

            for fish in fish_types:
                try:
                    # 定义输出桶图像的路径
                    output_bucket_filename = f"{fish}_bucket.png"
                    output_bucket_path = os.path.join(items_path_new, output_bucket_filename)

                    # 复制 water_bucket.png 到新的桶图像
                    shutil.copy(water_bucket_path, output_bucket_path)
                    log(f"Copied 'water_bucket.png' to '{output_bucket_filename}'")

                    # 查找覆盖图像
                    overlay_image_name = f"{fish}_bucket_{width}.png"  # 使用宽度作为size
                    overlay_img_path = os.path.join(overlay_folder, overlay_image_name)

                    if os.path.exists(overlay_img_path):
                        try:
                            # 打开覆盖图像
                            overlay_img = Image.open(overlay_img_path).convert("RGBA")
                            # 打开新的桶图像
                            bucket_img = Image.open(output_bucket_path).convert("RGBA")

                            # 叠加覆盖图像
                            combined_img = Image.alpha_composite(bucket_img, overlay_img)
                            # 保存叠加后的图像
                            combined_img.save(output_bucket_path)
                            log(f"Processed '{output_bucket_filename}' with overlay '{overlay_image_name}'")
                        except Exception as e:
                            log(f"Error overlaying image '{overlay_image_name}' on '{output_bucket_filename}': {e}")
                            traceback.print_exc()
                    else:
                        log(f"Overlay image '{overlay_image_name}' not found in 'water_bucket' directory")
                except Exception as e:
                    log(f"Error processing '{fish}_bucket.png': {e}")
                    traceback.print_exc()

        else:
            log(f"'water_bucket.png' not found in {temp_dir}")
    except Exception as e:
        log(f"Error processing 'water_bucket.png': {e}")
        traceback.print_exc()

def generate_crossbow(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/item 目录下的 bow.png,
    """
    try:
        # 定义 items_path_new
        items_path_new = os.path.join(temp_dir, "assets", "minecraft", "textures", "item")
        log(f"Processing crossbow images in: {items_path_new}")
        
        # 获取 crossbow_dir，假设 crossbow 文件夹在 .exe 同目录下
        if getattr(sys, 'frozen', False):
            # 如果是打包后的 .exe
            application_path = os.path.dirname(sys.executable)
        else:
            # 如果是未打包的脚本
            application_path = os.path.dirname(os.path.abspath(__file__))
        
        crossbow_dir = os.path.join(application_path, 'crossbow')
        log(f"Using crossbow directory: {crossbow_dir}")
        
        # 定义 crossbow_base_path 映射
        size_to_path = {
            (16, 16): os.path.join(crossbow_dir, 'crossbow_16.png'),
            (32, 32): os.path.join(crossbow_dir, 'crossbow_32.png'),
            (64, 64): os.path.join(crossbow_dir, 'crossbow_64.png'),
            (128, 128): os.path.join(crossbow_dir, 'crossbow_128.png'),
            (256, 256): os.path.join(crossbow_dir, 'crossbow_256.png'),
        }
        
        # 处理 crossbow_standby.png 基于 bow.png
        bow_path = os.path.join(items_path_new, 'bow.png')
        if os.path.exists(bow_path):
            log(f"Found 'bow.png' at {bow_path}, processing...")
            bow_img = Image.open(bow_path).convert("RGBA")
            bow_size = bow_img.size
            
            if bow_size in size_to_path:
                crossbow_base_path = size_to_path[bow_size]
                if os.path.exists(crossbow_base_path):
                    log(f"Found crossbow base image at {crossbow_base_path}")
                    crossbow_base_img = Image.open(crossbow_base_path).convert("RGBA")
                    crossbow_standby_img = overlay_images(crossbow_base_img, bow_img, (0, 0))
                    crossbow_standby_output_path = os.path.join(items_path_new, 'crossbow_standby.png')
                    crossbow_standby_img.save(crossbow_standby_output_path)
                    log(f"Created 'crossbow_standby.png' in {items_path_new}")
                else:
                    log(f"No '{crossbow_base_path}' found in {crossbow_dir}")
            else:
                log(f"'bow.png' size is not supported: {bow_size}")
        else:
            log(f"No 'bow.png' found in {items_path_new}")
        
        # 处理 pulling images 和 crossbow_arrow.png 基于 bow_pulling_0.png
        bow_pulling_0_path = os.path.join(items_path_new, 'bow_pulling_0.png')
        if os.path.exists(bow_pulling_0_path):
            log(f"Found 'bow_pulling_0.png' at {bow_pulling_0_path}, processing...")
            bow_pulling_img = Image.open(bow_pulling_0_path).convert("RGBA")
            bow_pulling_size = bow_pulling_img.size
            
            if bow_pulling_size in size_to_path:
                crossbow_base_path = size_to_path[bow_pulling_size]
                if os.path.exists(crossbow_base_path):
                    log(f"Found crossbow base image at {crossbow_base_path}")
                    crossbow_base_img = Image.open(crossbow_base_path).convert("RGBA")
                    crossbow_files = [
                        'crossbow_pulling_0.png',
                        'crossbow_pulling_1.png',
                        'crossbow_pulling_2.png',
                        'crossbow_arrow.png'
                    ]
                    bow_files = [
                        'bow_pulling_0.png',
                        'bow_pulling_1.png',
                        'bow_pulling_2.png',
                        'bow_pulling_2.png'  # 使用 bow_pulling_2.png 作为 crossbow_arrow.png
                    ]
                    
                    for i, bow_file in enumerate(bow_files):
                        bow_file_path = os.path.join(items_path_new, bow_file)
                        if os.path.exists(bow_file_path):
                            log(f"Found '{bow_file}' at {bow_file_path}, processing...")
                            bow_img = Image.open(bow_file_path).convert("RGBA")
                            new_img = overlay_images(crossbow_base_img, bow_img, (0, 0))
                            crossbow_output_path = os.path.join(items_path_new, crossbow_files[i])
                            new_img.save(crossbow_output_path)
                            log(f"Created '{crossbow_files[i]}' in {items_path_new}")
                            
                            # 额外处理 crossbow_arrow.png
                            if crossbow_files[i] == 'crossbow_arrow.png':
                                # 定义路径
                                crossbow_arrow_path = crossbow_output_path
                                crossbow_firework_path = os.path.join(items_path_new, 'crossbow_firework.png')
                                
                                # 步骤1: 复制 crossbow_arrow.png 到 crossbow_firework.png
                                shutil.copyfile(crossbow_arrow_path, crossbow_firework_path)
                                log(f"Copied 'crossbow_arrow.png' to 'crossbow_firework.png' in {items_path_new}")
                                
                                # 步骤2: 叠加 crossbow_firework_{size}.png 到 crossbow_firework.png
                                size = bow_pulling_size[0]  # 假设图像是正方形
                                crossbow_firework_overlay_filename = f'crossbow_firework_{size}.png'
                                crossbow_firework_overlay_path = os.path.join(crossbow_dir, crossbow_firework_overlay_filename)
                                
                                if os.path.exists(crossbow_firework_overlay_path):
                                    log(f"Found '{crossbow_firework_overlay_filename}' at {crossbow_firework_overlay_path}, overlaying...")
                                    crossbow_firework_overlay_img = Image.open(crossbow_firework_overlay_path).convert("RGBA")
                                    crossbow_firework_img = Image.open(crossbow_firework_path).convert("RGBA")
                                    
                                    # 叠加图像
                                    combined_img = overlay_images(crossbow_firework_img, crossbow_firework_overlay_img, (0, 0))
                                    combined_img.save(crossbow_firework_path)
                                    log(f"Created 'crossbow_firework.png' in {items_path_new} by overlaying '{crossbow_firework_overlay_filename}'")
                                else:
                                    log(f"No '{crossbow_firework_overlay_filename}' found in {crossbow_dir}")
                        else:
                            log(f"No '{bow_file}' found in {items_path_new}")
                else:
                    log(f"No '{crossbow_base_path}' found in {crossbow_dir}")
            else:
                log(f"'bow_pulling_0.png' size is not supported: {bow_pulling_size}")
        else:
            log(f"No 'bow_pulling_0.png' found in {items_path_new}")

    except Exception as e:
        log(f"Error processing 'crossbow.png': {e}")
        traceback.print_exc()

def process_chest_folder(temp_dir):
    chest_path= os.path.join(temp_dir, "assets", "minecraft", "textures", "entity","chest")
    chest_files = ['ender.png', 'normal.png', 'trapped.png', 'christmas.png','normal_double.png', 'christmas_double.png','trapped_double.png']
    
    for chest_file in chest_files:
        chest_file_path = os.path.join(chest_path, chest_file)
        if os.path.exists(chest_file_path):
            try:
                img = Image.open(chest_file_path).convert("RGBA")
                width, height = img.size
                log(f"Processing '{chest_file}' with size: {width}x{height}")

                # Determine if the chest is single or double
                if chest_file in ['ender.png', 'normal.png', 'trapped.png', 'christmas.png']:
                    # Single chest image size determination
                    if width == 64 and height == 64:
                        scale_factor = 1
                    elif width == 128 and height == 128:
                        scale_factor = 2
                    elif width == 256 and height == 256:
                        scale_factor = 4
                    elif width == 512 and height == 512:
                        scale_factor = 8
                    elif width == 1024 and height == 1024:
                        scale_factor = 16
                    else:
                        log(f"Unsupported image size for '{chest_file}': {width}x{height}")
                        continue

                    def scaled_box(x1, y1, x2, y2):
                        return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

                    # Process single chests
                    swap_and_mirror(img, scaled_box(14, 0, 28, 14), scaled_box(28, 0, 42, 14))
                    swap_and_mirror(img, scaled_box(14, 14, 28, 19), scaled_box(42, 14, 56, 19))
                    swap_and_mirror(img, scaled_box(14, 19, 28, 33), scaled_box(28, 19, 42, 33))
                    swap_and_mirror(img, scaled_box(14, 33, 28, 43), scaled_box(42, 33, 56, 43))

                    mirror_boxes = [
                        scaled_box(14, 0, 28, 14), scaled_box(28, 0, 42, 14),
                        scaled_box(0, 14, 14, 19), scaled_box(28, 14, 42, 19), 
                        scaled_box(14, 19, 28, 33), scaled_box(28, 19, 42, 33),
                        scaled_box(0, 33, 14, 43), scaled_box(28, 33, 42, 43)
                    ]
                    for box in mirror_boxes:
                        region = img.crop(box).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                        img.paste(region, box)

                    img.save(chest_file_path)
                    log(f"Processed '{chest_file}' and swapped and mirrored specified regions.")

                elif chest_file in ['normal_double.png', 'trapped_double.png', 'christmas_double.png']:
                    # Double chest image size determination
                    if width == 128 and height == 64:
                        scale_factor = 1
                    elif width == 256 and height == 128:
                        scale_factor = 2
                    elif width == 512 and height == 256:
                        scale_factor = 4
                    elif width == 1024 and height == 512:
                        scale_factor = 8
                    else:
                        log(f"Unsupported image size for '{chest_file}': {width}x{height}")
                        continue
                    
                    def scaled_box(x1, y1, x2, y2):
                        return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

                    # Create transparent images for left and right parts based on scale_factor
                    left_img_size = (64 * scale_factor, 64 * scale_factor)
                    right_img_size = (64 * scale_factor, 64 * scale_factor)
                    left_img = Image.new("RGBA", left_img_size, (0, 0, 0, 0))
                    right_img = Image.new("RGBA", right_img_size, (0, 0, 0, 0))

                    # Determine the prefix based on the chest type
                    prefix = 'christmas' if 'christmas' in chest_file else ('normal' if 'normal' in chest_file else 'trapped')

                    # Generate double chest images
                    generate_double_chest_images(left_img, right_img, prefix, img, scaled_box, scale_factor)

                    # Save the processed left and right images with the correct prefix
                    left_img.save(os.path.join(chest_path, f"{prefix}_left.png"))
                    right_img.save(os.path.join(chest_path, f"{prefix}_right.png"))
                    log(f"Processed '{chest_file}' and saved '{prefix}_left.png' and '{prefix}_right.png'.")

            except Exception as e:
                log(f"Error processing '{chest_file}': {e}")
                continue

    log("Chest images processing completed.")

def generate_netherite_block(temp_dir):
    log(f"Processing diamond_block image in: {temp_dir}")
    try:
        diamond_block_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'block', 'diamond_block.png')
        netherite_block_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'block', 'netherite_block.png')
        
        if os.path.exists(diamond_block_path):
            # 复制 diamond_block.png 并处理
            shutil.copy(diamond_block_path, netherite_block_path)
            img = Image.open(netherite_block_path).convert("RGBA")
            
            new_image_data = []
            for item in img.getdata():
                if item[3] == 0:
                    new_image_data.append(item)
                else:
                    hsva = rgba_to_hsv(item)
                    new_hue = 310 / 360.0  # 色相改为恒定310
                    new_saturation = hsva[1] / 3
                    new_value = hsva[2] / 3
                    new_image_data.append(hsv_to_rgba((new_hue, new_saturation, new_value, hsva[3])))
            new_image = Image.new("RGBA", img.size)
            new_image.putdata(new_image_data)
            new_image.save(netherite_block_path)
            log(f"Processed 'diamond_block.png' to 'netherite_block.png' with adjusted HSV values")

            # 复制 diamond_block.png.mcmeta 并重命名
            diamond_block_mcmeta_path = diamond_block_path + '.mcmeta'
            netherite_block_mcmeta_path = netherite_block_path + '.mcmeta'
            if os.path.exists(diamond_block_mcmeta_path):
                shutil.copy(diamond_block_mcmeta_path, netherite_block_mcmeta_path)
                log(f"Copied and renamed 'diamond_block.png.mcmeta' to 'netherite_block.png.mcmeta'")
        else:
            log(f"No 'diamond_block.png' found in {temp_dir}")
    except Exception as e:
        log(f"Error processing 'diamond_block.png': {e}")
        traceback.print_exc()

def generate_netherite_ingot(temp_dir):
    log(f"Processing gold_ingot image in: {temp_dir}")
    try:
        gold_ingot_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'item', 'gold_ingot.png')
        netherite_ingot_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'item', 'netherite_ingot.png')
        
        if os.path.exists(gold_ingot_path):
            # 复制 gold_ingot.png 并处理
            shutil.copy(gold_ingot_path, netherite_ingot_path)
            img = Image.open(netherite_ingot_path).convert("RGBA")
            
            new_image_data = []
            for item in img.getdata():
                if item[3] == 0:
                    new_image_data.append(item)
                else:
                    hsva = rgba_to_hsv(item)
                    new_hue = 310 / 360.0  # 色相改为恒定310
                    new_saturation = hsva[1] / 3
                    new_value = hsva[2] / 3
                    new_image_data.append(hsv_to_rgba((new_hue, new_saturation, new_value, hsva[3])))
            new_image = Image.new("RGBA", img.size)
            new_image.putdata(new_image_data)
            new_image.save(netherite_ingot_path)
            log(f"Processed 'gold_ingot.png' to 'netherite_ingot.png' with adjusted HSV values")

            # 复制 gold_ingot.png.mcmeta 并重命名
            gold_ingot_mcmeta_path = gold_ingot_path + '.mcmeta'
            netherite_ingot_mcmeta_path = netherite_ingot_path + '.mcmeta'
            if os.path.exists(gold_ingot_mcmeta_path):
                shutil.copy(gold_ingot_mcmeta_path, netherite_ingot_mcmeta_path)
                log(f"Copied and renamed 'gold_ingot.png.mcmeta' to 'netherite_ingot.png.mcmeta'")
        else:
            log(f"No 'gold_ingot.png' found in {temp_dir}")
    except Exception as e:
        log(f"Error processing 'gold_ingot.png': {e}")
        traceback.print_exc()


def generate_copper_ingot(temp_dir):
    log(f"Processing iron_ingot image in: {temp_dir}")
    try:
        iron_ingot_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'item', 'iron_ingot.png')
        copper_ingot_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'item', 'copper_ingot.png')
        
        if os.path.exists(iron_ingot_path):
            # 复制 iron_ingot.png 并处理
            shutil.copy(iron_ingot_path, copper_ingot_path)
            img = Image.open(copper_ingot_path).convert("RGBA")
            
            # 使用专门的铜材质颜色调整函数，确保效果明显
            img = adjust_copper_color(img)
            img.save(copper_ingot_path)
            log(f"Processed 'iron_ingot.png' to 'copper_ingot.png' with brass color")

            # 复制 iron_ingot.png.mcmeta 并重命名
            iron_ingot_mcmeta_path = iron_ingot_path + '.mcmeta'
            copper_ingot_mcmeta_path = copper_ingot_path + '.mcmeta'
            if os.path.exists(iron_ingot_mcmeta_path):
                shutil.copy(iron_ingot_mcmeta_path, copper_ingot_mcmeta_path)
                log(f"Copied and renamed 'iron_ingot.png.mcmeta' to 'copper_ingot.png.mcmeta'")
        else:
            log(f"No 'iron_ingot.png' found in {temp_dir}")
    except Exception as e:
        log(f"Error processing 'iron_ingot.png': {e}")
        traceback.print_exc()


def generate_copper_block(temp_dir):
    log(f"Processing iron_block images in: {temp_dir}")
    try:
        blocks_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'block')
        iron_block_path = os.path.join(blocks_path, 'iron_block.png')
        
        if os.path.exists(iron_block_path):
            width, height = Image.open(iron_block_path).size
            
            # 定义不同氧化阶段的颜色（按照氧化程度依次变青）
            colors = {
                'copper': None,  # 使用默认的铜色调整函数
                'exposed': (100, 180, 160, 255),  # 轻度氧化 - 浅青绿色
                'weathered': (70, 190, 180, 255),  # 中度氧化 - 中等青色
                'oxidized': (50, 210, 210, 255)   # 重度氧化 - 纯青色
            }
            
            # 1. 普通铜方块 - 使用铜色调整函数
            copper_block_path = os.path.join(blocks_path, 'copper_block.png')
            shutil.copy(iron_block_path, copper_block_path)
            copper_img = Image.open(copper_block_path).convert("RGBA")
            
            # 使用专门的铜材质颜色调整函数
            copper_img = adjust_copper_color(copper_img)
            copper_img.save(copper_block_path)
            log(f"Generated 'copper_block.png' with copper color")
            
            # 2. 轻度氧化铜方块 - 浅青绿色调，保留纹理细节
            exposed_copper_path = os.path.join(blocks_path, 'exposed_copper.png')
            shutil.copy(iron_block_path, exposed_copper_path)
            exposed_img = Image.open(exposed_copper_path).convert("RGBA")
            exposed_pixels = exposed_img.load()
            
            # 首先应用铜色调整
            exposed_img = adjust_copper_color(exposed_img)
            
            # 然后将整体颜色调整为浅青绿色
            for y in range(height):
                for x in range(width):
                    r, g, b, a = exposed_pixels[x, y]
                    if a > 0:
                        # 应用浅青绿色调
                        new_r = int(r * 0.8 + colors['exposed'][0] * 0.2)
                        new_g = int(g * 0.7 + colors['exposed'][1] * 0.3)
                        new_b = int(b * 0.6 + colors['exposed'][2] * 0.4)
                        exposed_pixels[x, y] = (min(255, new_r), min(255, new_g), min(255, new_b), a)
            
            exposed_img.save(exposed_copper_path)
            log(f"Generated 'exposed_copper.png' with light cyan-green color")
            
            # 3. 中度氧化铜方块 - 中等青色，纹理细节较少
            weathered_copper_path = os.path.join(blocks_path, 'weathered_copper.png')
            shutil.copy(iron_block_path, weathered_copper_path)
            weathered_img = Image.open(weathered_copper_path).convert("RGBA")
            weathered_pixels = weathered_img.load()
            
            # 首先应用铜色调整
            weathered_img = adjust_copper_color(weathered_img)
            
            # 然后将整体颜色调整为中等青色
            for y in range(height):
                for x in range(width):
                    r, g, b, a = weathered_pixels[x, y]
                    if a > 0:
                        # 应用中等青色调
                        new_r = int(r * 0.6 + colors['weathered'][0] * 0.4)
                        new_g = int(g * 0.5 + colors['weathered'][1] * 0.5)
                        new_b = int(b * 0.4 + colors['weathered'][2] * 0.6)
                        weathered_pixels[x, y] = (min(255, new_r), min(255, new_g), min(255, new_b), a)
            
            weathered_img.save(weathered_copper_path)
            log(f"Generated 'weathered_copper.png' with medium cyan color")
            
            # 4. 重度氧化铜方块 - 纯青色，只有一个颜色
            oxidized_copper_path = os.path.join(blocks_path, 'oxidized_copper.png')
            shutil.copy(iron_block_path, oxidized_copper_path)
            oxidized_img = Image.open(oxidized_copper_path).convert("RGBA")
            oxidized_pixels = oxidized_img.load()
            
            # 直接设置为纯青色，不保留纹理细节
            for y in range(height):
                for x in range(width):
                    r, g, b, a = oxidized_pixels[x, y]
                    if a > 0:
                        oxidized_pixels[x, y] = colors['oxidized']  # 纯青色
            
            oxidized_img.save(oxidized_copper_path)
            log(f"Generated 'oxidized_copper.png' with full pure cyan color")
            
            # 复制 .mcmeta 文件（如果存在）
            iron_block_mcmeta_path = iron_block_path + '.mcmeta'
            if os.path.exists(iron_block_mcmeta_path):
                shutil.copy(iron_block_mcmeta_path, copper_block_path + '.mcmeta')
                shutil.copy(iron_block_mcmeta_path, exposed_copper_path + '.mcmeta')
                shutil.copy(iron_block_mcmeta_path, weathered_copper_path + '.mcmeta')
                shutil.copy(iron_block_mcmeta_path, oxidized_copper_path + '.mcmeta')
                log(f"Copied and renamed 'iron_block.png.mcmeta' for all copper block variants")
        else:
            log(f"No 'iron_block.png' found in {temp_dir}")
    except Exception as e:
        log(f"Error processing 'iron_block.png': {e}")
        traceback.print_exc()

def generate_copper_tools(temp_dir):
    log(f"Processing and copying copper tools/items in: {temp_dir}")
    items_path_new = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'item')
    items_to_copy_and_process = [
        'iron_sword', 'iron_helmet', 'iron_chestplate', 'iron_leggings', 'iron_boots',
        'iron_axe', 'iron_pickaxe', 'iron_shovel', 'iron_hoe', 'iron_horse_armor'
    ]
    # 备选材质列表，按优先级排序
    # 注意：不同材质的命名可能有所不同，需要确保使用正确的文件名格式
    alternative_materials = ['diamond', 'golden', 'stone', 'netherite']
    
    for item in items_to_copy_and_process:
        try:
            original_path = os.path.join(items_path_new, f'{item}.png')
            new_path = os.path.join(items_path_new, f'copper_{item[5:]}.png')
            
            # 检查原始铁材质是否存在
            if os.path.exists(original_path):
                shutil.copy(original_path, new_path)
                log(f"Copied and renamed '{item}.png' to 'copper_{item[5:]}.png'")
            else:
                log(f"'{item}.png' does not exist, trying alternative materials...")
                # 尝试使用备选材质
                found_alternative = False
                for material in alternative_materials:
                    # 根据材质类型使用正确的文件名格式
                    if material == 'golden':
                        alt_material = 'gold'  # 金色工具使用'gold'而不是'golden'
                    else:
                        alt_material = material
                    
                    alt_item = f'{alt_material}_{item[5:]}'
                    alt_path = os.path.join(items_path_new, f'{alt_item}.png')
                    if os.path.exists(alt_path):
                        shutil.copy(alt_path, new_path)
                        log(f"Copied and renamed '{alt_item}.png' to 'copper_{item[5:]}.png' as alternative")
                        found_alternative = True
                        break
                
                if not found_alternative:
                    log(f"No suitable alternative found for '{item}.png', skipping...")
                    continue
            
            # 使用铜材质专用的颜色调整函数
            img = Image.open(new_path).convert("RGBA")
            img = adjust_copper_color(img)
            img.save(new_path)
            log(f"Processed copper image 'copper_{item[5:]}.png'")

            # 复制mcmeta文件（如果存在）
            original_meta_path = original_path + '.mcmeta'
            new_meta_path = new_path + '.mcmeta'
            if os.path.exists(original_meta_path):
                shutil.copy(original_meta_path, new_meta_path)
                log(f"Copied and renamed '{item}.png.mcmeta' to 'copper_{item[5:]}.png.mcmeta'")

        except Exception as e:
            log(f"Error processing and copying copper item '{item}': {e}")
            traceback.print_exc()


def generate_copper_armor_models(temp_dir):
    log(f"Processing copper armor layers in: {temp_dir}")
    armor_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'models', 'armor')
    armor_files = ['iron_layer_1.png', 'iron_layer_2.png']
    # 备选材质列表，按优先级排序
    # 注意：不同材质的命名可能有所不同，需要确保使用正确的文件名格式
    alternative_materials = ['diamond', 'golden', 'chainmail', 'leather']
    
    for armor_file in armor_files:
        try:
            original_path = os.path.join(armor_path, armor_file)
            new_path = os.path.join(armor_path, armor_file.replace('iron', 'copper'))
            
            # 检查原始铁材质是否存在
            if os.path.exists(original_path):
                shutil.copy(original_path, new_path)
                log(f"Copied and renamed '{armor_file}' to '{armor_file.replace('iron', 'copper')}'")
            else:
                log(f"'{original_path}' does not exist, trying alternative materials...")
                # 尝试使用备选材质
                found_alternative = False
                for material in alternative_materials:
                    # 根据材质类型使用正确的文件名格式
                    if material == 'golden':
                        alt_file = armor_file.replace('iron', 'gold')  # 金色盔甲使用'gold'而不是'golden'
                    else:
                        alt_file = armor_file.replace('iron', material)
                    
                    alt_path = os.path.join(armor_path, alt_file)
                    if os.path.exists(alt_path):
                        shutil.copy(alt_path, new_path)
                        log(f"Copied and renamed '{alt_file}' to '{armor_file.replace('iron', 'copper')}' as alternative")
                        found_alternative = True
                        break
                
                if not found_alternative:
                    log(f"No suitable alternative found for '{armor_file}', skipping...")
                    continue
            
            # 使用铜材质专用的颜色调整函数
            img = Image.open(new_path).convert("RGBA")
            img = adjust_copper_color(img)
            img.save(new_path)
            log(f"Processed copper armor image '{armor_file.replace('iron', 'copper')}'")
        except Exception as e:
            log(f"Error processing and copying copper armor layer '{armor_file}': {e}")
            traceback.print_exc()


def delete_enchanted_item_glint(temp_dir):
    log(f"Deleting enchanted item glint in: {temp_dir}")
    enchanted_item_glint_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'misc', 'enchanted_item_glint.png')
    if os.path.exists(enchanted_item_glint_path):
        os.remove(enchanted_item_glint_path)
        log(f"Deleted 'enchanted_item_glint.png' from {temp_dir}")
    else:
        log(f"No 'enchanted_item_glint.png' found in {temp_dir}")

# Function to process and copy items
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
                    hsva = rgba_to_hsv(item)
                    new_hue = 60 / 360.0  # 将 H 值改为 60
                    new_saturation = hsva[1]
                    if hsva[1] == 0:  # 如果 S 值为 0，则将其提高 60
                        new_saturation = min(1, new_saturation + 60 / 100.0)
                    new_image_data.append(hsv_to_rgba((new_hue, new_saturation, hsva[2], hsva[3])))
            new_image = Image.new("RGBA", img.size)
            new_image.putdata(new_image_data)
            new_image.save(spectral_arrow_path)
            log(f"Processed 'arrow.png' to 'spectral_arrow.png' with H value changed to 60 and S value adjusted if it was 0")

            # 复制 arrow.png.mcmeta 并重命名为 spectral_arrow.png.mcmeta（如果存在）
            arrow_mcmeta_path = arrow_path + '.mcmeta'
            spectral_arrow_mcmeta_path = spectral_arrow_path + '.mcmeta'
            if os.path.exists(arrow_mcmeta_path):
                shutil.copy(arrow_mcmeta_path, spectral_arrow_mcmeta_path)
                log(f"Copied and renamed 'arrow.png.mcmeta' to 'spectral_arrow.png.mcmeta'")

        else:
            log(f"No 'arrow.png' found in {items_path_new}")
    except Exception as e:
        log(f"Error processing 'arrow.png': {e}")
        traceback.print_exc()

def generate_netherite_armor_models(temp_dir):
    log(f"Processing armor layers in: {temp_dir}")
    armor_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'models', 'armor')
    armor_files = ['diamond_layer_1.png', 'diamond_layer_2.png']
    for armor_file in armor_files:
        try:
            original_path = os.path.join(armor_path, armor_file)
            new_path = os.path.join(armor_path, armor_file.replace('diamond', 'netherite'))
            log(f"Checking if {original_path} exists.")
            if os.path.exists(original_path):
                shutil.copy(original_path, new_path)
                log(f"Copied and renamed '{armor_file}' to '{armor_file.replace('diamond', 'netherite')}'")
                process_image(new_path)
                log(f"Processed image '{armor_file.replace('diamond', 'netherite')}'")
            else:
                log(f"'{original_path}' does not exist.")
        except Exception as e:
            log(f"Error processing and copying armor layer '{armor_file}': {e}")
            traceback.print_exc()

def generate_snow_bucket(temp_dir):
    try:
        milk_bucket_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'item', 'milk_bucket.png')
        if os.path.exists(milk_bucket_path):
            # Open milk_bucket.png and get its size
            milk_bucket_img = Image.open(milk_bucket_path).convert("RGBA")
            width, height = milk_bucket_img.size
            if width != height:
                log(f"'milk_bucket.png' is not a square image, skipping powder_snow_bucket processing.")
                return

            # Copy milk_bucket.png to powder_snow_bucket.png
            powder_snow_bucket_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'item', 'powder_snow_bucket.png')
            shutil.copy(milk_bucket_path, powder_snow_bucket_path)

            # Find the overlay image
            size = width  # since width == height
            overlay_filename = f"powder_snow_bucket_{size}.png"
            overlay_dir = os.path.join(os.getcwd(), 'powder_snow_bucket')
            overlay_path = os.path.join(overlay_dir, overlay_filename)

            if os.path.exists(overlay_path):
                # Open the overlay image
                overlay_img = Image.open(overlay_path).convert("RGBA")
                # Open the powder_snow_bucket.png image
                bucket_img = Image.open(powder_snow_bucket_path).convert("RGBA")

                # Overlay the images
                combined_img = Image.alpha_composite(bucket_img, overlay_img)
                combined_img.save(powder_snow_bucket_path)
                log(f"Processed 'powder_snow_bucket.png' with overlay '{overlay_filename}'")
            else:
                log(f"Overlay image '{overlay_filename}' not found in 'powder_snow_bucket' directory")
        else:
            log(f"'milk_bucket.png' not found in {temp_dir}")
    except Exception as e:
        log(f"Error processing 'powder_snow_bucket.png': {e}")
        traceback.print_exc()

def rename_mcpatcher_to_optifine(temp_dir):
    try:
        # 重命名 mcpatcher 文件夹为 optifine 文件夹
        mcpatcher_path = os.path.join(temp_dir, 'assets', 'minecraft', 'mcpatcher')
        optifine_path = os.path.join(temp_dir, 'assets', 'minecraft', 'optifine')

        if os.path.exists(mcpatcher_path):
            if not os.path.exists(optifine_path):
                os.rename(mcpatcher_path, optifine_path)
                log(f"已将 'mcpatcher' 重命名为 'optifine' 在 {os.path.join(temp_dir, 'assets', 'minecraft')}")
            else:
                log(f"跳过重命名，因为 'optifine' 文件夹已存在于 {os.path.join(temp_dir, 'assets', 'minecraft')}")
        else:
            log(f"未找到 'mcpatcher' 文件夹在 {os.path.join(temp_dir, 'assets', 'minecraft')}")
    except Exception as e:
        log(f"Error processing 'mcpatcher': {e}")
        traceback.print_exc()

def generate_smithing_ui(temp_dir):
    log(f"Processing smithing image in: {temp_dir}")
    try:
        anvil_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container', 'anvil.png')
        smithing_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container', 'smithing.png')

        if os.path.exists(anvil_path):
            img = Image.open(anvil_path).convert("RGBA")
            width, height = img.size

            if width == 256 and height == 256:
                scale_factor = 1
                smithing_image_path = os.path.join('smithing', 'smithing_256.png')
            elif width == 512 and height == 512:
                scale_factor = 2
                smithing_image_path = os.path.join('smithing', 'smithing_512.png')
            elif width == 1024 and height == 1024:
                scale_factor = 4
                smithing_image_path = os.path.join('smithing', 'smithing_1024.png')
            elif width == 2048 and height == 2048:
                scale_factor = 8
                smithing_image_path = os.path.join('smithing', 'smithing_2048.png')
            else:
                log(f"Unsupported image size for 'anvil.png': {width}x{height}")
                return

            img_copy = img.copy()

            # 定义覆盖区域并填充颜色
            cover_box = (10 * scale_factor, 5 * scale_factor, 169 * scale_factor, 37 * scale_factor)
            fill_color = img.getpixel((5 * scale_factor, 4 * scale_factor))  # 获取 (5,4) 的像素颜色

            log(f"Filling region {cover_box} with color {fill_color}")
            for x in range(cover_box[0], cover_box[2]):
                for y in range(cover_box[1], cover_box[3]):
                    img_copy.putpixel((x, y), fill_color)

            # 覆盖图像（如果存在）
            if os.path.exists(smithing_image_path):
                overlay_img = Image.open(smithing_image_path).convert("RGBA")
                img_copy.paste(overlay_img, (0, 0), overlay_img)
                log(f"Overlayed {smithing_image_path} onto smithing.png")
            else:
                log(f"No overlay image found for size {width}x{height}")

            # 新步骤1：将 (0,166)-(110,198) 的区域设置为全透明（根据 scale_factor 缩放）
            try:
                transparent_box = (
                    0 * scale_factor,
                    166 * scale_factor,
                    110 * scale_factor,
                    198 * scale_factor
                )
                log(f"Setting region {transparent_box} to fully transparent in grindstone.png")
                # 创建一个全透明的图像
                transparent_region = Image.new('RGBA', (transparent_box[2] - transparent_box[0], 
                                                      transparent_box[3] - transparent_box[1]), 
                                               (0, 0, 0, 0))
                # 粘贴到指定区域
                img_copy.paste(transparent_region, (transparent_box[0], transparent_box[1]))
                log(f"Region {transparent_box} set to transparent")
            except Exception as e:
                log(f"Error setting transparency in 'grindstone.png': {e}")
                traceback.print_exc()

            # 保存生成的 smithing.png
            try:
                img_copy.save(smithing_path)
                log(f"Processed 'anvil.png' and saved as 'smithing.png'")
            except Exception as e:
                log(f"Error saving 'smithing.png': {e}")
                return
        else:
            log(f"No 'anvil.png' found in {temp_dir}")

    except Exception as e:
        log(f"Error processing smithing image in '{temp_dir}': {e}") 
        traceback.print_exc()







def generate_redwood_cherry_bamboo_planks(temp_dir):
    blocks_path_new = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'block')
    process_block_image(blocks_path_new, 'oak_planks.png', 'mangrove_planks.png', hue_shift=-59, brightness_adjust=-15, saturation_adjust=0)
    process_block_image(blocks_path_new, 'oak_planks.png', 'cherry_planks.png', hue_shift=-80, brightness_adjust=40, saturation_adjust=0)
    process_block_image(blocks_path_new, 'oak_planks.png', 'bamboo_planks.png', hue_shift=25, brightness_adjust=20, saturation_adjust=0)

def fix_tabs(temp_dir):
    """
    处理creative_inventory_path目录中的tabs.png，
    """
    log(f"Processing tabs image: {temp_dir}")
    try:
        creative_inventory_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container', 'creative_inventory')
        tabs_path = os.path.join(creative_inventory_path, 'tabs.png')
        if not os.path.exists(tabs_path):
            log(f"'tabs.png' not found in {creative_inventory_path}")
            return

        img = Image.open(tabs_path).convert("RGBA")
        width, height = img.size

        # 确定 scale_factor
        scale_factor, is_exact = determine_scale_factor(width, height)

        log(f"Determined scale_factor: {scale_factor} (Exact match: {is_exact})")

        def scaled_coords(x1, y1, x2, y2):
            return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

        def scaled_point(x, y):
            return (x * scale_factor, y * scale_factor)

        pixels = img.load()

        # Step 1: 将 (168,0)-(196,128) 向右平移14个像素
        move_region = scaled_coords(168, 0, 196, 128)
        move_right = 14 * scale_factor
        move_dest_box = (move_region[0] + move_right, move_region[1], move_region[2] + move_right, move_region[3])

        log(f"Moving region {move_region} to {move_dest_box}")
        region_to_move = img.crop(move_region)
        img.paste(region_to_move, move_dest_box)
        log(f"Moved region {move_region} to {move_dest_box}")

        # Step 2: 平移指定区域
        # 定义需要平移的区域及其平移像素数 (以256x256为例)
        shift_operations = [
            ((15, 0, 41, 128), 2),
            ((43, 0, 69, 128), 4),
            ((71, 0, 97, 128), 6),
            ((99, 0, 125, 128), 8),
            ((127, 0, 153, 128), 10),
            ((155, 0, 168, 128), 12)
        ]

        # 根据 scale_factor 进行缩放
        scaled_shift_operations = [ (scaled_coords(x1, y1, x2, y2), shift * scale_factor) for ((x1, y1, x2, y2), shift) in shift_operations ]

        log(f"Shifting specified regions: {scaled_shift_operations}")
        for (shift_source_box, shift_pixels) in scaled_shift_operations:
            x1, y1, x2, y2 = shift_source_box
            shift_dest_box = (x1 - shift_pixels, y1, x2 - shift_pixels, y2)

            log(f"Shifting region {shift_source_box} to {shift_dest_box} by {shift_pixels} pixels left")
            region_to_shift = img.crop(shift_source_box)
            img.paste(region_to_shift, shift_dest_box)
            log(f"Shifted region {shift_source_box} to {shift_dest_box}")

        # Step 3: 复制 (0,0)-(26,128) 到 (156,0)-(182,128)
        copy_region = scaled_coords(0, 0, 26, 128)
        paste_position = scaled_point(156, 0)  # (156,0)
        log(f"Copying region {copy_region} to {paste_position}")
        region_copy = img.crop(copy_region)
        img.paste(region_copy, paste_position)
        log(f"Copied region {copy_region} to {paste_position}")

        # 保存修改后的图像，覆盖原图
        img.save(tabs_path)
        log(f"Processed 'tabs.png' and saved the modified image")

    except Exception as e:
        log(f"Error processing 'tabs.png' in '{creative_inventory_path}': {e}")
        traceback.print_exc()

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

def fix_smithing2_villager2_ui(temp_dir):
    container_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container')
    log(f"Processing smithing2 image in: {container_path}")
    try:
        anvil_path = os.path.join(container_path, 'anvil.png')
        smithing2_new_path = os.path.join(container_path, 'smithing.png')

        # 获取 crossbow_dir，假设 crossbow 文件夹在 .exe 同目录下
        if getattr(sys, 'frozen', False):
            # 如果是打包后的 .exe
            application_path = os.path.dirname(sys.executable)
        else:
            # 如果是未打包的脚本
            application_path = os.path.dirname(os.path.abspath(__file__))
        
        smithing2_path = os.path.join(application_path, 'smithing2')
        
        if os.path.exists(anvil_path):
            img = Image.open(anvil_path).convert("RGBA")
            width, height = img.size

            if width == 256 and height == 256:
                scale_factor = 1
                smithing2_image_path = os.path.join('smithing2', 'smithing2_256.png')
            elif width == 512 and height == 512:
                scale_factor = 2
                smithing2_image_path = os.path.join('smithing2', 'smithing2_512.png')
            elif width == 1024 and height == 1024:
                scale_factor = 4
                smithing2_image_path = os.path.join('smithing2', 'smithing2_1024.png')
            elif width == 2048 and height == 2048:
                scale_factor = 8
                smithing2_image_path = os.path.join('smithing2', 'smithing2_2048.png')
            else:
                log(f"Unsupported image size for 'anvil.png': {width}x{height}")
                return

            img_copy = img.copy()
            cover_box = (5 * scale_factor, 5 * scale_factor, 171 * scale_factor, 72 * scale_factor)
            fill_color = img.getpixel((5 * scale_factor, 4 * scale_factor))
            
            for x in range(cover_box[0], cover_box[2]):
                for y in range(cover_box[1], cover_box[3]):
                    img_copy.putpixel((x, y), fill_color)
                    
            region = img_copy.crop((7 * scale_factor, 83 * scale_factor, 25 * scale_factor, 101 * scale_factor))
            img_copy.paste(region, (7 * scale_factor, 47 * scale_factor))
            img_copy.paste(region, (25 * scale_factor, 47 * scale_factor))
            img_copy.paste(region, (43 * scale_factor, 47 * scale_factor))
            img_copy.paste(region, (97 * scale_factor, 47 * scale_factor))
            
            if os.path.exists(smithing2_image_path):
                overlay_img = Image.open(smithing2_image_path).convert("RGBA")
                img_copy.paste(overlay_img, (0, 0), overlay_img)
                img_copy.save(smithing2_new_path)
                log(f"Processed 'anvil.png' and saved as 'smithing.png'")
            else:
                log(f"No overlay image found for size {width}x{height}")
        else:
            log(f"No 'anvil.png' found in {container_path}")

        villager_path = os.path.join(container_path, 'villager.png')
        if os.path.exists(villager_path):
            img = Image.open(villager_path).convert("RGBA")
            width, height = img.size

            # 确定 scale_factor
            scale_factor, is_exact = determine_scale_factor(width, height)

            log(f"Processing villager.png, size: {width}x{height}, scale_factor: {scale_factor} (Exact match: {is_exact})")

            def scaled_box(x1, y1, x2, y2):
                """
    根据 scale_factor 缩放裁剪坐标
    """
                return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

            def scaled_point(x, y):
                """
    根据 scale_factor 缩放单点坐标
    """
                return (x * scale_factor, y * scale_factor)

            # 定义新图像的尺寸（宽度翻倍，保持高度不变）
            new_width = width * 2
            new_height = height
            temp_villager_path = os.path.join(container_path, 'villager_temp.png')  # 临时保存路径

            # 创建一个新的透明图像
            villager2_img = Image.new('RGBA', (new_width, new_height), (0, 0, 0, 0))
            log(f"Created new transparent image: {temp_villager_path}, size: {villager2_img.size}")

            # 裁剪原图并粘贴到新图像
            crop_region = scaled_box(0, 0, 240, 166)
            paste_position = (100 * scale_factor, 0)  # (100, 0) scaled
            try:
                cropped_img = img.crop(crop_region)
                villager2_img.paste(cropped_img, paste_position)
                log(f"Pasted cropped region {crop_region} to {paste_position}")
            except Exception as e:
                log(f"Error cropping and pasting regions: {e}")
                traceback.print_exc()
                return

            # 获取脚本所在目录（假设覆盖图像在与exe同目录的villager2文件夹中）
            if getattr(sys, 'frozen', False):
                # 如果是打包后的exe
                script_dir = os.path.dirname(sys.executable)
            else:
                # 如果是脚本运行
                script_dir = os.path.dirname(os.path.abspath(__file__))
            
            overlay_folder = os.path.join(script_dir, 'villager2')
            overlay_image_name = f'villager2_{256 * scale_factor}.png'
            overlay_img_path = os.path.join(overlay_folder, overlay_image_name)

            if os.path.exists(overlay_img_path):
                try:
                    overlay_img = Image.open(overlay_img_path).convert("RGBA")
                    villager2_img.paste(overlay_img, (0, 0), overlay_img)
                    log(f"Overlayed {overlay_image_name} onto temp_villager.png")
                except Exception as e:
                    log(f"Error overlaying image {overlay_image_name}: {e}")
                    traceback.print_exc()
                    return
            else:
                log(f"No overlay image found: {overlay_img_path}")

            # 新步骤1：覆盖颜色区域 (185,17)-(186,18) -> (186,24)-(208,39)
            try:
                # 定义源颜色区域 (185,17)-(186,18) scaled
                source_box = scaled_box(185, 17, 186, 18)  # (185,17)-(186,18), 1x1 像素

                # 获取源颜色
                source_color = villager2_img.getpixel((185 * scale_factor, 17 * scale_factor))
                log(f"Color at (185,17) scaled: {source_color}")

                # 定义覆盖区域 (186,24)-(208,39) scaled
                cover_box = scaled_box(186, 24, 208, 39)
                cover_x1, cover_y1, cover_x2, cover_y2 = cover_box

                cover_width = cover_x2 - cover_x1
                cover_height = cover_y2 - cover_y1

                # 创建一个单色图像用于覆盖
                cover_img = Image.new('RGBA', (cover_width, cover_height), source_color)

                # 粘贴覆盖图像到 villager2_img
                villager2_img.paste(cover_img, (cover_x1, cover_y1))
                log(f"Covered region {cover_box} in villager2_img with color {source_color}")
            except Exception as e:
                log(f"Error covering region (186,24)-(208,39) in 'villager2_img': {e}")
                traceback.print_exc()

            # 新步骤2：移动矩形区域 (133,48)-(242,76) 向上平移16个像素
            try:
                # 定义原始区域 (133,48)-(242,76) scaled
                original_region = scaled_box(133, 48, 242, 76)
                orig_x1, orig_y1, orig_x2, orig_y2 = original_region

                # 定义目标区域，向上平移16个像素 scaled
                shift_y = 16 * scale_factor
                target_position = (orig_x1, orig_y1 - shift_y)

                # 裁剪原始区域
                cropped_move = villager2_img.crop(original_region)

                # 粘贴到目标区域
                villager2_img.paste(cropped_move, target_position)
                log(f"Moved region {original_region} to {target_position} in villager2_img")
            except Exception as e:
                log(f"Error moving region (133,48)-(242,76) in 'villager2_img': {e}")
                traceback.print_exc()

            # 新步骤3：覆盖颜色区域 (132,60)-(133,61) -> (133,60)-(242,76)
            try:
                # 定义源颜色区域 (132,60)-(133,61) scaled
                source_box_2 = scaled_box(132, 60, 133, 61)  # (132,60)-(133,61), 1x1 像素

                # 获取源颜色
                source_color_2 = villager2_img.getpixel((132 * scale_factor, 60 * scale_factor))
                log(f"Color at (132,60) scaled: {source_color_2}")

                # 定义覆盖区域 (133,60)-(242,76) scaled
                cover_box_2 = scaled_box(133, 60, 242, 76)
                cover_x1_2, cover_y1_2, cover_x2_2, cover_y2_2 = cover_box_2

                cover_width_2 = cover_x2_2 - cover_x1_2
                cover_height_2 = cover_y2_2 - cover_y1_2

                # 创建一个单色图像用于覆盖
                cover_img_2 = Image.new('RGBA', (cover_width_2, cover_height_2), source_color_2)

                # 粘贴覆盖图像到 villager2_img
                villager2_img.paste(cover_img_2, (cover_x1_2, cover_y1_2))
                log(f"Covered region {cover_box_2} in villager2_img with color {source_color_2}")
            except Exception as e:
                log(f"Error covering region (133,60)-(242,76) in 'villager2_img': {e}")
                traceback.print_exc()

            # 新步骤4：将 (0,166)-(110,198) 的区域设置为全透明（根据 scale_factor 缩放）
            try:
                transparent_box = (
                    0 * scale_factor,
                    166 * scale_factor,
                    110 * scale_factor,
                    198 * scale_factor
                )
                log(f"Setting region {transparent_box} to fully transparent in villager2_img")
                # 创建一个全透明的图像
                transparent_region = Image.new('RGBA', (transparent_box[2] - transparent_box[0], 
                                                      transparent_box[3] - transparent_box[1]), 
                                               (0, 0, 0, 0))
                # 粘贴到指定区域
                villager2_img.paste(transparent_region, (transparent_box[0], transparent_box[1]))
                log(f"Region {transparent_box} set to transparent")
            except Exception as e:
                log(f"Error setting transparency in 'villager2_img': {e}")
                traceback.print_exc()

            # 新步骤5：在保存前处理 anvil.png 并粘贴到 villager.png（根据 scale_factor 缩放）
            try:
                anvil_path = os.path.join(container_path, 'anvil.png')
                if os.path.exists(anvil_path):
                    # 打开 anvil.png 并等比缩放到与 villager2_img 相同的尺寸
                    anvil_img = Image.open(anvil_path).convert("RGBA")
                    if anvil_img.size != villager2_img.size:
                        anvil_img = anvil_img.resize(villager2_img.size, Image.Resampling.NEAREST)
                        log(f"Resized 'anvil.png' from {anvil_img.size} to {villager2_img.size} using nearest neighbor")

                    # 定义源区域 (176,0)-(204,21) scaled
                    source_box = (
                        176 * scale_factor,
                        0 * scale_factor,
                        204 * scale_factor,
                        21 * scale_factor
                    )
                    cropped_region = anvil_img.crop(source_box)
                    log(f"Cropped region {source_box} from 'anvil.png'")

                    # 定义目标位置 (176,0) scaled
                    target_position = (176 * scale_factor, 0 * scale_factor)

                    # 粘贴裁剪后的区域到 villager2_img
                    villager2_img.paste(cropped_region, target_position, cropped_region)
                    log(f"Pasted cropped region {source_box} from 'anvil.png' to {target_position} in 'villager2_img'")
                else:
                    log(f"No 'anvil.png' found in {container_path} for paste step")
            except Exception as e:
                log(f"Error processing and pasting from 'anvil.png': {e}")
                traceback.print_exc()

            # 备份原始 villager.png
            try:
                backup_villager_path = os.path.join(container_path, 'villager_backup.png')
                if not os.path.exists(backup_villager_path):
                    shutil.copy(villager_path, backup_villager_path)
                    log(f"Backup of original 'villager.png' created at {backup_villager_path}")
                else:
                    log(f"Backup already exists at {backup_villager_path}")
            except Exception as e:
                log(f"Error creating backup of 'villager.png': {e}")
                traceback.print_exc()

            # 保存生成的新的 villager.png，覆盖原有的 villager.png
            try:
                villager2_img.save(villager_path)
                log(f"Saved new 'villager.png' at {villager_path}")
            except Exception as e:
                log(f"Error saving new 'villager.png': {e}")
                traceback.print_exc()
                return
        else:
            log(f"No 'villager.png' found in {container_path}")

    except Exception as e:
        log(f"Error processing smithing2 villager2 image in '{container_path}': {e}")
        traceback.print_exc()

def cut_gui(temp_dir):
    try:
        gui_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui')
        process_icons_in_dir(temp_dir)
        process_widgets_in_dir(temp_dir)
        process_tabs_in_dir(temp_dir)
        process_resource_packs_in_dir(temp_dir)
        process_server_selection_in_dir(temp_dir)
        process_title_in_dir(temp_dir)
        process1_anvil_image(temp_dir)
        process1_beacon_image(temp_dir)
        process1_blast_furnace_image(temp_dir)
        process1_brewing_stand_image(temp_dir)
        process1_cartography_table_image(temp_dir)
        process1_enchanting_table_image(temp_dir)
        process1_furnace_image(temp_dir)
        process1_smoker_image(temp_dir)
        process1_grindstone_image(temp_dir)
        process1_horse_image(temp_dir)
        process1_inventory_image(temp_dir)
        process1_loom_image(temp_dir)
        process1_stonecutter_image(temp_dir)
        process1_smithing_image(temp_dir)
        process1_villager2_image(temp_dir)
        process1_slider_image(gui_path)
    except Exception as e:
        log(f"Error processing cut gui image in '{temp_dir}': {e}")
        traceback.print_exc()

def delete_shaders_folder(temp_dir):
    delete_folder(os.path.join(temp_dir, "assets/minecraft/shaders"))

def delete_font_folder(temp_dir):
    delete_folder(os.path.join(temp_dir, "assets/minecraft/font"))

def generate_pale_planks(temp_dir):
    blocks_path_new = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'block')
    process_block_image(blocks_path_new, 'oak_planks.png', 'pale_oak_planks.png', hue_shift=0, brightness_adjust=30, saturation_adjust=-100)

def fix2_horse_ui(temp_dir):
    """
    处理 temp_dir 目录中的 horse 图片文件，
    """
    log(f"Processing horse images in: {temp_dir}")
    try:
        # 定义源文件和目标文件的对应关系
        files_to_copy = {
            'armor_slot.png': 'horse_armor.png',
            'llama_armor_slot.png': 'llama_armor.png',
            'saddle_slot.png': 'saddle.png'
        }

        # 定义源目录和目标目录
        source_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites', 'container', 'horse')
        target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'sprites','container', 'slot')

        # 确保目标目录存在，如果不存在则创建
        os.makedirs(target_dir, exist_ok=True)
        log(f"Ensured target directory exists: {target_dir}")

        # 遍历需要复制的文件
        for source_file, target_file in files_to_copy.items():
            source_path = os.path.join(source_dir, source_file)
            target_path = os.path.join(target_dir, target_file)

            # 检查源文件是否存在
            if os.path.exists(source_path):
                try:
                    shutil.copyfile(source_path, target_path)
                    log(f"Copied '{source_path}' to '{target_path}'")
                except Exception as e:
                    log(f"Error copying '{source_path}' to '{target_path}': {e}")
                    traceback.print_exc()
            else:
                log(f"Source file '{source_path}' does not exist. Skipping.")

    except Exception as e:
        log(f"Error processing horse images in '{temp_dir}': {e}")
        traceback.print_exc()

def fix_armor_models(temp_dir):
    """
    移动并重命名 armor 模型文件。
    """
    # 定义源目录和目标目录
    armor_source_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'models', 'armor')
    humanoid_target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'entity', 'equipment', 'humanoid')
    humanoid_leggings_target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'entity', 'equipment', 'humanoid_leggings')

    # 定义需要移动和重命名的文件映射
    layer_mappings = {
        'layer_1': {
            'source_dir': armor_source_dir,
            'target_dir': humanoid_target_dir,
            'files': {
                'chainmail_layer_1.png': 'chainmail.png',
                'diamond_layer_1.png': 'diamond.png',
                'iron_layer_1.png': 'iron.png',
                'gold_layer_1.png': 'gold.png',
                'leather_layer_1.png': 'leather.png',
                'leather_layer_1_overlay.png': 'leather_overlay.png',
                'netherite_layer_1.png': 'netherite.png',
                'copper_layer_1.png': 'copper.png',  # 添加铜盔甲层1
            }
        },
        'layer_2': {
            'source_dir': armor_source_dir,
            'target_dir': humanoid_leggings_target_dir,
            'files': {
                'chainmail_layer_2.png': 'chainmail.png',
                'diamond_layer_2.png': 'diamond.png',
                'iron_layer_2.png': 'iron.png',
                'gold_layer_2.png': 'gold.png',
                'leather_layer_2.png': 'leather.png',
                'leather_layer_2_overlay.png': 'leather_overlay.png',
                'netherite_layer_2.png': 'netherite.png',
                'copper_layer_2.png': 'copper.png',  # 添加铜盔甲层2
            }
        }
    }

    # 遍历每个层级的文件映射
    for layer, config in layer_mappings.items():
        source_dir = config['source_dir']
        target_dir = config['target_dir']
        files = config['files']

        # 确保目标目录存在
        os.makedirs(target_dir, exist_ok=True)

        for src_name, dest_name in files.items():
            src_path = os.path.join(source_dir, src_name)
            dest_path = os.path.join(target_dir, dest_name)

            if os.path.exists(src_path):
                try:
                    shutil.move(src_path, dest_path)
                    log(f"已移动并重命名 '{src_name}' 为 '{dest_name}' 到 '{target_dir}'")
                except Exception as e:
                    log(f"移动 '{src_name}' 到 '{target_dir}' 时出错: {e}")
                    traceback.print_exc()
            else:
                log(f"未找到文件: {src_path}")

def reverse_fix_armor_models(temp_dir):
    """
    将 armor 模型文件从 humanoid 和 humanoid_leggings 文件夹移动并重命名回 armor 文件夹。
    """
    # 定义源目录和目标目录
    armor_target_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'models', 'armor')
    humanoid_source_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'entity', 'equipment', 'humanoid')
    humanoid_leggings_source_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'entity', 'equipment', 'humanoid_leggings')

    # 定义需要移动和重命名的文件映射
    layer_mappings = {
        'humanoid': {
            'source_dir': humanoid_source_dir,
            'files': {
                'chainmail.png': 'chainmail_layer_1.png',
                'diamond.png': 'diamond_layer_1.png',
                'iron.png': 'iron_layer_1.png',
                'gold.png': 'gold_layer_1.png',
                'leather.png': 'leather_layer_1.png',
                'leather_overlay.png': 'leather_layer_1_overlay.png',
                'netherite.png': 'netherite_layer_1.png',
                'copper.png': 'copper_layer_1.png',  # 添加铜盔甲层1
            }
        },
        'humanoid_leggings': {
            'source_dir': humanoid_leggings_source_dir,
            'files': {
                'chainmail.png': 'chainmail_layer_2.png',
                'diamond.png': 'diamond_layer_2.png',
                'iron.png': 'iron_layer_2.png',
                'gold.png': 'gold_layer_2.png',
                'leather.png': 'leather_layer_2.png',
                'leather_overlay.png': 'leather_layer_2_overlay.png',
                'netherite.png': 'netherite_layer_2.png',
                'copper.png': 'copper_layer_2.png',  # 添加铜盔甲层2
            }
        }
    }

    # 确保目标目录存在
    os.makedirs(armor_target_dir, exist_ok=True)

    # 遍历每个源文件夹的文件映射
    for layer, config in layer_mappings.items():
        source_dir = config['source_dir']
        files = config['files']

        for src_name, dest_name in files.items():
            src_path = os.path.join(source_dir, src_name)
            dest_path = os.path.join(armor_target_dir, dest_name)

            if os.path.exists(src_path):
                try:
                    shutil.move(src_path, dest_path)
                    log(f"已移动并重命名 '{src_name}' 为 '{dest_name}' 到 '{armor_target_dir}'")
                except Exception as e:
                    log(f"移动 '{src_name}' 到 '{armor_target_dir}' 时出错: {e}")
                    traceback.print_exc()
            else:
                log(f"未找到文件: {src_path}")

def reverse_fix_clock_compass(temp_dir):
    items_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'items')
    compass_images = [os.path.join(items_path, f"compass_{i:02d}.png") for i in range(0, 32)]  # 从 0 到 31
    merge_images(compass_images, os.path.join(items_path, "compass.png"))        
    clock_images = [os.path.join(items_path, f"clock_{i:02d}.png") for i in range(0, 64)]
    merge_images(clock_images, os.path.join(items_path, "clock.png"))
    create_mcmeta_file(os.path.join(items_path, "compass.png.mcmeta"))
    create_mcmeta_file(os.path.join(items_path, "clock.png.mcmeta"))

def reverse_fix_ui_survival(temp_dir):
    log(f"Reversing inventory image in: {temp_dir}")
    inventory_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container', 'inventory.png')
    mob_effect_path= os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'mob_effect' )
    if os.path.exists(inventory_path):
        img = Image.open(inventory_path).convert("RGBA")
        width, height = img.size

        # Determine scale factor based on image size
        if width == 256 and height == 256:
            scale_factor = 1
        elif width == 512 and height == 512:
            scale_factor = 2
        elif width == 1024 and height == 1024:
            scale_factor = 4
        elif width == 2048 and height == 2048:
            scale_factor = 8
        else:
            log(f"Unsupported image size for 'inventory.png': {width}x{height}")
            return

        def scaled_coords(x1, y1, x2, y2):
            return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

        def scaled_point(x, y):
            return (x * scale_factor, y * scale_factor)

        # Step 1: Set (0, 198) to (144, 254) region to transparent
        transparent_box = scaled_coords(0, 198, 144, 254)
        log(f"Setting region {transparent_box} to transparent")
        for x in range(transparent_box[0], transparent_box[2]):
            for y in range(transparent_box[1], transparent_box[3]):
                img.putpixel((x, y), (0, 0, 0, 0))  # Set pixel to fully transparent

        # Step 2: Load images from mob_effect_path if provided and folder exists
        if mob_effect_path:
            mob_effect_folder = mob_effect_path
            if os.path.exists(mob_effect_folder):
                log(f"Mob effect folder found: {mob_effect_folder}. Proceeding with steps 2 and 3.")
                
                mob_effect_images = [
                    "speed.png", "slowness.png", "haste.png", "mining_fatigue.png", "strength.png",
                    "weakness.png", "poison.png", "regeneration.png",
                    "invisibility.png", "hunger.png", "jump_boost.png", "nausea.png", "night_vision.png",
                    "blindness.png", "resistance.png", "fire_resistance.png",
                    "water_breathing.png", "wither.png", "absorption.png"
                ]

                # Step 3: Place each mob effect image in the corresponding position
                image_width, image_height = 18 * scale_factor, 18 * scale_factor  # Adjusted based on scale_factor
                log(f"Pasting mob effect images onto 'inventory.png'")

                for i, effect_image in enumerate(mob_effect_images):
                    effect_image_path = os.path.join(mob_effect_folder, effect_image)
                    if os.path.exists(effect_image_path):
                        try:
                            effect_img = Image.open(effect_image_path).convert("RGBA")
                            # Resize the effect image if necessary
                            if effect_img.size != (image_width, image_height):
                                    log(f"Resizing '{effect_image}' from {effect_img.size} to ({image_width}, {image_height})")
                                    effect_img = effect_img.resize((image_width, image_height), Image.Resampling.NEAREST)

                            row = i // 8  # Determine the row (0, 1, or 2)
                            col = i % 8   # Determine the column (0 to 7)
                            
                            # Calculate position for each image in the scaled region
                            x_offset = col * image_width
                            y_offset = row * image_height
                            position = (transparent_box[0] + x_offset, transparent_box[1] + y_offset)

                            img.paste(effect_img, position, effect_img)  # Paste with transparency mask
                            log(f"Pasted '{effect_image}' at position {position}")
                        except Exception as e:
                            log(f"Failed to process '{effect_image}': {e}")
                    else:
                        log(f"Image '{effect_image}' not found in '{mob_effect_folder}'")
            else:
                log(f"Mob effect folder '{mob_effect_folder}' does not exist. Skipping steps 2 and 3.")
        else:
            log("No 'mob_effect_path' provided. Skipping steps 2 and 3.")

        # Step 4: Process custom color fill and region adjustments
        # Fill regions with (90, 10) color
        log("Filling specified regions with color")
        color_fill_region(img, *scaled_coords(76, 61, 94, 79), *scaled_point(90, 10))

        # Reverse translation (scale back the regions)
        log("Moving specified regions")
        move_region(img, *scaled_coords(96, 16, 172, 54), *scaled_point(-10, 8))

        # Additional color fill for specific regions
        log("Filling additional specified regions with color")
        color_fill_region(img, *scaled_coords(96, 16, 172, 25), *scaled_point(90, 10))
        color_fill_region(img, *scaled_coords(161, 25, 172, 54), *scaled_point(90, 10))

        # Save the processed image
        img.save(inventory_path)
        log(f"Reversed 'inventory.png' in {temp_dir}")
    else:
        log(f"No 'inventory.png' found in {temp_dir}")

def reverse_fix_ui_creative(temp_dir):
    creative_inventory_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container', 'creative_inventory', 'tab_inventory.png')
    log(f"Performing reverse operation on creative inventory image: {creative_inventory_path}")
    try:
        img = Image.open(creative_inventory_path).convert("RGBA")
        width, height = img.size

        if width == 256 and height == 256:
            scale_factor = 1
        elif width == 512 and height == 512:
            scale_factor = 2
        elif width == 1024 and height == 1024:
            scale_factor = 4
        elif width == 2048 and height == 2048:
            scale_factor = 8
        else:
            log(f"Unsupported image size for 'tab_inventory.png': {width}x{height}")
            return

        def scaled_coords(x1, y1, x2, y2):
            return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

        def scaled_point(x, y):
            return (x * scale_factor, y * scale_factor)

        # 1. Fill (34, 19, 52, 37) with the color at (164, 27)
        fill_color = img.getpixel(scaled_point(164, 27))
        for x in range(scaled_coords(34, 19, 52, 37)[0], scaled_coords(34, 19, 52, 37)[2]):
            for y in range(scaled_coords(34, 19, 52, 37)[1], scaled_coords(34, 19, 52, 37)[3]):
                img.putpixel((x, y), fill_color)

        # 2. Crop the region (51, 0, 129, 53) and paste it at (6, 0, 84, 53)
        source_box = scaled_coords(51, 0, 129, 53)
        dest_box = scaled_coords(6, 0, 84, 53)
        region = img.crop(source_box)
        img.paste(region, dest_box)

        # 3. Fill the region (84, 0, 129, 53) with the color at (164, 27)
        fill_box = scaled_coords(84, 0, 129, 53)
        for x in range(fill_box[0], fill_box[2]):
            for y in range(fill_box[1], fill_box[3]):
                img.putpixel((x, y), fill_color)

        # Save the modified image
        img.save(creative_inventory_path)
        log(f"Reverse operation completed and 'tab_inventory.png' has been saved.")
    except Exception as e:
        log(f"Error performing reverse operation on image '{creative_inventory_path}': {e}")
        traceback.print_exc()

def reverse_fix_brewing_stand_ui(temp_dir):
    container_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui', 'container')
    log(f"Reversing brewing stand image in: {container_path}")
    try:
        brewing_stand_path = os.path.join(container_path, 'brewing_stand.png')

        if os.path.exists(brewing_stand_path):
            img = Image.open(brewing_stand_path).convert("RGBA")
            width, height = img.size

            # Determine scale factor based on image size
            if width == 256 and height == 256:
                scale_factor = 1
            elif width == 512 and height == 512:
                scale_factor = 2
            elif width == 1024 and height == 1024:
                scale_factor = 4
            elif width == 2048 and height == 2048:
                scale_factor = 8
            else:
                log(f"Unsupported image size for 'brewing_stand.png': {width}x{height}")
                return

            # Step 1: Get the color from (7, 4) to fill regions
            fill_color = img.getpixel((7 * scale_factor, 4 * scale_factor))

            # Step 2: Fill (41, 43) to (79, 49) region with the color
            fill_box1 = (41 * scale_factor, 43 * scale_factor, 79 * scale_factor, 49 * scale_factor)
            for x in range(fill_box1[0], fill_box1[2]):
                for y in range(fill_box1[1], fill_box1[3]):
                    img.putpixel((x, y), fill_color)

            # Step 3: Fill (14, 14) to (55, 43) region with the color
            fill_box2 = (14 * scale_factor, 14 * scale_factor, 55 * scale_factor, 43 * scale_factor)
            for x in range(fill_box2[0], fill_box2[2]):
                for y in range(fill_box2[1], fill_box2[3]):
                    img.putpixel((x, y), fill_color)

            # Step 4: Move the region (55, 50) to (119, 75) upwards by 5 pixels
            move_box = (55 * scale_factor, 50 * scale_factor, 119 * scale_factor, 75 * scale_factor)
            region_to_move = img.crop(move_box)
            img.paste(region_to_move, (55 * scale_factor, 45 * scale_factor))

            # Step 5: Fill (55, 70) to (119, 75) region with the color
            fill_box3 = (55 * scale_factor, 70 * scale_factor, 119 * scale_factor, 75 * scale_factor)
            for x in range(fill_box3[0], fill_box3[2]):
                for y in range(fill_box3[1], fill_box3[3]):
                    img.putpixel((x, y), fill_color)

            # Save the processed image
            img.save(brewing_stand_path)
            log(f"Reversed 'brewing_stand.png' in {container_path}")
        else:
            log(f"No 'brewing_stand.png' found in {container_path}")
    except Exception as e:
        log(f"Error reversing brewing stand image in '{container_path}': {e}")
        traceback.print_exc()

def reverse_fix_particles(temp_dir):
    particle_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'particle')
    entity_dir = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'entity')
    log(f"合并粒子图片来自: {particle_dir} 和实体图片来自: {entity_dir}")
    try:
        # 定义文件名到其在网格中的位置 (row, col) 的映射
        filename_to_position = {
            'generic_0.png': (0, 0),
            'generic_1.png': (0, 1),
            'generic_2.png': (0, 2),
            'generic_3.png': (0, 3),
            'generic_4.png': (0, 4),
            'generic_5.png': (0, 5),
            'generic_6.png': (0, 6),
            'generic_7.png': (0, 7),
            'splash_0.png': (1, 3),
            'splash_1.png': (1, 4),
            'splash_2.png': (1, 5),
            'splash_3.png': (1, 6),
            'bubble.png': (2, 0),
            'fishing_hook.png': (2, 1),
            'flame.png': (3, 0),
            'lava.png': (3, 1),
            'note.png': (4, 0),
            'critical_hit.png': (4, 1),
            'enchanted_hit.png': (4, 2),
            'heart.png': (5, 0),
            'angry.png': (5, 1),
            'glint.png': (5, 2),
            'drip_hang.png': (7, 0),
            'drip_fall.png': (7, 1),
            'drip_land.png': (7, 2),
            'effect_0.png': (8, 0),
            'effect_1.png': (8, 1),
            'effect_2.png': (8, 2),
            'effect_3.png': (8, 3),
            'effect_4.png': (8, 4),
            'effect_5.png': (8, 5),
            'effect_6.png': (8, 6),
            'effect_7.png': (8, 7),
            'spell_0.png': (9, 0),
            'spell_1.png': (9, 1),
            'spell_2.png': (9, 2),
            'spell_3.png': (9, 3),
            'spell_4.png': (9, 4),
            'spell_5.png': (9, 5),
            'spell_6.png': (9, 6),
            'spell_7.png': (9, 7),
            'spark_0.png': (10, 0),
            'spark_1.png': (10, 1),
            'spark_2.png': (10, 2),
            'spark_3.png': (10, 3),
            'spark_4.png': (10, 4),
            'spark_5.png': (10, 5),
            'spark_6.png': (10, 6),
            'spark_7.png': (10, 7)
        }

        # 确定每个小图的大小 (假设所有小图大小相同)
        split_size = None
        for filename in filename_to_position.keys():
            # 尝试从 particle_dir 和 entity_dir 获取图片
            file_path_particle = os.path.join(particle_dir, filename)
            if os.path.exists(file_path_particle):
                img = Image.open(file_path_particle)
                split_size = img.width  # 假设小图为正方形
                break

            # 如果 particle_dir 没找到，则从 entity_dir 寻找
            file_path_entity = os.path.join(entity_dir, filename)
            if os.path.exists(file_path_entity):
                img = Image.open(file_path_entity)
                split_size = img.width
                break

        if split_size is None:
            log("未找到粒子图片或实体图片以确定分割大小。")
            return

        # 创建一个新的空白图片 (16行 x 16列)
        rows = 16
        cols = 16
        merged_size = (cols * split_size, rows * split_size)
        merged_image = Image.new("RGBA", merged_size, (0, 0, 0, 0))  # 透明背景

        # 遍历映射并粘贴每个小图片
        for filename, (row, col) in filename_to_position.items():
            file_path_particle = os.path.join(particle_dir, filename)
            file_path_entity = os.path.join(entity_dir, filename)
            
            if os.path.exists(file_path_particle):
                img = Image.open(file_path_particle).convert("RGBA")
                merged_image.paste(img, (col * split_size, row * split_size), img)
                log(f"粘贴 '{filename}' 从粒子目录，在行 {row}, 列 {col}")
            elif os.path.exists(file_path_entity):
                img = Image.open(file_path_entity).convert("RGBA")
                merged_image.paste(img, (col * split_size, row * split_size), img)
                log(f"粘贴 '{filename}' 从实体目录，在行 {row}, 列 {col}")
            else:
                log(f"缺少图片: '{filename}'")

        # 保存合并后的 particles.png
        particles_output_path = os.path.join(particle_dir, 'particles.png')
        merged_image.save(particles_output_path)
        log(f"已保存合并后的 'particles.png' 到 {particles_output_path}")

        # 合并后删除小图片
        for filename in filename_to_position.keys():
            file_path_particle = os.path.join(particle_dir, filename)
            file_path_entity = os.path.join(entity_dir, filename)
            if os.path.exists(file_path_particle):
                os.remove(file_path_particle)
                log(f"已删除粒子小图片: '{file_path_particle}'")
            if os.path.exists(file_path_entity):
                os.remove(file_path_entity)
                log(f"已删除实体小图片: '{file_path_entity}'")

    except Exception as e:
        log(f"合并粒子图片时出错: {e}")
        traceback.print_exc()

def reverse_rename_blocks_items(temp_dir):
    textures_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures')           
    item_dir_old = os.path.join(textures_path, 'item')
    items_dir_new = os.path.join(textures_path, 'items')
    if os.path.exists(item_dir_old):
        os.rename(item_dir_old, items_dir_new)
        log(f"已将 'item' 重命名为 'items' 在 {textures_path}")
    else:
        log(f"未找到 'item' 文件夹在 {textures_path} 以重命名为 'items'")

    block_dir_old = os.path.join(textures_path, 'block')
    blocks_dir_new = os.path.join(textures_path, 'blocks')
    if os.path.exists(block_dir_old):
        os.rename(block_dir_old, blocks_dir_new)
        log(f"已将 'block' 重命名为 'blocks' 在 {textures_path}")
    else:
        log(f"未找到 'block' 文件夹在 {textures_path} 以重命名为 'blocks'")

        # 定义重命名对
    rename_pairs = {
        'gold_sword.png': 'golden_sword.png',
        'wood_sword.png': 'wooden_sword.png',
        'gold_helmet.png': 'golden_helmet.png',
        'gold_chestplate.png': 'golden_chestplate.png',
        'gold_leggings.png': 'golden_leggings.png',
        'gold_boots.png': 'golden_boots.png',
        'apple_golden.png': 'golden_apple.png',
        'bow_standby.png': 'bow.png',
        'book_enchanted.png': 'enchanted_book.png',
        'wood_axe.png': 'wooden_axe.png',
        'wood_pickaxe.png': 'wooden_pickaxe.png',
        'wood_shovel.png': 'wooden_shovel.png',
        'wood_hoe.png': 'wooden_hoe.png',
        'gold_axe.png': 'golden_axe.png',
        'gold_pickaxe.png': 'golden_pickaxe.png',
        'gold_shovel.png': 'golden_shovel.png',
        'gold_hoe.png': 'golden_hoe.png',
        'fishing_rod_uncast.png': 'fishing_rod.png',
        'potion_bottle_empty.png': 'glass_bottle.png',
        'potion_bottle_drinkable.png': 'potion.png',
        'potion_bottle_splash.png': 'splash_potion.png',
        'potion_bottle_lingering.png': 'lingering_potion.png',
        'spider_eye_fermented.png': 'fermented_spider_eye.png',
        'melon_speckled.png': 'glistering_melon_slice.png',
        'melon.png': 'melon_slice.png',
        'carrot_golden.png': 'golden_carrot.png',
        'porkchop_raw.png': 'porkchop.png',
        'porkchop_cooked.png': 'cooked_porkchop.png',
        'chicken_raw.png': 'chicken.png',
        'chicken_cooked.png': 'cooked_chicken.png',
        'rabbit_raw.png': 'rabbit.png',
        'rabbit_cooked.png': 'cooked_rabbit.png',
        'beef_raw.png': 'beef.png',
        'beef_cooked.png': 'cooked_beef.png',
        'boat.png': 'oak_boat.png',
        'book_normal.png': 'book.png',
        'book_writable.png': 'writable_book.png',
        'book_written.png': 'written_book.png',
        'bucket_empty.png': 'bucket.png',
        'bucket_lava.png': 'lava_bucket.png',
        'bucket_water.png': 'water_bucket.png',
        'bucket_milk.png': 'milk_bucket.png',
        'door_acacia.png': 'acacia_door.png',
        'door_birch.png': 'birch_door.png',
        'door_dark_oak.png': 'dark_oak_door.png',
        'door_iron.png': 'iron_door.png',
        'door_jungle.png': 'jungle_door.png',
        'door_spruce.png': 'spruce_door.png',
        'door_wood.png': 'oak_door.png',
        'dye_powder_black.png': 'ink_sac.png',
        'dye_powder_blue.png': 'lapis_lazuli.png',
        'dye_powder_brown.png': 'cocoa_beans.png',
        'dye_powder_cyan.png': 'cyan_dye.png',
        'dye_powder_gray.png': 'gray_dye.png',
        'dye_powder_green.png': 'green_dye.png',
        'dye_powder_light_blue.png': 'light_blue_dye.png',
        'dye_powder_lime.png': 'lime_dye.png',
        'dye_powder_magenta.png': 'magenta_dye.png',
        'dye_powder_orange.png': 'orange_dye.png',
        'dye_powder_pink.png': 'pink_dye.png',
        'dye_powder_purple.png': 'purple_dye.png',
        'dye_powder_red.png': 'red_dye.png',
        'dye_powder_silver.png': 'light_gray_dye.png',
        'dye_powder_white.png': 'bone_meal.png',
        'dye_powder_yellow.png': 'yellow_dye.png',
        'fireball.png': 'fire_charge.png',
        'fireworks.png': 'firework_rocket.png',
        'fireworks_charge.png': 'firework_star.png',
        'firework_charge_overlay.png': 'firework_star_overlay.png',
        'fish_cod_raw.png': 'cod.png',
        'fish_cod_cooked.png': 'cooked_cod.png',
        'fish_salmon_raw.png': 'salmon.png',
        'fish_salmon_cooked.png': 'cooked_salmon.png',
        'fish_clownfish_raw.png': 'tropical_fish.png',
        'fish_pufferfish_raw.png': 'pufferfish.png',
        'map_empty.png': 'map.png',
        'map_filled.png': 'filled_map.png',
        'minecart_chest.png': 'chest_minecart.png',
        'minecart_command_block.png': 'command_block_minecart.png',
        'minecart_furnace.png': 'furnace_minecart.png',
        'minecart_hopper.png': 'hopper_minecart.png',
        'minecart_normal.png': 'minecart.png',
        'minecart_tnt.png': 'tnt_minecart.png',
        'mutton_cooked.png': 'cooked_mutton.png',
        'mutton_raw.png': 'mutton.png',
        'netherbrick.png': 'nether_brick.png',
        'potato_baked.png': 'baked_potato.png',
        'potato_poisonous.png': 'poisonous_potato.png',
        'record_11.png': 'music_disc_11.png',
        'record_13.png': 'music_disc_13.png',
        'record_blocks.png': 'music_disc_blocks.png',
        'record_cat.png': 'music_disc_cat.png',
        'record_chirp.png': 'music_disc_chirp.png',
        'record_far.png': 'music_disc_far.png',
        'record_mail.png': 'music_disc_mail.png',
        'record_mellohi.png': 'music_disc_mellohi.png',
        'record_stal.png': 'music_disc_stal.png',
        'record_strad.png': 'music_disc_strad.png',
        'record_wait.png': 'music_disc_wait.png',
        'record_ward.png': 'music_disc_ward.png',
        'record_mall.png': 'music_disc_mall.png',
        'redstone_dust.png': 'redstone.png',
        'reeds.png': 'sugar_cane.png',
        'seeds_melon.png': 'melon_seeds.png',
        'seeds_pumpkin.png': 'pumpkin_seeds.png',
        'seeds_wheat.png': 'wheat_seeds.png',
        'sign.png': 'oak_sign.png',
        'slimeball.png': 'slime_ball.png',
        'wooden_armorstand.png': 'armor_stand.png',
        'gold_horse_armor.png': 'golden_horse_armor.png'
        }

    # 反转 rename_pairs
    rename_pairs_reversed = {v: k for k, v in rename_pairs.items()}
    log("使用反转的 rename_pairs 进行文件重命名用于1.21.4转1.8转换")

    items_path_new = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'items')
    if os.path.exists(items_path_new):
        rename_items(items_path_new, rename_pairs_reversed)
        log("已使用反转的 rename_pairs 重命名 items")
    else:
        log(f"未找到 'item' 文件夹在 {temp_dir}/assets/minecraft/textures")

    # 确保 blocks_path_new 指向 assets/minecraft/textures/block
    blocks_path_new = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'blocks')
    if os.path.exists(blocks_path_new):
        rename_and_process_blocks(blocks_path_new, reverse=True)
        log("已使用反转的 rename_pairs 重命名并处理 blocks")
    else:
        log(f"未找到 'block' 文件夹在 {temp_dir}/assets/minecraft/textures")

def reverse_process_chest_folder(temp_dir):
    """
    逆向处理单个箱子图像，将它们恢复到原始状态。
    """
    chest_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'entity', 'chest')
    log(f"Reversing processing of chest images in: {chest_path}")
    single_chest_files = ['ender.png', 'normal.png', 'trapped.png', 'christmas.png']
    
    for chest_file in single_chest_files:
        chest_file_path = os.path.join(chest_path, chest_file)
        if os.path.exists(chest_file_path):
            try:
                img = Image.open(chest_file_path).convert("RGBA")
                width, height = img.size
                log(f"Reversing '{chest_file}' with size: {width}x{height}")

                # Determine scale_factor based on image size
                if width == 64 and height == 64:
                    scale_factor = 1
                elif width == 128 and height == 128:
                    scale_factor = 2
                elif width == 256 and height == 256:
                    scale_factor = 4
                elif width == 512 and height == 512:
                    scale_factor = 8
                elif width == 1024 and height == 1024:
                    scale_factor = 16
                else:
                    log(f"Unsupported image size for '{chest_file}': {width}x{height}")
                    continue

                def scaled_box(x1, y1, x2, y2):
                    return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)

                # 定义需要交换和镜像的区域
                swap_and_mirror_boxes = [
                    (scaled_box(14, 0, 28, 14), scaled_box(28, 0, 42, 14)),
                    (scaled_box(14, 14, 28, 19), scaled_box(42, 14, 56, 19)),
                    (scaled_box(14, 19, 28, 33), scaled_box(28, 19, 42, 33)),
                    (scaled_box(14, 33, 28, 43), scaled_box(42, 33, 56, 43))
                ]

                # 逆向交换和镜像操作
                for box1, box2 in swap_and_mirror_boxes:
                    swap_and_mirror(img, box1, box2)

                # 定义需要镜像的区域
                mirror_boxes = [
                    scaled_box(14, 0, 28, 14), scaled_box(28, 0, 42, 14),
                    scaled_box(0, 14, 14, 19), scaled_box(28, 14, 42, 19), 
                    scaled_box(14, 19, 28, 33), scaled_box(28, 19, 42, 33),
                    scaled_box(0, 33, 14, 43), scaled_box(28, 33, 42, 43)
                ]

                # 逆向镜像操作（再次应用镜像以恢复原始）
                for box in mirror_boxes:
                    region = img.crop(box).transpose(Image.FLIP_LEFT_RIGHT).transpose(Image.FLIP_TOP_BOTTOM)
                    img.paste(region, box)

                img.save(chest_file_path)
                log(f"Reversed processing of '{chest_file}' successfully.")

            except Exception as e:
                log(f"Error reversing '{chest_file}': {e}")
                continue
        else:
            log(f"File '{chest_file}' does not exist in the path '{chest_path}'. Skipping.")

    log("Reversing chest images processing completed.")

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

# 定义 pack_format 顺序
PACK_FORMAT_ORDER = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 13, 15, 18, 22, 32, 34, 42, 46, 55, 63, 64, 69, 75, 1000
]

def new_pack_format_generate(temp_dir, pack_meta_path, target_pack_format):
    """
    生成1.21.9及以上版本的pack.mcmeta格式
    :param temp_dir: 临时目录
    :param pack_meta_path: pack.mcmeta文件路径
    :param target_pack_format: 目标版本的pack_format
    """
    try:
        # 读取原pack.mcmeta文件
        try:
            with open(pack_meta_path, 'r', encoding='utf-8') as f:
                original_data = json.load(f)
        except UnicodeDecodeError:
            log("UTF-8 解码失败，尝试使用 ISO-8859-1")
            with open(pack_meta_path, 'r', encoding='iso-8859-1') as f:
                original_data = json.load(f)
        
        # 提取原description
        original_description = ""
        if 'pack' in original_data and 'description' in original_data['pack']:
            original_description = original_data['pack']['description']
        
        # 创建新的pack.mcmeta格式
        new_data = {
            "pack": {
                "pack_format": 34,
                "supported_formats": [34, target_pack_format],
                "min_format": [34, 0],
                "max_format": [target_pack_format, 0],
                "description": original_description
            }
        }
        
        # 覆写原文件
        with open(pack_meta_path, 'w', encoding='utf-8') as f:
            json.dump(new_data, f, ensure_ascii=False, indent=4)
        
        log(f"已生成1.21.9及以上版本的pack.mcmeta格式，目标版本: {target_pack_format}")
        
    except Exception as e:
        log(f"生成新格式pack.mcmeta时出错: {e}")

# 定义相邻版本转换映射
ADJACENT_CONVERSIONS = {
    (1, 2): [delete_blockstates_models, generate_tipped_arrow_images, fix_ui_survival, fix_ui_creative, fix_ui_sub_hand, generate_boat, generate_potion_lingering, generate_shulker_box_ui, fix_brewing_stand_ui, fix_clock_compass, overlay_icons],
    (2, 3): [generate_shulker_box_ui, delete_horse_folder, fix_horse_ui],
    (3, 4): [rename_blocks_items, fix_sign, fix_sign_entities, generate_furnace, fix_machinery_ui, fix_particles, generate_fish_bucket, generate_crossbow],
    (4, 5): [process_chest_folder, generate_netherite_block, generate_netherite_ingot, delete_enchanted_item_glint, generate_netherite_tools, generate_netherite_armor_models, generate_smithing_ui],
    (5, 6): [delete_font_folder],
    (6, 7): [generate_snow_bucket],
    (7, 8): [rename_mcpatcher_to_optifine],
    (8, 9): [],  # 没有定义步骤，空列表，跳过
    (9, 12): [fix_tabs,generate_redwood_cherry_bamboo_planks],
    (12, 13): [fix_smithing2_villager2_ui, fix_slider],
    (13, 15): [],
    (15, 18): [cut_gui],
    (18, 22): [],
    (22, 32): [],
    (32, 34): [delete_shaders_folder],
    (34, 42): [delete_shaders_folder],
    (42, 46): [fix2_horse_ui, fix_armor_models, generate_pale_planks],
    (46, 55): [],
    (55, 63): [],
    (63, 64): [],
    (64, 69): [generate_copper_ingot, generate_copper_block, generate_copper_tools, generate_copper_armor_models],
    (69, 69.0): [],
}

ADJACENT_CONVERSIONS_REVERSE = {
    (69.0, 69): [],
    (69, 64): [],
    (63, 64): [],
    (55, 63): [],
    (46, 55): [],
    (46, 42): [reverse_fix_armor_models],
    (42, 34): [],
    (34, 32): [],
    (32, 22): [],
    (22, 18): [],
    (18, 15): [],
    (15, 13): [],
    (13, 12): [],
    (12, 9): [],
    (9, 8): [],
    (8, 7): [],
    (7, 6): [],
    (6, 5): [],
    (5, 4): [reverse_process_chest_folder],
    (4, 3): [reverse_rename_blocks_items, reverse_fix_particles, delete_horse_folder],
    (3, 2): [delete_horse_folder],
    (2, 1): [delete_blockstates_models, reverse_fix_clock_compass, reverse_fix_ui_survival, reverse_fix_ui_creative, reverse_fix_brewing_stand_ui],
}

def process_zip(temp_dir, original_file_path, pack_format1, pack_format2, progress_callback=None, file_progress_weight=1.0, parent_folder_path=None):
    """
    处理材质包转换的主要函数。
    """
    import time
    start_time = time.time()
    
    # 获取版本顺序的索引
    try:
        # 检查是否是Bedrock版转换（pack_format2=1000）
        if pack_format2 == 1000:
            # Bedrock版转换需要先转换到Java 1.21.11（pack_format=75）
            start_index = PACK_FORMAT_ORDER.index(pack_format1)
            # 找到Java 1.21.11对应的pack_format索引
            java_1_21_11_format = 75
            if java_1_21_11_format not in PACK_FORMAT_ORDER:
                raise ValueError(f"不支持的Java 1.21.11 pack_format: {java_1_21_11_format}")
            end_index = PACK_FORMAT_ORDER.index(java_1_21_11_format)
        else:
            start_index = PACK_FORMAT_ORDER.index(pack_format1)
            end_index = PACK_FORMAT_ORDER.index(pack_format2)
    except ValueError:
        log(f"无效的 pack_format: {pack_format1} 或 {pack_format2}")
        raise ValueError(f"无效的 pack_format: {pack_format1} 或 {pack_format2}") from None
    
    # 判断转换方向
    if start_index <= end_index:
        # 低版本转高版本的转换
        direction = 'forward'
    else:
        # 高版本转低版本的转换
        direction = 'reverse'

    # 根据转换方向选择相应的映射
    if direction == 'forward':
        conversion_map = ADJACENT_CONVERSIONS
    else:
        conversion_map = ADJACENT_CONVERSIONS_REVERSE

    # 计算总转换步骤数
    total_steps = 0
    steps_to_execute = []
    
    # 预先收集所有要执行的步骤
    for i in range(start_index, end_index) if direction == 'forward' else range(start_index, end_index, -1):
        current_format = PACK_FORMAT_ORDER[i]
        next_format = PACK_FORMAT_ORDER[i + 1] if direction == 'forward' else PACK_FORMAT_ORDER[i - 1]
        conversion_key = (current_format, next_format)
        actions = conversion_map.get(conversion_key)
        
        if actions:
            # 过滤掉需要跳过的步骤
            for action in actions:
                if conversion_key == (9, 12) and pack_format2 > 15 and action == fix_tabs:
                    continue
                steps_to_execute.append((current_format, next_format, action))
                total_steps += 1
    
    log(f"总转换步骤数: {total_steps}")
    
    # 依次执行相邻版本的转换
    try:
        executed_steps = 0
        
        for current_format, next_format, action in steps_to_execute:
            log(f"正在转换: {PACK_FORMAT_MAP[current_format]} -> {PACK_FORMAT_MAP[next_format]}")
            action(temp_dir)
            
            # 更新执行步骤计数并计算进度
            executed_steps += 1
            
            # 如果有进度回调，计算并报告进度
            if progress_callback:
                # 计算当前文件的进度（0-100%）
                file_progress = (executed_steps / total_steps) * 100
                # 应用权重得到整体进度
                overall_progress = int(file_progress * file_progress_weight)
                progress_callback(overall_progress)
        
        # 更新 pack.mcmeta 文件中的 pack_format 字段
        pack_meta_path = os.path.join(temp_dir, 'pack.mcmeta')
        if os.path.exists(pack_meta_path):
            # 检查是否为1.21.9及以上版本（pack_format2 >= 69）
            if pack_format2 >= 69:
                new_pack_format_generate(temp_dir, pack_meta_path, pack_format2)
            else:
                try:
                    with open(pack_meta_path, 'r', encoding='utf-8') as f:
                        data = json.load(f)
                except UnicodeDecodeError:
                    log("UTF-8 解码失败，尝试使用 ISO-8859-1")
                    with open(pack_meta_path, 'r', encoding='iso-8859-1') as f:
                        data = json.load(f)

                # 检查 'pack' 和 'pack_format' 字段是否存在
                if 'pack' in data and 'pack_format' in data['pack']:
                    original_pack_format = data['pack']['pack_format']
                    data['pack']['pack_format'] = pack_format2  # 更新为目标版本的 pack_format

                    # 处理 description 字段
                    if 'description' in data['pack']:
                        data['pack']['description'] = fix_description(data['pack']['description'])

                    # 写回文件，保存更新
                    with open(pack_meta_path, 'w', encoding='utf-8') as f:
                        json.dump(data, f, ensure_ascii=False, indent=4)
                    log(f"已更新 'pack_format' 从 {original_pack_format} 到 {pack_format2} 在 {pack_meta_path}")
                else:
                    log("无效的 pack.mcmeta 结构。未找到 'pack_format'.")
        
        # 生成新的文件名：[目标版本]原文件名.zip，避免批量处理时文件名重复
        # 获取原始文件名（不含扩展名）
        original_filename = os.path.basename(original_file_path)
        base_name = os.path.splitext(original_filename)[0]
        
        # 检测并移除原始文件名中的版本前缀（如果匹配PACK_FORMAT_MAP中的版本）
        import re
        # 获取所有有效的版本字符串
        valid_versions = list(PACK_FORMAT_MAP.values())
        # 构建正则表达式，匹配以[版本号]开头的字符串
        pattern = r'^\[({})\]'.format('|'.join(re.escape(v) for v in valid_versions))
        # 移除匹配的版本前缀
        base_name = re.sub(pattern, '', base_name)
        # 去除可能的空格
        base_name = base_name.strip()
        
        # 基本文件名 - 添加新的目标版本前缀
        base_new_filename = f"[{PACK_FORMAT_MAP[pack_format2]}]{base_name}"
        new_filename = f"{base_new_filename}.zip"
        
        # 获取原始文件的路径信息
        original_path = original_file_path
        
        # 智能确定输出目录
        if parent_folder_path:
            # 如果提供了父文件夹路径（文件来自拖放的文件夹），则在父文件夹的同级目录创建输出文件夹
            # 例如：C:\材质包\材质包文件夹\材质包1.zip -> C:\材质包\[目标版本]材质包文件夹\
            grandparent_dir = os.path.dirname(parent_folder_path)
            parent_folder_name = os.path.basename(parent_folder_path)
            
            # 检测并移除父文件夹名称中的版本前缀（如果匹配PACK_FORMAT_MAP中的版本）
            import re
            valid_versions = list(PACK_FORMAT_MAP.values())
            pattern = r'^\[({})\]'.format('|'.join(re.escape(v) for v in valid_versions))
            # 移除匹配的版本前缀
            parent_folder_name = re.sub(pattern, '', parent_folder_name)
            # 去除可能的空格
            parent_folder_name = parent_folder_name.strip()
            
            output_dir = os.path.join(grandparent_dir, f"[{PACK_FORMAT_MAP[pack_format2]}]{parent_folder_name}")
            os.makedirs(output_dir, exist_ok=True)
        else:
            # 对于单个文件拖放，直接放在原文件的目录下，不创建额外文件夹
            output_dir = os.path.dirname(original_path)
        
        # 设置输出文件路径
        processed_zip_path = os.path.join(output_dir, new_filename)
        
        # 如果文件已存在，添加计数器以避免覆盖
        counter = 1
        while os.path.exists(processed_zip_path):
            new_filename = f"[{PACK_FORMAT_MAP[pack_format2]}]{base_name}_{counter}.zip"
            processed_zip_path = os.path.join(output_dir, new_filename)
            counter += 1

        # 重新压缩文件
        with zipfile.ZipFile(processed_zip_path, 'w') as zipf:  # 使用 zipfile.ZipFile
            for root, dirs, files in os.walk(temp_dir):
                for file in files:
                    file_path = os.path.join(root, file)
                    # 相对路径以保持文件结构
                    arcname = os.path.relpath(file_path, temp_dir)
                    zipf.write(file_path, arcname)
        
        log(f"已生成转换后的材质包: {processed_zip_path}")
        
        # 计算转换时间
        end_time = time.time()
        elapsed_time = end_time - start_time
        log(f"转换完成，耗时: {elapsed_time:.2f} 秒")
        
        # 删除临时目录（带重试机制）
        max_retries = 3
        retry_count = 0
        
        while retry_count < max_retries:
            try:
                shutil.rmtree(temp_dir)
                log(f"已删除临时目录: {temp_dir}")
                break
            except PermissionError as e:
                retry_count += 1
                if retry_count < max_retries:
                    log(f"删除临时目录时权限被拒绝，重试 ({retry_count}/{max_retries}): {e}")
                    time.sleep(0.5)  # 等待0.5秒后重试
                else:
                    log(f"警告: 无法删除临时目录（已重试{max_retries}次）: {temp_dir}")
                    log(f"错误详情: {e}")
                    # 不抛出错误，继续执行
            except Exception as e:
                log(f"警告: 删除临时目录时发生错误: {e}")
                # 不抛出错误，继续执行
                break
        
        # 更新进度为100%
        if progress_callback:
            progress_callback(100)
        
        return processed_zip_path
    except Exception as e:
        log(f"转换过程中发生错误: {e}")
        # 如果有进度回调，将进度更新为100%表示处理结束
        if progress_callback:
            progress_callback(100)
        raise RuntimeError(f"转换过程中发生错误: {e}") from e

def main_menu():
    global frame, result_label, icons_path, select_button, selected_files, new_button, root

    root = TkinterDnD.Tk()
    root.title("ZIP处理器")
    root.geometry("980x720")
    root.configure(bg="#f0f0f0")
    root.drop_target_register(DND_FILES)
    root.dnd_bind('<<Drop>>', on_file_drop)

    # 设置默认主题
    set_default_theme(root)

    global frame
    frame = tk.Frame(root, bg="#f0f0f0")
    frame.pack(pady=20, fill="both", expand=True)

    selected_files = tk.Variable(value=[])

    # 显示主界面
    show_main_menu()

    icons_path = os.path.join(os.getcwd(), 'icons')

    # 初始化 new_button
    new_button = tk.Button(frame, text="隐藏按钮", command=lambda: None, bg="#FFFFE0", fg="black", font=("微软雅黑", 14), padx=20, pady=10)
    new_button.pack(pady=10)
    new_button.pack_forget()  # 确保new_button在初始状态下是隐藏的

    root.mainloop()


def cleanup_residual_temp_dirs():
    """
    清理残留的临时目录
    """
    import tempfile
    import shutil
    import time
    
    temp_root = tempfile.gettempdir()
    residual_dirs = []
    
    try:
        for item in os.listdir(temp_root):
            if item.startswith('mcpack_') and item.endswith('_temp'):
                item_path = os.path.join(temp_root, item)
                if os.path.isdir(item_path):
                    residual_dirs.append(item_path)
        
        if residual_dirs:
            log(f"发现 {len(residual_dirs)} 个残留的临时目录，开始清理...")
            
            for dir_path in residual_dirs:
                max_retries = 3
                retry_count = 0
                
                while retry_count < max_retries:
                    try:
                        shutil.rmtree(dir_path)
                        log(f"已清理残留目录: {dir_path}")
                        break
                    except PermissionError:
                        retry_count += 1
                        if retry_count < max_retries:
                            time.sleep(0.5)
                        else:
                            log(f"无法清理残留目录（可能被占用）: {dir_path}")
                    except Exception as e:
                        log(f"清理残留目录时出错: {dir_path}, 错误: {e}")
                        break
        else:
            log("没有发现残留的临时目录")
            
    except Exception as e:
        log(f"清理残留目录时发生错误: {e}")

# 示例主应用设置
if __name__ == "__main__":
    # 启动时清理残留的临时目录
    cleanup_residual_temp_dirs()
    main_menu()
