use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::converters::zip::{extract_resource_pack, repack_resource_pack};
use crate::{log_info, log_warn};

const PACK_FORMAT_LABELS: [&str; 27] = [
    "Java 1.6-1.8",
    "Java 1.9-1.10",
    "Java 1.11-1.12",
    "Java 1.13-1.14",
    "Java 1.15-1.16.1",
    "Java 1.16.2-1.16.5",
    "Java 1.17",
    "Java 1.18",
    "Java 1.19-1.19.2",
    "Java 1.19.3",
    "Java 1.19.4",
    "Java 1.20-1.20.1",
    "Java 1.20.2",
    "Java 1.20.3-1.20.4",
    "Java 1.20.5-1.20.6",
    "Java 1.21-1.21.1",
    "Java 1.21.2-1.21.3",
    "Java 1.21.4",
    "Java 1.21.5",
    "Java 1.21.6",
    "Java 1.21.7-1.21.8",
    "Java 1.21.9-1.21.10",
    "Java 1.21.11",
    "Java 26.1-26.1.2",
    "Java 26.2",
    "Java 26.3",
    "Bedrock Latest",
];

fn pack_format_label(pack_format: u32) -> &'static str {
    match pack_format {
        1 => "Java 1.6-1.8",
        2 => "Java 1.9-1.10",
        3 => "Java 1.11-1.12",
        4 => "Java 1.13-1.14",
        5 => "Java 1.15-1.16.1",
        6 => "Java 1.16.2-1.16.5",
        7 => "Java 1.17",
        8 => "Java 1.18",
        9 => "Java 1.19-1.19.2",
        12 => "Java 1.19.3",
        13 => "Java 1.19.4",
        15 => "Java 1.20-1.20.1",
        18 => "Java 1.20.2",
        22 => "Java 1.20.3-1.20.4",
        32 => "Java 1.20.5-1.20.6",
        34 => "Java 1.21-1.21.1",
        42 => "Java 1.21.2-1.21.3",
        46 => "Java 1.21.4",
        55 => "Java 1.21.5",
        63 => "Java 1.21.6",
        64 => "Java 1.21.7-1.21.8",
        69 => "Java 1.21.9-1.21.10",
        75 => "Java 1.21.11",
        84 => "Java 26.1-26.1.2",
        88 => "Java 26.2",
        97 => "Java 26.3",
        1000 => "Bedrock Latest",
        _ => "Unknown",
    }
}

pub fn pack_format_label_for_output(pack_format: u32) -> &'static str {
    pack_format_label(pack_format)
}

/// 已知目标版本标签 → pack_format（与前端 `MINECRAFT_VERSIONS` 一致）。
const TARGET_LABELS: &[(&str, u32)] = &[
    ("1.6-1.8", 1),
    ("1.9-1.10", 2),
    ("1.11-1.12", 3),
    ("1.13-1.14", 4),
    ("1.15-1.16.1", 5),
    ("1.16.2-1.16.5", 6),
    ("1.17", 7),
    ("1.18", 8),
    ("1.19-1.19.2", 9),
    ("1.19.3", 12),
    ("1.19.4", 13),
    ("1.20-1.20.1", 15),
    ("1.20.2", 18),
    ("1.20.3-1.20.4", 22),
    ("1.20.5-1.20.6", 32),
    ("1.21-1.21.1", 34),
    ("1.21.2-1.21.3", 42),
    ("1.21.4", 46),
    ("1.21.5", 55),
    ("1.21.6", 63),
    ("1.21.7-1.21.8", 64),
    ("1.21.9-1.21.10", 69),
    ("1.21.11", 75),
    ("26.1-26.1.2", 84),
    ("26.2", 88),
    ("26.3", 97),
];

/// 把命令行/脚本书写的目标版本解析成 pack_format。
///
/// 接受：纯数字（`34`）、标签（`1.21-1.21.1`）、短版本（`1.21`）、
/// `26.3`、`bedrock` / `bedrock latest`（→ 1000）。大小写与空格不敏感。
pub fn resolve_target_format(spec: &str) -> Result<u32, String> {
    let raw = spec.trim();
    if raw.is_empty() {
        return Err("未指定目标版本（--to <版本|pack_format>）".to_string());
    }
    if let Ok(n) = raw.parse::<u32>() {
        if n > 0 {
            return Ok(n);
        }
    }
    let lower = raw.to_ascii_lowercase();
    let s = lower.strip_prefix("java").unwrap_or(&lower).trim();
    if s.starts_with("bedrock") {
        return Ok(1000);
    }
    // 1) 完整标签精确匹配
    for (label, fmt) in TARGET_LABELS {
        if s == *label {
            return Ok(*fmt);
        }
    }
    // 2) 前缀匹配（"1.21" → "1.21-1.21.1"，"1.20" → "1.20-1.20.1"）
    for (label, fmt) in TARGET_LABELS {
        if label.starts_with(s) {
            return Ok(*fmt);
        }
    }
    Err(format!(
        "未知目标版本：{}（可用：pack_format 数字如 34、版本如 1.21.4 / 26.3、bedrock）",
        spec
    ))
}

lazy_static! {
    static ref VERSION_PREFIX_RE: Regex = {
        let patterns = PACK_FORMAT_LABELS
            .iter()
            .map(|label| regex::escape(label))
            .collect::<Vec<_>>()
            .join("|");
        // 不锚定开头：命名模板改版后，[版本标签] 可能出现在名称中间
        // 或末尾（如 [Name] [Ver] → “我的包 [Java 1.20-1.20.1]”），
        // 全部替换才能避免再转换时新旧前缀堆叠。
        Regex::new(&format!(r"\[({})\]", patterns)).unwrap()
    };
}

