#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
KPT Bedrock Converter Python Interface
鍩哄博鐗堣浆鎹㈠櫒Python鎺ュ彛
"""

import os
import sys
import json
import zipfile
import tempfile
import shutil
import uuid
import logging
from pathlib import Path
from typing import Optional, Tuple

# 閰嶇疆鏃ュ織
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

class BedrockConverterInterface:
    """鍩哄博鐗堣浆鎹㈠櫒Python鎺ュ彛"""
    
    def __init__(self, output_dir: str = "converted_packs"):
        self.temp_dir = Path("temp_bedrock_conversion")
        self.output_dir = Path(output_dir)
        self.temp_dir.mkdir(exist_ok=True)
        self.output_dir.mkdir(exist_ok=True)
    
    def generate_random_uuid(self) -> str:
        """鐢熸垚闅忔満UUID瀛楃涓诧紙8-4-4-4-12鏍煎紡锛?""
        return str(uuid.uuid4())
    
    def get_original_pack_name(self, zip_path: str) -> str:
        """鑾峰彇鍘熷鏉愯川鍖呭悕绉?""
        stem = Path(zip_path).stem
        # 濡傛灉鏂囦欢鍚嶄互"[Java 1.21.11]"寮€澶达紝鍒欏幓鎺夎繖涓墠缂€
        if stem.startswith("[Java 1.21.11]"):
            return stem[len("[Java 1.21.11]"):]
        return stem
    
    def read_pack_mcmeta(self, zip_path: str) -> dict:
        """璇诲彇Java鐗堟潗璐ㄥ寘鐨刾ack.mcmeta鏂囦欢"""
        try:
            with zipfile.ZipFile(zip_path, 'r') as zip_file:
                if 'pack.mcmeta' in zip_file.namelist():
                    with zip_file.open('pack.mcmeta') as mcmeta_file:
                        content = mcmeta_file.read().decode('utf-8')
                        return json.loads(content)
                else:
                    logger.warning("pack.mcmeta鏂囦欢鏈壘鍒帮紝浣跨敤榛樿鎻忚堪")
                    return {"pack": {"description": "KPT Converted"}}
        except Exception as e:
            logger.error(f"璇诲彇pack.mcmeta澶辫触: {e}")
            return {"pack": {"description": "KPT Converted"}}
    
    def create_manifest_json(self, pack_name: str, original_description: str) -> dict:
        """鍒涘缓manifest.json鏂囦欢鍐呭"""
        return {
            "format_version": 2,
            "header": {
                "description": original_description,
                "name": pack_name,
                "uuid": self.generate_random_uuid(),
                "version": [1, 0, 0],
                "min_engine_version": [1, 16, 2]
            },
            "modules": [
                {
                    "description": original_description,
                    "type": "resources",
                    "uuid": self.generate_random_uuid(),
                    "version": [1, 0, 0]
                }
            ]
        }
    
    def reorganize_file_structure(self, extract_dir: Path) -> bool:
        """鏂囦欢缁撴瀯閲嶆帓锛氭彁鍗囩洰褰曞眰绾у苟閲嶅懡鍚嶆枃浠?""
        try:
            minecraft_dir = extract_dir / "assets" / "minecraft"
            
            if not minecraft_dir.exists():
                logger.error("minecraft鐩綍鏈壘鍒?)
                return False
            
            # 1. 灏唒ack.png閲嶅懡鍚嶄负pack_icon.png
            pack_png = extract_dir / "pack.png"
            if pack_png.exists():
                pack_icon = extract_dir / "pack_icon.png"
                pack_png.rename(pack_icon)
                logger.info("宸插皢pack.png閲嶅懡鍚嶄负pack_icon.png")
            
            # 2. 鎻愬崌minecraft/textures/font鍒颁竴绾х洰褰?
            textures_font_dir = minecraft_dir / "textures" / "font"
            if textures_font_dir.exists():
                target_font_dir = extract_dir / "font"
                if target_font_dir.exists():
                    shutil.rmtree(target_font_dir)
                shutil.move(str(textures_font_dir), str(target_font_dir))
                logger.info("宸叉彁鍗噁ont鐩綍鍒颁竴绾х洰褰?)
            
            # 3. 鎻愬崌minecraft/textures鍒颁竴绾х洰褰?
            textures_dir = minecraft_dir / "textures"
            if textures_dir.exists():
                target_textures_dir = extract_dir / "textures"
                if target_textures_dir.exists():
                    # 绉诲姩鏂囦欢鑰屼笉鏄浛鎹㈡暣涓洰褰?
                    self._merge_directories(textures_dir, target_textures_dir)
                    shutil.rmtree(textures_dir)
                else:
                    shutil.move(str(textures_dir), str(target_textures_dir))
                logger.info("宸叉彁鍗噒extures鐩綍鍒颁竴绾х洰褰?)
            
            # 4. 鎵ц鏂扮殑鏂囦欢閲嶆帓鎿嶄綔
            if not self._perform_additional_reorganization(extract_dir):
                return False
            
            # 5. 鍒犻櫎绌虹殑assets鐩綍
            assets_dir = extract_dir / "assets"
            if assets_dir.exists():
                try:
                    assets_dir.rmdir()  # 鍙湁鍦ㄧ洰褰曚负绌烘椂鎵嶈兘鍒犻櫎
                    logger.info("宸插垹闄ょ┖鐨刟ssets鐩綍")
                except OSError:
                    # 鐩綍涓嶄负绌猴紝妫€鏌ユ槸鍚﹀彧鍖呭惈minecraft
                    contents = list(assets_dir.iterdir())
                    if len(contents) == 1 and contents[0].name == "minecraft":
                        shutil.rmtree(contents[0])  # 鍒犻櫎minecraft鐩綍
                        assets_dir.rmdir()  # 鍒犻櫎绌虹殑assets鐩綍
                        logger.info("宸插垹闄ょ┖鐨刟ssets鍜宮inecraft鐩綍")
            
            return True
            
        except Exception as e:
            logger.error(f"鏂囦欢缁撴瀯閲嶆帓澶辫触: {e}")
            return False
    
    def _merge_directories(self, src: Path, dst: Path):
        """鍚堝苟鐩綍锛堜繚鐣欑洰鏍囩洰褰曚腑鐨勬枃浠讹級"""
        dst.mkdir(parents=True, exist_ok=True)
        
        for item in src.iterdir():
            src_item = src / item.name
            dst_item = dst / item.name
            
            if src_item.is_dir():
                if dst_item.exists():
                    self._merge_directories(src_item, dst_item)
                else:
                    shutil.move(str(src_item), str(dst_item))
            else:
                if not dst_item.exists():
                    shutil.copy2(src_item, dst_item)
    
    def convert_to_bedrock(self, java_pack_path: str, progress_callback=None) -> Tuple[bool, str]:
        """杞崲Java鐗堟潗璐ㄥ寘涓哄熀宀╃増鏍煎紡"""
        try:
            # 鑾峰彇杈撳叆鏂囦欢鐨勭洰褰曚綔涓鸿緭鍑虹洰褰?
            input_dir = Path(java_pack_path).parent
            pack_name = self.get_original_pack_name(java_pack_path)
            logger.info(f"寮€濮嬭浆鎹㈡潗璐ㄥ寘: {pack_name}")
            
            # 鏇存柊杩涘害锛氬紑濮嬭浆鎹?
            if progress_callback:
                progress_callback(0)
            
            # 1. 鍒涘缓涓存椂瑙ｅ帇鐩綍
            extract_dir = self.temp_dir / "extracted"
            if extract_dir.exists():
                shutil.rmtree(extract_dir)
            extract_dir.mkdir(parents=True)
            
            # 2. 瑙ｅ帇Java鐗堟潗璐ㄥ寘
            with zipfile.ZipFile(java_pack_path, 'r') as zip_file:
                zip_file.extractall(extract_dir)
                logger.info("宸茶В鍘婮ava鐗堟潗璐ㄥ寘")
            
            # 鏇存柊杩涘害锛氳В鍘嬪畬鎴?
            if progress_callback:
                progress_callback(20)
            
            # 3. 璇诲彇鍘熷pack.mcmeta鏂囦欢
            mcmeta = self.read_pack_mcmeta(java_pack_path)
            original_description = mcmeta.get("pack", {}).get("description", "KPT Converted")
            
            # 4. 鎵ц鏂囦欢缁撴瀯閲嶆帓
            if not self.reorganize_file_structure(extract_dir):
                return False, "鏂囦欢缁撴瀯閲嶆帓澶辫触"
            
            # 鏇存柊杩涘害锛氭枃浠剁粨鏋勯噸鎺掑畬鎴?
            if progress_callback:
                progress_callback(50)
            
            # 5. 鍒涘缓manifest.json
            manifest = self.create_manifest_json(pack_name, original_description)
            manifest_path = extract_dir / "manifest.json"
            with open(manifest_path, 'w', encoding='utf-8') as f:
                json.dump(manifest, f, indent=4, ensure_ascii=False)
            logger.info("宸插垱寤簃anifest.json")
            
            # 鏇存柊杩涘害锛歮anifest鍒涘缓瀹屾垚
            if progress_callback:
                progress_callback(70)
            
            # 6. 閲嶆柊鎵撳寘涓哄熀宀╃増鏍煎紡骞堕噸鍛藉悕涓簃cpack
            output_name = f"[Bedrock-Latest]{pack_name}"
            zip_output_path = input_dir / f"{output_name}.zip"
            mcpack_output_path = input_dir / f"{output_name}.mcpack"
            
            if zip_output_path.exists():
                zip_output_path.unlink()
            if mcpack_output_path.exists():
                mcpack_output_path.unlink()
            
            # 鎵撳寘涓存椂瑙ｅ帇鐩綍鐨勫唴瀹瑰埌zip鏂囦欢锛堟牴鐩綍鏄叿浣撴枃浠讹紝涓嶆槸鏂囦欢澶癸級
            with zipfile.ZipFile(zip_output_path, 'w', zipfile.ZIP_DEFLATED) as zip_file:
                for root, dirs, files in os.walk(extract_dir):
                    for file in files:
                        file_path = Path(root) / file
                        arcname = file_path.relative_to(extract_dir)  # 浣跨敤鐩稿璺緞浣滀负zip鍐呯殑璺緞
                        zip_file.write(file_path, arcname)
            
            # 鏇存柊杩涘害锛氭墦鍖呭畬鎴?
            if progress_callback:
                progress_callback(90)
            
            # 閲嶅懡鍚嶄负mcpack鏂囦欢锛堟湰璐ㄤ笂浠嶇劧鏄痾ip鏍煎紡锛?
            zip_output_path.rename(mcpack_output_path)
            
            logger.info(f"鉁?鎴愬姛鐢熸垚鍩哄博鐗堟潗璐ㄥ寘: {mcpack_output_path}")
            
            # 7. 娓呯悊涓存椂鐩綍
            shutil.rmtree(extract_dir)
            
            # 鏇存柊杩涘害锛氳浆鎹㈠畬鎴?
            if progress_callback:
                progress_callback(100)
            
            return True, str(mcpack_output_path)
            
        except Exception as e:
            logger.error(f"杞崲澶辫触: {e}")
            return False, f"杞崲澶辫触: {str(e)}"
    
    def cleanup(self):
        """娓呯悊涓存椂鏂囦欢"""
        if self.temp_dir.exists():
            shutil.rmtree(self.temp_dir)
        logger.info("宸叉竻鐞嗕复鏃舵枃浠?)

    def _perform_additional_reorganization(self, extract_dir: Path) -> bool:
        """鎵ц棰濆鐨勬枃浠堕噸鎺掓搷浣?""
        try:
            # 鑾峰彇鑴氭湰鎵€鍦ㄧ洰褰曪紝鐢ㄤ簬璁块棶java_ui鏂囦欢澶?
            script_dir = Path(__file__).parent
            
            # 姝ラ1: 灏?textures涓殑block閲嶅懡鍚嶄负block锛宨tem閲嶅懡鍚嶄负items
            textures_dir = extract_dir / "textures"
            if textures_dir.exists():
                # 澶勭悊block鏂囦欢澶归噸鍛藉悕
                block_dir = textures_dir / "block"
                if block_dir.exists():
                    # 閲嶅懡鍚嶄负block (淇濇寔涓嶅彉锛屼絾纭繚瀛樺湪)
                    logger.info("纭block鐩綍瀛樺湪")
                
                # 澶勭悊鐗╁搧鏂囦欢澶癸紝纭繚鏈€缁堝悕绉颁负items
                items_dir = textures_dir / "items"
                item_dir = textures_dir / "item"
                
                # 濡傛灉items鏂囦欢澶逛笉瀛樺湪锛屼絾item鏂囦欢澶瑰瓨鍦紝灏嗗叾閲嶅懡鍚嶄负items
                if not items_dir.exists() and item_dir.exists():
                    items_dir = textures_dir / "items"
                    if items_dir.exists():
                        shutil.rmtree(items_dir)
                    shutil.move(str(item_dir), str(items_dir))
                    logger.info("宸插皢/textures/item閲嶅懡鍚嶄负/textures/items")
                
            # 姝ラ1.1: 灏唅tems涓叏閮╣olden寮€澶寸殑鐗╁搧鍚嶇О鏀逛负gold寮€澶达紝wooden涓殑鏀规垚wood寮€澶?
            items_dir = textures_dir / "items"
            if items_dir.exists():
                renamed_count = 0
                for item_file in items_dir.iterdir():
                    if item_file.is_file():
                        old_name = item_file.stem
                        new_name = old_name
                        # 鍙湁golden_apple.png闇€瑕佹敼涓篴pple_golden.png
                        if old_name == "golden_apple":
                            new_name = "apple_golden"
                        # 鍏朵粬golden_寮€澶寸殑鏀逛负gold_
                        elif old_name.startswith("golden_"):
                            new_name = f"gold_{old_name[len('golden_'):]}"
                        # 灏唚ooden_寮€澶寸殑鏀逛负wood_
                        elif old_name.startswith("wooden_"):
                            new_name = f"wood_{old_name[len('wooden_'):]}"
                        
                        if new_name != old_name:
                            new_file_path = items_dir / f"{new_name}{item_file.suffix}"
                            # 濡傛灉鏂版枃浠跺悕宸插瓨鍦紝鍏堝垹闄?
                            if new_file_path.exists():
                                new_file_path.unlink()
                            # 閲嶅懡鍚嶆枃浠?
                            item_file.rename(new_file_path)
                            renamed_count += 1
                            logger.info(f"宸插皢鐗╁搧鏂囦欢 {old_name}{item_file.suffix} 閲嶅懡鍚嶄负 {new_name}{item_file.suffix}")
                if renamed_count > 0:
                    logger.info(f"宸插畬鎴?{renamed_count} 涓墿鍝佹枃浠剁殑閲嶅懡鍚?)
            
            # 姝ラ2: 灏?textures/gui涓嬬殑container鏂囦欢澶瑰鍒跺埌鍜実ui鍚屼竴绾х洰褰曪紝鍛藉悕涓簎i
            gui_dir = textures_dir / "gui"
            container_dir = gui_dir / "container"
            if container_dir.exists():
                ui_dir = textures_dir / "ui"
                if ui_dir.exists():
                    shutil.rmtree(ui_dir)
                shutil.copytree(str(container_dir), str(ui_dir))
                logger.info("宸插皢/textures/gui/container澶嶅埗鍒?textures/ui")
            
            # 姝ラ3: 灏嗘柊鐨剈i鏂囦欢澶逛腑creative_inventory鍐呭鎻愬彇鍑烘潵锛屾斁鍒皍i鐩綍涓嬶紝鐒跺悗鍒犻櫎creative_inventory鐩綍
            ui_dir = textures_dir / "ui"
            creative_inventory_dir = ui_dir / "creative_inventory"
            if creative_inventory_dir.exists():
                # 鎻愬彇creative_inventory鍐呭鍒皍i鐩綍
                for item in creative_inventory_dir.iterdir():
                    src_item = creative_inventory_dir / item.name
                    dst_item = ui_dir / item.name
                    if dst_item.exists():
                        shutil.rmtree(dst_item)
                    shutil.move(str(src_item), str(dst_item))
                
                # 鍒犻櫎creative_inventory鐩綍
                shutil.rmtree(creative_inventory_dir)
                logger.info("宸插皢ui/creative_inventory鍐呭鎻愬彇鍒皍i鐩綍骞跺垹闄reative_inventory鐩綍")
            
            # 姝ラ4: 灏唈ava_ui鐩綍涓嬬殑ui鏂囦欢澶瑰鍒跺埌bedrock鏉愯川鍖呮牴鐩綍
            java_ui_dir = script_dir / "java_ui"
            java_ui_folder = java_ui_dir / "ui"
            if java_ui_folder.exists():
                target_ui_dir = extract_dir / "ui"
                if target_ui_dir.exists():
                    shutil.rmtree(target_ui_dir)
                shutil.copytree(str(java_ui_folder), str(target_ui_dir))
                logger.info("宸插皢java_ui/ui鏂囦欢澶瑰鍒跺埌bedrock鏉愯川鍖呮牴鐩綍")
            
            # 姝ラ5: 灏唈ava_ui/textures/ui鏂囦欢澶逛腑鐨勫叏閮ㄦ枃浠跺鍒跺埌/textures/ui涓嬶紝寮鸿鏇挎崲
            java_ui_textures_dir = java_ui_dir / "textures" / "ui"
            if java_ui_textures_dir.exists():
                target_textures_ui_dir = textures_dir / "ui"
                # 纭繚鐩爣鐩綍瀛樺湪
                target_textures_ui_dir.mkdir(parents=True, exist_ok=True)
                
                # 澶嶅埗java_ui/textures/ui涓殑鎵€鏈夋枃浠跺埌/textures/ui锛屽己琛屾浛鎹?
                for item in java_ui_textures_dir.iterdir():
                    src_item = java_ui_textures_dir / item.name
                    dst_item = target_textures_ui_dir / item.name
                    
                    if dst_item.exists():
                        if dst_item.is_dir():
                            shutil.rmtree(dst_item)
                        else:
                            dst_item.unlink()
                    
                    if src_item.is_dir():
                        shutil.copytree(str(src_item), str(dst_item))
                    else:
                        shutil.copy2(str(src_item), str(dst_item))
                
                logger.info("宸插皢java_ui/textures/ui涓殑鍏ㄩ儴鏂囦欢澶嶅埗鍒?textures/ui锛堝己琛屾浛鎹級")
            
            # 姝ラ6: 灏唈ava_ui涓嬬殑gui涓殑container,spirits鏂囦欢澶瑰鍒跺埌/textures/gui涓嬶紝寮鸿鏇挎崲
            java_gui_dir = java_ui_dir / "gui"
            java_container_dir = java_gui_dir / "container"
            java_sprites_dir = java_gui_dir / "sprites"
            target_gui_dir = textures_dir / "gui"
            
            # 纭繚鐩爣鐩綍瀛樺湪
            target_gui_dir.mkdir(parents=True, exist_ok=True)
            
            # 澶嶅埗container鏂囦欢澶癸紙寮鸿鏇挎崲锛?
            if java_container_dir.exists():
                target_container_dir = target_gui_dir / "container"
                if target_container_dir.exists():
                    shutil.rmtree(target_container_dir)
                shutil.copytree(str(java_container_dir), str(target_container_dir))
                logger.info("宸插皢java_ui/gui/container澶嶅埗鍒?textures/gui/container锛堝己琛屾浛鎹級")
            
            # 澶嶅埗sprites鏂囦欢澶癸紙寮鸿鏇挎崲锛?
            if java_sprites_dir.exists():
                target_sprites_dir = target_gui_dir / "sprites"
                if target_sprites_dir.exists():
                    shutil.rmtree(target_sprites_dir)
                shutil.copytree(str(java_sprites_dir), str(target_sprites_dir))
                logger.info("宸插皢java_ui/gui/sprites澶嶅埗鍒?textures/gui/sprites锛堝己琛屾浛鎹級")
            
            return True
            
        except Exception as e:
            logger.error(f"鎵ц棰濆鏂囦欢閲嶆帓鎿嶄綔澶辫触: {e}")
            return False

# 渚垮埄鍑芥暟
def convert_java_to_bedrock(java_pack_path: str) -> Tuple[bool, str]:
    """杞崲Java鐗堟潗璐ㄥ寘涓哄熀宀╃増鏍煎紡锛堜究鍒╁嚱鏁帮級"""
    converter = BedrockConverterInterface()
    try:
        success, result = converter.convert_to_bedrock(java_pack_path)
        return success, result
    finally:
        converter.cleanup()

if __name__ == "__main__":
    # 娴嬭瘯浠ｇ爜
    if len(sys.argv) > 1:
        java_pack_path = sys.argv[1]
        success, result = convert_java_to_bedrock(java_pack_path)
        if success:
            print(f"鉁?杞崲鎴愬姛: {result}")
        else:
            print(f"鉂?杞崲澶辫触: {result}")
    else:
        print("鐢ㄦ硶: python bedrock_converter.py <java_pack_path>")

