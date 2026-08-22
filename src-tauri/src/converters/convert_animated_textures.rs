//! 动态贴图（animated texture）mcmeta 升级。
//!
//! 老版本资源包的动画贴图 mcmeta（`textures/items` 或 `textures/item` 下与
//! png 同名的 `.png.mcmeta`）通常只写 `{"animation": {}}`；高版本需要显式
//! 声明 frametime / interpolate 才能稳定播放。本任务：
//!
//!   1. 遍历 item / items 目录（新版为 item，items 兜底——目录规整任务
//!      rename_blocks_items 已把旧版 items 并入 item，本任务需排在其后执行）；
//!   2. 找到与 png 同名的 `.png.mcmeta`；
//!   3. 若 animation 已显式声明 frametime → 视为已升级，跳过；
//!   4. 否则读取同名 png 尺寸推导帧数：
//!        * 正方形 → 1 帧（非条带）
//!        * 横向条带（宽 > 高且宽能被高整除）→ 帧数 = 宽 / 高
//!        * 纵向条带（高 > 宽且高能被宽整除）→ 帧数 = 高 / 宽
//!        * 无法整除 → 跳过（保留原样，不瞎猜）
//!   5. 把 mcmeta 改写为高版本适配格式：
//!        { "animation": { "frametime": <帧数>, "interpolate": true } }
//!      （原有其它 animation 字段保留）

use std::fs;
use std::path::Path;

use crate::log_info;

pub fn convert_animated_textures(resource_pack_path: &Path) -> Result<(), String> {
    let textures = resource_pack_path.join("assets/minecraft/textures");
    // 新版路径为主，旧版 items 兜底
    for sub in ["item", "items"] {
        let dir = textures.join(sub);
        if !dir.is_dir() {
            continue;
        }
        upgrade_animation_mcmetas(&dir)?;
    }
    Ok(())
}

fn upgrade_animation_mcmetas(dir: &Path) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("read dir failed: {}", e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !path.is_file() || !name.to_ascii_lowercase().ends_with(".png.mcmeta") {
            continue;
        }
        // 同名 png："a.png.mcmeta" -> "a.png"
        let png_path = path.with_extension("");
        if !png_path.is_file() {
            continue;
        }
        match upgrade_one(&path, &png_path) {
            Ok(Some(frames)) => {
                log_info!("OKAY convert_animation [{}] frames={}", path.display(), frames)
            }
            Ok(None) => {}
            Err(e) => log_info!("convert_animation skip [{}]: {}", path.display(), e),
        }
    }
    Ok(())
}

fn upgrade_one(mcmeta: &Path, png: &Path) -> Result<Option<u32>, String> {
    let content = fs::read_to_string(mcmeta).map_err(|e| format!("read failed: {}", e))?;
    let trimmed = content.trim_start_matches('\u{feff}');
    let mut data: serde_json::Value =
        serde_json::from_str(trimmed).map_err(|e| format!("json failed: {}", e))?;

    // 没有 animation 字段 → 不是动画贴图
    if data.get("animation").is_none() {
        return Ok(None);
    }
    let anim = data["animation"]
        .as_object_mut()
        .ok_or_else(|| "animation 不是对象".to_string())?;
    // 已显式声明 frametime → 已是高版本格式，跳过
    if anim.contains_key("frametime") {
        return Ok(None);
    }

    let frames = image_frames(png)?;
    anim.insert("frametime".into(), serde_json::json!(frames));
    anim.insert("interpolate".into(), serde_json::json!(true));

    let pretty =
        serde_json::to_string_pretty(&data).map_err(|e| format!("serialize failed: {}", e))?;
    fs::write(mcmeta, pretty).map_err(|e| format!("write failed: {}", e))?;
    Ok(Some(frames))
}

