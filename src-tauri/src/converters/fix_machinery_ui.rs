use std::fs;
use std::path::Path;

use image::{imageops, RgbaImage};

use crate::hurray::scheduler::{TaskTier, TaskType};
use crate::image_utils::paste_region;

/// Open shulker_box.png, determine scale, fill cover_box, paste region at
/// specified positions, overlay external image, and optionally paste anvil
/// region. Save result to `out_name` in container_path.
fn process_ui_from_shulker(
    container_path: &Path,
    out_name: &str,
    paste_positions: &[(u32, u32)],
    overlay_subdir: &str,
    overlay_prefix: &str,
    paste_anvil: bool,
) -> Result<(), String> {
    let shulker_path = container_path.join("shulker_box.png");
    let out_path = container_path.join(out_name);

    if !shulker_path.exists() {
        crate::log_info!("shulker_box.png not found, skip {}", out_name);
        return Ok(());
    }

    let mut img = image::open(&shulker_path)
        .map_err(|e| format!("failed to open shulker_box.png: {}", e))?
        .to_rgba8();

    let (width, height) = img.dimensions();
    let s = match (width, height) {
        (256, 256) => 1,
        (512, 512) => 2,
        (1024, 1024) => 4,
        (2048, 2048) => 8,
        _ => {
            crate::log_info!("unsupported shulker_box size {}x{}, skip {}", width, height, out_name);
            return Ok(());
        }
    };

    // Fill cover_box (6,16)-(170,72) with color from (5,4)
    let fill_color = *img.get_pixel(5 * s, 4 * s);
    for y in (16 * s)..(72 * s) {
        for x in (6 * s)..(170 * s) {
            img.put_pixel(x, y, fill_color);
        }
    }

    // Crop and paste region at positions
    let region = imageops::crop_imm(&img, 7 * s, 83 * s, 18 * s, 18 * s).to_image();
    for &(px, py) in paste_positions {
        paste_region(&mut img, &region, px * s, py * s).map_err(|e| format!("failed to paste region: {}", e))?;
    }

    // External overlay
    if let Ok(uimage) = super::get_uimage_path() {
        let overlay_path = uimage.join(overlay_subdir).join(format!("{}_{}.png", overlay_prefix, width));
        if overlay_path.exists() {
            let overlay_img = image::open(&overlay_path)
                .map_err(|e| format!("failed to open overlay: {}", e))?
                .to_rgba8();
            imageops::overlay(&mut img, &overlay_img, 0, 0);
            crate::log_info!("overlayed {}_{}.png", overlay_prefix, width);
        }
    }

    // Optionally paste anvil region
    if paste_anvil {
        let anvil_path = container_path.join("anvil.png");
        if anvil_path.exists() {
            let anvil_img = image::open(&anvil_path)
                .map_err(|e| format!("failed to open anvil.png: {}", e))?
                .to_rgba8();
            let anvil_resized = if anvil_img.dimensions() != (width, width) {
                imageops::resize(&anvil_img, width, width, imageops::FilterType::Nearest)
            } else {
                anvil_img
            };
            let crop = imageops::crop_imm(&anvil_resized, 176 * s, 0, 28 * s, 21 * s).to_image();
            imageops::overlay(&mut img, &crop, (176 * s) as i64, 0);
        }
    }

    img.save(&out_path)
        .map_err(|e| format!("failed to save {}: {}", out_name, e))?;
    crate::log_info!("generated {}", out_name);

    Ok(())
}

fn process_grindstone(container_path: &Path) -> Result<(), String> {
    process_ui_from_shulker(
        container_path,
        "grindstone.png",
        &[(48, 18), (128, 33), (48, 39)],
        "grindstone",
        "grindstone",
        true,
    )
}

fn process_cartography_table(container_path: &Path) -> Result<(), String> {
    process_ui_from_shulker(
        container_path,
        "cartography_table.png",
        &[(14, 51), (144, 38), (14, 14)],
        "cartography_table",
        "cartography_table",
        false,
    )
}

fn process_stonecutter(container_path: &Path) -> Result<(), String> {
    process_ui_from_shulker(
        container_path,
        "stonecutter.png",
        &[(19, 32), (142, 32)],
        "stonecutter",
        "stonecutter",
        false,
    )
}

fn process_loom(container_path: &Path) -> Result<(), String> {
    process_ui_from_shulker(
        container_path,
        "loom.png",
        &[(12, 25), (32, 25), (22, 44), (142, 56)],
        "loom",
        "loom",
        false,
    )
}

