import os
import json
import shutil
import zipfile
import logging
import re
import os

# 设置日志
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

def clean_temp_overlay_dir(temp_overlay_dir):
    """
    清空temp_overlay目录
    保存overlay.json文件，然后删除其他所有文件和目录
    """
    try:
        if os.path.exists(temp_overlay_dir):
            # 检查overlay.json文件是否存在
            overlay_file = os.path.join(temp_overlay_dir, "overlay.json")
            overlay_content = None
            if os.path.exists(overlay_file):
                # 保存overlay.json文件内容
                with open(overlay_file, "r", encoding="utf-8-sig") as f:
                    overlay_content = json.load(f)
                
            # 遍历目录并删除所有内容
            for item in os.listdir(temp_overlay_dir):
                item_path = os.path.join(temp_overlay_dir, item)
                if os.path.isfile(item_path):
                    os.remove(item_path)
                elif os.path.isdir(item_path):
                    shutil.rmtree(item_path)
            
            logger.info(f"已清空temp_overlay目录: {temp_overlay_dir}")
            
            # 重新创建temp_overlay目录
            os.makedirs(temp_overlay_dir, exist_ok=True)
            
            # 恢复overlay.json文件
            if overlay_content:
                with open(overlay_file, "w", encoding="utf-8-sig") as f:
                    json.dump(overlay_content, f, ensure_ascii=False, indent=2, separators=(',', ': '))
                logger.info(f"已恢复overlay.json文件")
    except Exception as e:
        logger.error(f"清空temp_overlay目录过程中出错: {str(e)}")
        raise

def start_overlay():
    """
    开始制作覆盖包
    根据overlay.json中的parent_pack配置决定是解压母材质包还是创建默认文件夹结构
    """
    try:
        # 获取当前工作目录
        current_dir = os.getcwd()
        temp_overlay_dir = os.path.join(current_dir, "temp_overlay")
        overlay_file = os.path.join(temp_overlay_dir, "overlay.json")
        
        # 清空temp_overlay目录
        clean_temp_overlay_dir(temp_overlay_dir)
        
        # 检查overlay.json文件是否存在
        if not os.path.exists(overlay_file):
            logger.warning("overlay.json文件不存在，创建默认文件夹结构")
            create_default_structure(temp_overlay_dir)
            return
        
        # 读取overlay.json文件
        try:
            with open(overlay_file, "r", encoding="utf-8-sig") as f:
                settings = json.load(f)
        except json.JSONDecodeError:
            logger.error("overlay.json文件格式错误，创建默认文件夹结构")
            create_default_structure(temp_overlay_dir)
            return
        
        # 检查是否有parent_pack配置
        if "parent_pack" in settings and settings["parent_pack"].get("enabled", False):
            parent_pack_path = settings["parent_pack"].get("path", "")
            if parent_pack_path and os.path.exists(parent_pack_path):
                # 处理母材质包
                process_parent_pack(temp_overlay_dir, parent_pack_path)
            else:
                logger.warning("母材质包路径不存在，创建默认文件夹结构")
                create_default_structure(temp_overlay_dir)
        else:
            # 没有母材质包配置，创建默认文件夹结构
            create_default_structure(temp_overlay_dir)
            
    except Exception as e:
        logger.error(f"制作覆盖包过程中出错: {str(e)}")
        raise

def process_parent_pack(temp_overlay_dir, parent_pack_path):
    """
    处理母材质包
    - 在temp_overlay目录创建workspace文件夹
    - 将zip文件移动到workspace文件夹
    - 解压zip文件
    - 删除原zip文件
    """
    try:
        # 获取文件名
        file_name = os.path.basename(parent_pack_path)
        
        # 创建workspace文件夹
        workspace_dir = os.path.join(temp_overlay_dir, "workspace")
        os.makedirs(workspace_dir, exist_ok=True)
        
        # 移动zip文件到workspace文件夹
        new_zip_path = os.path.join(workspace_dir, file_name)
        if os.path.exists(new_zip_path):
            os.remove(new_zip_path)  # 删除已存在的文件
        shutil.move(parent_pack_path, new_zip_path)
        logger.info(f"已将母材质包移动到: {new_zip_path}")
        
        # 解压zip文件
        extract_dir = os.path.join(workspace_dir, os.path.splitext(file_name)[0])
        os.makedirs(extract_dir, exist_ok=True)
        
        with zipfile.ZipFile(new_zip_path, 'r') as zip_ref:
            zip_ref.extractall(extract_dir)
        logger.info(f"已解压母材质包到: {extract_dir}")
        
        # 删除原zip文件
        os.remove(new_zip_path)
        logger.info(f"已删除原zip文件: {new_zip_path}")
        
        # 记录解压信息到overlay.json
        update_overlay_with_workspace_info(temp_overlay_dir, extract_dir)
        
    except Exception as e:
        logger.error(f"处理母材质包过程中出错: {str(e)}")
        raise

