use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

// ── 预编译正则表达式（lazy_static，避免每次调用重复编译 + 消除 panic 面）────

lazy_static::lazy_static! {
    // fix_json_placeholders 用
    static ref RE_JSON_AT_PLACEHOLDER: regex::Regex =
        regex::Regex::new(r#"\[\s*@\s*,\s*@\s*,\s*@\s*\]"#).expect("valid regex: RE_JSON_AT_PLACEHOLDER");
    static ref RE_JSON_HASH_PLACEHOLDER: regex::Regex =
        regex::Regex::new(r#"\[\s*#\s*,\s*#\s*,\s*#\s*\]"#).expect("valid regex: RE_JSON_HASH_PLACEHOLDER");

    // replace_outline_placeholders 用 ― vec4 占位符
    static ref RE_VEC4_HASH: regex::Regex =
        regex::Regex::new(r"vec4\(\s*#\s*,\s*#\s*,\s*#\s*,\s*#\s*\)").expect("valid regex: RE_VEC4_HASH");
    static ref RE_VEC4_PCT: regex::Regex =
        regex::Regex::new(r"vec4\(\s*%\s*,\s*%\s*,\s*%\s*,\s*%\s*\)").expect("valid regex: RE_VEC4_PCT");
    static ref RE_VEC4_AMP: regex::Regex =
        regex::Regex::new(r"vec4\(\s*&\s*,\s*&\s*,\s*&\s*,\s*&\s*\)").expect("valid regex: RE_VEC4_AMP");
    static ref RE_VEC4_STAR: regex::Regex =
        regex::Regex::new(r"vec4\(\s*\*\s*,\s*\*\s*,\s*\*\s*,\s*\*\s*\)").expect("valid regex: RE_VEC4_STAR");

    // replace_outline_placeholders 用 ― 数组占位符
    static ref RE_ARR_HASH: regex::Regex =
        regex::Regex::new(r"\[\s*#\s*,\s*#\s*,\s*#\s*,\s*#\s*\]").expect("valid regex: RE_ARR_HASH");
    static ref RE_ARR_PCT: regex::Regex =
        regex::Regex::new(r"\[\s*%\s*,\s*%\s*,\s*%\s*,\s*%\s*\]").expect("valid regex: RE_ARR_PCT");
    static ref RE_ARR_AMP: regex::Regex =
        regex::Regex::new(r"\[\s*&\s*,\s*&\s*,\s*&\s*,\s*&\s*\]").expect("valid regex: RE_ARR_AMP");
    static ref RE_ARR_STAR: regex::Regex =
        regex::Regex::new(r"\[\s*\*\s*,\s*\*\s*,\s*\*\s*,\s*\*\s*\]").expect("valid regex: RE_ARR_STAR");

    // replace_outline_placeholders 用 ― 线条宽度 / 粗细
    static ref RE_BORDER_WIDTH: regex::Regex =
        regex::Regex::new(r"#define\s+BORDER_LINE_WIDTH\s+@").expect("valid regex: RE_BORDER_WIDTH");
    static ref RE_THICKNESS: regex::Regex =
        regex::Regex::new(r"\[\s*@\s*\]").expect("valid regex: RE_THICKNESS");
}

// ── 路径解析（委托给 resource_resolver）───────────────────────────────

pub use crate::resource_resolver::copy_dir_all;

pub fn is_dev_mode() -> bool {
    // 优先使用 Tauri 资源缓存判断：如果缓存路径存在则为生产环境
    if crate::resource_resolver::get_cached_resource("overlay").is_some() {
        return false;
    }
    std::env::current_dir()
        .map(|p| p.join("src-tauri").exists() || p.ends_with("src-tauri"))
        .unwrap_or(false)
}

pub fn user_data_root_dir() -> Result<PathBuf, String> {
    if let Some(docs) = dirs::document_dir() {
        let two_pyramid_docs = docs.join("2-Pyramid");
        if !two_pyramid_docs.exists() {
            fs::create_dir_all(&two_pyramid_docs).map_err(|e| e.to_string())?;
        }
        return Ok(two_pyramid_docs);
    }
    Err("无法获取用户文档目录".to_string())
}

/// 通过 Tauri 资源 API 解析 overlay 路径并缓存（委托 resource_resolver）
pub fn cache_overlay_path_from_app(app: &tauri::AppHandle) {
    crate::resource_resolver::cache_resource_from_app(app, "overlay");
}

/// 获取 overlay 模板根目录（多策略：缓存 → 文件系统 → 用户文档）
pub fn overlay_templates_dir() -> Result<PathBuf, String> {
    crate::resource_resolver::resolve_resource_dir("overlay", crate::resource_resolver::overlay_validator())
}

/// 获取 overlay 模板根目录（带 Tauri AppHandle，推荐在命令中使用）
pub fn overlay_templates_dir_from_app(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    // 优先通过 Tauri 资源 API 缓存
    crate::resource_resolver::cache_resource_from_app(app, "overlay");
    match crate::resource_resolver::resolve_resource_dir("overlay", crate::resource_resolver::overlay_validator()) {
        Ok(p) => Ok(p),
        Err(e) => {
            crate::log_info!("overlay_templates_dir_from_app: {}", e);
            Err(e)
        }
    }
}

pub fn overlay_temp_dir(project_name: Option<&str>) -> Result<PathBuf, String> {
    let base = user_data_root_dir()?.join("temp_overlay");
    if let Some(name) = project_name {
        Ok(base.join(name))
    } else {
        Ok(base)
    }
}

pub fn overlay_config_path() -> Result<PathBuf, String> {
    Ok(user_data_root_dir()?.join("overlay_projects.json"))
}

/// 修复 JSON 模板中的占位符：`[@, @, @]` → `[1.0, 1.0, 1.0]`，`[#, #, #]` → `[1.0, 1.0, 1.0]`
pub fn fix_json_placeholders(file_path: &Path) -> Result<(), String> {
    if !file_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("读取文件失败 {}: {}", file_path.display(), e))?;

    if !content.contains('@') && !content.contains('#') {
        return Ok(());
    }

    let fixed = RE_JSON_AT_PLACEHOLDER.replace_all(&content, "[1.0, 1.0, 1.0]");
    let fixed = RE_JSON_HASH_PLACEHOLDER.replace_all(&fixed, "[1.0, 1.0, 1.0]");

    fs::write(file_path, fixed.as_bytes())
        .map_err(|e| format!("写入文件失败 {}: {}", file_path.display(), e))?;

    crate::log_info!("fixed json placeholders: {}", file_path.display());
    Ok(())
}

/// 应用放大倍数到目录中的所有 JSON 模型文件
pub fn apply_scale_to_json_files(target_dir: &Path, big_items_config: &Value) -> Result<(), String> {
    if !target_dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(target_dir)
        .map_err(|e| format!("读取目录失败 {}: {}", target_dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !file_name.ends_with(".json") {
            continue;
        }

        // 提取物品名（compass_00 → compass）
        let base_name = file_name.strip_suffix(".json").unwrap_or(file_name);
        let item_name = if base_name.starts_with("compass_") {
            "compass"
        } else {
            base_name
        };

        // 检查是否有该物品的放大配置
        let item_config = match big_items_config.get(item_name) {
            Some(c) => c,
            None => continue,
        };

        let handheld_str = item_config
            .get("handheld_scale")
            .and_then(|v| v.as_str())
            .unwrap_or("1x");
        let dropped_str = item_config
            .get("dropped_scale")
            .and_then(|v| v.as_str())
            .unwrap_or("1x");

        let handheld_scale: f64 = handheld_str.trim_end_matches('x').parse().unwrap_or(1.0);
        let dropped_scale: f64 = dropped_str.trim_end_matches('x').parse().unwrap_or(1.0);

        // 读取 JSON
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取 {} 失败: {}", path.display(), e))?;
        let mut json: Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => {
                // 可能有未修复的占位符，尝试修复后重试
                crate::overlay::fix_json_placeholders(&path)?;
                let fixed_content = fs::read_to_string(&path)
                    .map_err(|e| format!("重新读取 {} 失败: {}", path.display(), e))?;
                serde_json::from_str(&fixed_content)
                    .map_err(|e| format!("解析 {} JSON 失败: {}", path.display(), e))?
            }
        };

        // 修改 scale 值
        if let Some(display) = json.get_mut("display") {
            // 手持（第三人称右手）
            if let Some(tp_right) = display.get_mut("thirdperson_righthand") {
                if let Some(scale) = tp_right.get_mut("scale") {
                    *scale = Value::Array(vec![
                        Value::Number(serde_json::Number::from_f64(handheld_scale).unwrap()),
                        Value::Number(serde_json::Number::from_f64(handheld_scale).unwrap()),
                        Value::Number(serde_json::Number::from_f64(handheld_scale).unwrap()),
                    ]);
                } else {
                    tp_right["scale"] = Value::Array(vec![
                        Value::Number(serde_json::Number::from_f64(handheld_scale).unwrap()),
                        Value::Number(serde_json::Number::from_f64(handheld_scale).unwrap()),
                        Value::Number(serde_json::Number::from_f64(handheld_scale).unwrap()),
                    ]);
                }
            }
            // 地面
            if let Some(ground) = display.get_mut("ground") {
                if let Some(scale) = ground.get_mut("scale") {
                    *scale = Value::Array(vec![
                        Value::Number(serde_json::Number::from_f64(dropped_scale).unwrap()),
                        Value::Number(serde_json::Number::from_f64(dropped_scale).unwrap()),
                        Value::Number(serde_json::Number::from_f64(dropped_scale).unwrap()),
                    ]);
                } else {
                    ground["scale"] = Value::Array(vec![
                        Value::Number(serde_json::Number::from_f64(dropped_scale).unwrap()),
                        Value::Number(serde_json::Number::from_f64(dropped_scale).unwrap()),
                        Value::Number(serde_json::Number::from_f64(dropped_scale).unwrap()),
                    ]);
                }
            }
        }

        let output = serde_json::to_string(&json)
            .map_err(|e| format!("序列化 {} 失败: {}", path.display(), e))?;
        fs::write(&path, output)
            .map_err(|e| format!("写入 {} 失败: {}", path.display(), e))?;

        crate::log_info!("applied scale (handheld:{}x, dropped:{}x) to {}", handheld_scale, dropped_scale, path.display());
    }

    Ok(())
}

/// 替换 outline shader 文件中的颜色和粗细占位符
pub fn replace_outline_placeholders(
    file_path: &Path,
    color: &Value,
    thickness: f64,
) -> Result<(), String> {
    let r = color.get("r").and_then(|v| v.as_f64()).unwrap_or(1.0);
    let g = color.get("g").and_then(|v| v.as_f64()).unwrap_or(1.0);
    let b = color.get("b").and_then(|v| v.as_f64()).unwrap_or(1.0);
    let a = color.get("a").and_then(|v| v.as_f64()).unwrap_or(1.0);

    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("读取 {} 失败: {}", file_path.display(), e))?;

    let rgba_str = format!("{r}, {g}, {b}, {a}");

    if file_name.ends_with(".vsh") || file_name.ends_with(".fsh") {
        // 着色器文件：替换 vec4(#, #, #, #) 等占位符
        let fixed = RE_VEC4_HASH.replace_all(&content, format!("vec4({rgba_str})"));
        let fixed = RE_VEC4_PCT.replace_all(&fixed, format!("vec4({rgba_str})"));
        let fixed = RE_VEC4_AMP.replace_all(&fixed, format!("vec4({rgba_str})"));
        let fixed = RE_VEC4_STAR.replace_all(&fixed, format!("vec4({rgba_str})"));

        // 仅在 vsh 文件中替换线条宽度
        let final_content = if file_name.ends_with(".vsh") {
            RE_BORDER_WIDTH.replace_all(&fixed, format!("#define BORDER_LINE_WIDTH {thickness}"))
        } else {
            fixed
        };

        fs::write(file_path, final_content.as_bytes())
            .map_err(|e| format!("写入 {} 失败: {}", file_path.display(), e))?;
    } else if file_name.ends_with(".json") {
        // 渲染管线 JSON 文件：替换 [#, #, #, #] 等占位符
        let fixed = RE_ARR_HASH.replace_all(&content, format!("[{rgba_str}]"));
        let fixed = RE_ARR_PCT.replace_all(&fixed, format!("[{rgba_str}]"));
        let fixed = RE_ARR_AMP.replace_all(&fixed, format!("[{rgba_str}]"));
        let fixed = RE_ARR_STAR.replace_all(&fixed, format!("[{rgba_str}]"));
        let final_content = RE_THICKNESS.replace_all(&fixed, format!("[{thickness}]"));

        fs::write(file_path, final_content.as_bytes())
            .map_err(|e| format!("写入 {} 失败: {}", file_path.display(), e))?;
    }

    crate::log_info!(
        "replaced outline placeholders in {}: color=({}), thickness={}",
        file_path.display(),
        rgba_str,
        thickness
    );
    Ok(())
}

