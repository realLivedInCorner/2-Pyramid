/// 将 RGBA 像素转换为 HSV (色相, 饱和度, 明度, Alpha)
pub fn rgba_to_hsva(r: u8, g: u8, b: u8, a: u8) -> (f32, f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let mut h = if delta == 0.0 { 0.0 }
    else if max == r { ((g - b) / delta) % 6.0 }
    else if max == g { ((b - r) / delta) + 2.0 }
    else { ((r - g) / delta) + 4.0 };

    h = (h * 60.0).rem_euclid(360.0) / 360.0;
    let s = if max == 0.0 { 0.0 } else { delta / max };
    let v = max;

    (h, s, v, a as f32 / 255.0)
}

/// �?HSVA 转回 RGBA
pub fn hsva_to_rgba(h: f32, s: f32, v: f32, a: f32) -> [u8; 4] {
    let h = h * 360.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 { (c, x, 0.0) }
    else if h < 120.0 { (x, c, 0.0) }
    else if h < 180.0 { (0.0, c, x) }
    else if h < 240.0 { (0.0, x, c) }
    else if h < 300.0 { (x, 0.0, c) }
    else { (c, 0.0, x) };

    [
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
        (a * 255.0) as u8,
    ]
}


