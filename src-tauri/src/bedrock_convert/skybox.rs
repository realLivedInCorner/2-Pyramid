//! j2b 天空盒：Java 六面图 → 国际/国服基岩 `environment/sky/cubemap_N.png`
//!
//! 逻辑移植自 skybox.py（亚克力）：
//! - 输入宽高比 3:2（2×3 方格）；否则按最近邻拉到 3:2
//! - 切块顺序 [4,5,2,3,1,0] 对应基岩 cubemap_0..5
//! - `move_y`：国际 -0.206 / 国服 +0.133；用透视贴合补侧/顶/底
//!
//! 依赖仓库已有 `image` crate；透视用 4 点单应 + 双线性采样。

use std::fs;
use std::path::Path;

use image::{Rgba, RgbaImage};

use crate::{log_info, log_warn};

use super::fsutil::remove_dir_quiet;

pub const BEDROCK_INTERNATIONAL_MOVE_Y: f32 = -0.206;
pub const BEDROCK_CHINA_MOVE_Y: f32 = 0.133;

/// j2b 入口：国际基岩。
pub fn convert_java_skybox_for_bedrock(textures_dst: &Path) {
    convert_skybox_with_move_y(textures_dst, BEDROCK_INTERNATIONAL_MOVE_Y, true);
}

pub fn convert_skybox_with_move_y(textures_dst: &Path, move_y: f32, international: bool) {
    let src = find_sky_source(textures_dst);
    let Some(src) = src else { return };
    let Ok(img) = image::open(&src) else {
        log_warn!("bedrock skybox: open failed {}", src.display());
        return;
    };
    let mut img = img.to_rgba8();
    let (w, h) = normalize_to_3x2(img.width(), img.height());
    if (w, h) != img.dimensions() {
        img = image::imageops::resize(&img, w, h, image::imageops::FilterType::CatmullRom);
    }
    let side = h / 2;
    if side < 8 {
        return;
    }

    let mut base: Vec<RgbaImage> = Vec::with_capacity(6);
    for cy in 0..2u32 {
        for cx in 0..3u32 {
            base.push(image::imageops::crop_imm(&img, cx * side, cy * side, side, side).to_image());
        }
    }
    // grid: 0 1 2 / 3 4 5 → cubemap: 4,5,2,3,1,0
    let mut pieces: Vec<RgbaImage> = vec![
        base[4].clone(),
        base[5].clone(),
        base[2].clone(),
        base[3].clone(),
        base[1].clone(),
        base[0].clone(),
    ];

    let size = side;
    let blank = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 255]));
    let mut result: Vec<RgbaImage> = Vec::with_capacity(6);

    if move_y.abs() < f32::EPSILON {
        result = pieces;
    } else {
        let mut top = pieces[4].clone();
        let mut bottom = pieces[5].clone();
        // 四个侧面
        for i in 0..4 {
            result.push(draw_side(size, &top, &pieces[i], &bottom, size as f32 * move_y, &blank));
            top = rotate_90_cw(&top);
            bottom = rotate_90_ccw(&bottom);
        }
        // 顶面 / 底面
        if move_y > 0.0 {
            result.push(draw(
                size, None, Some(&top), None, None, None,
                0.0, 1.0 / (1.0 + move_y * 2.0) - 1.0, 1.0, &blank,
            ));
            result.push(draw(
                size,
                Some(&crop_y(&pieces[0], ((size as f32 - 1.0) * (1.0 - move_y)) as u32, size - 1, 0, size - 1)),
                Some(&bottom),
                Some(&crop_y(&rotate_180(&pieces[2]), 0, ((size as f32 - 1.0) * move_y) as u32, 0, size - 1)),
                Some(&crop_y(&rotate_90_ccw(&pieces[3]), 0, size - 1, ((size as f32 - 1.0) * (1.0 - move_y)) as u32, size - 1)),
                Some(&crop_y(&rotate_90_cw(&pieces[1]), 0, size - 1, 0, ((size as f32 - 1.0) * move_y) as u32)),
                0.0,
                1.0 / (1.0 + move_y * -2.0) - 1.0,
                1.0,
                &blank,
            ));
        } else {
            result.push(draw(
                size,
                Some(&crop_y(&rotate_180(&pieces[2]), ((size as f32 - 1.0) * (1.0 + move_y)) as u32, size - 1, 0, size - 1)),
                Some(&top),
                Some(&crop_y(&pieces[0], 0, ((size as f32 - 1.0) * (0.0 - move_y)) as u32, 0, size - 1)),
                Some(&crop_y(&rotate_90_cw(&pieces[3]), 0, size - 1, ((size as f32 - 1.0) * (1.0 + move_y)) as u32, size - 1)),
                Some(&crop_y(&rotate_90_ccw(&pieces[1]), 0, size - 1, 0, ((size as f32 - 1.0) * (0.0 - move_y)) as u32)),
                0.0,
                1.0 / (1.0 + move_y * 2.0) - 1.0,
                1.0,
                &blank,
            ));
            result.push(draw(
                size, None, Some(&bottom), None, None, None,
                0.0, 1.0 / (1.0 + move_y * -2.0) - 1.0, 1.0, &blank,
            ));
        }
        let _ = &mut pieces;
    }

    let out_dir = textures_dst.join("environment").join("sky");
    let _ = fs::create_dir_all(&out_dir);
    remove_dir_quiet(&textures_dst.join("environment").join("cubemaps"));
    let mut n = 0usize;
    for (i, face) in result.iter().enumerate().take(6) {
        let path = out_dir.join(format!("cubemap_{}.png", i));
        if face.save(&path).is_ok() {
            n += 1;
        }
    }
    let label = if international { "international" } else { "china" };
    log_info!(
        "OKAY bedrock [skybox {} move_y={} cubemap × {}]",
        label, move_y, n
    );
}