def create_default_structure(temp_overlay_dir):
    """
    创建默认的文件夹结构
    - /assets/minecraft/textures/models/item
    - /assets/minecraft/shaders/core
    - /assets/minecraft/lang
    """
    try:
        # 创建必要的文件夹结构
        folders_to_create = [
            os.path.join(temp_overlay_dir, "assets", "minecraft", "models"),
            os.path.join(temp_overlay_dir, "assets", "minecraft", "shaders", "core"),
            os.path.join(temp_overlay_dir, "assets", "minecraft", "lang")
        ]
        
        for folder in folders_to_create:
            os.makedirs(folder, exist_ok=True)
            logger.info(f"已创建文件夹: {folder}")
            
    except Exception as e:
        logger.error(f"创建默认文件夹结构过程中出错: {str(e)}")
        raise

def update_overlay_with_workspace_info(temp_overlay_dir, workspace_path):
    """
    更新overlay.json文件，记录workspace信息
    """
    try:
        overlay_file = os.path.join(temp_overlay_dir, "overlay.json")
        
        # 读取现有配置
        if os.path.exists(overlay_file):
            with open(overlay_file, "r", encoding="utf-8-sig") as f:
                settings = json.load(f)
        else:
            settings = {}
        
        # 添加workspace信息
        settings["workspace"] = {
            "path": workspace_path,
            "processed": True,
            "timestamp": str(os.path.getmtime(workspace_path))
        }
        
        # 保存更新后的配置（使用紧凑格式）
        with open(overlay_file, "w", encoding="utf-8-sig") as f:
            json.dump(settings, f, ensure_ascii=False, separators=(',', ':'))
        
        logger.info(f"已更新overlay.json文件，添加workspace信息")
        
    except Exception as e:
        logger.error(f"更新overlay.json文件过程中出错: {str(e)}")
        # 不抛出异常，继续执行

def fix_json_placeholders(file_path):
    """
    修复JSON文件中的占位符问题
    将@和#替换为1.0，确保JSON格式正确
    注意：只修改目标目录中的文件，不修改原始文件
    """
    try:
        # 检查文件是否存在
        if not os.path.exists(file_path):
            logger.warning(f"文件不存在: {file_path}")
            return
        
        with open(file_path, "r", encoding="utf-8") as f:
            content = f.read()
            
        # 检查是否包含需要修复的占位符
        if '@' in content or '#' in content:
            # 使用更简单、更健壮的正则表达式来替换占位符
            # 替换所有[ @ , @ , @ ] 类型的占位符（考虑各种空白字符）
            content = re.sub(r'\[\s*@\s*,\s*@\s*,\s*@\s*\]', '[1.0, 1.0, 1.0]', content)
            # 替换所有[ # , # , # ] 类型的占位符
            content = re.sub(r'\[\s*#\s*,\s*#\s*,\s*#\s*\]', '[1.0, 1.0, 1.0]', content)
            
            # 写回文件
            with open(file_path, "w", encoding="utf-8") as f:
                f.write(content)
            
            logger.info(f"已修复JSON文件中的占位符: {file_path}")
    except Exception as e:
        logger.error(f"修复JSON文件占位符过程中出错: {str(e)}")


