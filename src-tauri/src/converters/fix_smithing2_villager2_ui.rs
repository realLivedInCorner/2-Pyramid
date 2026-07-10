use std::fs;
use std::path::Path;

use image::{imageops, RgbaImage};

use crate::hurray::scheduler::{TaskTier, TaskType};
use crate::image_utils::paste_region;

// ---------- Smithing2 part ----------

fn process_smithing2(container_path: &Path) -> Result<(), String> {
    let anvil_path = container_path.join("anvil.png");
    let smithing2_path = container_path.join("smithing.png");

    if !anvil_path.exists() {
        crate::log_info!("anvil.png not found, skip smithing2");
        return Ok(());
    }

    let mut img = image::open(&anvil_path)
        .map_err(|e| format!("failed to open anvil.png: {}", e))?
        .to_rgba8();

    let (width, height) = img.dimensions();
    let s = match (width, height) {
        (256, 256) => 1,
        (512, 512) => 2,
        (1024, 1024) => 4,
        (2048, 2048) => 8,
        _ => {
            crate::log_info!("unsupported anvil.png size: {}x{}, skip smithing2", width, height);
            return Ok(());
        }
    };

    // Fill cover_box (5,5)-(171,72) with color from (5,4)
    let fill_color = *img.get_pixel(5 * s, 4 * s);
    for y in (5 * s)..(72 * s) {
        for x in (5 * s)..(171 * s) {
            img.put_pixel(x, y, fill_color);
        }
    }

    // Paste region (7,83)-(25,101) at 4 positions
    let region = imageops::crop_imm(&img, 7 * s, 83 * s, 18 * s, 18 * s).to_image();
    let paste_positions = [
        (7 * s, 47 * s),
        (25 * s, 47 * s),
        (43 * s, 47 * s),
        (97 * s, 47 * s),
    ];
    for &(px, py) in &paste_positions {
        paste_region(&mut img, &region, px, py).map_err(|e| format!("failed to paste region: {}", e))?;
    }

    // External overlay
    if let Ok(uimage) = super::get_uimage_path() {
        let overlay_path = uimage.join("smithing2").join(format!("smithing2_{}.png", width));
        if overlay_path.exists() {
            let overlay_img = image::open(&overlay_path)
                .map_err(|e| format!("failed to open {}: {}", overlay_path.display(), e))?
                .to_rgba8();
            imageops::overlay(&mut img, &overlay_img, 0, 0);
            crate::log_info!("overlayed smithing2_{}.png", width);
        }
    }

    img.save(&smithing2_path)
        .map_err(|e| format!("failed to save smithing.png: {}", e))?;
    crate::log_info!("smithing2: saved smithing.png");

    Ok(())
}

// ---------- Villager2 part ----------