/// 处理 `no_shadow` (Python `process_core_shadow`)：从 overlay 模板的 `core_inventory/`
/// 目录复制**全部文件**到目标 shaders/core 目录，覆盖同名文件。
///
/// Python 端参考实现 (`overlay.py:process_core_shadow`) 的语义是复制整个 `core_inventory`
/// 目录下的所有文件；旧 Rust 实现只复制 `rendertype_gui.vsh` 单文件，对齐后改为全量复制。
///
/// 返回成功复制的文件数；若 `core_inventory` 目录不存在则返回 0（不报错）。
pub fn process_core_shadow(overlay_dir: &Path, target_dir: &Path) -> Result<usize, String> {
    let core_inventory_dir = overlay_dir.join("core_inventory");
    if !core_inventory_dir.exists() {
        crate::log_info!("core_inventory dir not found, skip: {}", core_inventory_dir.display());
        return Ok(0);
    }

    fs::create_dir_all(target_dir).map_err(|e| format!("创建 shaders/core 目录失败: {}", e))?;

    let mut copied = 0usize;
    for entry in fs::read_dir(&core_inventory_dir)
        .map_err(|e| format!("读取 {} 失败: {}", core_inventory_dir.display(), e))?
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let src = entry.path();
        if !src.is_file() {
            continue;
        }
        let Some(file_name) = src.file_name() else {
            continue;
        };
        let dest = target_dir.join(file_name);
        fs::copy(&src, &dest)
            .map_err(|e| format!("复制 {} -> {} 失败: {}", src.display(), dest.display(), e))?;
        copied += 1;
        crate::log_info!("core_shadow copied: {} -> {}", src.display(), dest.display());
    }
    Ok(copied)
}