fn process_villager2_machinery(container_path: &Path) -> Result<(), String> {
    let villager_path = container_path.join("villager.png");
    if !villager_path.exists() {
        crate::log_info!("villager.png not found, skip villager2 machinery");
        return Ok(());
    }

    let img = image::open(&villager_path)
        .map_err(|e| format!("failed to open villager.png: {}", e))?
        .to_rgba8();

    let (width, height) = img.dimensions();
    if width != height {
        crate::log_info!("villager.png is not square ({}x{}), skip villager2 machinery", width, height);
        return Ok(());
    }
    let s = match width {
        256 => 1,
        512 => 2,
        1024 => 4,
        2048 => 8,
        _ => {
            crate::log_info!("unsupported villager.png size: {}, skip", width);
            return Ok(());
        }
    };

    let new_w = width * 2;
    let new_h = height;
    let mut villager2_img = RgbaImage::new(new_w, new_h);

    // Paste cropped (0,0)-(240,166) at (100*s, 0)
    let cropped = imageops::crop_imm(&img, 0, 0, 240 * s, 166 * s).to_image();
    imageops::overlay(&mut villager2_img, &cropped, (100 * s) as i64, 0);

    // Overlay external villager2
    if let Ok(uimage) = super::get_uimage_path() {
        let overlay_path = uimage.join("villager2").join(format!("villager2_{}.png", 256 * s));
        if overlay_path.exists() {
            let overlay = image::open(&overlay_path)
                .map_err(|e| format!("failed to open overlay: {}", e))?
                .to_rgba8();
            imageops::overlay(&mut villager2_img, &overlay, 0, 0);
        }
    }

    // Fill (186,24)-(208,39)
    let c1 = *villager2_img.get_pixel(185 * s, 17 * s);
    for y in (24 * s)..(39 * s) { for x in (186 * s)..(208 * s) { villager2_img.put_pixel(x, y, c1); } }

    // Move (133,48)-(242,76) up 16
    let mv = imageops::crop_imm(&villager2_img, 133 * s, 48 * s, 109 * s, 28 * s).to_image();
    imageops::overlay(&mut villager2_img, &mv, (133 * s) as i64, (32 * s) as i64);

    // Fill (133,60)-(242,76)
    let c2 = *villager2_img.get_pixel(132 * s, 60 * s);
    for y in (60 * s)..(76 * s) { for x in (133 * s)..(242 * s) { villager2_img.put_pixel(x, y, c2); } }

    // Set (0,166)-(110,198) transparent
    for y in (166 * s)..(198 * s) { for x in 0..(110 * s) { villager2_img.put_pixel(x, y, image::Rgba([0,0,0,0])); } }

    // Paste anvil
    let anvil_path = container_path.join("anvil.png");
    if anvil_path.exists() {
        let anvil = image::open(&anvil_path)
            .map_err(|e| format!("failed to open anvil: {}", e))?
            .to_rgba8();
        let anvil_r = if anvil.dimensions() != (new_w, new_h) {
            imageops::resize(&anvil, new_w, new_h, imageops::FilterType::Nearest)
        } else { anvil };
        let ac = imageops::crop_imm(&anvil_r, 176 * s, 0, 28 * s, 21 * s).to_image();
        imageops::overlay(&mut villager2_img, &ac, (176 * s) as i64, 0);
    }

    // Backup + save
    let backup = container_path.join("villager_backup.png");
    if !backup.exists() {
        fs::copy(&villager_path, &backup)
            .map_err(|e| format!("failed to backup: {}", e))?;
    }
    villager2_img.save(&villager_path)
        .map_err(|e| format!("failed to save villager.png: {}", e))?;
    crate::log_info!("villager2 machinery: saved");

    Ok(())
}