fn strip_version_prefix(name: &str) -> String {
    let cleaned = VERSION_PREFIX_RE.replace_all(name, "");
    // 标签被移除后压缩多余空白（两侧与中间的双空格）
    let mut collapsed = String::with_capacity(cleaned.len());
    let mut pending_space = false;
    for ch in cleaned.trim().chars() {
        if ch.is_whitespace() {
            pending_space = true;
        } else {
            if pending_space {
                collapsed.push(' ');
                pending_space = false;
            }
            collapsed.push(ch);
        }
    }
    collapsed
}

fn read_text_with_fallback(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let text = match String::from_utf8(bytes.clone()) {
        Ok(text) => text,
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    };
    // 去掉 UTF-8 BOM 与首尾空白（部分来源的 mcmeta 带 BOM，serde 会解析失败）
    Ok(text.trim_start_matches('\u{feff}').to_string())
}

fn find_pack_mcmeta(temp_dir: &Path) -> Option<PathBuf> {
    let direct = temp_dir.join("pack.mcmeta");
    if direct.exists() {
        return Some(direct);
    }

    for entry in WalkDir::new(temp_dir).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() && entry.file_name().to_string_lossy().eq_ignore_ascii_case("pack.mcmeta") {
            return Some(entry.into_path());
        }
    }

    None
}

/// 严格解析 pack.mcmeta：必须能取出 pack.pack_format 数值才返回 Some。
/// 与 read_pack_format（缺省回退 1）不同，这里用于「候选文件是否为
/// 真正的 mcmeta」判定，解析失败一律 None。
fn parse_pack_format_strict(path: &Path) -> Option<u32> {
    let content = read_text_with_fallback(path).ok()?;
    let data: Value = serde_json::from_str(&content).ok()?;
    let value = data
        .get("pack")
        .and_then(|p| p.get("pack_format"))
        .or_else(|| data.get("pack").and_then(|p| p.get("format")));
    match value {
        Some(Value::Number(n)) => n.as_u64().map(|v| v as u32),
        Some(Value::String(s)) => s.trim().parse::<u32>().ok(),
        _ => None,
    }
}

/// 目录规整（优先级最高、转换开始前最先执行）：
///
/// 定位真正的 pack.mcmeta 并统一提升到解压根目录（对齐原 Python 版
/// 「将 pack.mcmeta 复制到根目录」的结构修复步骤）。
///
/// 防呆规则：
///   * 根目录已有 pack.mcmeta（不区分大小写）→ 直接使用；
///   * 否则递归查找候选：文件名精确为 pack.mcmeta，或文件名形如
///     `pack.mcmeta.*`（任意后缀，如 pack.mcmeta.txt / pack.mcmeta.json ——
///     部分用户或下载工具会给文件多加一个扩展名）；
///   * 候选必须能解析出 pack_format 数值才认定为真 mcmeta；
///   * 认定后：根目录内的多扩展名文件直接改名为 pack.mcmeta，
///     嵌套的复制到根目录并删除原位文件（避免重复打包）；
///   * 没有任何合法候选 → 返回 None（沿用旧的「新建 pack.mcmeta」逻辑）。
fn normalize_pack_structure(temp_dir: &Path) -> Option<PathBuf> {
    let root_meta = temp_dir.join("pack.mcmeta");
    if root_meta.is_file() {
        return Some(root_meta);
    }

    // 递归收集候选：先精确名（pack.mcmeta），后多扩展名（pack.mcmeta.*）
    let mut exact: Vec<PathBuf> = Vec::new();
    let mut fuzzy: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(temp_dir).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.eq_ignore_ascii_case("pack.mcmeta") {
            exact.push(entry.into_path());
        } else if Path::new(&name)
            .file_stem()
            .map(|s| s.to_string_lossy().eq_ignore_ascii_case("pack.mcmeta"))
            .unwrap_or(false)
        {
            fuzzy.push(entry.into_path());
        }
    }
    // WalkDir 顺序不稳定：按路径层级从浅到深排序，根目录候选优先
    exact.sort_by_key(|p| p.components().count());
    fuzzy.sort_by_key(|p| p.components().count());

    for candidate in exact.into_iter().chain(fuzzy) {
        // 内容必须能解析出 format 数值才视为真正的 mcmeta
        if parse_pack_format_strict(&candidate).is_none() {
            continue;
        }
        if candidate == root_meta {
            return Some(root_meta);
        }
        let _ = fs::copy(&candidate, &root_meta);
        if root_meta.is_file() {
            // 已统一为根目录 pack.mcmeta：原位文件删除，避免重复进包
            let _ = fs::remove_file(&candidate);
            crate::log_info!(
                "OKAY normalize_pack_structure [{} -> pack.mcmeta]",
                candidate.display()
            );
            return Some(root_meta);
        }
        // 复制失败（文件被占用等）：退回原位路径，仍可继续转换
        crate::log_info!(
            "promote pack.mcmeta copy failed, fallback to {}",
            candidate.display()
        );
        return Some(candidate);
    }

    None
}

fn read_pack_format(pack_meta_path: &Path) -> Result<u32, String> {
    let content = read_text_with_fallback(pack_meta_path)?;
    let data: Value = serde_json::from_str(&content)
        .map_err(|e| format!("failed to parse {}: {}", pack_meta_path.display(), e))?;

    let pack_value = data.get("pack").and_then(|p| p.get("pack_format"));

    if let Some(value) = pack_value {
        if let Some(num) = value.as_u64() {
            return Ok(num as u32);
        }
        if let Some(text) = value.as_str() {
            if let Ok(num) = text.parse::<u32>() {
                return Ok(num);
            }
        }
    }

    Ok(1)
}