/// 解压母包到 workspace，返回 workspace 根路径
pub fn process_parent_pack_workspace(
    temp_dir: &Path,
    parent_pack_path: &str,
) -> Result<PathBuf, String> {
    let parent_path = Path::new(parent_pack_path);
    if !parent_path.exists() {
        return Err(format!("母包文件不存在: {}", parent_pack_path));
    }

    let file_name = parent_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("parent_pack.zip");

    let workspace_dir = temp_dir.join("workspace");
    fs::create_dir_all(&workspace_dir)
        .map_err(|e| format!("创建 workspace 目录失败: {}", e))?;

    // 复制 zip 到 workspace
    let zip_in_workspace = workspace_dir.join(file_name);
    fs::copy(parent_path, &zip_in_workspace)
        .map_err(|e| format!("复制母包到 workspace 失败: {}", e))?;

    // 解压（使用共享工具函数，含 ZIP bomb 防护）
    let extract_name = file_name.strip_suffix(".zip").unwrap_or(file_name);
    let extract_dir = workspace_dir.join(extract_name);
    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir).map_err(|e| format!("清理旧解压目录失败: {}", e))?;
    }

    crate::converters::zip::extract_zip_to_dir(&zip_in_workspace, &extract_dir)?;

    // 删除 zip 原文件
    fs::remove_file(&zip_in_workspace)
        .map_err(|e| format!("删除 workspace 中的 zip 失败: {}", e))?;

    crate::log_info!("parent pack extracted to workspace: {}", extract_dir.display());
    Ok(extract_dir)
}