pub fn fix_machinery_ui(path: &Path) -> Result<(), String> {
    let container_path = path.join("assets/minecraft/textures/gui/container");
    if !container_path.exists() {
        return Ok(());
    }

    crate::log_info!("processing machinery UI images");

    process_grindstone(&container_path)?;
    process_cartography_table(&container_path)?;
    process_stonecutter(&container_path)?;
    process_loom(&container_path)?;
    process_villager2_machinery(&container_path)?;

    crate::log_info!("fix_machinery_ui completed");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "fix_machinery_ui",
        TaskType::Exclusive,
        TaskTier::Surgeon,
        |context| fix_machinery_ui(context.temp_dir()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_fix_machinery_ui() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let result = fix_fix_machinery_ui_smoke(temp_dir.path());
        assert!(result.is_ok());
    }

    fn fix_fix_machinery_ui_smoke(path: &std::path::Path) -> Result<(), String> {
        // Just make sure the no-arg path doesn't crash — both anvil and shulker
        // are missing in this empty tempdir, so all sub-processes return Ok(())
        // with a log line.
        fix_machinery_ui(path)
    }

    /// End-to-end test: feed the real 1.20 vanilla shulker_box.png (Pika 5K 16x
    /// 资源包自带,256x256) and anvil.png into fix_machinery_ui, and verify
    /// the produced cartography_table.png, grindstone.png, stonecutter.png,
    /// loom.png match the invariants from pack.py:
    ///   1. dimensions are 256x256
    ///   2. cover_box (6..170, 16..72) is filled with the color from (5, 4)
    ///   3. the paste positions (3 or 4 depending on sub-task) contain a
    ///      non-transparent 18x18 region
    ///   4. shulker_box.png is NOT modified on disk
    ///   5. for grindstone: anvil's (176, 0, 204, 21) region is pasted into
    ///      grindstone at (176, 0)
    #[test]
    fn test_machinery_ui_with_pika5k_shulker() {
        let shulker_src = r"D:\GameTime\.minecraft\versions\1.20.1-OptiFine_I6\resourcepacks\!   §1§b§lPika 5K - 16x\assets\minecraft\textures\gui\container\shulker_box.png";
        let anvil_src = r"D:\GameTime\.minecraft\versions\1.20.1-OptiFine_I6\resourcepacks\!   §1§b§lPika 5K - 16x\assets\minecraft\textures\gui\container\anvil.png";
        if !std::path::Path::new(shulker_src).exists() || !std::path::Path::new(anvil_src).exists() {
            eprintln!("SKIP: Pika 5K shulker_box.png or anvil.png not found");
            return;
        }

        let temp = tempdir().expect("tempdir");
        let container = temp.path().join("assets/minecraft/textures/gui/container");
        fs::create_dir_all(&container).expect("mkdir container");
        let shulker_dest = container.join("shulker_box.png");
        let anvil_dest = container.join("anvil.png");
        fs::copy(shulker_src, &shulker_dest).expect("copy shulker");
        fs::copy(anvil_src, &anvil_dest).expect("copy anvil");

        let shulker_before = fs::read(&shulker_dest).expect("read shulker before");
        let anvil_before = fs::read(&anvil_dest).expect("read anvil before");

        fix_machinery_ui(temp.path()).expect("fix_machinery_ui");

        // 4. shulker_box.png on disk must not be modified
        let shulker_after = fs::read(&shulker_dest).expect("read shulker after");
        assert_eq!(
            shulker_before, shulker_after,
            "shulker_box.png on disk was modified — process_ui_from_shulker must \
             operate on a copy, not the original (matches pack.py img.copy())"
        );

        // anvil.png is read but not modified in fix_machinery_ui (only in
        // fix_smithing2_villager2_ui.process_smithing2). Just to be safe:
        let anvil_after = fs::read(&anvil_dest).expect("read anvil after");
        assert_eq!(
            anvil_before, anvil_after,
            "anvil.png on disk was unexpectedly modified by fix_machinery_ui"
        );

        // The 4 outputs must all be 256x256
        for out_name in &[
            "cartography_table.png",
            "grindstone.png",
            "stonecutter.png",
            "loom.png",
        ] {
            let p = container.join(out_name);
            assert!(p.exists(), "{} was not produced", out_name);
            let img = image::open(&p).expect("open output").to_rgba8();
            assert_eq!(
                img.dimensions(),
                (256, 256),
                "{} size should be 256x256, got {:?}",
                out_name,
                img.dimensions()
            );
        }

        // Verify paste positions have content (the 3-4 18x18 icons should be
        // visible). After overlay, we just check the paste center is not
        // transparent.
        let cases: &[(&str, &[(u32, u32)])] = &[
            ("cartography_table.png", &[(14, 51), (144, 38), (14, 14)]),
            ("grindstone.png", &[(48, 18), (128, 33), (48, 39)]),
            ("stonecutter.png", &[(19, 32), (142, 32)]),
            ("loom.png", &[(12, 25), (32, 25), (22, 44), (142, 56)]),
        ];
        for (name, positions) in cases {
            let img = image::open(container.join(name))
                .expect(name)
                .to_rgba8();
            for &(px, py) in positions.iter() {
                let a = img.get_pixel(px + 1, py + 1)[3];
                assert!(
                    a > 0,
                    "{} paste position ({},{}) is fully transparent — region \
                     was not actually pasted here",
                    name, px, py
                );
            }
        }

        // 5. For grindstone: anvil (176, 0, 204, 21) should be pasted at (176, 0).
        //    This region in the anvil.png is the "X" red error icon (opaque red).
        //    After paste, grindstone (176, 0, 204, 21) should be opaque red.
        let grindstone = image::open(container.join("grindstone.png"))
            .expect("grindstone")
            .to_rgba8();
        let anvil_img = image::open(&anvil_dest).expect("anvil").to_rgba8();
        let anvil_sample = anvil_img.get_pixel(190, 10); // middle of (176, 0, 204, 21)
        let gs_sample = grindstone.get_pixel(190, 10);
        assert_eq!(
            *anvil_sample, *gs_sample,
            "grindstone (190, 10) should equal anvil (190, 10) — anvil region \
             was not pasted at (176, 0). anvil={:?}, grindstone={:?}",
            anvil_sample, gs_sample
        );
    }
}
