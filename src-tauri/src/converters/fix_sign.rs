// fix_sign.rs
//
// Generates 11 wood-variant sign textures from the base oak_sign.png using
// hue/brightness/saturation adjustment in HSV color space (matches pack.py
// `fix_sign` which calls `adjust_hue_brightness`).
//
// The previous implementation used an inlined HSL converter (rgb_to_hsl /
// hsl_to_rgb) — HSL and HSV produce visually different results, so the
// generated signs did not match the Python reference. The fix is to delegate
// to the shared `adjust_hue_brightness` helper which uses HSV (the same
// algorithm pack.py uses).
use std::path::Path;
use std::fs;

use crate::converters::adjust_hue_brightness::adjust_hue_brightness;
use crate::hurray::context::HurrayContext;
use crate::hurray::scheduler::{TaskType, TaskTier};

/// Repairs sign textures.
///
/// # Arguments
/// - `context`: Hurray context (temp_dir points at the extracted pack)
///
/// Mirrors pack.py `fix_sign` (lines 7021-7083):
/// 1. if `oak_sign.png` is missing, skip
/// 2. delete existing `spruce_sign.png` (if any)
/// 3. rename `oak_sign.png` -> `spruce_sign.png`
/// 4. generate 11 variants via `adjust_hue_brightness` (HSV)
///
/// # Returns
/// - `Ok(())` on success
/// - `Err(String)` on I/O or image errors
pub fn fix_sign(context: &HurrayContext) -> Result<(), String> {
    let resource_pack_path = context.temp_dir();
    let item_path = resource_pack_path.join("assets/minecraft/textures/item");
    let oak_sign_path = item_path.join("oak_sign.png");

    if !oak_sign_path.exists() {
        crate::log_info!("未找到 oak_sign.png，跳过告示牌处理");
        return Ok(());
    }

    crate::log_info!("处理告示牌图: {:?}", item_path);

    let spruce_sign_path = item_path.join("spruce_sign.png");
    if spruce_sign_path.exists() {
        fs::remove_file(&spruce_sign_path)
            .map_err(|e| format!("删除文件失败: {:?}", e))?;
        crate::log_info!("已删除现有的 spruce_sign.png");
    }

    fs::rename(&oak_sign_path, &spruce_sign_path)
        .map_err(|e| format!("重命名文件失败: {:?}", e))?;
    crate::log_info!("已将 oak_sign.png 重命名为 spruce_sign.png");

    let base_img = image::open(&spruce_sign_path)
        .map_err(|e| format!("无法打开图像: {:?}", e))?
        .to_rgba8();

    // (filename, hue_shift 0-360, brightness_shift -100..100, saturation_shift -100..100)
    // Matches pack.py `fix_sign` lines 7047-7059.
    let sign_variants: [(&str, f32, f32, f32); 11] = [
        ("oak_sign.png",        0.0,   15.0,    0.0),
        ("birch_sign.png",      0.0,   40.0,    0.0),
        ("acacia_sign.png",   -23.0,   10.0,    0.0),
        ("dark_oak_sign.png",   0.0,  -15.0,    0.0),
        ("jungle_sign.png",   -10.0,    4.6,    0.0),
        ("crimson_sign.png",  -59.0,  -30.0,    0.0),
        ("warped_sign.png",   130.0,  -33.0,    0.0),
        ("mangrove_sign.png", -59.0,  -10.0,    0.0),
        ("pale_oak_sign.png",   0.0,   30.0, -100.0),
        ("bamboo_sign.png",    25.0,   20.0,    0.0),
        ("cherry_sign.png",   -80.0,   20.0,    0.0),
    ];

    for (filename, hue, bright, sat) in sign_variants.iter() {
        let adjusted = adjust_hue_brightness(base_img.clone(), *hue, *bright, *sat);
        let output_path = item_path.join(filename);
        adjusted
            .save(&output_path)
            .map_err(|e| format!("无法保存图像: {:?}", e))?;
        crate::log_info!("已生成 {}", filename);
    }

    crate::log_info!("已修复告示牌纹理");
    Ok(())
}

/// Register the sign-fix task with the engine.
pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "fix_sign",
        TaskType::Exclusive,
        TaskTier::Surgeon,
        fix_sign,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use crate::hurray::context::HurrayContext;

    #[test]
    fn test_fix_sign() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let resource_pack_path = temp_dir.path().to_str().unwrap();

        let textures_path = temp_dir.path().join("assets/minecraft/textures/item");
        fs::create_dir_all(&textures_path).expect("Failed to create test directory structure");

        // Need an oak_sign.png to trigger the body
        let oak = textures_path.join("oak_sign.png");
        let img = image::RgbaImage::new(16, 16);
        img.save(&oak).expect("Failed to create oak_sign.png");

        let context = HurrayContext::new(resource_pack_path);
        let result = fix_sign(&context);
        assert!(result.is_ok());
    }
}