fn normalize_description(value: &Value) -> String {
    let raw = match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::String(text) => text.clone(),
                _ => item.to_string(),
            })
            .collect::<Vec<_>>()
            .join(" "),
        _ => value.to_string(),
    };

    raw.replace('\n', " ").replace('\r', " ")
}

/// 游戏识别用的资源包版本 major.minor。
/// 26.3 官方 Resource Pack 为 **97.1**（只写 97 / [97,0] 会显示不兼容）。
fn pack_format_major_minor(target: u32) -> (u32, u32) {
    match target {
        97 => (97, 1), // Java 26.3 Wilderness Bound
        _ => (target, 0),
    }
}

fn write_pack_format(pack_meta_path: &Path, target_version: u32) -> Result<(), String> {
    let content = if pack_meta_path.exists() {
        read_text_with_fallback(pack_meta_path)?
    } else {
        String::new()
    };

    let mut data: Value = if content.trim().is_empty() {
        json!({"pack": {"pack_format": target_version, "description": "Converted by 2-Pyramid"}})
    } else {
        serde_json::from_str(&content).unwrap_or_else(|_| {
            json!({"pack": {"pack_format": target_version, "description": "Converted by 2-Pyramid"}})
        })
    };

    let description = data
        .get("pack")
        .and_then(|pack| pack.get("description"))
        .map(normalize_description)
        .unwrap_or_else(|| "Converted by 2-Pyramid".to_string());

    let (maj, min) = pack_format_major_minor(target_version);
    if target_version >= 69 {
        // 26.x 用 min/max_format；不要塞 pack_format:34 + 跨大版本 supported_formats
        // （26.3 上会显示不兼容）。min 与 max 落在同一 major，minor 覆盖 0..=目标。
        //
        // 注意：只改 pack 对象内部，**不要整体重建文档** —— 重建会抹掉顶层
        // 的 overlays / filter 等字段，用 overlays 的包转换后失效。
        if data.get("pack").is_none() {
            data["pack"] = json!({});
        }
        if let Some(pack) = data["pack"].as_object_mut() {
            pack.remove("pack_format");
            pack.remove("supported_formats");
            pack.insert("min_format".to_string(), json!([maj, 0]));
            pack.insert("max_format".to_string(), json!([maj, min]));
            pack.insert("description".to_string(), json!(description));
        }
    } else {
        if data.get("pack").is_none() {
            data["pack"] = json!({});
        }
        data["pack"]["pack_format"] = json!(target_version);
        data["pack"]["description"] = json!(description);
    }

    let pretty = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("failed to serialize pack.mcmeta: {}", e))?;
    fs::write(pack_meta_path, pretty)
        .map_err(|e| format!("failed to write {}: {}", pack_meta_path.display(), e))
}

pub fn build_output_path_for_batch(
    input_zip: &Path,
    target_version: u32,
    parent_folder_path: Option<&str>,
    output_dir_override: Option<&str>,
) -> Result<PathBuf, String> {
    build_output_path(input_zip, target_version, parent_folder_path, output_dir_override)
}

/// Render the user's output-naming template.
///
/// Supported placeholders:
///   * `[Ver]`  — target version label in brackets, e.g. `[Java 1.20-1.20.1]`
///   * `[Time]` — timestamp `YYYYMMDD-HHMMSS`
///   * `[Date]` — date `YYYY-MM-DD`
///   * `[Name]` — source pack name (kept for legacy templates)
///
/// Unknown text passes through verbatim (sanitised for the filesystem),
/// so `[Ver]欢迎使用2-Pyramid` → `[Java 1.20-1.20.1]欢迎使用2-Pyramid`.
/// An empty/whitespace-only template falls back to the pack name alone.
fn apply_naming_template(template: &str, name: &str, version: &str, time: &str, date: &str) -> String {
    let rendered = template
        .replace("[Ver]", &format!("[{}]", version))
        .replace("[Name]", name)
        .replace("[Time]", time)
        .replace("[Date]", date);
    let trimmed = sanitize_filename_component(rendered.trim());
    if trimmed.is_empty() {
        name.to_string()
    } else {
        trimmed
    }
}

/// Replace characters that are illegal in Windows file names so a
/// user-supplied template can never produce an uncreatable path.
fn sanitize_filename_component(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect()
}