def process_big_items(temp_overlay_dir, settings):
    """
    处理放大物品功能
    - 检测big_item目录
    - 移动文件到正确位置
    - 处理放大倍数设置
    """
    try:
        # 获取big_item目录路径
        current_dir = os.getcwd()
        big_item_dir = os.path.join(current_dir, "big_item")
        
        # 检查big_item目录是否存在
        if not os.path.exists(big_item_dir):
            logger.warning("big_item目录不存在，跳过放大物品处理")
            return
        
        logger.info("开始处理放大物品...")
        
        # 确定目标目录
        target_dir = None
        if "workspace" in settings and os.path.exists(settings["workspace"].get("path", "")):
            # 有母材质包的情况
            workspace_path = settings["workspace"]["path"]
            target_dir = os.path.join(workspace_path, "assets", "minecraft", "models")
        else:
            # 没有母材质包的情况
            target_dir = os.path.join(temp_overlay_dir, "assets", "minecraft", "models")
        
        # 确保目标目录存在
        os.makedirs(target_dir, exist_ok=True)
        
        # 需要排除的文件
        excluded_files = ["block.json", "generated.json", "handheld.json", "shield.json", "shield_blocking.json"]
        
        # 存储放大倍数设置
        # 先读取overlay.json中已有的配置
        big_items_config = settings.get("big_items", {})
        # 为了兼容旧版本，也检查"big_item"字段
        if not big_items_config:
            big_items_config = settings.get("big_item", {})
        
        # 复制并修复的文件列表
        copied_files = []
        
        # 遍历big_item目录
        for item in os.listdir(big_item_dir):
            item_path = os.path.join(big_item_dir, item)
            
            # 处理compass_json文件夹
            if item == "compass_json" and os.path.isdir(item_path):
                # 遍历compass_json文件夹中的所有文件
                for compass_file in os.listdir(item_path):
                    compass_file_path = os.path.join(item_path, compass_file)
                    if os.path.isfile(compass_file_path) and compass_file.endswith(".json"):
                        # 先复制文件到目标目录
                        target_file_path = os.path.join(target_dir, compass_file)
                        shutil.copy2(compass_file_path, target_file_path)
                        copied_files.append(target_file_path)
                        logger.info(f"已复制指南针文件: {compass_file} 到 {target_dir}")
                

                continue
            
            # 处理普通JSON文件
            if os.path.isfile(item_path) and item.endswith(".json") and item not in excluded_files:
                # 先复制文件到目标目录，不修改原始文件
                target_file_path = os.path.join(target_dir, item)
                shutil.copy2(item_path, target_file_path)
                copied_files.append(target_file_path)
                logger.info(f"已复制文件: {item} 到 {target_dir}")
                

        
        # 复制完成后，修复目标目录中的所有文件
        for file_path in copied_files:
            fix_json_placeholders(file_path)
        
        # 应用放大倍数到JSON文件
        # 先检查目标目录是否存在
        if not os.path.exists(target_dir):
            logger.warning(f"目标目录不存在: {target_dir}")
            return
        
        # 检查目录中是否有JSON文件
        json_files = [f for f in os.listdir(target_dir) if f.endswith('.json')]
        if not json_files:
            logger.warning(f"目标目录中没有找到JSON文件: {target_dir}")
            return
        
        # 应用放大倍数到JSON文件
        apply_scale_to_json_files(target_dir, big_items_config)
        
        # 只有当有放大物品配置时，才保存到overlay.json
        if big_items_config:
            update_overlay_with_big_items_info(temp_overlay_dir, big_items_config)
        
    except Exception as e:
        logger.error(f"处理放大物品过程中出错: {str(e)}")
        raise

def apply_scale_to_json_files(target_dir, big_items_config):
    """
    应用放大倍数到JSON文件
    """
    try:
        # 遍历目标目录中的所有JSON文件
        for file_name in os.listdir(target_dir):
            if file_name.endswith(".json"):
                file_path = os.path.join(target_dir, file_name)
                
                # 获取物品名称（不包括扩展名）
                base_name = os.path.splitext(file_name)[0]
                
                # 特殊处理compass文件（带数字后缀的情况）
                if base_name.startswith("compass_") and any(char.isdigit() for char in base_name.split("_")[1]):
                    item_name = "compass"
                else:
                    # 对于其他文件，使用完整的基础名称
                    item_name = base_name
                
                # 检查是否有该物品的放大配置
                if item_name in big_items_config:
                    config = big_items_config[item_name]
                    handheld_scale = float(config["handheld_scale"].replace("x", ""))
                    dropped_scale = float(config["dropped_scale"].replace("x", ""))
                    
                    # 读取JSON文件
                    try:
                        with open(file_path, "r", encoding="utf-8") as f:
                            json_data = json.load(f)
                    except json.JSONDecodeError as e:
                        logger.error(f"文件 {file_name} 不是有效的JSON格式: {str(e)}")
                        # 尝试再次修复这个文件
                        fix_json_placeholders(file_path)
                        # 尝试重新读取
                        try:
                            with open(file_path, "r", encoding="utf-8") as f:
                                json_data = json.load(f)
                            logger.info(f"修复后成功加载文件: {file_name}")
                        except Exception as e2:
                            logger.error(f"修复后仍然无法加载文件 {file_name}: {str(e2)}")
                            continue  # 跳过这个文件
                    
                    # 更新scale值
                    if "display" in json_data:
                        # 确保thirdperson_righthand结构存在
                        if "thirdperson_righthand" not in json_data["display"]:
                            json_data["display"]["thirdperson_righthand"] = {}
                        if "scale" not in json_data["display"]["thirdperson_righthand"]:
                            json_data["display"]["thirdperson_righthand"]["scale"] = [1.0, 1.0, 1.0]
                        
                        # 更新手持状态（thirdperson_righthand）的scale
                        json_data["display"]["thirdperson_righthand"]["scale"] = [handheld_scale, handheld_scale, handheld_scale]
                        
                        # 确保ground结构存在
                        if "ground" not in json_data["display"]:
                            json_data["display"]["ground"] = {}
                        if "scale" not in json_data["display"]["ground"]:
                            json_data["display"]["ground"]["scale"] = [1.0, 1.0, 1.0]
                        
                        # 更新地面状态（ground）的scale
                        json_data["display"]["ground"]["scale"] = [dropped_scale, dropped_scale, dropped_scale]
                    
                    # 保存更新后的JSON文件（使用紧凑格式）
                    with open(file_path, "w", encoding="utf-8") as f:
                        json.dump(json_data, f, ensure_ascii=False, separators=(',', ':'))
                    
                    logger.info(f"已应用放大倍数到文件: {file_name}")
    except Exception as e:
        logger.error(f"应用放大倍数到JSON文件过程中出错: {str(e)}")
        raise

