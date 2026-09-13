use std::path::Path;

use image::RgbaImage;
use walkdir::WalkDir;

/// 单像素 alpha/RGB 修复。规则对齐 Python `fix_alpha_layers_in_textures`。
/// 返回是否修改了当前像素或其邻居。
fn fix_pixel(
    img: &mut RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> bool {
    let mut changed = false;
    let p = *img.get_pixel(x, y);
    let (r, g, b, a) = (p[0], p[1], p[2], p[3]);
    let mut out = [r, g, b, a];

    // Rule 1: zero alpha with non-zero RGB → wipe RGB
    if a == 0 && (r != 0 || g != 0 || b != 0) {
        out[0] = 0;
        out[1] = 0;
        out[2] = 0;
        changed = true;
    // Rule 2: non-zero low alpha with zero RGB → brighten toward a*0.8
    } else if a > 0 && a < 255 && r == 0 && g == 0 && b == 0 {
        let gray = ((a as f32) * 0.8).round().clamp(0.0, 255.0) as u8;
        out[0] = gray;
        out[1] = gray;
        out[2] = gray;
        changed = true;
    // Rule 3+4: alpha/RGB out-of-range — u8 通道天然有界，跳过
    // Rule 5: semi-transparent, brightness far from a*0.8 → rescale RGB
    } else if a > 0 && a < 255 {
        let brightness = (r as f32 + g as f32 + b as f32) / 3.0;
        let expected = (a as f32) * 0.8;
        if (brightness - expected).abs() > 50.0 {
            let scale = expected / brightness.max(1.0);
            out[0] = (r as f32 * scale).round().clamp(0.0, 255.0) as u8;
            out[1] = (g as f32 * scale).round().clamp(0.0, 255.0) as u8;
            out[2] = (b as f32 * scale).round().clamp(0.0, 255.0) as u8;
            changed = true;
        } else {
            // Rule 9：仅在 Rule 5 未触发时校正饱和度
            let r2 = r as f32;
            let g2 = g as f32;
            let b2 = b as f32;
            let maxc = r2.max(g2).max(b2);
            let minc = r2.min(g2).min(b2);
            if maxc > 0.0 {
                let saturation = (maxc - minc) / maxc;
                let expected_sat = (a as f32 / 255.0).clamp(0.0, 0.95);
                if (saturation - expected_sat).abs() > 0.3 {
                    let new_min = maxc * (1.0 - expected_sat);
                    let range = (maxc - minc).max(1.0);
                    let scale = (maxc - new_min) / range;
                    let map = |c: f32| (new_min + (c - minc) * scale).clamp(0.0, 255.0) as u8;
                    out[0] = map(r2);
                    out[1] = map(g2);
                    out[2] = map(b2);
                    changed = true;
                }
            }
        }
    // Rule 6: opaque, channel std-dev high → blend 30% toward gray
    } else if a == 255 && (r != 0 || g != 0 || b != 0) {
        let avg = (r as f32 + g as f32 + b as f32) / 3.0;
        let std_dev = (((r as f32 - avg).powi(2)
            + (g as f32 - avg).powi(2)
            + (b as f32 - avg).powi(2))
            / 3.0)
            .sqrt();
        if std_dev > 80.0 {
            let balance = 0.3;
            out[0] = (r as f32 * (1.0 - balance) + avg * balance).round().clamp(0.0, 255.0) as u8;
            out[1] = (g as f32 * (1.0 - balance) + avg * balance).round().clamp(0.0, 255.0) as u8;
            out[2] = (b as f32 * (1.0 - balance) + avg * balance).round().clamp(0.0, 255.0) as u8;
            changed = true;
        }
    // Rule 8: opaque almost-black surrounded by bright opaque → lift
    } else if a == 255 && r < 30 && g < 30 && b < 30 {
        let mut neighbor_brightness = Vec::new();
        for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx >= 0 && ny >= 0 && (nx as u32) < width && (ny as u32) < height {
                let n = img.get_pixel(nx as u32, ny as u32);
                if n[3] == 255 {
                    neighbor_brightness.push((n[0] as f32 + n[1] as f32 + n[2] as f32) / 3.0);
                }
            }
        }
        if !neighbor_brightness.is_empty() {
            let avg = neighbor_brightness.iter().sum::<f32>() / neighbor_brightness.len() as f32;
            if avg > 100.0 {
                let value = avg.round().clamp(0.0, 255.0) as u8;
                out[0] = value;
                out[1] = value;
                out[2] = value;
                changed = true;
            }
        }
    }

    // Rule 7/10：不透明像素旁的半透明邻居 — 独立于 Rule 6/8，
    // 否则非黑不透明像素会先被 Rule 6 的 else-if 吃掉。
    if a == 255 {
        for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || (nx as u32) >= width || (ny as u32) >= height {
                continue;
            }
            let n = *img.get_pixel(nx as u32, ny as u32);
            if n[3] == 0 || n[3] == 255 {
                continue;
            }
            let nb = (n[0] as f32 + n[1] as f32 + n[2] as f32) / 3.0;
            let expected = (n[3] as f32) * 0.8;
            if (nb - expected).abs() <= 50.0 {
                continue;
            }
            let scale = expected / nb.max(1.0);
            let nr = (n[0] as f32 * scale).round().clamp(0.0, 255.0) as u8;
            let ng = (n[1] as f32 * scale).round().clamp(0.0, 255.0) as u8;
            let nbb = (n[2] as f32 * scale).round().clamp(0.0, 255.0) as u8;
            img.put_pixel(nx as u32, ny as u32, image::Rgba([nr, ng, nbb, n[3]]));
            changed = true;
        }
    }

    if out[0] != r || out[1] != g || out[2] != b || out[3] != a {
        img.put_pixel(x, y, image::Rgba(out));
    }

    changed
}