pub fn build_output_path(
    input_zip: &Path,
    target_version: u32,
    parent_folder_path: Option<&str>,
    output_dir_override: Option<&str>,
) -> Result<PathBuf, String> {
    let label = pack_format_label(target_version);
    let base_name = input_zip
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("resource_pack");
    let cleaned_base = strip_version_prefix(base_name);

    // User-configurable naming template. Legacy values from before the
    // template system are migrated on read:
    //   "default" / empty → "[Ver][Name]"
    //   "timestamp"       → "[Ver][Time]"
    //   "overwrite"       → "[Name]"
    let naming = crate::commands::read_config_file()
        .ok()
        .and_then(|c| c.output_naming)
        .unwrap_or_default();
    let template = match naming.as_str() {
        "" | "default" => "[Ver][Name]".to_string(),
        "timestamp" => "[Ver][Time]".to_string(),
        "overwrite" => "[Name]".to_string(),
        other => other.to_string(),
    };

    let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let date = Local::now().format("%Y-%m-%d").to_string();
    let stem = apply_naming_template(&template, &cleaned_base, label, &stamp, &date);
    // 基岩版产物为 .mcpack（本质仍是 zip）
    let ext = if target_version == 1000 { "mcpack" } else { "zip" };
    let mut file_name = format!("{}.{}", stem, ext);

    let output_dir = if let Some(override_dir) = output_dir_override {
        Path::new(override_dir).to_path_buf()
    } else if let Some(parent_folder) = parent_folder_path {
        let parent_path = Path::new(parent_folder);
        let parent_name = parent_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("resource_packs");
        let cleaned_parent = strip_version_prefix(parent_name);
        let grandparent = parent_path.parent().unwrap_or(parent_path);
        grandparent.join(format!("[{}]{}", label, cleaned_parent))
    } else {
        input_zip.parent().unwrap_or_else(|| Path::new(".")).to_path_buf()
    };

    fs::create_dir_all(&output_dir)
        .map_err(|e| format!("failed to create output directory {}: {}", output_dir.display(), e))?;

    // Collision handling is always "append a numeric suffix" — the
    // user chose never to overwrite (see naming settings discussion).
    let mut output_path = output_dir.join(&file_name);
    let mut counter = 1;
    while output_path.exists() {
        file_name = format!("{} ({}).{}", stem, counter, ext);
        output_path = output_dir.join(&file_name);
        counter += 1;
    }

    Ok(output_path)
}

pub fn process_extracted_dir_only(
    input_zip: &Path,
    temp_dir: &Path,
    target_version: u32,
) -> Result<(), String> {
    // 目录规整最先执行：定位/提升 pack.mcmeta（含 pack.mcmeta.txt 防呆）
    let pack_meta_path = normalize_pack_structure(temp_dir)
        .or_else(|| find_pack_mcmeta(temp_dir))
        .unwrap_or_else(|| temp_dir.join("pack.mcmeta"));
    let source_version = read_pack_format(&pack_meta_path).unwrap_or(1);

    crate::invoke_conversion::invoke_conversion(
        input_zip,
        temp_dir,
        target_version,
        source_version,
    )
    .map_err(|e| format!("conversion pipeline failed: {}", e))?;

    write_pack_format(&pack_meta_path, target_version)?;

    Ok(())
}

pub fn convert_resource_pack(file_path: &str, target_version: u32) -> Result<String, String> {
    process_zip(file_path, target_version, None, 1.0, None, None, false, true)
}

/// 仅执行 Bedrock 结构转换边任务（j2b: 84→1000 / b2j: 1000→84）。
/// 任务注册与逻辑均在 converters/bedrock；此处只驱动 Scheduler。
fn run_bedrock_edge_task(
    work_dir: &Path,
    source_version: u32,
    target_version: u32,
    pack_name: &str,
) -> Result<(), String> {
    use crate::hurray::context::HurrayContext;
    use crate::hurray::scheduler::Scheduler;
    use crate::hurray::texture::TexturePool;

    let mut scheduler = Scheduler::new();
    crate::converters::bedrock::register_tasks(&mut scheduler);

    let work_dir_str = work_dir.to_str().unwrap_or("");
    let context = HurrayContext::new(work_dir_str);
    context.set_data("pack_name", pack_name);
    let mut texture_pool = TexturePool::new();
    scheduler
        .execute_version_conversion(
            &context,
            &mut texture_pool,
            source_version,
            target_version,
        )
        .map_err(|e| format!("bedrock edge task failed: {}", e))?;
    context
        .execute_cleanup()
        .map_err(|e| format!("bedrock cleanup failed: {}", e))?;
    Ok(())
}

/// 转换耗时（秒）。`pure` = 纯转换（引擎），`total` = 含 IO 的总时间。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionTiming {
    pub pure_s: f32,
    pub total_s: f32,
    /// IO：解压、重新打包与临时目录清理
    pub extract_s: f32,
    pub pack_s: f32,
    /// 临时目录清理（删除解压出来的整棵树；几千个小文件时不可忽略）
    pub cleanup_s: f32,
    /// 引擎内部分段：预检（基岩探测/结构规整/结构分析）、转换管线、收尾（mcmeta/edge）
    pub preflight_s: f32,
    pub pipeline_s: f32,
    pub post_s: f32,
    /// 逐任务画像（按耗时降序；并行任务含线程争用）。随返回值一起给出，
    /// 便于 CLI/报告使用（全局画像表在日志输出时已被取走）。
    pub task_profile: Vec<crate::hurray::scheduler::TaskTiming>,
}

/// 兼容入口：只关心输出路径的调用方（单文件转换、单测）用这个。
pub fn process_zip(
    original_file_path: &str,
    pack_format2: u32,
    _progress_callback: Option<fn(f64, &str)>,
    _file_weight: f64,
    parent_folder_path: Option<&str>,
    output_dir_override: Option<&str>,
    fix_alpha_layers: bool,
    adapt_shaders: bool,
) -> Result<String, String> {
    process_zip_timed(
        original_file_path,
        pack_format2,
        _progress_callback,
        _file_weight,
        parent_folder_path,
        output_dir_override,
        fix_alpha_layers,
        adapt_shaders,
    )
    .map(|(path, _timing)| path)
}

