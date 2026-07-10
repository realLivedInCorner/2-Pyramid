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
