//! 基岩版（Bedrock）转换 —— 规格来自 `doc/`（Python 时代遗留设计，
//! 见 doc/bedrock_converter.py 与 doc/rust/）：
//!
//! 流程分两阶段：
//!   1. Java 阶段：先把包按普通流程转换到 1.21.11（pack_format 75），
//!      （在 version_converter::process_zip 中分流执行）
//!   2. Bedrock 阶段（本模块）：结构重组 + manifest.json + .mcpack 打包
//!
//! 结构重组步骤（与 Python 版对齐）：
//!   * pack.png → pack_icon.png
//!   * assets/minecraft/textures/font → font/（一级目录）
//!   * assets/minecraft/textures → textures/（合并式提升）
//!   * textures/item → textures/items（items 不存在时）
//!   * items 改名：golden_apple→apple_golden，golden_*→gold_*，wooden_*→wood_*
//!   * textures/gui/container → textures/ui
//!   * textures/ui/creative_inventory/* 提取到 textures/ui 后删除该目录
//!   * java_ui 模板（ui/ textures/ui/ gui/container gui/sprites）复制替换
//!     —— 模板目录位于 doc/java_ui（设计文档同目录），运行时按
//!     资源解析策略查找（exe 同级 java_ui/ 亦可）；缺失时跳过
//!   * 清理空的 assets/ 目录
//!   * 生成 manifest.json（format_version 2、随机 UUID、min_engine_version
//!     [1,16,2]），description 取自 pack.mcmeta

use std::fs;
use std::path::{Path, PathBuf};

use crate::log_info;

/// 把已转换到 1.21.11 的 Java 包目录重组为基岩版结构。
/// `pack_name`：清理过版本前缀的包名（manifest 的 header.name）。
pub fn convert_java_to_bedrock(temp_dir: &Path, pack_name: &str) -> Result<(), String> {
    // 1. pack.png → pack_icon.png
    let pack_png = temp_dir.join("pack.png");
    if pack_png.exists() {
        let pack_icon = temp_dir.join("pack_icon.png");
        if pack_icon.exists() {
            let _ = fs::remove_file(&pack_icon);
        }
        fs::rename(&pack_png, &pack_icon).map_err(|e| format!("rename pack.png failed: {}", e))?;
        log_info!("OKAY bedrock [pack.png -> pack_icon.png]");
    }

    let minecraft = temp_dir.join("assets").join("minecraft");

    // 2. textures/font → font/
    let font_src = minecraft.join("textures").join("font");
    let font_dst = temp_dir.join("font");
    if font_src.exists() {
        if font_dst.exists() {
            fs::remove_dir_all(&font_dst).map_err(|e| format!("remove font dir failed: {}", e))?;
        }
        fs::rename(&font_src, &font_dst).map_err(|e| format!("move font failed: {}", e))?;
        log_info!("OKAY bedrock [textures/font -> font/]");
    }

    // 3. minecraft/textures → textures/（合并式提升）
    let textures_src = minecraft.join("textures");
    let textures_dst = temp_dir.join("textures");
    if textures_src.exists() {
        merge_dir(&textures_src, &textures_dst)?;
        let _ = fs::remove_dir_all(&textures_src);
        log_info!("OKAY bedrock [minecraft/textures -> textures/]");
    }

    // 4. item → items + 物品改名
    let item_dir = textures_dst.join("item");
    let items_dir = textures_dst.join("items");
    if item_dir.exists() && !items_dir.exists() {
        fs::rename(&item_dir, &items_dir).map_err(|e| format!("rename item failed: {}", e))?;
        log_info!("OKAY bedrock [textures/item -> textures/items]");
    }
    if items_dir.exists() {
        let renamed = rename_items(&items_dir)?;
        if renamed > 0 {
            log_info!("OKAY bedrock [items 改名 {} 个]", renamed);
        }
    }

    // 5. gui/container → ui
    let container = textures_dst.join("gui").join("container");
    let ui_dir = textures_dst.join("ui");
    if container.exists() {
        if ui_dir.exists() {
            fs::remove_dir_all(&ui_dir).map_err(|e| format!("remove ui dir failed: {}", e))?;
        }
        fs::rename(&container, &ui_dir).map_err(|e| format!("move container failed: {}", e))?;
        log_info!("OKAY bedrock [textures/gui/container -> textures/ui]");
    }

    // 6. ui/creative_inventory/* 提取到 ui/ 后删除目录
    let creative = ui_dir.join("creative_inventory");
    if creative.exists() {
        move_contents_up(&creative, &ui_dir)?;
        let _ = fs::remove_dir_all(&creative);
        log_info!("OKAY bedrock [ui/creative_inventory 提取到 ui/]");
    }

    // 7. java_ui 模板（缺失则跳过——模板待补齐）
    apply_java_ui_templates(temp_dir, &textures_dst)?;

    // 8. 清理空的 assets/
    cleanup_empty_assets(temp_dir);

    // 9. manifest.json
    let description = read_description(temp_dir);
    write_manifest(temp_dir, pack_name, &description)?;
    log_info!("OKAY bedrock [manifest.json]");

    Ok(())
}