def update_overlay_with_big_items_info(temp_overlay_dir, big_items_config):
    """
    更新overlay.json文件，记录放大物品配置
    """
    try:
        overlay_file = os.path.join(temp_overlay_dir, "overlay.json")
        
        # 读取现有配置
        if os.path.exists(overlay_file):
            with open(overlay_file, "r", encoding="utf-8-sig") as f:
                settings = json.load(f)
        else:
            settings = {}
        
        # 添加放大物品配置
        settings["big_items"] = big_items_config
        
        # 保存更新后的配置
        with open(overlay_file, "w", encoding="utf-8-sig") as f:
            json.dump(settings, f, ensure_ascii=False, indent=2, separators=(',', ': '))
        
        logger.info(f"已更新overlay.json文件，添加放大物品配置")
        
    except Exception as e:
        logger.error(f"更新overlay.json文件过程中出错: {str(e)}")
        # 不抛出异常，继续执行

def process_small_items(temp_overlay_dir, settings):
    """
    处理缩小物品功能
    将small_item配置中指定的文件移动到对应目录
    """
    try:
        # 获取small_item配置
        small_items_config = settings.get("small_item", {})
        if not small_items_config:
            logger.info("没有找到small_item配置，跳过缩小物品处理")
            return
        
        # 获取big_item目录路径
        current_dir = os.getcwd()
        big_item_dir = os.path.join(current_dir, "big_item")
        
        # 确定目标目录
        target_dir = None
        if "workspace" in settings and os.path.exists(settings["workspace"].get("path", "")):
            # 有母材质包的情况
            workspace_path = settings["workspace"]["path"]
            target_dir = os.path.join(workspace_path, "assets", "minecraft", "textures", "models", "item")
        else:
            # 没有母材质包的情况
            target_dir = os.path.join(temp_overlay_dir, "assets", "minecraft", "textures", "models", "item")
        
        # 确保目标目录存在
        os.makedirs(target_dir, exist_ok=True)
        
        # 遍历small_item配置中的所有物品
        for item_name, item_config in small_items_config.items():
            if item_config.get("type") == "zoom_out" and item_config.get("should_shrink", False):
                # 构建源文件路径
                source_file = os.path.join(big_item_dir, f"{item_name}.json")
                
                # 检查源文件是否存在
                if os.path.exists(source_file):
                    # 构建目标文件路径
                    target_file = os.path.join(target_dir, f"{item_name}.json")
                    
                    # 移动文件（覆盖已存在的文件）
                    shutil.copy2(source_file, target_file)
                    logger.info(f"已复制缩小物品文件: {item_name}.json 到 {target_dir}")
                else:
                    logger.warning(f"未找到small_item配置中指定的文件: {item_name}.json")
        
    except Exception as e:
        logger.error(f"处理缩小物品过程中出错: {str(e)}")
        raise