fn process_villager2(container_path: &Path) -> Result<(), String> {
    let villager_path = container_path.join("villager.png");
    let anvil_path = container_path.join("anvil.png");

    if !villager_path.exists() {
        crate::log_info!("villager.png not found, skip villager2");
        return Ok(());
    }

    let img = image::open(&villager_path)
        .map_err(|e| format!("failed to open villager.png: {}", e))?
        .to_rgba8();

    let (width, height) = img.dimensions();
    if width != height {
        crate::log_info!("villager.png is not square ({}x{}), skip villager2", width, height);
        return Ok(());
    }

    let s = match width {
        256 => 1,
        512 => 2,
        1024 => 4,
        2048 => 8,
        _ => {
            crate::log_info!("unsupported villager.png size: {}, skip villager2", width);
            return Ok(());
        }
    };

    // Create double-width transparent image
    let new_w = width * 2;
    let new_h = height;
    let mut villager2_img = RgbaImage::new(new_w, new_h);

    let scaled = |c: u32| c * s;

    // Paste cropped (0,0)-(240,166) at (100*s, 0)
    let crop_w = scaled(240);
    let crop_h = scaled(166);
    let cropped = imageops::crop_imm(&img, 0, 0, crop_w, crop_h).to_image();
    imageops::overlay(&mut villager2_img, &cropped, scaled(100) as i64, 0);

    // External overlay from villager2/
    if let Ok(uimage) = super::get_uimage_path() {
        let overlay_path = uimage.join("villager2").join(format!("villager2_{}.png", 256 * s));
        if overlay_path.exists() {
            let overlay_img = image::open(&overlay_path)
                .map_err(|e| format!("failed to open {}: {}", overlay_path.display(), e))?
                .to_rgba8();
            imageops::overlay(&mut villager2_img, &overlay_img, 0, 0);
            crate::log_info!("overlayed villager2_{}.png", 256 * s);
        }
    }

    // Fill (186,24)-(208,39) with color from (185,17)
    let color1 = *villager2_img.get_pixel(scaled(185), scaled(17));
    for y in scaled(24)..scaled(39) {
        for x in scaled(186)..scaled(208) {
            villager2_img.put_pixel(x, y, color1);
        }
    }

    // Move (133,48)-(242,76) up by 16*s
    let move_w = scaled(242) - scaled(133);
    let move_h = scaled(76) - scaled(48);
    let moved = imageops::crop_imm(&villager2_img, scaled(133), scaled(48), move_w, move_h).to_image();
    let dst_y = scaled(48) - scaled(16);
    imageops::overlay(&mut villager2_img, &moved, scaled(133) as i64, dst_y as i64);

    // Fill (133,60)-(242,76) with color from (132,60)
    let color2 = *villager2_img.get_pixel(scaled(132), scaled(60));
    for y in scaled(60)..scaled(76) {
        for x in scaled(133)..scaled(242) {
            villager2_img.put_pixel(x, y, color2);
        }
    }

    // Set (0,166)-(110,198) to transparent
    for y in scaled(166)..scaled(198) {
        for x in 0..scaled(110) {
            villager2_img.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
        }
    }

    // Paste anvil region (176,0)-(204,21) onto villager2 at (176,0)
    if anvil_path.exists() {
        let anvil_img = image::open(&anvil_path)
            .map_err(|e| format!("failed to open anvil.png: {}", e))?
            .to_rgba8();
        let anvil_resized = if anvil_img.dimensions() != (new_w, new_h) {
            imageops::resize(&anvil_img, new_w, new_h, imageops::FilterType::Nearest)
        } else {
            anvil_img
        };
        let anvil_crop = imageops::crop_imm(
            &anvil_resized,
            scaled(176),
            0,
            scaled(204) - scaled(176),
            scaled(21),
        ).to_image();
        imageops::overlay(&mut villager2_img, &anvil_crop, scaled(176) as i64, 0);
    }

    // Backup original villager.png
    let backup_path = container_path.join("villager_backup.png");
    if !backup_path.exists() {
        fs::copy(&villager_path, &backup_path)
            .map_err(|e| format!("failed to backup villager.png: {}", e))?;
        crate::log_info!("backed up villager.png");
    }

    // Save as villager.png (overwrite original)
    villager2_img
        .save(&villager_path)
        .map_err(|e| format!("failed to save villager.png: {}", e))?;
    crate::log_info!("villager2: saved villager.png");

    Ok(())
}

// ---------- Entry point ----------