/// 合并 src 到 dst：目录递归合并，文件直接复制（保留 dst 已有文件）。
fn merge_dir(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(dst).map_err(|e| format!("create dir failed: {}", e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("read dir failed: {}", e))? {
        let entry = entry.map_err(|e| format!("read entry failed: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            merge_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("copy failed: {}", e))?;
        }
    }
    Ok(())
}

/// items 物品改名（与 Python 版一致）：golden_apple→apple_golden、
/// golden_*→gold_*、wooden_*→wood_*。返回改名数量。
/// 对 `.png.mcmeta` 与 `.png` 成对处理（先剥掉后缀层再改名，保证配对）。
fn rename_items(items_dir: &Path) -> Result<usize, String> {
    let mut renamed = 0;
    let entries: Vec<_> = fs::read_dir(items_dir)
        .map_err(|e| format!("read items failed: {}", e))?
        .filter_map(|e| e.ok())
        .collect();
    for entry in entries {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let lower = file_name.to_ascii_lowercase();
        // 只处理 png 与其 mcmeta：先剥掉后缀层拿到物品名
        let (stem, suffix) = if lower.ends_with(".png.mcmeta") {
            (
                file_name[..file_name.len() - ".png.mcmeta".len()].to_string(),
                ".png.mcmeta".to_string(),
            )
        } else if lower.ends_with(".png") {
            (
                file_name[..file_name.len() - ".png".len()].to_string(),
                ".png".to_string(),
            )
        } else {
            continue;
        };

        let new_stem = if stem == "golden_apple" {
            "apple_golden".to_string()
        } else if let Some(rest) = stem.strip_prefix("golden_") {
            format!("gold_{}", rest)
        } else if let Some(rest) = stem.strip_prefix("wooden_") {
            format!("wood_{}", rest)
        } else {
            continue;
        };
        let new_path = items_dir.join(format!("{}{}", new_stem, suffix));
        if new_path.exists() {
            let _ = fs::remove_file(&new_path);
        }
        fs::rename(&path, &new_path).map_err(|e| format!("rename item failed: {}", e))?;
        renamed += 1;
    }
    Ok(renamed)
}

/// 把目录内容全部上提一级（递归，文件移动、目录合并）。
fn move_contents_up(src: &Path, dst: &Path) -> Result<(), String> {
    for entry in fs::read_dir(src).map_err(|e| format!("read dir failed: {}", e))? {
        let entry = entry.map_err(|e| format!("read entry failed: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            merge_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| format!("copy failed: {}", e))?;
        }
    }
    Ok(())
}

/// 应用 java_ui 模板（模板目录缺失时跳过，不视为失败——doc 内容不完整，
/// 模板资源后续补齐）。
fn apply_java_ui_templates(temp_dir: &Path, textures_dst: &Path) -> Result<(), String> {
    let java_ui = find_java_ui_dir();
    let Some(java_ui) = java_ui else {
        log_info!("bedrock: java_ui 模板缺失，跳过 UI 模板替换");
        return Ok(());
    };

    // 1. java_ui/ui → 根目录 ui/（强制替换）
    let src_ui = java_ui.join("ui");
    if src_ui.exists() {
        let dst_ui = temp_dir.join("ui");
        if dst_ui.exists() {
            let _ = fs::remove_dir_all(&dst_ui);
        }
        copy_force(&src_ui, &dst_ui)?;
        log_info!("OKAY bedrock [java_ui/ui -> ui/]");
    }

    // 2. java_ui/textures/ui → textures/ui（强制替换）
    let src_tui = java_ui.join("textures").join("ui");
    if src_tui.exists() {
        let dst_tui = textures_dst.join("ui");
        fs::create_dir_all(&dst_tui).map_err(|e| format!("create dir failed: {}", e))?;
        copy_force(&src_tui, &dst_tui)?;
        log_info!("OKAY bedrock [java_ui/textures/ui -> textures/ui]");
    }

    // 3. java_ui/gui/container → textures/gui/container（强制替换）
    let src_container = java_ui.join("gui").join("container");
    if src_container.exists() {
        let dst_container = textures_dst.join("gui").join("container");
        copy_force(&src_container, &dst_container)?;
        log_info!("OKAY bedrock [java_ui/gui/container -> textures/gui/container]");
    }

    // 4. java_ui/gui/sprites → textures/gui/sprites（强制替换）
    let src_sprites = java_ui.join("gui").join("sprites");
    if src_sprites.exists() {
        let dst_sprites = textures_dst.join("gui").join("sprites");
        copy_force(&src_sprites, &dst_sprites)?;
        log_info!("OKAY bedrock [java_ui/gui/sprites -> textures/gui/sprites]");
    }

    Ok(())
}

/// 查找 java_ui 模板目录：
///   1. 统一资源解析（exe 同级 java_ui/、resources/java_ui/、_up_/、
///      src-tauri/java_ui/ 等，与 UImage/overlay 同策略）
///   2. 开发期参考目录 doc/java_ui（自当前目录向上逐级查找）
/// 找不到返回 None（调用方跳过模板替换，不视为失败）。
fn find_java_ui_dir() -> Option<PathBuf> {
    // 1. 统一资源解析
    if let Ok(p) = crate::resource_resolver::resolve_resource_dir("java_ui", |p| {
        p.join("ui").is_dir()
    }) {
        return Some(p);
    }
    // 2. doc/java_ui（向上查找，兼容 cargo test / tauri dev 的 cwd）
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = cwd;
        for _ in 0..6 {
            let candidate = dir.join("doc").join("java_ui");
            if candidate.is_dir() && candidate.join("ui").is_dir() {
                return Some(candidate);
            }
            if !dir.pop() {
                break;
            }
        }
    }
    None
}

/// 递归复制（同名文件覆盖）。
fn copy_force(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(dst).map_err(|e| format!("create dir failed: {}", e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("read dir failed: {}", e))? {
        let entry = entry.map_err(|e| format!("read entry failed: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_force(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| format!("copy failed: {}", e))?;
        }
    }
    Ok(())
}

/// 清理空的 assets/（只含 minecraft 时连 minecraft 一起删）。
fn cleanup_empty_assets(temp_dir: &Path) {
    let assets = temp_dir.join("assets");
    if !assets.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(&assets) else { return };
    let mut only_minecraft = true;
    let mut count = 0;
    for entry in entries.flatten() {
        count += 1;
        if entry.file_name() != "minecraft" {
            only_minecraft = false;
            break;
        }
    }
    if only_minecraft && count <= 1 {
        if count == 1 {
            let _ = fs::remove_dir_all(assets.join("minecraft"));
        }
        let _ = fs::remove_dir(&assets);
    }
}

/// 从 pack.mcmeta 读取 description（缺失时用默认文案）。
fn read_description(temp_dir: &Path) -> String {
    let mcmeta = temp_dir.join("pack.mcmeta");
    let Ok(content) = fs::read_to_string(&mcmeta) else {
        return "Converted by 2-Pyramid".to_string();
    };
    let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) else {
        return "Converted by 2-Pyramid".to_string();
    };
    match data
        .get("pack")
        .and_then(|p| p.get("description"))
        .and_then(|d| d.as_str())
    {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => "Converted by 2-Pyramid".to_string(),
    }
}

/// 生成 manifest.json（format_version 2 + 随机 UUID）。
fn write_manifest(temp_dir: &Path, pack_name: &str, description: &str) -> Result<(), String> {
    let manifest = serde_json::json!({
        "format_version": 2,
        "header": {
            "description": description,
            "name": pack_name,
            "uuid": uuid::Uuid::new_v4().to_string(),
            "version": [1, 0, 0],
            "min_engine_version": [1, 16, 2],
        },
        "modules": [{
            "description": description,
            "type": "resources",
            "uuid": uuid::Uuid::new_v4().to_string(),
            "version": [1, 0, 0],
        }],
    });
    let pretty = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("serialize manifest failed: {}", e))?;
    fs::write(temp_dir.join("manifest.json"), pretty)
        .map_err(|e| format!("write manifest failed: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_bedrock_reorganization() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();

        // Java 结构：assets/minecraft/textures/{font,item,gui/container}
        let tex = root.join("assets/minecraft/textures");
        fs::create_dir_all(tex.join("font")).expect("mkdir");
        fs::create_dir_all(tex.join("item")).expect("mkdir");
        fs::create_dir_all(tex.join("gui/container/creative_inventory")).expect("mkdir");
        fs::write(tex.join("font/glyph.png"), b"f").expect("write");
        fs::write(tex.join("item/golden_apple.png"), b"ga").expect("write");
        fs::write(tex.join("item/golden_apple.png.mcmeta"), b"{}").expect("write");
        fs::write(tex.join("item/wooden_sword.png"), b"ws").expect("write");
        fs::write(tex.join("item/stone.png"), b"s").expect("write");
        fs::write(tex.join("gui/container/widgets.png"), b"w").expect("write");
        // creative_inventory 位于 container 内，随 container→ui 改名后提取
        fs::write(tex.join("gui/container/creative_inventory/tab.png"), b"t").expect("write");
        fs::write(root.join("pack.png"), b"p").expect("write");
        fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"pack_format":75,"description":"测试包"}}"#,
        )
        .expect("write");

        convert_java_to_bedrock(root, "测试包").expect("convert");

        assert!(root.join("pack_icon.png").exists(), "pack.png 应改名 pack_icon.png");
        assert!(root.join("font/glyph.png").exists(), "font 应提升到根目录");
        assert!(root.join("textures/items/apple_golden.png").exists(), "golden_apple 应改名 apple_golden");
        assert!(root.join("textures/items/apple_golden.png.mcmeta").exists(), "mcmeta 跟随改名");
        assert!(root.join("textures/items/wood_sword.png").exists(), "wooden_sword 应改名 wood_sword");
        assert!(root.join("textures/items/stone.png").exists(), "普通物品保留");
        assert!(root.join("textures/ui/widgets.png").exists(), "container 应改为 ui");
        assert!(root.join("textures/ui/tab.png").exists(), "creative_inventory 内容应提取到 ui");
        assert!(!root.join("textures/ui/creative_inventory").exists(), "creative_inventory 应删除");
        assert!(!root.join("assets").exists(), "空 assets 应删除");

        // manifest.json 校验
        let manifest: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(root.join("manifest.json")).expect("read"))
                .expect("json");
        assert_eq!(manifest["format_version"], serde_json::json!(2));
        assert_eq!(manifest["header"]["name"], serde_json::json!("测试包"));
        assert_eq!(manifest["header"]["description"], serde_json::json!("测试包"));
        assert!(manifest["header"]["uuid"].as_str().unwrap().len() == 36);
        assert!(manifest["modules"][0]["uuid"].as_str().unwrap().len() == 36);
    }

    #[test]
    fn test_missing_textures_ok() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        fs::write(root.join("pack.mcmeta"), r#"{"pack":{"pack_format":75}}"#).expect("write");
        // 没有 assets/textures 也不应报错
        convert_java_to_bedrock(root, "empty").expect("convert");
        assert!(root.join("manifest.json").exists());
    }
}