# 修改start_overlay函数，在末尾调用process_big_items函数
def start_overlay():
    """
    开始制作覆盖包
    根据overlay.json中的parent_pack配置决定是解压母材质包还是创建默认文件夹结构
    """
    try:
        # 获取当前工作目录
        current_dir = os.getcwd()
        temp_overlay_dir = os.path.join(current_dir, "temp_overlay")
        overlay_file = os.path.join(temp_overlay_dir, "overlay.json")
        
        # 清空temp_overlay目录
        clean_temp_overlay_dir(temp_overlay_dir)
        
        # 读取设置（即使文件不存在也要创建一个默认设置对象）
        settings = {}
        if os.path.exists(overlay_file):
            try:
                with open(overlay_file, "r", encoding="utf-8-sig") as f:
                    settings = json.load(f)
            except json.JSONDecodeError:
                logger.error("overlay.json文件格式错误，使用默认设置")
        
        # 检查是否有parent_pack配置
        if "parent_pack" in settings and settings["parent_pack"].get("enabled", False):
            parent_pack_path = settings["parent_pack"].get("path", "")
            if parent_pack_path and os.path.exists(parent_pack_path):
                # 处理母材质包
                process_parent_pack(temp_overlay_dir, parent_pack_path)
                # 重新读取设置，因为处理母材质包可能更新了设置
                if os.path.exists(overlay_file):
                    with open(overlay_file, "r", encoding="utf-8-sig") as f:
                        settings = json.load(f)
            else:
                logger.warning("母材质包路径不存在，创建默认文件夹结构")
                create_default_structure(temp_overlay_dir)
        else:
            # 没有母材质包配置，创建默认文件夹结构
            create_default_structure(temp_overlay_dir)
        
        # 处理放大物品功能
        process_big_items(temp_overlay_dir, settings)
        
        # 处理缩小物品功能
        process_small_items(temp_overlay_dir, settings)
        
        # 处理core_outline相关配置
        process_core_outline(temp_overlay_dir, settings)
        
        # 处理core_shadow相关配置
        process_core_shadow(temp_overlay_dir, settings)

        # 处理自定义物品名称
        process_custom_names(temp_overlay_dir, settings)

        # 处理自定义附魔闪光
        process_custom_glint(temp_overlay_dir, settings)
            
    except Exception as e:
        logger.error(f"制作覆盖包过程中出错: {str(e)}")
        raise

def process_custom_names(temp_overlay_dir, settings):
    """
    处理自定义物品名称
    - 从overlay.json中读取selected_language和lang_itemname设置
    - 根据设置更新语言文件中的物品名称
    """
    try:
        # 检查是否有自定义名称配置
        if "lang_itemname" not in settings or not settings["lang_itemname"]:
            logger.info("没有检测到自定义物品名称配置，跳过处理")
            return
        
        logger.info("检测到自定义物品名称配置，开始处理")
        
        # 读取选择的语言
        selected_language = settings.get("selected_language", "zh_cn")
        lang_file = f"{selected_language}.json"
        
        # 获取原始语言文件路径
        current_dir = os.getcwd()
        source_lang_file = os.path.join(current_dir, "lang", lang_file)
        
        # 检查原始语言文件是否存在
        if not os.path.exists(source_lang_file):
            logger.warning(f"原始语言文件不存在: {source_lang_file}")
            return
        
        # 确定目标目录
        target_dir = None
        if "workspace" in settings and os.path.exists(settings["workspace"].get("path", "")):
            # 有母材质包的情况
            workspace_path = settings["workspace"]["path"]
            target_dir = os.path.join(workspace_path, "assets", "minecraft", "lang")
        else:
            # 没有母材质包的情况
            target_dir = os.path.join(temp_overlay_dir, "assets", "minecraft", "lang")
        
        # 确保目标目录存在
        os.makedirs(target_dir, exist_ok=True)
        
        # 构建目标语言文件路径
        target_lang_file = os.path.join(target_dir, lang_file)
        
        # 读取原始语言文件
        with open(source_lang_file, "r", encoding="utf-8-sig") as f:
            lang_data = json.load(f)
        
        # 读取自定义名称配置
        custom_names = settings["lang_itemname"]
        
        # 替换物品名称
        for item_id, custom_name in custom_names.items():
            if item_id in lang_data:
                # 保留原始名称的键，只替换值
                original_name = lang_data[item_id]
                lang_data[item_id] = custom_name
                logger.info(f"已替换物品名称: {item_id} - {original_name} -> {custom_name}")
            else:
                logger.warning(f"在语言文件中未找到物品ID: {item_id}")
        
        # 保存更新后的语言文件（使用紧凑格式）
        with open(target_lang_file, "w", encoding="utf-8-sig") as f:
            json.dump(lang_data, f, ensure_ascii=False, separators=(',', ':'))
        
        logger.info(f"已保存更新后的语言文件到: {target_lang_file}")
        
    except Exception as e:
        logger.error(f"处理自定义物品名称过程中出错: {str(e)}")
        raise