fn find_sky_source(textures_dst: &Path) -> Option<std::path::PathBuf> {
    for p in [
        textures_dst.join("environment").join("sky.png"),
        textures_dst.join("environment").join("sky.jpg"),
        textures_dst.join("gui").join("title").join("background").join("panorama_0.png"),
    ] {
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn normalize_to_3x2(w: u32, h: u32) -> (u32, u32) {
    if w == 0 || h == 0 {
        return (768, 512);
    }
    if w * 2 == h * 3 {
        return (w, h);
    }
    let h2 = h.max(256);
    let w2 = (h2 * 3) / 2;
    (w2, h2)
}

/// 2D 点
type Pt = (f32, f32);

/// 计算 dst→src 的 8 参数透视（用于反向采样）。
fn perspective_coeffs(src: [Pt; 4], dst: [Pt; 4]) -> [f32; 8] {
    // 解 8 元线性方程：标准 4 点 homography
    let mut a = [[0f64; 9]; 8];
    for i in 0..4 {
        let (sx, sy) = src[i];
        let (dx, dy) = dst[i];
        let (sx, sy, dx, dy) = (sx as f64, sy as f64, dx as f64, dy as f64);
        let row0 = i * 2;
        let row1 = i * 2 + 1;
        a[row0] = [sx, sy, 1.0, 0.0, 0.0, 0.0, -dx * sx, -dx * sy, dx];
        a[row1] = [0.0, 0.0, 0.0, sx, sy, 1.0, -dy * sx, -dy * sy, dy];
    }
    // 高斯消元（9 列，前 8 未知，最后一列常数）
    for i in 0..8 {
        let mut pivot = i;
        for r in i..8 {
            if a[r][i].abs() > a[pivot][i].abs() {
                pivot = r;
            }
        }
        if pivot != i {
            a.swap(pivot, i);
        }
        let d = a[i][i];
        if d.abs() < 1e-12 {
            return [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
        }
        for c in i..9 {
            a[i][c] /= d;
        }
        for r in 0..8 {
            if r == i {
                continue;
            }
            let f = a[r][i];
            if f.abs() < 1e-12 {
                continue;
            }
            for c in i..9 {
                a[r][c] -= f * a[i][c];
            }
        }
    }
    [
        a[0][8] as f32, a[1][8] as f32, a[2][8] as f32,
        a[3][8] as f32, a[4][8] as f32, a[5][8] as f32,
        a[6][8] as f32, a[7][8] as f32,
    ]
}

fn sample_bilinear(img: &RgbaImage, x: f32, y: f32) -> Rgba<u8> {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return Rgba([0, 0, 0, 0]);
    }
    let x = x.clamp(0.0, (w as f32 - 1.0).max(0.0));
    let y = y.clamp(0.0, (h as f32 - 1.0).max(0.0));
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(w.saturating_sub(1));
    let y1 = (y0 + 1).min(h.saturating_sub(1));
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let p00 = img.get_pixel(x0, y0).0;
    let p10 = img.get_pixel(x1, y0).0;
    let p01 = img.get_pixel(x0, y1).0;
    let p11 = img.get_pixel(x1, y1).0;
    let mut out = [0u8; 4];
    for c in 0..4 {
        let a = p00[c] as f32 * (1.0 - fx) + p10[c] as f32 * fx;
        let b = p01[c] as f32 * (1.0 - fx) + p11[c] as f32 * fx;
        out[c] = (a * (1.0 - fy) + b * fy).round().clamp(0.0, 255.0) as u8;
    }
    Rgba(out)
}

/// 将 overlay 透视贴到 base 的四点上（目标点顺序：左上、右上、左下、右下）。
fn paste_perspective(
    base: &RgbaImage,
    overlay: &RgbaImage,
    dst: [Pt; 4],
) -> RgbaImage {
    let (bw, bh) = base.dimensions();
    let (ow, oh) = overlay.dimensions();
    if ow == 0 || oh == 0 {
        return base.clone();
    }
    // 源四角（与 python w-1/h-1 对齐）
    let src = [
        (0.0, 0.0),
        ((ow - 1) as f32, 0.0),
        (0.0, (oh - 1) as f32),
        ((ow - 1) as f32, (oh - 1) as f32),
    ];
    // 反向映射：dst 像素 → src 采样点
    let coeffs = perspective_coeffs(dst, src);
    let (a, b, c, d, e, f, g, h) = (
        coeffs[0], coeffs[1], coeffs[2], coeffs[3],
        coeffs[4], coeffs[5], coeffs[6], coeffs[7],
    );
    let mut out = base.clone();
    // 仅在目标包围盒内采样
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for p in dst {
        min_x = min_x.min(p.0);
        min_y = min_y.min(p.1);
        max_x = max_x.max(p.0);
        max_y = max_y.max(p.1);
    }
    let x0 = min_x.floor().max(0.0) as u32;
    let y0 = min_y.floor().max(0.0) as u32;
    let x1 = (max_x.ceil() as u32).min(bw.saturating_sub(1));
    let y1 = (max_y.ceil() as u32).min(bh.saturating_sub(1));

    for y in y0..=y1 {
        for x in x0..=x1 {
            let dx = x as f32;
            let dy = y as f32;
            let den = g * dx + h * dy + 1.0;
            if den.abs() < 1e-6 {
                continue;
            }
            let sx = (a * dx + b * dy + c) / den;
            let sy = (d * dx + e * dy + f) / den;
            // 源外不写
            if sx < -0.5 || sy < -0.5 || sx > ow as f32 - 0.5 || sy > oh as f32 - 0.5 {
                continue;
            }
            let p = sample_bilinear(overlay, sx, sy);
            // alpha 混合
            let sa = p.0[3] as f32 / 255.0;
            if sa <= 0.0 {
                continue;
            }
            let dst_pix = out.get_pixel(x, y).0;
            let mut mixed = [0u8; 4];
            for c in 0..3 {
                mixed[c] = (p.0[c] as f32 * sa + dst_pix[c] as f32 * (1.0 - sa))
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
            mixed[3] = dst_pix[3];
            out.put_pixel(x, y, Rgba(mixed));
        }
    }
    out
}

fn to_2d(size: f32, px: f32, py: f32, pz: f32, mz: f32) -> Pt {
    let hs = size / 2.0;
    (((px - hs) * mz / pz) + hs, ((py - hs) * mz / pz) + hs)
}

/// 对应 skybox.py draw()：侧面 + 可选顶/底/左右
#[allow(clippy::too_many_arguments)]
fn draw(
    size: u32,
    image_t: Option<&RgbaImage>,
    image_m: Option<&RgbaImage>,
    image_b: Option<&RgbaImage>,
    image_l: Option<&RgbaImage>,
    image_r: Option<&RgbaImage>,
    my: f32,
    moz: f32,
    mz: f32,
    blank: &RgbaImage,
) -> RgbaImage {
    let s = size as f32;
    let mut temp = blank.clone();
    if let Some(m) = image_m {
        temp = paste_perspective(
            &temp,
            m,
            [
                to_2d(s, 0.0, my, moz + 1.0, mz),
                to_2d(s, s - 1.0, my, moz + 1.0, mz),
                to_2d(s, 0.0, s - 1.0 + my, moz + 1.0, mz),
                to_2d(s, s - 1.0, s - 1.0 + my, moz + 1.0, mz),
            ],
        );
    }
    if moz != 0.0 || mz != 1.0 || my != 0.0 {
        if let Some(t) = image_t {
            temp = paste_perspective(
                &temp,
                t,
                [
                    (0.0, 0.0),
                    (s, 0.0),
                    to_2d(s, 0.0, my, moz + 1.0, mz),
                    to_2d(s, s, my, moz + 1.0, mz),
                ],
            );
        }
        if let Some(b) = image_b {
            temp = paste_perspective(
                &temp,
                b,
                [
                    to_2d(s, 0.0, s - 1.0 + my, moz + 1.0, mz),
                    to_2d(s, s, s - 1.0 + my, moz + 1.0, mz),
                    (0.0, s - 1.0),
                    (s, s - 1.0),
                ],
            );
        }
        if let Some(l) = image_l {
            temp = paste_perspective(
                &temp,
                l,
                [
                    (0.0, 0.0),
                    to_2d(s, 0.0, my, moz + 1.0, mz),
                    (0.0, s - 1.0),
                    to_2d(s, 0.0, s + my, moz + 1.0, mz),
                ],
            );
        }
        if let Some(r) = image_r {
            temp = paste_perspective(
                &temp,
                r,
                [
                    to_2d(s, s - 1.0, my, moz + 1.0, mz),
                    (s, 0.0),
                    to_2d(s, s - 1.0, s + my, moz + 1.0, mz),
                    (s, s - 1.0),
                ],
            );
        }
    }
    temp
}

fn draw_side(
    size: u32,
    image_t: &RgbaImage,
    image_m: &RgbaImage,
    image_b: &RgbaImage,
    my: f32,
    blank: &RgbaImage,
) -> RgbaImage {
    let s = size as f32;
    let mut temp = paste_perspective(
        blank,
        image_m,
        [
            (0.0, my),
            (s - 1.0, my),
            (0.0, s - 1.0 + my),
            (s - 1.0, s - 1.0 + my),
        ],
    );
    if my > 0.0 {
        let z = (s * my / (s + 2.0 * my)).floor() as i32;
        let m = (s * z as f32) / (s - (2.0 * z as f32));
        let oh = image_t.height();
        let y0 = (oh as i32 - 1 - z).max(0) as u32;
        let strip = image::imageops::crop_imm(image_t, 0, y0, image_t.width(), z.max(1) as u32).to_image();
        temp = paste_perspective(
            &temp,
            &strip,
            [
                (-m, 0.0),
                (s - 1.0 + m, 0.0),
                (0.0, my),
                (s - 1.0, my),
            ],
        );
    } else if my < 0.0 {
        let z = (s * (-my) / (s + 2.0 * (-my))).floor() as i32;
        let m = (s * z as f32) / (s - (2.0 * z as f32));
        let strip = image::imageops::crop_imm(image_b, 0, 0, image_b.width(), z.max(1) as u32).to_image();
        temp = paste_perspective(
            &temp,
            &strip,
            [
                (0.0, s - 1.0 + my),
                (s - 1.0, s - 1.0 + my),
                (-m, s - 1.0),
                (s - 1.0 + m, s - 1.0),
            ],
        );
    }
    temp
}

// ── 图像旋转 / 裁剪（对应 cv2.rotate / numpy slice） ──

fn rotate_90_cw(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mut out = RgbaImage::new(h, w);
    for y in 0..h {
        for x in 0..w {
            out.put_pixel(h - 1 - y, x, *img.get_pixel(x, y));
        }
    }
    out
}

fn rotate_90_ccw(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mut out = RgbaImage::new(h, w);
    for y in 0..h {
        for x in 0..w {
            out.put_pixel(y, w - 1 - x, *img.get_pixel(x, y));
        }
    }
    out
}

fn rotate_180(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mut out = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            out.put_pixel(w - 1 - x, h - 1 - y, *img.get_pixel(x, y));
        }
    }
    out
}

/// y0..y1, x0..x1 半开区间裁剪（与 numpy 切片一致）
fn crop_y(img: &RgbaImage, y0: u32, y1: u32, x0: u32, x1: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let y0 = y0.min(h);
    let y1 = y1.min(h).max(y0);
    let x0 = x0.min(w);
    let x1 = x1.min(w).max(x0);
    let hh = y1.saturating_sub(y0);
    let ww = x1.saturating_sub(x0);
    if hh == 0 || ww == 0 {
        return RgbaImage::new(1, 1);
    }
    image::imageops::crop_imm(img, x0, y0, ww, hh).to_image()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cubemap_output_count_and_size() {
        let temp = tempdir().unwrap();
        let tex = temp.path().join("textures");
        let env = tex.join("environment");
        fs::create_dir_all(&env).unwrap();
        RgbaImage::from_pixel(960, 640, Rgba([10, 20, 30, 255]))
            .save(env.join("sky.png"))
            .unwrap();

        convert_java_skybox_for_bedrock(&tex);

        for i in 0..6 {
            assert!(env.join("sky").join(format!("cubemap_{}.png", i)).exists());
        }
        let c0 = image::open(env.join("sky/cubemap_0.png")).unwrap();
        assert_eq!(c0.width(), 320);
        assert_eq!(c0.height(), 320);
    }

    #[test]
    fn test_perspective_paste_covers_target() {
        let base = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 255]));
        let ov = RgbaImage::from_pixel(32, 32, Rgba([255, 0, 0, 255]));
        let out = paste_perspective(
            &base,
            &ov,
            [(10.0, 10.0), (50.0, 8.0), (12.0, 50.0), (48.0, 48.0)],
        );
        let p = out.get_pixel(30, 30).0;
        assert!(p[0] > 200, "中心应被覆盖为红色，实际 {:?}", p);
    }

    #[test]
    fn test_perspective_axis_aligned_identity() {
        let base = RgbaImage::from_pixel(32, 32, Rgba([0, 0, 0, 255]));
        let ov = RgbaImage::from_pixel(16, 16, Rgba([0, 255, 0, 255]));
        let out = paste_perspective(
            &base,
            &ov,
            [(0.0, 0.0), (15.0, 0.0), (0.0, 15.0), (15.0, 15.0)],
        );
        let p = out.get_pixel(8, 8).0;
        assert!(p[1] > 200, "应贴上绿色，实际 {:?}", p);
        let q = out.get_pixel(24, 24).0;
        assert!(q[1] < 40, "范围外应保持黑底，实际 {:?}", q);
    }

    #[test]
    fn test_china_and_noop() {
        let temp = tempdir().unwrap();
        let tex = temp.path().join("textures");
        fs::create_dir_all(&tex).unwrap();
        convert_skybox_with_move_y(&tex, BEDROCK_CHINA_MOVE_Y, false);
        assert!(!tex.join("environment/sky").exists());
    }
}
