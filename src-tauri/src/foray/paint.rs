//! 轻量像素编辑：涂抹 / 吸管 / HSV / 撤销栈。

use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

pub const MAX_UNDO: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HsvAdjust {
    /// -180..180
    pub dh: f32,
    /// -100..100
    pub ds: f32,
    /// -100..100
    pub dv: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrushStamp {
    pub x: i32,
    pub y: i32,
    pub radius: i32,
    pub opacity: f32,
    pub color: [u8; 4],
}

pub struct PaintSession {
    pub width: u32,
    pub height: u32,
    img: RgbaImage,
    undo: Vec<RgbaImage>,
}

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let mut h = 0.0;
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
    (h, s, max)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let h = h.rem_euclid(1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (
        ((r + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

impl PaintSession {
    pub fn open_png(data: &[u8]) -> Result<Self, String> {
        let img = image::load_from_memory(data)
            .map_err(|e| format!("png decode: {e}"))?
            .to_rgba8();
        let (width, height) = img.dimensions();
        if width > 8192 || height > 8192 {
            return Err("png too large to edit".into());
        }
        Ok(Self {
            width,
            height,
            img,
            undo: Vec::new(),
        })
    }

    pub fn to_png(&self) -> Result<Vec<u8>, String> {
        let mut buf = std::io::Cursor::new(Vec::new());
        self.img
            .write_to(&mut buf, image::ImageFormat::Png)
            .map_err(|e| format!("png encode: {e}"))?;
        Ok(buf.into_inner())
    }

    fn push_undo(&mut self) {
        if self.undo.len() >= MAX_UNDO {
            self.undo.remove(0);
        }
        self.undo.push(self.img.clone());
    }

    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo.pop() {
            self.img = prev;
            true
        } else {
            false
        }
    }

    pub fn sample(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let p = self.img.get_pixel(x, y);
        Some(p.0)
    }

    /// 吸管：直接写像素（调用方通常用 sample 后再 brush）。
    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        if x < self.width && y < self.height {
            self.push_undo();
            self.img.put_pixel(x, y, Rgba(rgba));
        }
    }

    pub fn apply_hsv(&mut self, adj: &HsvAdjust) {
        self.push_undo();
        let dh = adj.dh / 360.0;
        let ds = adj.ds / 100.0;
        let dv = adj.dv / 100.0;
        for y in 0..self.height {
            for x in 0..self.width {
                let p = *self.img.get_pixel(x, y);
                if p[3] == 0 {
                    continue;
                }
                let (h, s, v) = rgb_to_hsv(p[0], p[1], p[2]);
                let nh = (h + dh).rem_euclid(1.0);
                let ns = (s + ds).clamp(0.0, 1.0);
                let nv = (v + dv).clamp(0.0, 1.0);
                let (r, g, b) = hsv_to_rgb(nh, ns, nv);
                self.img.put_pixel(x, y, Rgba([r, g, b, p[3]]));
            }
        }
    }

    /// 圆形软笔刷涂抹；一次调用一次 undo 快照。
    pub fn brush(&mut self, stamp: &BrushStamp) {
        self.push_undo();
        let r = stamp.radius.max(0);
        let opacity = stamp.opacity.clamp(0.0, 1.0);
        let color = stamp.color;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r * r {
                    continue;
                }
                let x = stamp.x + dx;
                let y = stamp.y + dy;
                if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
                    continue;
                }
                let (x, y) = (x as u32, y as u32);
                let base = *self.img.get_pixel(x, y);
                let mut out = [0u8; 4];
                for i in 0..3 {
                    out[i] = (base[i] as f32 * (1.0 - opacity) + color[i] as f32 * opacity)
                        .round() as u8;
                }
                out[3] = (base[3] as f32 * (1.0 - opacity) + color[3] as f32 * opacity).round()
                    as u8;
                self.img.put_pixel(x, y, Rgba(out));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_png() -> Vec<u8> {
        let img = RgbaImage::from_pixel(4, 4, Rgba([10, 20, 30, 255]));
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        buf.into_inner()
    }

    #[test]
    fn brush_changes_pixel_and_undo() {
        let mut s = PaintSession::open_png(&tiny_png()).unwrap();
        s.brush(&BrushStamp {
            x: 1,
            y: 1,
            radius: 1,
            opacity: 1.0,
            color: [255, 0, 0, 255],
        });
        let p = s.sample(1, 1).unwrap();
        assert_eq!(p[0], 255);
        assert!(s.undo());
        let p2 = s.sample(1, 1).unwrap();
        assert_eq!(p2[0], 10);
    }

    #[test]
    fn hsv_shift_changes_channel() {
        let mut s = PaintSession::open_png(&tiny_png()).unwrap();
        s.apply_hsv(&HsvAdjust {
            dh: 0.0,
            ds: 50.0,
            dv: 0.0,
        });
        let p = s.sample(0, 0).unwrap();
        assert!(p != [10, 20, 30, 255]);
    }

    #[test]
    fn encode_roundtrip() {
        let s = PaintSession::open_png(&tiny_png()).unwrap();
        let out = s.to_png().unwrap();
        assert!(out.starts_with(&[0x89, b'P', b'N', b'G']));
    }
}
