//! 自定义背景图片 + 自动主题色提取。
//!
//! `set_background` 把用户选择的图片复制到 `~/.2pyr/background/`
//! （支持 png / jpg / jpeg / webp / gif / bmp），可选地从图片像素
//! 直方图提取主题色并写入 `palette.theme_color`，然后把路径、展示
//! 方式（cover/contain/stretch/tile）与透色强度写入 settings.json。
//! 前端用 `convertFileSrc` 直接渲染该文件，CSS 的 background-size
//! 随窗口尺寸自适应，无需任何 JS 干预。

use std::collections::HashMap;
use std::path::Path;

/// 支持的图片扩展名（小写）。
const ALLOWED_EXTS: [&str; 6] = ["png", "jpg", "jpeg", "webp", "gif", "bmp"];

fn background_dir() -> Result<std::path::PathBuf, String> {
    let base = dirs::home_dir().ok_or_else(|| "Failed to get user home directory".to_string())?;
    Ok(base.join(".2pyr").join("background"))
}

fn allowed_ext(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    ALLOWED_EXTS.iter().find(|e| **e == ext).copied()
}

/// 主题色提取：缩小到 64×64，跳过透明像素，把颜色量化到
/// 4 bit/通道 做直方图统计，取出现次数最多的颜色。结果形如
/// `#3a7bd5`。图片无法解码时返回 None（不影响背景设置本身）。
pub fn dominant_color(path: &Path) -> Option<String> {
    let img = image::open(path).ok()?.to_rgba8();
    let small = image::imageops::resize(&img, 64, 64, image::imageops::FilterType::Triangle);
    let mut hist: HashMap<(u8, u8, u8), u32> = HashMap::new();
    for p in small.pixels() {
        if p[3] < 128 {
            continue; // 跳过透明像素
        }
        let key = (p[0] >> 4, p[1] >> 4, p[2] >> 4);
        *hist.entry(key).or_insert(0) += 1;
    }
    hist.into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|((r, g, b), _)| {
            // 4 bit → 8 bit：复制高四位到低四位
            format!("#{:02x}{:02x}{:02x}", (r << 4) | r, (g << 4) | g, (b << 4) | b)
        })
}

/// 设置（更换）背景：复制图片 → 可选提取主题色 → 写配置。
#[tauri::command]
pub fn set_background(
    file_path: String,
    fit: String,
    opacity: f64,
    extract_color: bool,
) -> Result<serde_json::Value, String> {
    let src = Path::new(&file_path);
    if !src.exists() {
        return Err(format!("File not found: {}", file_path));
    }
    let ext = allowed_ext(src)
        .ok_or_else(|| "Unsupported image format (png/jpg/webp/gif/bmp)".to_string())?;

    let dir = background_dir()?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create background dir: {}", e))?;

    // 清掉旧背景文件，避免多次更换后堆积
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("background.") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    let dest = dir.join(format!("background.{}", ext));
    std::fs::copy(src, &dest)
        .map_err(|e| format!("Failed to copy image: {}", e))?;

    let theme = if extract_color { dominant_color(&dest) } else { None };

    let normalized_fit = match fit.as_str() {
        "contain" | "stretch" | "tile" => fit,
        _ => "cover".to_string(),
    };

    let mut cfg = crate::commands::read_config_file()?;
    cfg.background_image = Some(dest.to_string_lossy().to_string());
    cfg.background_fit = Some(normalized_fit);
    cfg.background_opacity = Some(opacity.clamp(0.1, 1.0));
    if let Some(ref color) = theme {
        let mut palette = cfg.palette.clone().unwrap_or_else(|| serde_json::json!({}));
        if let Some(obj) = palette.as_object_mut() {
            obj.insert("theme_color".into(), serde_json::json!(color));
        }
        cfg.palette = Some(palette);
    }
    crate::commands::write_config_file(&cfg)?;

    crate::log_info!("OKAY set_background [{}]", file_path);
    Ok(serde_json::json!({
        "background_path": cfg.background_image,
        "theme_color": theme,
    }))
}

/// 移除自定义背景（删除文件 + 清配置，主题色保留）。
#[tauri::command]
pub fn clear_background() -> Result<(), String> {
    let dir = background_dir()?;
    if dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("background.") {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
    let mut cfg = crate::commands::read_config_file()?;
    cfg.background_image = None;
    crate::commands::write_config_file(&cfg)?;
    crate::log_info!("OKAY clear_background");
    Ok(())
}

/// 按路径读取图片字节（前端 asset 协议不可用时的回退方案）。
/// 大小上限 50MB，仅接受图片扩展名。
#[tauri::command]
pub fn read_image_bytes(path: String) -> Result<Vec<u8>, String> {
    let p = Path::new(&path);
    if allowed_ext(p).is_none() {
        return Err("Unsupported image format".to_string());
    }
    let meta = std::fs::metadata(p).map_err(|e| format!("Failed to stat file: {}", e))?;
    if meta.len() > 50 * 1024 * 1024 {
        return Err("Image too large (>50MB)".to_string());
    }
    std::fs::read(p).map_err(|e| format!("Failed to read file: {}", e))
}