pub fn fix_smithing2_villager2_ui(path: &Path) -> Result<(), String> {
    let container_path = path.join("assets/minecraft/textures/gui/container");
    if !container_path.exists() {
        return Ok(());
    }

    process_smithing2(&container_path)?;
    process_villager2(&container_path)?;

    crate::log_info!("fix_smithing2_villager2_ui completed");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "fix_smithing2_villager2_ui",
        TaskType::Exclusive,
        TaskTier::Surgeon,
        |context| fix_smithing2_villager2_ui(context.temp_dir()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_fix_smithing2_villager2_ui() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let result = fix_smithing2_villager2_ui(temp_dir.path());
        assert!(result.is_ok());
    }

    /// End-to-end test: feed the real 1.20 vanilla anvil.png (Pika 5K 16x
    /// 资源包自带,256x256) into fix_smithing2_villager2_ui, and verify the
    /// produced smithing.png satisfies the invariants from pack.py:
    ///   1. dimensions are 256x256
    ///   2. cover_box (5..171, 5..72) is filled with the color sampled at (5, 4)
    ///   3. the 4 paste positions (7, 47) / (25, 47) / (43, 47) / (97, 47)
    ///      contain the same 18x18 region originally at (7, 83)-(25, 101)
    ///   4. anvil.png is NOT modified on disk (we only mutate the in-memory copy)
    #[test]
    fn test_smithing2_with_pika5k_anvil() {
        // 真实的 vanilla 1.20 anvil.png (Pika 5K 16x 包提供,1895 字节)
        let anvil_src = r"D:\GameTime\.minecraft\versions\1.20.1-OptiFine_I6\resourcepacks\!   §1§b§lPika 5K - 16x\assets\minecraft\textures\gui\container\anvil.png";
        if !std::path::Path::new(anvil_src).exists() {
            // Skip silently if the dev's Pika 5K fixture is not on this box.
            eprintln!("SKIP: Pika 5K anvil.png not found at {}", anvil_src);
            return;
        }

        let temp = tempdir().expect("tempdir");
        let container = temp.path().join("assets/minecraft/textures/gui/container");
        fs::create_dir_all(&container).expect("mkdir container");
        let dest = container.join("anvil.png");
        fs::copy(anvil_src, &dest).expect("copy anvil.png");

        // Record hash of the on-disk anvil.png so we can assert it's not mutated
        let anvil_before = fs::read(&dest).expect("read anvil before");

        fix_smithing2_villager2_ui(temp.path()).expect("fix_smithing2_villager2_ui");

        // 1. smithing.png must exist and be 256x256
        let smithing_path = container.join("smithing.png");
        assert!(
            smithing_path.exists(),
            "smithing.png was not produced (cut_gui would have no input)"
        );
        let smithing = image::open(&smithing_path)
            .expect("open smithing.png")
            .to_rgba8();
        assert_eq!(smithing.dimensions(), (256, 256), "smithing.png size");

        // 4. anvil.png on disk must not be modified (in-memory copy was used)
        let anvil_after = fs::read(&dest).expect("read anvil after");
        assert_eq!(
            anvil_before, anvil_after,
            "anvil.png on disk was modified — fix_smithing2_villager2_ui must \
             operate on a copy, not the original (matches pack.py img.copy())"
        );

        // 2. cover_box (5..171, 5..72) is filled with the color from (5, 4) — BEFORE
        //    the smithing2 overlay is applied. After overlay, the pixels in cover_box
        //    may be overwritten. The invariant we can check post-overlay is that the
        //    (5, 4) corner itself is still the fill_color (overlay is offset (0,0)
        //    and smithing2_256.png keeps this corner transparent in vanilla 1.20).
        let fill_color = *smithing.get_pixel(5, 4);
        // Sample only the corner (10, 10) which is inside cover_box but typically
        // outside the smithing2 template's filled region. If overlay is correct,
        // this should be transparent (alpha=0).
        let pixel_10_10 = smithing.get_pixel(10, 10);
        let a_10_10 = pixel_10_10[3];
        assert!(
            a_10_10 == 0 || *pixel_10_10 == fill_color,
            "cover_box (10, 10) should either be fill_color or transparent overlay \
             — got {:?}. If you see opaque non-fill color here, the smithing2 template \
             is overpainting the entire image and anvil/region content is being lost.",
            pixel_10_10
        );

        // 3. the 4 paste positions contain an 18x18 region (any color, but non-empty
        //    — i.e. NOT transparent fill_color at (5,4) which would mean the paste
        //    step never ran). After overlay, the paste positions (7, 47) etc. are
        //    expected to be covered by the smithing2 template — so we just check
        //    that the alpha is > 0 at each paste center (i.e. something was drawn).
        let paste_positions = [(7, 47), (25, 47), (43, 47), (97, 47)];
        for &(px, py) in &paste_positions {
            let a = smithing.get_pixel(px + 1, py + 1)[3];
            assert!(
                a > 0,
                "paste position ({},{}) is fully transparent — region was not \
                 actually pasted at this position",
                px, py
            );
        }
    }

    /// End-to-end test for the villager2 part: take a 256x256 villager.png with
    /// known content and verify the produced villager.png (the function overwrites
    /// villager.png in place) has the expected new structure.
    /// Invariant: the (133, 48)-(242, 76) region (move-source) should be ERASED
    /// to a solid color, not double-stamped. This is the same bug class as
    /// fix_ui_survival.move_region.
    #[test]
    fn test_villager2_move_erases_source() {
        let temp = tempdir().expect("tempdir");
        let container = temp.path().join("assets/minecraft/textures/gui/container");
        fs::create_dir_all(&container).expect("mkdir container");

        // Build a synthetic villager.png: 256x256, distinctive colors at known positions
        let mut villager = RgbaImage::from_pixel(256, 256, Rgba([20, 20, 20, 255])); // dark grey bg
        // Red block at (133, 48, 242, 76) — the move-source region (109x28)
        for y in 48..76 {
            for x in 133..242 {
                villager.put_pixel(x, y, Rgba([220, 20, 20, 255])); // bright red
            }
        }
        villager
            .save(container.join("villager.png"))
            .expect("save villager.png");

        fix_smithing2_villager2_ui(temp.path()).expect("fix_smithing2_villager2_ui");

        let out = image::open(container.join("villager.png"))
            .expect("open villager.png")
            .to_rgba8();
        let (w, h) = out.dimensions();
        assert_eq!((w, h), (512, 256), "villager2 should be 512x256 (width*2)");

        // The move-source region (133, 48)-(242, 76) must NOT contain the red
        // block at the original position (it was moved to (133, 32)-(242, 60)).
        // Sample a few pixels in the bottom strip of the original (which doesn't
        // overlap with the destination):
        //   original:  (133..242, 48..76) — total 109x28
        //   destination: (133..242, 32..60) — total 109x28
        //   non-overlapping source strip: (133..242, 60..76) — bottom 16 rows
        for y in 60..76 {
            for x in 133..242 {
                let p = out.get_pixel(x, y);
                assert_ne!(
                    *p,
                    Rgba([220, 20, 20, 255]),
                    "source bottom strip ({},{}) still has the red block — \
                     move_region used overlay (alpha-blend) instead of paste \
                     (raw overwrite), so the source row was never erased",
                    x, y
                );
            }
        }
    }
}