def process_core_outline(temp_overlay_dir, settings):
    """
    处理core_outline相关配置
    - 检测overlay.json中的core_outline配置
    - 根据配置移动相应文件并替换占位符
    """
    try:
        current_dir = os.getcwd()
        
        # 确定目标目录
        target_dir = None
        if "workspace" in settings and os.path.exists(settings["workspace"].get("path", "")):
            # 有母材质包的情况
            workspace_path = settings["workspace"]["path"]
            # 获取材质包名称（workspace_path的最后一级目录）
            texture_pack_name = os.path.basename(workspace_path)
            target_dir = os.path.join(workspace_path, "assets", "minecraft", "shaders", "core")
        else:
            # 没有母材质包的情况
            target_dir = os.path.join(temp_overlay_dir, "assets", "minecraft", "shaders", "core")
        
        # 确保目标目录存在
        os.makedirs(target_dir, exist_ok=True)
        
        # 检查是否有core_outline_rainbow_hexian配置（和弦彩虹边框）
        if "core_outline_rainbow_hexian" in settings and settings["core_outline_rainbow_hexian"].get("enabled", False):
            logger.info("检测到core_outline_rainbow_hexian配置，使用和弦彩虹边框")
            source_dir = os.path.join(current_dir, "core_rainbow_outline_hexian")

            # 直接复制所有文件到目标目录
            for file_name in os.listdir(source_dir):
                source_file = os.path.join(source_dir, file_name)
                if os.path.isfile(source_file):
                    target_file = os.path.join(target_dir, file_name)
                    shutil.copy2(source_file, target_file)
                    logger.info(f"已复制和弦彩虹边框文件: {file_name} 到 {target_dir}")
        # 检查是否有core_outline_rainbow配置
        elif "core_outline_rainbow" in settings and settings["core_outline_rainbow"].get("enabled", False):
            logger.info("检测到core_outline_rainbow配置，使用彩虹边框")
            source_dir = os.path.join(current_dir, "core_rainbow_outline")

            # 直接复制所有文件到目标目录
            for file_name in os.listdir(source_dir):
                source_file = os.path.join(source_dir, file_name)
                if os.path.isfile(source_file):
                    target_file = os.path.join(target_dir, file_name)
                    shutil.copy2(source_file, target_file)
                    logger.info(f"已复制彩虹边框文件: {file_name} 到 {target_dir}")
        elif "core_outline" in settings and settings["core_outline"].get("enabled", False):
            logger.info("检测到core_outline配置，使用自定义颜色边框")
            source_dir = os.path.join(current_dir, "core_outline")
            outline_settings = settings["core_outline"]
            
            # 获取颜色和粗细设置
            color = outline_settings.get("color", {"r": 1.0, "g": 1.0, "b": 1.0, "a": 1.0})
            thickness = outline_settings.get("thickness", 2.0)
            
            # 复制文件到目标目录并替换占位符
            for file_name in os.listdir(source_dir):
                source_file = os.path.join(source_dir, file_name)
                target_file = os.path.join(target_dir, file_name)
                
                if os.path.isfile(source_file):
                    shutil.copy2(source_file, target_file)
                    logger.info(f"已复制边框文件: {file_name} 到 {target_dir}")
                    
                    # 根据文件类型处理占位符
                    if file_name.endswith(".vsh") or file_name.endswith(".fsh"):
                        # 处理着色器文件
                        with open(target_file, "r", encoding="utf-8") as f:
                            content = f.read()
                        
                        # 替换颜色占位符
                        rgba_str = f"{color['r']}, {color['g']}, {color['b']}, {color['a']}"
                        content = content.replace("vec4(#, #, #, #)", f"vec4({rgba_str})")
                        content = content.replace("vec4(%, %, %, %)", f"vec4({rgba_str})")
                        content = content.replace("vec4(&, &, &, &)", f"vec4({rgba_str})")
                        content = content.replace("vec4(*, *, *, *)", f"vec4({rgba_str})")
                        
                        # 只在vsh文件中替换线条宽度（彩色边框不需要调整粗细）
                        if file_name.endswith(".vsh"):
                            content = content.replace("#define BORDER_LINE_WIDTH @", f"#define BORDER_LINE_WIDTH {thickness}")
                        
                        # 写回文件
                        with open(target_file, "w", encoding="utf-8") as f:
                            f.write(content)
                        logger.info(f"已替换{file_name}中的占位符")
                    elif file_name.endswith(".json"):
                        # 处理JSON文件
                        with open(target_file, "r", encoding="utf-8") as f:
                            content = f.read()
                        
                        # 替换颜色占位符
                        rgba_str = f"{color['r']}, {color['g']}, {color['b']}, {color['a']}"
                        content = content.replace("[#, #, #, #]", f"[{rgba_str}]")
                        content = content.replace("[%, %, %, %]", f"[{rgba_str}]")
                        content = content.replace("[&, &, &, &]", f"[{rgba_str}]")
                        content = content.replace("[*, *, *, *]", f"[{rgba_str}]")
                        
                        # 替换线条宽度占位符
                        content = content.replace("[@]", f"[{thickness}]")
                        
                        # 写回文件
                        with open(target_file, "w", encoding="utf-8") as f:
                            f.write(content)
                        logger.info(f"已替换{file_name}中的占位符")
    except Exception as e:
        logger.error(f"处理core_outline过程中出错: {str(e)}")
        raise

