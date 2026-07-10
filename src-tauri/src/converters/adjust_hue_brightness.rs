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


