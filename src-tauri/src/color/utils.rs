use image::{Rgba, RgbaImage};

pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let mut h = 0.0f32;
    if delta > 0.0 {
        if (max - r).abs() < f32::EPSILON {
            h = (g - b) / delta;
        } else if (max - g).abs() < f32::EPSILON {
            h = (b - r) / delta + 2.0;
        } else {
            h = (r - g) / delta + 4.0;
        }
        h /= 6.0;
        if h < 0.0 {
            h += 1.0;
        }
    }

    let s = if max <= 0.0 { 0.0 } else { delta / max };
    let v = max;

    (h, s, v)
}

pub fn hsv_to_rgba(h: f32, s: f32, v: f32, a: u8) -> Rgba<u8> {
    let h = h.rem_euclid(1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    let r = ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;

    Rgba([r, g, b, a])
}

pub fn apply_netherite_transform(img: &mut RgbaImage) {
    for pixel in img.pixels_mut() {
        if pixel[3] == 0 {
            continue;
        }
        let (h, s, v) = rgb_to_hsv(pixel[0], pixel[1], pixel[2]);
        let new_h = 310.0 / 360.0;
        let new_s = (s / 3.0).clamp(0.0, 1.0);
        let new_v = (v / 3.0).clamp(0.0, 1.0);
        let rgba = hsv_to_rgba(new_h, new_s, new_v, pixel[3]);
        *pixel = rgba;
    }
}

pub fn apply_spectral_arrow_transform(img: &mut RgbaImage) {
    for pixel in img.pixels_mut() {
        if pixel[3] == 0 {
            continue;
        }
        let (h, s, v) = rgb_to_hsv(pixel[0], pixel[1], pixel[2]);
        let new_h = 60.0 / 360.0;
        let mut new_s = s;
        if new_s <= 0.0 {
            new_s = (new_s + 0.6).min(1.0);
        }
        let rgba = hsv_to_rgba(new_h, new_s, v, pixel[3]);
        *pixel = rgba;
    }
}

pub fn adjust_copper_color(img: &mut RgbaImage) {
    for pixel in img.pixels_mut() {
        let a = pixel[3];
        if a == 0 {
            continue;
        }
        let r = pixel[0] as f32;
        let g = pixel[1] as f32;
        let b = pixel[2] as f32;

        let new_r = (r * 1.15 + 25.0).round().clamp(0.0, 255.0) as u8;
        let new_g = (g * 0.8 - 10.0).round().clamp(0.0, 255.0) as u8;
        let new_b = (b * 0.5 - 15.0).round().clamp(0.0, 255.0) as u8;

        *pixel = Rgba([new_r, new_g, new_b, a]);
    }
}