def process_core_shadow(temp_overlay_dir, settings):
    """
    处理core_shadow相关配置
    - 检测overlay.json中的core_shadow配置
    - 根据配置移动core_inventory中的文件到对应目录
    """
    try:
        current_dir = os.getcwd()
        
        # 检查是否有core_shadow配置
        if "core_shadow" not in settings or not settings["core_shadow"].get("enabled", False):
            logger.info("没有检测到启用的core_shadow配置，跳过处理")
            return
        
        logger.info("检测到core_shadow配置，开始处理")
        
        # 确定目标目录
        target_dir = None
        if "workspace" in settings and os.path.exists(settings["workspace"].get("path", "")):
            # 有母材质包的情况
            workspace_path = settings["workspace"]["path"]
            target_dir = os.path.join(workspace_path, "assets", "minecraft", "shaders", "core")
        else:
            # 没有母材质包的情况
            target_dir = os.path.join(temp_overlay_dir, "assets", "minecraft", "shaders", "core")
        
        # 确保目标目录存在
        os.makedirs(target_dir, exist_ok=True)
        
        # 获取core_inventory目录路径
        core_inventory_dir = os.path.join(current_dir, "core_inventory")
        
        # 检查core_inventory目录是否存在
        if not os.path.exists(core_inventory_dir):
            logger.warning("core_inventory目录不存在")
            return
        
        # 复制core_inventory中的所有文件到目标目录
        for file_name in os.listdir(core_inventory_dir):
            source_file = os.path.join(core_inventory_dir, file_name)
            if os.path.isfile(source_file):
                target_file = os.path.join(target_dir, file_name)
                shutil.copy2(source_file, target_file)
                logger.info(f"已复制core_inventory文件: {file_name} 到 {target_dir}")
        
    except Exception as e:
        logger.error(f"处理core_shadow过程中出错: {str(e)}")
        raise

def process_custom_glint(temp_overlay_dir, settings):
    """
    处理自定义附魔闪光
    - 检测overlay.json中的custom_glint配置
    - 复制enchanted_item_glint.png到目标位置
    """
    try:
        if not settings.get("custom_glint", False):
            logger.info("没有检测到custom_glint配置，跳过处理")
            return

        logger.info("检测到custom_glint配置，开始处理")

        current_dir = os.getcwd()

        # 确定目标目录
        target_dir = None
        if "workspace" in settings and os.path.exists(settings["workspace"].get("path", "")):
            workspace_path = settings["workspace"]["path"]
            target_dir = os.path.join(workspace_path, "assets", "minecraft", "textures", "misc")
        else:
            target_dir = os.path.join(temp_overlay_dir, "assets", "minecraft", "textures", "misc")

        os.makedirs(target_dir, exist_ok=True)

        # 尝试从多个位置查找附魔闪光源文件
        source_dirs = [
            os.path.join(current_dir, "enchanted_item_glint"),
            os.path.join(current_dir, "overlay", "enchanted_item_glint"),
            os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "overlay", "enchanted_item_glint"),
        ]
        source_file = None
        for src_dir in source_dirs:
            candidate = os.path.join(src_dir, "enchanted_item_glint.png")
            if os.path.exists(candidate):
                source_file = candidate
                break

        if source_file and os.path.exists(source_file):
            target_file = os.path.join(target_dir, "enchanted_item_glint.png")
            shutil.copy2(source_file, target_file)
            logger.info(f"已复制附魔闪光文件到: {target_file}")
        else:
            logger.warning("附魔闪光源文件不存在，请确认enchanted_item_glint目录存在")

    except Exception as e:
        logger.error(f"处理custom_glint过程中出错: {str(e)}")
        raise