/// 由贴图尺寸推导帧数（只读 png 头，不做全量解码）。
fn image_frames(png: &Path) -> Result<u32, String> {
    let (w, h) = image::io::Reader::open(png)
        .map_err(|e| format!("open png failed: {}", e))?
        .with_guessed_format()
        .map_err(|e| format!("guess format failed: {}", e))?
        .into_dimensions()
        .map_err(|e| format!("read dimensions failed: {}", e))?;

    if w == h {
        return Ok(1);
    }
    if w > h && w % h == 0 {
        return Ok(w / h);
    }
    if h > w && h % w == 0 {
        return Ok(h / w);
    }
    Err(format!("无法从尺寸推导帧数 ({}x{})", w, h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_png(path: &Path, w: u32, h: u32) {
        let img = image::RgbaImage::from_pixel(w, h, image::Rgba([255, 0, 0, 255]));
        img.save(path).expect("save png");
    }

    #[test]
    fn test_upgrade_vertical_strip() {
        let temp = tempdir().expect("tempdir");
        let dir = temp.path().join("assets/minecraft/textures/item");
        fs::create_dir_all(&dir).expect("mkdir");
        // 纵向条带：32x64 → 2 帧
        write_png(&dir.join("water.png"), 32, 64);
        fs::write(&dir.join("water.png.mcmeta"), r#"{"animation": {}}"#).expect("write");

        convert_animated_textures(temp.path()).expect("convert");
        let out = fs::read_to_string(dir.join("water.png.mcmeta")).expect("read");
        let v: serde_json::Value = serde_json::from_str(&out).expect("json");
        assert_eq!(v["animation"]["frametime"], serde_json::json!(2));
        assert_eq!(v["animation"]["interpolate"], serde_json::json!(true));
    }

    #[test]
    fn test_upgrade_horizontal_strip() {
        let temp = tempdir().expect("tempdir");
        let dir = temp.path().join("assets/minecraft/textures/items");
        fs::create_dir_all(&dir).expect("mkdir");
        // 横向条带：64x32 → 2 帧
        write_png(&dir.join("lava.png"), 64, 32);
        fs::write(&dir.join("lava.png.mcmeta"), r#"{"animation": {}}"#).expect("write");

        convert_animated_textures(temp.path()).expect("convert");
        let out = fs::read_to_string(dir.join("lava.png.mcmeta")).expect("read");
        let v: serde_json::Value = serde_json::from_str(&out).expect("json");
        assert_eq!(v["animation"]["frametime"], serde_json::json!(2));
    }

    #[test]
    fn test_skip_explicit_frametime_and_non_animated() {
        let temp = tempdir().expect("tempdir");
        let dir = temp.path().join("assets/minecraft/textures/item");
        fs::create_dir_all(&dir).expect("mkdir");
        // 已显式 frametime → 不动
        write_png(&dir.join("a.png"), 32, 64);
        fs::write(
            &dir.join("a.png.mcmeta"),
            r#"{"animation": {"frametime": 5, "interpolate": false}}"#,
        )
        .expect("write");
        // 没有 animation 字段 → 不动
        write_png(&dir.join("b.png"), 32, 32);
        fs::write(&dir.join("b.png.mcmeta"), r#"{"texture": "x"}"#).expect("write");

        convert_animated_textures(temp.path()).expect("convert");
        let a = fs::read_to_string(dir.join("a.png.mcmeta")).expect("read");
        assert!(a.contains("\"frametime\": 5"));
        assert!(a.contains("false"));
        let b = fs::read_to_string(dir.join("b.png.mcmeta")).expect("read");
        assert!(!b.contains("frametime"));
    }

    #[test]
    fn test_skip_undeterminable_dimensions() {
        let temp = tempdir().expect("tempdir");
        let dir = temp.path().join("assets/minecraft/textures/item");
        fs::create_dir_all(&dir).expect("mkdir");
        // 48x32：48 % 32 != 0 → 无法推导，保持原样
        write_png(&dir.join("c.png"), 48, 32);
        fs::write(&dir.join("c.png.mcmeta"), r#"{"animation": {}}"#).expect("write");

        convert_animated_textures(temp.path()).expect("convert");
        let out = fs::read_to_string(dir.join("c.png.mcmeta")).expect("read");
        assert!(!out.contains("frametime"));
    }
}
