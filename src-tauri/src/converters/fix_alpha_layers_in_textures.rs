use std::path::Path;

use image::RgbaImage;
use walkdir::WalkDir;

fn fix_pixel(
    img: &mut RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> bool {
    let mut changed = false;
    let mut pixel = *img.get_pixel(x, y);
    let (r, g, b, a) = (pixel[0], pixel[1], pixel[2], pixel[3]);

    // Rules 1–8 below mirror pack.py fix_alpha_layers_in_textures.
    // Rules 9 and 10 (saturation matching + opaque-edge neighbour fix) were
    // missing from the previous Rust port — see the inline notes.

    // Rule 1: zero alpha with non-zero RGB → wipe RGB
    if a == 0 && (r != 0 || g != 0 || b != 0) {
        pixel[0] = 0;
        pixel[1] = 0;
        pixel[2] = 0;
        changed = true;
    // Rule 2: non-zero low alpha with zero RGB → brighten toward a*0.8
    } else if a > 0 && a < 255 && r == 0 && g == 0 && b == 0 {
        let gray = ((a as f32) * 0.8).round().clamp(0.0, 255.0) as u8;
        pixel[0] = gray;
        pixel[1] = gray;
        pixel[2] = gray;
        changed = true;
    // Rule 3 + 4 (alpha / RGB out-of-range clamps) are no-ops for RgbaImage
    // (u8 channel), so they are skipped here.
    // Rule 5: semi-transparent pixel whose brightness differs from a*0.8 by
    //         more than 50 → rescale RGB so brightness matches alpha.
    } else if a > 0 && a < 255 {
        let brightness = (r as f32 + g as f32 + b as f32) / 3.0;
        let expected = (a as f32) * 0.8;
        if (brightness - expected).abs() > 50.0 {
            let scale = expected / brightness.max(1.0);
            pixel[0] = (r as f32 * scale).round().clamp(0.0, 255.0) as u8;
            pixel[1] = (g as f32 * scale).round().clamp(0.0, 255.0) as u8;
            pixel[2] = (b as f32 * scale).round().clamp(0.0, 255.0) as u8;
            changed = true;
        }
    // Rule 6: opaque pixel whose channel std-dev exceeds 80 → blend 30%
    //         toward the channel average to rebalance.
    } else if a == 255 && (r != 0 || g != 0 || b != 0) {
        let avg = (r as f32 + g as f32 + b as f32) / 3.0;
        let std_dev = (((r as f32 - avg).powi(2)
            + (g as f32 - avg).powi(2)
            + (b as f32 - avg).powi(2))
            / 3.0)
            .sqrt();
        if std_dev > 80.0 {
            let balance = 0.3;
            let new_r = (r as f32 * (1.0 - balance) + avg * balance).round();
            let new_g = (g as f32 * (1.0 - balance) + avg * balance).round();
            let new_b = (b as f32 * (1.0 - balance) + avg * balance).round();
            pixel[0] = new_r.clamp(0.0, 255.0) as u8;
            pixel[1] = new_g.clamp(0.0, 255.0) as u8;
            pixel[2] = new_b.clamp(0.0, 255.0) as u8;
            changed = true;
        }
    // Rule 8: opaque pixel that's almost black but surrounded by bright
    //         opaque neighbours → set to the neighbour average (anti-stamp).
    } else if a == 255 && r < 30 && g < 30 && b < 30 {
        let mut neighbor_brightness = Vec::new();
        for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx >= 0 && ny >= 0 && (nx as u32) < width && (ny as u32) < height {
                let neighbor = img.get_pixel(nx as u32, ny as u32);
                if neighbor[3] == 255 {
                    neighbor_brightness.push((neighbor[0] as f32 + neighbor[1] as f32 + neighbor[2] as f32) / 3.0);
                }
            }
        }
        if !neighbor_brightness.is_empty() {
            let avg = neighbor_brightness.iter().sum::<f32>() / neighbor_brightness.len() as f32;
            if avg > 100.0 {
                let value = avg.round().clamp(0.0, 255.0) as u8;
                pixel[0] = value;
                pixel[1] = value;
                pixel[2] = value;
                changed = true;
            }
        }
    // Rule 7 + 10: opaque pixel adjacent to a semi-transparent neighbour
    //               (py: "ensure a=255 pixels have smooth edges") → adjust
    //               the semi-transparent neighbour's RGB so its brightness
    //               matches its alpha.
    // Rule 9:    semi-transparent pixel whose saturation diverges from
    //               alpha/255 by more than 0.3 → nudge saturation toward
    //               alpha. Runs after Rule 7/10 because both need a>0 && a<255
    //               checks; they are guarded with explicit `else if` branches.
    } else if a == 255 {
        // Rule 7/10: nudge any neighbouring semi-transparent pixel
        for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx >= 0 && ny >= 0 && (nx as u32) < width && (ny as u32) < height {
                let neighbor = img.get_pixel(nx as u32, ny as u32);
                if neighbor[3] > 0 && neighbor[3] < 255 {
                    let brightness = (r as f32 + g as f32 + b as f32) / 3.0;
                    let expected = (neighbor[3] as f32) * 0.8;
                    let scale = expected / brightness.max(1.0);
                    let new_r = (r as f32 * scale).round().clamp(0.0, 255.0) as u8;
                    let new_g = (g as f32 * scale).round().clamp(0.0, 255.0) as u8;
                    let new_b = (b as f32 * scale).round().clamp(0.0, 255.0) as u8;
                    img.put_pixel(nx as u32, ny as u32, image::Rgba([new_r, new_g, new_b, neighbor[3]]));
                    changed = true;
                }
            }
        }
    }

    // Rule 9 is handled in a second pass below because it needs the post-rule-7
    // pixel value (rule 7 may have changed neighbour alpha, not the current
    // pixel). We re-read the current pixel here.
    if !changed {
        let p = img.get_pixel(x, y);
        if p[3] > 0 && p[3] < 255 {
            let r2 = p[0] as f32;
            let g2 = p[1] as f32;
            let b2 = p[2] as f32;
            let max_rgb = r2.max(g2).max(b2);
            let min_rgb = r2.min(g2).min(b2);
            if max_rgb > 0.0 {
                let saturation = (max_rgb - min_rgb) / max_rgb;
                let expected_sat = p[3] as f32 / 255.0;
                if (saturation - expected_sat).abs() > 0.3 {
                    let gray = (r2 + g2 + b2) / 3.0;
                    if saturation > expected_sat {
                        let factor = expected_sat / saturation.max(0.1);
                        let nr = (r2 * factor + gray * (1.0 - factor)).clamp(0.0, 255.0) as u8;
                        let ng = (g2 * factor + gray * (1.0 - factor)).clamp(0.0, 255.0) as u8;
                        let nb = (b2 * factor + gray * (1.0 - factor)).clamp(0.0, 255.0) as u8;
                        img.put_pixel(x, y, image::Rgba([nr, ng, nb, p[3]]));
                        changed = true;
                    } else {
                        let factor = expected_sat / saturation.max(0.1);
                        let nr = ((r2 - gray) * factor + gray).clamp(0.0, 255.0).min(255.0) as u8;
                        let ng = ((g2 - gray) * factor + gray).clamp(0.0, 255.0).min(255.0) as u8;
                        let nb = ((b2 - gray) * factor + gray).clamp(0.0, 255.0).min(255.0) as u8;
                        img.put_pixel(x, y, image::Rgba([nr, ng, nb, p[3]]));
                        changed = true;
                    }
                }
            }
        }
    }

    if changed {
        img.put_pixel(x, y, image::Rgba([pixel[0], pixel[1], pixel[2], pixel[3]]));
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

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "fix_alpha_layers_in_textures",
        crate::hurray::scheduler::TaskType::Exclusive,
        crate::hurray::scheduler::TaskTier::Surgeon,
        |context| fix_alpha_layers_in_textures(context.temp_dir()),
    );
}