def package_overlay_resource_pack():
    """
    封装覆盖包为可使用的资源包
    根据overlay.json中的parent_pack配置决定封装方式
    """
    try:
        # 获取当前工作目录
        current_dir = os.getcwd()
        temp_overlay_dir = os.path.join(current_dir, "temp_overlay")
        overlay_file = os.path.join(temp_overlay_dir, "overlay.json")
        
        # 检查overlay.json文件是否存在
        if not os.path.exists(overlay_file):
            logger.error("overlay.json文件不存在，无法封装覆盖包")
            return
        
        # 读取overlay.json文件
        with open(overlay_file, "r", encoding="utf-8-sig") as f:
            settings = json.load(f)
        
        # 检查是否有parent_pack配置
        if "parent_pack" in settings and settings["parent_pack"].get("enabled", False) and "workspace" in settings:
            # 有母材质包的情况
            workspace_path = settings["workspace"].get("path", "")
            if workspace_path and os.path.exists(workspace_path):
                # 获取原始材质包名称
                original_pack_name = os.path.basename(workspace_path)
                # 创建新的压缩包名称
                new_pack_name = f"[覆盖]{original_pack_name}.zip"
                # 压缩workspace中的材质包
                zip_file_path = os.path.join(temp_overlay_dir, new_pack_name)
                
                # 创建压缩文件
                with zipfile.ZipFile(zip_file_path, 'w', zipfile.ZIP_DEFLATED) as zipf:
                    for root, dirs, files in os.walk(workspace_path):
                        for file in files:
                            file_path = os.path.join(root, file)
                            # 计算相对路径，确保文件结构正确
                            arcname = os.path.relpath(file_path, os.path.dirname(workspace_path))
                            zipf.write(file_path, arcname)
                
                logger.info(f"已将workspace中的材质包压缩为: {zip_file_path}")
                return zip_file_path
        else:
            # 没有母材质包的情况
            # 创建pack.mcmeta文件
            pack_mcmeta_path = os.path.join(temp_overlay_dir, "pack.mcmeta")
            mcmeta_content = {
                "pack": {
                    "pack_format": 15,
                    "description": "你的覆盖包"
                }
            }
            with open(pack_mcmeta_path, "w", encoding="utf-8") as f:
                json.dump(mcmeta_content, f, ensure_ascii=False, separators=(',', ':'))
            
            logger.info(f"已创建pack.mcmeta文件: {pack_mcmeta_path}")
            
            # 压缩assets目录和pack.mcmeta文件
            zip_file_path = os.path.join(temp_overlay_dir, "你的覆盖包.zip")
            
            # 创建压缩文件
            with zipfile.ZipFile(zip_file_path, 'w', zipfile.ZIP_DEFLATED) as zipf:
                # 添加pack.mcmeta文件
                zipf.write(pack_mcmeta_path, "pack.mcmeta")
                
                # 添加assets目录
                assets_dir = os.path.join(temp_overlay_dir, "assets")
                if os.path.exists(assets_dir):
                    for root, dirs, files in os.walk(assets_dir):
                        for file in files:
                            file_path = os.path.join(root, file)
                            # 计算相对路径，确保文件结构正确
                            arcname = os.path.relpath(file_path, temp_overlay_dir)
                            zipf.write(file_path, arcname)
            
            logger.info(f"已压缩覆盖包文件: {zip_file_path}")
            
            # 删除临时创建的pack.mcmeta文件
            if os.path.exists(pack_mcmeta_path):
                os.remove(pack_mcmeta_path)
                logger.info(f"已删除临时文件: {pack_mcmeta_path}")
            
            return zip_file_path
        
    except Exception as e:
        logger.error(f"封装覆盖包过程中出错: {str(e)}")
        raise

if __name__ == "__main__":
    # 直接运行时测试功能
    start_overlay()
    package_overlay_resource_pack()