/// 完整转换：返回 `(输出路径, 耗时)`。日志里同时记录「纯转换时间」与
/// 「总时间（含 IO）」两个口径。
pub fn process_zip_timed(
    original_file_path: &str,
    pack_format2: u32,
    _progress_callback: Option<fn(f64, &str)>,
    _file_weight: f64,
    parent_folder_path: Option<&str>,
    output_dir_override: Option<&str>,
    fix_alpha_layers: bool,
    adapt_shaders: bool,
) -> Result<(String, ConversionTiming), String> {
    let input_zip = Path::new(original_file_path);
    if !input_zip.exists() {
        return Err(format!("input file not found: {}", input_zip.display()));
    }

    // 目标为 Bedrock（1000）或输入为 Bedrock 包时的编排。
    // 结构转换逻辑在 converters/bedrock/*；此处只调度 Scheduler 边任务。
    let is_bedrock_target = pack_format2 == 1000;
    // Bedrock 中间态统一到最新 Java 26.3（pack_format 97），再经边 (97→1000) 重组
    let java_target = if is_bedrock_target { 97 } else { pack_format2 };

    // ── 计时：总时间（含 IO） / 纯转换时间（引擎） / IO 分解 ──────────
    // total = extract(IO) + engine(纯转换) + repack(IO)
    let total_start = std::time::Instant::now();

    let temp_dir = tempfile::Builder::new()
        .prefix(crate::converters::zip::WORK_DIR_PREFIX)
        .tempdir()
        .map_err(|e| format!("failed to create temp dir: {}", e))?;
    let temp_dir_path = temp_dir.path().to_string_lossy().to_string();

    // 启动时顺手清理陈旧残留（异常退出/后台删除未完成的 .2pyr-work-*）；
    // 只删超过 2 小时的目录，避免误伤正在并发的其他转换。
    let swept = crate::converters::zip::sweep_stale_work_dirs(
        &std::env::temp_dir(),
        std::time::Duration::from_secs(2 * 60 * 60),
    );
    if swept > 0 {
        log_info!("swept {} stale work dir(s) from previous runs", swept);
    }

    let extract_start = std::time::Instant::now();
    extract_resource_pack(original_file_path, &temp_dir_path)?;
    let extract_elapsed = extract_start.elapsed();

    // 纯转换区段：b2j 预转换（若有）、结构分析、引擎管线、mcmeta 改写、j2b
    let engine_start = std::time::Instant::now();

    let mut source_version: u32;
    if crate::converters::bedrock::is_bedrock_resource_pack(temp_dir.path()) {
        log_info!("detected Bedrock resource pack source; running b2j first");
        // b2j 产出 Java 26.3（97）树
        run_bedrock_edge_task(temp_dir.path(), 1000, 97, "Converted Pack")?;
        source_version = 97;
        let pack_meta_path = temp_dir.path().join("pack.mcmeta");
        if pack_meta_path.exists() {
            if let Ok(v) = read_pack_format(&pack_meta_path) {
                source_version = v;
            }
        }
    } else {
        let pack_meta_path = normalize_pack_structure(temp_dir.path())
            .or_else(|| find_pack_mcmeta(temp_dir.path()))
            .unwrap_or_else(|| temp_dir.path().join("pack.mcmeta"));
        if !pack_meta_path.exists() {
            log_warn!("pack.mcmeta not found, creating a new one at {}", pack_meta_path.display());
        }
        source_version = read_pack_format(&pack_meta_path).unwrap_or(1);
    }
    log_info!("detected pack_format: {}", source_version);

    // 结构分析（只读、只记录，不改变本次转换行为）：
    // 多版本包（overlays / supported_formats / 版本折叠目录 / 一包多根）目前
    // 只转换基础层，覆盖层原样保留——先把真实结构打进日志，便于据此定语义。
    match crate::converters::pack_analysis::analyze_dir(temp_dir.path()) {
        Ok(report) => {
            log_info!("pack structure: {}", report.summary());
            for layer in &report.layers {
                if layer.directory.is_empty() {
                    continue;
                }
                log_info!(
                    "  layer {} formats={:?} files={} overrides={}",
                    layer.directory,
                    layer.formats,
                    layer.file_count,
                    layer.override_count
                );
            }
            for dir in &report.folding_dirs {
                log_info!("  folding dir {} files={}", dir.directory, dir.file_count);
            }
            for w in &report.warnings {
                log_warn!("  pack analysis: {}", w);
            }
        }
        Err(e) => log_warn!("pack analysis failed: {}", e),
    }

    if is_bedrock_target {
        log_info!("bedrock target: convert to Java 26.3 (format 97) first, then j2b");
    }

    // 预检阶段结束：基岩探测 / 结构规整 / 结构分析 / 源格式读取
    let preflight_elapsed = engine_start.elapsed();

    // Bedrock 目标时 Java 中间态跳过 GuiSurgeon，避免 sprite 手术干扰 j2b
    let pipeline_start = std::time::Instant::now();
    crate::invoke_conversion::invoke_conversion_ex(
        input_zip,
        temp_dir.path(),
        java_target,
        source_version,
        !is_bedrock_target,
        fix_alpha_layers,
        adapt_shaders,
    )
    .map_err(|e| format!("conversion pipeline failed: {}", e))?;
    let pipeline_elapsed = pipeline_start.elapsed();

    // 收尾阶段：mcmeta 改写 + 可选 j2b
    let post_start = std::time::Instant::now();
    let pack_meta_path = temp_dir.path().join("pack.mcmeta");
    if pack_meta_path.exists() {
        write_pack_format(&pack_meta_path, java_target)?;
    }

    if is_bedrock_target {
        let base_name = input_zip
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("resource_pack");
        let pack_name = strip_version_prefix(base_name);
        // 26.3 → Bedrock
        run_bedrock_edge_task(temp_dir.path(), 97, 1000, &pack_name)?;
    }
    let post_elapsed = post_start.elapsed();

    // 纯转换区段结束（预检 + 引擎管线 + 收尾）
    let engine_elapsed = engine_start.elapsed();

    let pack_start = std::time::Instant::now();
    let output_path = build_output_path(input_zip, pack_format2, parent_folder_path, output_dir_override)?;

    repack_resource_pack(&temp_dir_path, &output_path.to_string_lossy())?;
    let pack_elapsed = pack_start.elapsed();

    if !output_path.exists() {
        return Err(format!("output file not found after repack: {}", output_path.display()));
    }

    let total_elapsed = total_start.elapsed();

    // 临时目录清理：默认交给后台（GUI 用后台线程；CLI/脚本用脱离进程的
    // `rmdir`，进程立即退出、删除继续在系统里完成），删除 4000+ 文件在
    // Windows 上要 1.5–2.5s，全部是杀软逐个扫描的开销，不该占用等待时间；
    // `2PYR_SYNC_CLEANUP=1` 可强制同步删除以便基准测试。
    let cleanup_start = std::time::Instant::now();
    let work_path = temp_dir.into_path();
    let cleanup_done_inline = crate::converters::zip::dispatch_cleanup(&work_path);
    let cleanup_elapsed = if cleanup_done_inline {
        cleanup_start.elapsed()
    } else {
        std::time::Duration::ZERO
    };
    let cleanup_async = !cleanup_done_inline;
    let total_elapsed = total_elapsed + cleanup_elapsed;

    // 逐任务画像：取走本次转换的任务耗时，输出 top-N（并行任务含线程争用，
    // 属"墙钟占用"而非纯 CPU 时间）。取走的列表随 timing 一起返回，供 CLI 报告。
    let task_timings = crate::hurray::scheduler::take_task_timings();
    if !task_timings.is_empty() {
        let task_sum: f32 = task_timings.iter().map(|t| t.seconds).sum();
        log_info!(
            "task profile: {} tasks, sum={:.2}s (parallel wall time, incl. contention)",
            task_timings.len(),
            task_sum
        );
        for t in task_timings.iter().take(8) {
            log_info!(
                "  task {:<28} {:>7.2}s  [{}]{}{}",
                t.task,
                t.seconds,
                t.tier,
                if t.parallel { " parallel" } else { "" },
                if t.seconds >= 1.0 { "  ← 大头" } else { "" }
            );
        }
    }

    let timing = ConversionTiming {
        pure_s: engine_elapsed.as_secs_f32(),
        total_s: total_elapsed.as_secs_f32(),
        extract_s: extract_elapsed.as_secs_f32(),
        pack_s: pack_elapsed.as_secs_f32(),
        cleanup_s: cleanup_elapsed.as_secs_f32(),
        preflight_s: preflight_elapsed.as_secs_f32(),
        pipeline_s: pipeline_elapsed.as_secs_f32(),
        post_s: post_elapsed.as_secs_f32(),
        task_profile: task_timings,
    };
    // 一行同时给出两个口径：pure = 纯转换（引擎），total = 含 IO 的总时间。
    // 括号里是 IO 分解与引擎分段，便于判断瓶颈在引擎哪一段还是磁盘。
    log_info!(
        "conversion timing: pure={:.2}s total={:.2}s (extract={:.2}s, pack={:.2}s, cleanup={}, io={:.2}s)",
        timing.pure_s,
        timing.total_s,
        timing.extract_s,
        timing.pack_s,
        if cleanup_async {
            "async".to_string()
        } else {
            format!("{:.2}s", timing.cleanup_s)
        },
        timing.extract_s + timing.pack_s + timing.cleanup_s
    );
    log_info!(
        "engine breakdown: preflight={:.2}s pipeline={:.2}s post={:.2}s | tasks: {} runs, sum={:.2}s, scheduler+worker overhead={:.2}s",
        timing.preflight_s,
        timing.pipeline_s,
        timing.post_s,
        timing.task_profile.len(),
        timing.task_profile.iter().map(|t| t.seconds).sum::<f32>(),
        (timing.pipeline_s - timing.task_profile.iter().map(|t| t.seconds).sum::<f32>()).max(0.0)
    );

    Ok((output_path.to_string_lossy().to_string(), timing))
}