// ── 单元测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // ── fix_json_placeholders ──

    #[test]
    fn fix_placeholders_replaces_at_and_hash_arrays() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("model.json");
        fs::write(&path, r#"{ "display": { "thirdperson": { "scale": [@, @, @] } } }"#).unwrap();

        fix_json_placeholders(&path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("[1.0, 1.0, 1.0]"),
            "expected placeholder to be replaced, got: {}",
            content
        );
    }

    #[test]
    fn fix_placeholders_handles_hash_placeholder() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("model.json");
        fs::write(&path, r#"{ "scale": [#, #, #] }"#).unwrap();

        fix_json_placeholders(&path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("[1.0, 1.0, 1.0]"));
    }

    #[test]
    fn fix_placeholders_is_noop_when_no_marker() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("clean.json");
        let original = r#"{ "display": { "scale": [1.0, 1.0, 1.0] } }"#;
        fs::write(&path, original).unwrap();

        fix_json_placeholders(&path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, original);
    }

    #[test]
    fn fix_placeholders_handles_missing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("does_not_exist.json");
        // 不应报错
        let result = fix_json_placeholders(&path);
        assert!(result.is_ok());
    }

    // ── apply_scale_to_json_files ──

    #[test]
    fn apply_scale_updates_thirdperson_and_ground() {
        let dir = tempdir().unwrap();
        let model_path = dir.path().join("diamond_sword.json");
        fs::write(
            &model_path,
            r#"{
                "display": {
                    "thirdperson_righthand": { "scale": [1.0, 1.0, 1.0] },
                    "ground": { "scale": [1.0, 1.0, 1.0] }
                }
            }"#,
        )
        .unwrap();

        let config = serde_json::json!({
            "diamond_sword": {
                "handheld_scale": "2.5x",
                "dropped_scale": "1.5x"
            }
        });
        apply_scale_to_json_files(dir.path(), &config).unwrap();

        let updated: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&model_path).unwrap()).unwrap();
        let tp = &updated["display"]["thirdperson_righthand"]["scale"];
        let gd = &updated["display"]["ground"]["scale"];
        assert_eq!(tp[0].as_f64().unwrap(), 2.5);
        assert_eq!(tp[1].as_f64().unwrap(), 2.5);
        assert_eq!(tp[2].as_f64().unwrap(), 2.5);
        assert_eq!(gd[0].as_f64().unwrap(), 1.5);
    }

    #[test]
    fn apply_scale_creates_missing_scale_field() {
        let dir = tempdir().unwrap();
        let model_path = dir.path().join("stick.json");
        // 没有 scale 字段,只有 display
        fs::write(
            &model_path,
            r#"{ "display": { "thirdperson_righthand": { "rotation": [0, 90, 0] } } }"#,
        )
        .unwrap();

        let config = serde_json::json!({
            "stick": { "handheld_scale": "1.2x", "dropped_scale": "1x" }
        });
        apply_scale_to_json_files(dir.path(), &config).unwrap();

        let updated: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&model_path).unwrap()).unwrap();
        assert!(updated["display"]["thirdperson_righthand"]["scale"].is_array());
    }

    #[test]
    fn apply_scale_skips_unconfigured_item() {
        let dir = tempdir().unwrap();
        let model_path = dir.path().join("unrelated.json");
        let original = r#"{ "display": { "scale": [1.0, 1.0, 1.0] } }"#;
        fs::write(&model_path, original).unwrap();

        let config = serde_json::json!({ "other_item": { "handheld_scale": "2x" } });
        apply_scale_to_json_files(dir.path(), &config).unwrap();

        let content = fs::read_to_string(&model_path).unwrap();
        assert_eq!(content, original);
    }

    #[test]
    fn apply_scale_compass_normalizes_suffix() {
        // compass_00 / compass_15 等后缀 → 都用 "compass" 的配置
        let dir = tempdir().unwrap();
        let model = dir.path().join("compass_15.json");
        fs::write(
            &model,
            r#"{ "display": { "thirdperson_righthand": { "scale": [1.0, 1.0, 1.0] } } }"#,
        )
        .unwrap();

        let config = serde_json::json!({
            "compass": { "handheld_scale": "1.8x", "dropped_scale": "1.8x" }
        });
        apply_scale_to_json_files(dir.path(), &config).unwrap();

        let updated: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&model).unwrap()).unwrap();
        let tp = &updated["display"]["thirdperson_righthand"]["scale"];
        assert_eq!(tp[0].as_f64().unwrap(), 1.8);
    }

    #[test]
    fn apply_scale_returns_ok_for_missing_dir() {
        let dir = tempdir().unwrap();
        let ghost = dir.path().join("not_here");
        let config = serde_json::json!({});
        assert!(apply_scale_to_json_files(&ghost, &config).is_ok());
    }

    // ── replace_outline_placeholders ──

    #[test]
    fn replace_outline_vsh_uses_rgba_and_thickness() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rendertype_gui.vsh");
        fs::write(
            &path,
            r#"
                in vec4 color;
                void main() {
                    vec4 outlineColor = vec4(#, #, #, #);
                    gl_Position = vec4(0.0);
                    #define BORDER_LINE_WIDTH @
                }
            "#,
        )
        .unwrap();

        let color = serde_json::json!({"r": 0.5, "g": 0.6, "b": 0.7, "a": 0.8});
        replace_outline_placeholders(&path, &color, 4.0).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("vec4(0.5, 0.6, 0.7, 0.8)"));
        assert!(content.contains("#define BORDER_LINE_WIDTH 4"));
        // vec4 颜色占位符应已被替换
        assert!(!content.contains("vec4(#, #, #, #)"));
    }

    #[test]
    fn replace_outline_fsh_only_replaces_color_not_thickness() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rendertype_gui.fsh");
        fs::write(&path, "out vec4 frag; void main() { frag = vec4(*, *, *, *); }").unwrap();

        let color = serde_json::json!({"r": 1.0, "g": 0.0, "b": 0.0, "a": 1.0});
        replace_outline_placeholders(&path, &color, 9.0).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("vec4(1, 0, 0, 1)"));
        // fsh 不应被注入 thickness
        assert!(!content.contains("BORDER_LINE_WIDTH"));
    }

    #[test]
    fn replace_outline_json_uses_array_form() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rendertype_gui.json");
        fs::write(&path, r#"{ "blend": { "color": [%, %, %, %], "width": [@] } }"#).unwrap();

        let color = serde_json::json!({"r": 0.1, "g": 0.2, "b": 0.3, "a": 0.4});
        replace_outline_placeholders(&path, &color, 3.5).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("[0.1, 0.2, 0.3, 0.4]"));
        assert!(content.contains("[3.5]"));
    }

    // ── process_core_shadow ──

    #[test]
    fn process_core_shadow_copies_all_files() {
        let overlay = tempdir().unwrap();
        let core_inv = overlay.path().join("core_inventory");
        fs::create_dir_all(&core_inv).unwrap();
        fs::write(core_inv.join("rendertype_gui.vsh"), "// vsh content").unwrap();
        fs::write(core_inv.join("rendertype_gui.fsh"), "// fsh content").unwrap();
        fs::write(core_inv.join("rendertype_gui.json"), "{}").unwrap();

        let target = tempdir().unwrap();
        let target_dir = target.path().join("shaders").join("core");
        let copied = process_core_shadow(overlay.path(), &target_dir).unwrap();

        assert_eq!(copied, 3);
        assert!(target_dir.join("rendertype_gui.vsh").exists());
        assert!(target_dir.join("rendertype_gui.fsh").exists());
        assert!(target_dir.join("rendertype_gui.json").exists());
        // 复制的是实际内容,不是空
        let vsh = fs::read_to_string(target_dir.join("rendertype_gui.vsh")).unwrap();
        assert_eq!(vsh, "// vsh content");
    }

    #[test]
    fn process_core_shadow_returns_zero_when_source_missing() {
        let overlay = tempdir().unwrap();
        let target = tempdir().unwrap();
        let copied = process_core_shadow(overlay.path(), target.path()).unwrap();
        assert_eq!(copied, 0);
        // 仍然创建了 target_dir
        assert!(target.path().exists());
    }

    #[test]
    fn process_core_shadow_overwrites_existing_files() {
        let overlay = tempdir().unwrap();
        let core_inv = overlay.path().join("core_inventory");
        fs::create_dir_all(&core_inv).unwrap();
        fs::write(core_inv.join("rendertype_gui.vsh"), "new content").unwrap();

        let target = tempdir().unwrap();
        let target_dir = target.path().join("core");
        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join("rendertype_gui.vsh"), "old content").unwrap();

        process_core_shadow(overlay.path(), &target_dir).unwrap();
        let overwritten = fs::read_to_string(target_dir.join("rendertype_gui.vsh")).unwrap();
        assert_eq!(overwritten, "new content");
    }

    // ── process_parent_pack_workspace ──

    #[test]
    fn process_parent_pack_extracts_zip_to_workspace() {
        use std::io::Write;

        let src = tempdir().unwrap();
        let zip_path = src.path().join("ParentPack.zip");
        {
            let zip_file = fs::File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(zip_file);
            let options = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file("pack.mcmeta", options).unwrap();
            zip.write_all(br#"{"pack":{"pack_format":15}}"#).unwrap();
            zip.start_file("assets/minecraft/lang/en_us.json", options).unwrap();
            zip.write_all(br#"{"item.apple":"Apple"}"#).unwrap();
            zip.finish().unwrap();
        }

        let project = tempdir().unwrap();
        let ws = process_parent_pack_workspace(project.path(), &zip_path.to_string_lossy()).unwrap();

        // workspace 在 project/workspace/ParentPack/
        assert!(ws.is_dir());
        assert!(ws.join("pack.mcmeta").exists());
        assert!(ws.join("assets/minecraft/lang/en_us.json").exists());
        // 临时 zip 应被删
        assert!(!ws.parent().unwrap().join("ParentPack.zip").exists());
    }

    #[test]
    fn process_parent_pack_errors_on_missing_zip() {
        let project = tempdir().unwrap();
        let result = process_parent_pack_workspace(project.path(), "Z:/this/does/not/exist.zip");
        assert!(result.is_err());
    }
}