pub fn fix_alpha_layers_in_textures(resource_pack_path: &Path) -> Result<(), String> {
    let mut search_dirs = Vec::new();
    for name in ["items", "item", "blocks", "block", "entity", "gui", "misc"] {
        let dir = resource_pack_path.join("assets/minecraft/textures").join(name);
        if dir.exists() {
            search_dirs.push(dir);
        }
    }

    let mut total_count = 0usize;
    let mut fixed_count = 0usize;

    for search_dir in search_dirs {
        for entry in WalkDir::new(&search_dir).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("png")) != Some(true) {
                continue;
            }

            total_count += 1;
            let mut img = match image::open(path) {
                Ok(img) => img.to_rgba8(),
                Err(_) => continue,
            };

            let (width, height) = img.dimensions();
            let mut changed = false;
            for x in 0..width {
                for y in 0..height {
                    if fix_pixel(&mut img, x, y, width, height) {
                        changed = true;
                    }
                }
            }

            if changed {
                if img.save(path).is_ok() {
                    fixed_count += 1;
                }
            }
        }
    }

    crate::log_info!("alpha layer fix done, scanned={}, fixed={}", total_count, fixed_count);
    Ok(())
}

/// 注册到 Scheduler；由 invoke_conversion 在用户勾选「图层修复」时调用。
pub fn register_scheduler_task(scheduler: &mut crate::hurray::scheduler::Scheduler) {
    scheduler.register_task(
        "fix_alpha_layers_in_textures",
        crate::hurray::scheduler::TaskType::Exclusive,
        crate::hurray::scheduler::TaskTier::Surgeon,
        |context| fix_alpha_layers_in_textures(context.temp_dir()),
    );
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "fix_alpha_layers_in_textures",
        crate::hurray::scheduler::TaskType::Exclusive,
        crate::hurray::scheduler::TaskTier::Surgeon,
        |context| fix_alpha_layers_in_textures(context.temp_dir()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn img2(pixels: [[u8; 4]; 4]) -> RgbaImage {
        let mut img = RgbaImage::new(2, 2);
        for (i, p) in pixels.iter().enumerate() {
            img.put_pixel((i % 2) as u32, (i / 2) as u32, Rgba(*p));
        }
        img
    }

    #[test]
    fn rule1_wipes_rgb_on_zero_alpha() {
        let mut img = img2([[10, 20, 30, 0], [0, 0, 0, 255], [0, 0, 0, 255], [0, 0, 0, 255]]);
        assert!(fix_pixel(&mut img, 0, 0, 2, 2));
        assert_eq!(*img.get_pixel(0, 0), Rgba([0, 0, 0, 0]));
    }

    #[test]
    fn rule5_rescales_semi_transparent_brightness() {
        // a=100 → expected 80；当前 brightness=200，差 120 > 50
        let mut img = img2([[200, 200, 200, 100], [0, 0, 0, 255], [0, 0, 0, 255], [0, 0, 0, 255]]);
        assert!(fix_pixel(&mut img, 0, 0, 2, 2));
        let p = img.get_pixel(0, 0);
        let br = (p[0] as f32 + p[1] as f32 + p[2] as f32) / 3.0;
        assert!((br - 80.0).abs() < 1.5, "brightness {} should approach 80", br);
        assert_eq!(p[3], 100);
    }

    #[test]
    fn rule7_fixes_neighbor_rgb_not_current() {
        // 当前 (0,0) 不透明亮色；邻居 (1,0) 半透明且过暗
        let mut img = img2([[220, 220, 220, 255], [10, 10, 10, 128], [0, 0, 0, 255], [0, 0, 0, 255]]);
        assert!(fix_pixel(&mut img, 0, 0, 2, 2));
        let n = img.get_pixel(1, 0);
        let br = (n[0] as f32 + n[1] as f32 + n[2] as f32) / 3.0;
        let expected = 128.0 * 0.8;
        assert!((br - expected).abs() < 1.5, "neighbor brightness {} should ~{}", br, expected);
        // 当前像素不应被改写成邻居的亮度
        assert_eq!(*img.get_pixel(0, 0), Rgba([220, 220, 220, 255]));
    }

    #[test]
    fn rule9_desaturates_oversaturated_semi_transparent() {
        // a=128, expected_sat≈0.5；当前饱和度 1.0
        let mut img = img2([[200, 0, 0, 128], [0, 0, 0, 255], [0, 0, 0, 255], [0, 0, 0, 255]]);
        assert!(fix_pixel(&mut img, 0, 0, 2, 2));
        let p = img.get_pixel(0, 0);
        let maxc = p[0].max(p[1]).max(p[2]) as f32;
        let minc = p[0].min(p[1]).min(p[2]) as f32;
        let sat = if maxc > 0.0 { (maxc - minc) / maxc } else { 0.0 };
        assert!((sat - 0.5).abs() < 0.15, "sat {} should ~0.5", sat);
        assert_eq!(p[3], 128);
    }
}
