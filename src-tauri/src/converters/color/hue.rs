use image::{ImageBuffer, Rgba, RgbaImage};

pub fn adjust_hue_brightness(
    mut img: RgbaImage,
    hue_shift: f32,       // 0-360
    brightness_shift: f32, // -100-100
    saturation_shift: f32, // -100-100
) -> RgbaImage {
    let (width, height) = img.dimensions();
    let hue_shift_normalized = hue_shift / 360.0;
    let brightness_factor = brightness_shift / 100.0;
    let saturation_factor = saturation_shift / 100.0;

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            if pixel[3] == 0 { continue; }

            // 1. RGB to HSV
            let (h, s, v) = rgb_to_hsv(pixel[0], pixel[1], pixel[2]);

            // 2. 调整 H, S, V
            let new_h = (h + hue_shift_normalized).rem_euclid(1.0);
            let new_s = (s + saturation_factor).clamp(0.0, 1.0);
            let new_v = (v + brightness_factor).clamp(0.0, 1.0);

            // 3. HSV back to RGB
            let (r, g, b) = hsv_to_rgb(new_h, new_s, new_v);
            img.put_pixel(x, y, Rgba([r, g, b, pixel[3]]));
        }
    }
    img
}

/// 把色相**钉在**目标区间（度），而不是相对偏移。
/// 源图近灰（如橡木叶 S≈2%）时，逐像素色相噪声很大，直接 shift 会漂成蓝/紫。
/// 这里用目标色相 + 按原 V 的微扰，饱和度直接设成 `sat`。
///
/// - `target_hue`：中心色相（度）
/// - `hue_jitter`：允许的左右抖动（度），默认建议 4–8
/// - `sat`：目标饱和度 0–1
/// - `v_min` / `v_max`：输出亮度夹取
pub fn force_hue_saturation(
    mut img: RgbaImage,
    target_hue: f32,
    hue_jitter: f32,
    sat: f32,
    v_min: f32,
    v_max: f32,
) -> RgbaImage {
    let (width, height) = img.dimensions();
    let target = (target_hue / 360.0).rem_euclid(1.0);
    let jitter = (hue_jitter / 360.0).abs();
    let sat = sat.clamp(0.0, 1.0);

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            if pixel[3] == 0 { continue; }
            let (_h, _s, v) = rgb_to_hsv(pixel[0], pixel[1], pixel[2]);
            // 用亮度当伪随机源，稳定可复现，不会引入额外噪声
            let t = (v * 17.0 + (x as f32 * 0.013) + (y as f32 * 0.007)) % 1.0;
            let offset = (t - 0.5) * 2.0 * jitter;
            let new_h = (target + offset).rem_euclid(1.0);
            let new_v = v.clamp(v_min, v_max);
            let (r, g, b) = hsv_to_rgb(new_h, sat, new_v);
            img.put_pixel(x, y, Rgba([r, g, b, pixel[3]]));
        }
    }
    img
}

// 高性能转换辅助函数
fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let mut h = 0.0;
    if delta > 0.0 {
        if max == r { h = ((g - b) / delta).rem_euclid(6.0); }
        else if max == g { h = (b - r) / delta + 2.0; }
        else { h = (r - g) / delta + 4.0; }
        h /= 6.0;
    }
    (h, if max == 0.0 { 0.0 } else { delta / max }, max)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0).rem_euclid(2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h * 6.0) as u32 {
        0 => (c, x, 0.0), 1 => (x, c, 0.0), 2 => (0.0, c, x),
        3 => (0.0, x, c), 4 => (x, 0.0, c), _ => (c, 0.0, x),
    };
    (((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8)
}

/// 注册色相亮度调整任务
///
/// # 参数
/// - `engine`: Hurray 引擎
pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "adjust_hue_brightness", crate::hurray::scheduler::TaskType::Parallel, crate::hurray::scheduler::TaskTier::Surgeon, |_context| {
            // adjust_hue_brightness 是一个工具函数，不需要直接注册为任务
            // 它会被其他模块在内部调用
            Ok(())
        }
    );
}