pub fn register_task(_engine: &mut crate::hurray::engine::HurrayEngine) {
    // Not a standalone task: this module orchestrates end-to-end ZIP conversion.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// 耗时口径回归：`process_zip_timed` 必须同时给出「纯转换」与「总时间」
    /// 两个数，且 total ≥ pure（total 额外含解压与打包 IO）。
    #[test]
    fn resolve_target_format_accepts_common_forms() {
        // 数字
        assert_eq!(resolve_target_format("34").unwrap(), 34);
        assert_eq!(resolve_target_format("1000").unwrap(), 1000);
        // 完整标签（前端同款）
        assert_eq!(resolve_target_format("1.21-1.21.1").unwrap(), 34);
        assert_eq!(resolve_target_format("Java 1.20-1.20.1").unwrap(), 15);
        // 短版本前缀
        assert_eq!(resolve_target_format("1.21").unwrap(), 34);
        assert_eq!(resolve_target_format("1.20").unwrap(), 15);
        assert_eq!(resolve_target_format("26.1").unwrap(), 84);
        assert_eq!(resolve_target_format("26.3").unwrap(), 97);
        // 基岩
        assert_eq!(resolve_target_format("bedrock").unwrap(), 1000);
        assert_eq!(resolve_target_format("Bedrock Latest").unwrap(), 1000);
        // 空白/大小写
        assert_eq!(resolve_target_format("  1.21.4 ").unwrap(), 46);
        // 错误
        assert!(resolve_target_format("").is_err());
        assert!(resolve_target_format("9.99").is_err());
    }

    #[test]
    fn timing_reports_pure_and_total() {
        use std::io::Write;

        let dir = tempdir().expect("tempdir");
        let zip_path = dir.path().join("tiny.zip");

        // 造一个最小但合法（含可解码 PNG）的包
        let png = {
            let img = image::RgbaImage::from_pixel(16, 16, image::Rgba([200, 40, 40, 255]));
            let mut buf = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgba8(img)
                .write_to(&mut buf, image::ImageFormat::Png)
                .expect("encode png");
            buf.into_inner()
        };
        {
            let file = std::fs::File::create(&zip_path).expect("create zip");
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::FileOptions::default();
            zip.start_file("pack.mcmeta", opts).expect("start");
            zip.write_all(br#"{"pack":{"pack_format":15,"description":"tiny"}}"#)
                .expect("write");
            zip.start_file("pack.png", opts).expect("start");
            zip.write_all(&png).expect("write");
            zip.start_file("assets/minecraft/textures/item/apple.png", opts)
                .expect("start");
            zip.write_all(&png).expect("write");
            zip.start_file("assets/minecraft/textures/block/stone.png", opts)
                .expect("start");
            zip.write_all(&png).expect("write");
            zip.finish().expect("finish");
        }

        let out_dir = dir.path().join("out");
        fs::create_dir_all(&out_dir).expect("mkdir out");

        let (_path, timing) = process_zip_timed(
            zip_path.to_str().expect("path"),
            34,
            None,
            1.0,
            None,
            Some(out_dir.to_str().expect("out path")),
            false,
            true,
        )
        .expect("conversion should succeed");

        assert!(timing.total_s > 0.0, "total time must be measured");
        assert!(timing.pure_s > 0.0, "pure time must be measured");
        assert!(
            timing.total_s >= timing.pure_s,
            "total ({}) must be >= pure ({})",
            timing.total_s,
            timing.pure_s
        );
        assert!(
            timing.extract_s >= 0.0 && timing.pack_s >= 0.0,
            "IO breakdown must be non-negative"
        );
    }

    /// End-to-end smoke test: copy the Pika 5K 16x resource pack's container
    /// folder into a tempdir, run the full conversion pipeline targeting
    /// pack_format 34 (1.21), and verify the result has the expected
    /// structure (sprite atlas files exist, pack.mcmeta updated, container
    /// files modified).
    #[test]
    fn test_full_pipeline_pika5k_to_121() {
        let rp_root = r"D:\GameTime\.minecraft\versions\1.20.1-OptiFine_I6\resourcepacks\!   §1§b§lPika 5K - 16x";
        if !std::path::Path::new(rp_root).exists() {
            eprintln!("SKIP: Pika 5K not found at {}", rp_root);
            return;
        }

        let temp = tempdir().expect("tempdir");
        let src_container = std::path::Path::new(rp_root)
            .join("assets/minecraft/textures/gui/container");
        let dst_container = temp.path().join("assets/minecraft/textures/gui/container");
        fs::create_dir_all(&dst_container).expect("mkdir container");
        for entry in fs::read_dir(&src_container).expect("read src") {
            let entry = entry.expect("entry");
            let ft = entry.file_type().expect("ft");
            if ft.is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".png")
            {
                fs::copy(entry.path(), dst_container.join(entry.file_name()))
                    .expect("copy");
            }
        }

        // Drop a pack.mcmeta declaring pack_format=6 (1.16-1.20 era)
        let mcmeta = temp.path().join("pack.mcmeta");
        fs::write(
            &mcmeta,
            r#"{"pack":{"pack_format":6,"description":"Pika 5K - 16x (test fixture)"}}"#,
        )
        .expect("write mcmeta");

        let src_zip = std::path::Path::new(rp_root).join("pack.mcmeta");
        if let Err(e) = process_extracted_dir_only(&src_zip, temp.path(), 34) {
            panic!("process_extracted_dir_only failed: {}", e);
        }

        // 1. pack.mcmeta updated
        let new_mcmeta = fs::read_to_string(&mcmeta).expect("read mcmeta");
        assert!(
            new_mcmeta.contains("pack_format") && new_mcmeta.contains(": 34"),
            "pack.mcmeta should be updated to pack_format 34: {}",
            new_mcmeta
        );
        assert!(
            new_mcmeta.contains("Pika 5K"),
            "original description should be preserved: {}",
            new_mcmeta
        );

        // 2. Sprite atlas populated by cut_gui / GuiSurgeon. The
        //    container-level smithing.png / cartography_table.png were
        //    generated by fix_smithing2_villager2_ui / fix_machinery_ui
        //    EARLIER in the pipeline, then consumed by cut_gui to make
        //    sprites, then DELETED by the deferred cleanup step (1.21
        //    sprite mode doesn't keep the legacy atlas PNGs). The fact
        //    that they were deleted is itself a sign the pipeline ran end
        //    to end. Verify the resulting sprites instead.
        let sprites = temp
            .path()
            .join("assets/minecraft/textures/gui/sprites/container");
        assert!(
            sprites.join("smithing/template_slot.png").exists(),
            "smithing/template_slot.png must be produced by cut_gui from fix_smithing2_villager2_ui's smithing.png"
        );
        assert!(
            sprites.join("smithing/error.png").exists(),
            "smithing/error.png must be produced by cut_gui"
        );
        assert!(
            sprites.join("cartography_table/duplicated_map.png").exists(),
            "cartography_table/duplicated_map.png must be produced by cut_gui from fix_machinery_ui's cartography_table.png"
        );
        assert!(
            sprites.join("grindstone/input_slot.png").exists(),
            "grindstone/input_slot.png must be produced by cut_gui from fix_machinery_ui's grindstone.png"
        );

        // 3. Confirm the deferred cleanup actually removed legacy atlas PNGs.
        //    If cut_gui deferred them but cleanup didn't run, the legacy
        //    PNGs would still be present.
        assert!(
            !dst_container.join("smithing.png").exists(),
            "smithing.png should have been cleaned up after cut_gui"
        );
        assert!(
            !dst_container.join("anvil.png").exists(),
            "anvil.png should have been cleaned up after cut_gui"
        );
    }

    /// 防呆：pack.mcmeta.txt（多扩展名）含合法 format 数值 → 提升为根目录 pack.mcmeta
    #[test]
    fn test_normalize_promotes_pack_mcmeta_txt() {
        let temp = tempdir().expect("tempdir");
        let nested = temp.path().join("sub/pack.mcmeta.txt");
        fs::create_dir_all(nested.parent().unwrap()).expect("mkdir");
        fs::write(
            &nested,
            r#"{"pack":{"pack_format":15,"description":"txt 后缀的材质包"}}"#,
        )
        .expect("write");

        let found = normalize_pack_structure(temp.path()).expect("should find candidate");
        assert_eq!(found, temp.path().join("pack.mcmeta"));
        assert!(
            temp.path().join("pack.mcmeta").is_file(),
            "pack.mcmeta should be promoted to root"
        );
        assert!(
            !nested.exists(),
            "original pack.mcmeta.txt should be removed after promotion"
        );
    }

    /// 防呆：pack.mcmeta.txt 但内容不是 mcmeta（无 format 数值）→ 不采纳
    #[test]
    fn test_normalize_rejects_fake_pack_mcmeta_txt() {
        let temp = tempdir().expect("tempdir");
        let fake = temp.path().join("pack.mcmeta.txt");
        fs::write(&fake, "这不是 mcmeta 文件").expect("write");

        assert!(
            normalize_pack_structure(temp.path()).is_none(),
            "fake pack.mcmeta.txt must not be promoted"
        );
        assert!(fake.exists());
        assert!(!temp.path().join("pack.mcmeta").exists());
    }

    /// 前缀替换：标签在开头/末尾/中间都应被剥掉，且空白被压缩
    #[test]
    fn test_strip_version_prefix_anywhere() {
        assert_eq!(strip_version_prefix("[Java 1.20-1.20.1]我的包"), "我的包");
        assert_eq!(strip_version_prefix("我的包 [Java 1.20-1.20.1]"), "我的包");
        assert_eq!(strip_version_prefix("我的包[Java 1.20-1.20.1]"), "我的包");
        assert_eq!(
            strip_version_prefix("[Java 1.16.2-1.16.5] [Java 1.20-1.20.1] 我的 包"),
            "我的 包"
        );
        // 无标签的名称原样保留
        assert_eq!(strip_version_prefix("我的包"), "我的包");
    }

    /// 26.x（target ≥ 69）改写 pack.mcmeta 时不得丢掉顶层字段。
    /// 回归点：旧实现用 `data = json!({...})` 整体替换文档，`overlays` 被抹掉。
    #[test]
    fn test_write_pack_format_keeps_top_level_overlays() {
        let temp = tempfile::tempdir().expect("tempdir");
        let meta = temp.path().join("pack.mcmeta");
        fs::write(
            &meta,
            r#"{
  "pack": { "pack_format": 34, "description": "我的整合包" },
  "overlays": {
    "entries": [
      { "formats": [34], "directory": "overlay_34" }
    ]
  }
}"#,
        )
        .expect("write");

        // 目标 97 = 26.3（major/minor 形式）
        write_pack_format(&meta, 97).expect("write_pack_format");

        let out: Value =
            serde_json::from_str(&fs::read_to_string(&meta).expect("read")).expect("parse");

        // 顶层 overlays 必须保留
        assert!(
            out.get("overlays").is_some(),
            "顶层 overlays 被丢弃：{:?}",
            out
        );
        assert_eq!(
            out["overlays"]["entries"][0]["directory"], "overlay_34",
            "overlays 内容被改写"
        );
        // pack 内的写法切换为 min/max_format
        assert!(out["pack"].get("min_format").is_some(), "缺少 min_format");
        assert!(out["pack"].get("max_format").is_some(), "缺少 max_format");
        assert!(
            out["pack"].get("pack_format").is_none(),
            "26.x 不应再写 pack_format"
        );
        // description 保留原值（经归一化）
        assert_eq!(out["pack"]["description"], "我的整合包");
    }

    /// 非 26.x 分支：只改 pack 字段，其他顶层内容原样保留
    #[test]
    fn test_write_pack_format_legacy_keeps_other_keys() {
        let temp = tempfile::tempdir().expect("tempdir");
        let meta = temp.path().join("pack.mcmeta");
        fs::write(
            &meta,
            r#"{"pack": {"pack_format": 34, "description": "包"}, "filter": {"block": []}}"#,
        )
        .expect("write");

        write_pack_format(&meta, 15).expect("write_pack_format");
        let out: Value =
            serde_json::from_str(&fs::read_to_string(&meta).expect("read")).expect("parse");

        assert_eq!(out["pack"]["pack_format"], 15);
        assert!(out.get("filter").is_some(), "顶层 filter 被丢弃");
    }
}
