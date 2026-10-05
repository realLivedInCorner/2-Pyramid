    use super::*;
    use image::imageops;

    const GUI: &str = "assets/minecraft/textures/gui/container";
    const ANVIL: &str = "assets/minecraft/textures/gui/container/anvil.png";
    const SMITHING: &str = "assets/minecraft/textures/gui/container/smithing.png";
    const VILLAGER: &str = "assets/minecraft/textures/gui/container/villager.png";
    const BACKUP: &str = "assets/minecraft/textures/gui/container/villager_backup.png";

    /// `UImage` 目录（与其它试点同一个解析函数）。
    fn uimage_dir() -> Option<std::path::PathBuf> {
        match crate::image_utils::get_uimage_path() {
            Ok(p) => Some(p),
            Err(e) => {
                crate::log_info!("UImage path not available: {}", e);
                None
            }
        }
    }

    fn read_rgba(tx: &Tx<'_>, path: &str) -> Result<Option<RgbaImage>, AromError> {
        let Some(bytes) = tx.read(path)? else {
            return Ok(None);
        };
        match image::load_from_memory(&bytes) {
            Ok(img) => Ok(Some(img.to_rgba8())),
            Err(e) => Err(AromError::Io(format!("failed to open {path}: {e}"))),
        }
    }

    fn png_bytes(img: &RgbaImage) -> Result<Vec<u8>, AromError> {
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img.clone())
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .map_err(|e| AromError::Io(format!("png encode failed: {e}")))?;
        Ok(buf)
    }

    /// 旧 `paste_region` 在**同尺寸**区域上的等价物：整块覆盖（含 alpha）。
    fn paste(img: &mut RgbaImage, region: &RgbaImage, dx: u32, dy: u32) {
        let (rw, rh) = region.dimensions();
        for y in 0..rh {
            for x in 0..rw {
                let px = dx + x;
                let py = dy + y;
                if px < img.width() && py < img.height() {
                    img.put_pixel(px, py, *region.get_pixel(x, y));
                }
            }
        }
    }

    fn fill_rect(img: &mut RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32, color: image::Rgba<u8>) {
        for y in y0..y1 {
            for x in x0..x1 {
                img.put_pixel(x, y, color);
            }
        }
    }

    /// 旧 `process_smithing2`。
    fn process_smithing2(tx: &mut Tx<'_>, outcome: &mut Outcome) -> Result<(), AromError> {
        if !tx.exists(ANVIL) {
            crate::log_info!("anvil.png not found, skip smithing2");
            return Ok(());
        }
        let Some(mut img) = read_rgba(tx, ANVIL)? else {
            return Ok(());
        };
        let (width, height) = img.dimensions();
        let s = match (width, height) {
            (256, 256) => 1,
            (512, 512) => 2,
            (1024, 1024) => 4,
            (2048, 2048) => 8,
            _ => {
                crate::log_info!(
                    "unsupported anvil.png size: {}x{}, skip smithing2",
                    width,
                    height
                );
                return Ok(());
            }
        };

        // 用 (5,4) 的颜色填 (5,5)-(171,72)
        let fill_color = *img.get_pixel(5 * s, 4 * s);
        fill_rect(&mut img, 5 * s, 5 * s, 171 * s, 72 * s, fill_color);

        // 把 (7,83) 起的 18x18 区域贴到 4 个位置
        let region = imageops::crop_imm(&img, 7 * s, 83 * s, 18 * s, 18 * s).to_image();
        for &(px, py) in &[
            (7 * s, 47 * s),
            (25 * s, 47 * s),
            (43 * s, 47 * s),
            (97 * s, 47 * s),
        ] {
            paste(&mut img, &region, px, py);
        }

        // 外部叠加（与其它试点一致：UImage 在真实文件系统上）
        if let Some(uimage) = uimage_dir() {
            let overlay_path = uimage.join("smithing2").join(format!("smithing2_{}.png", width));
            if overlay_path.exists() {
                if let Ok(overlay_img) = image::open(&overlay_path).map(|i| i.to_rgba8()) {
                    imageops::overlay(&mut img, &overlay_img, 0, 0);
                    crate::log_info!("overlayed smithing2_{}.png", width);
                }
            }
        }

        tx.put(SMITHING, png_bytes(&img)?)?;
        outcome.changed += 1;
        outcome.notes.push("smithing2: saved smithing.png".into());
        Ok(())
    }

    /// 旧 `process_villager2`。
    fn process_villager2(tx: &mut Tx<'_>, outcome: &mut Outcome) -> Result<(), AromError> {
        if !tx.exists(VILLAGER) {
            crate::log_info!("villager.png not found, skip villager2");
            return Ok(());
        }
        let Some(img) = read_rgba(tx, VILLAGER)? else {
            return Ok(());
        };
        let (width, height) = img.dimensions();
        if width != height {
            crate::log_info!(
                "villager.png is not square ({}x{}), skip villager2",
                width,
                height
            );
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

        let scaled = |c: u32| c * s;
        let new_w = width * 2;
        let new_h = height;
        let mut v2 = RgbaImage::new(new_w, new_h);

        // 把 (0,0)-(240,166) 贴到 (100*s, 0)
        let cropped = imageops::crop_imm(&img, 0, 0, scaled(240), scaled(166)).to_image();
        imageops::overlay(&mut v2, &cropped, scaled(100) as i64, 0);

        // 外部叠加 villager2/villager2_{256*s}.png
        if let Some(uimage) = uimage_dir() {
            let overlay_path = uimage
                .join("villager2")
                .join(format!("villager2_{}.png", 256 * s));
            if overlay_path.exists() {
                if let Ok(overlay_img) = image::open(&overlay_path).map(|i| i.to_rgba8()) {
                    imageops::overlay(&mut v2, &overlay_img, 0, 0);
                    crate::log_info!("overlayed villager2_{}.png", 256 * s);
                }
            }
        }

        // 用 (185,17) 的颜色填 (186,24)-(208,39)
        let color1 = *v2.get_pixel(scaled(185), scaled(17));
        fill_rect(&mut v2, scaled(186), scaled(24), scaled(208), scaled(39), color1);

        // 把 (133,48)-(242,76) 上移 16*s
        let move_w = scaled(242) - scaled(133);
        let move_h = scaled(76) - scaled(48);
        let moved = imageops::crop_imm(&v2, scaled(133), scaled(48), move_w, move_h).to_image();
        let dst_y = scaled(48) - scaled(16);
        imageops::overlay(&mut v2, &moved, scaled(133) as i64, dst_y as i64);

        // 用 (132,60) 的颜色填 (133,60)-(242,76)
        let color2 = *v2.get_pixel(scaled(132), scaled(60));
        fill_rect(&mut v2, scaled(133), scaled(60), scaled(242), scaled(76), color2);

        // (0,166)-(110,198) 置为全透明
        fill_rect(&mut v2, 0, scaled(166), scaled(110), scaled(198), image::Rgba([0, 0, 0, 0]));

        // 把 anvil 的 (176,0)-(204,21) 贴到 villager2 的同位置（尺寸不符时按 Nearest 缩放到新尺寸）
        if tx.exists(ANVIL) {
            if let Some(anvil_img) = read_rgba(tx, ANVIL)? {
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
                )
                .to_image();
                imageops::overlay(&mut v2, &anvil_crop, scaled(176) as i64, 0);
            }
        }

        // 备份原图（仅当备份不存在）
        if !tx.exists(BACKUP) {
            let bytes = tx.read(VILLAGER)?.unwrap_or_default();
            tx.put(BACKUP, bytes)?;
            outcome.changed += 1;
            outcome.notes.push("backed up villager.png".into());
        }

        // 覆盖写回 villager.png
        tx.put(VILLAGER, png_bytes(&v2)?)?;
        outcome.changed += 1;
        outcome.notes.push("villager2: saved villager.png".into());
        Ok(())
    }

    pub fn decl() -> TaskDecl {
        TaskDecl::new("fix_smithing2_villager2_ui", Tier::Surgeon)
            .reads(ScopeSet::prefix(GUI))
            .writes(ScopeSet::prefix(GUI))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        if !tx.has_prefix(GUI)? {
            return Ok(Outcome::default());
        }
        let mut outcome = Outcome::default();
        process_smithing2(tx, &mut outcome)?;
        process_villager2(tx, &mut outcome)?;
        crate::log_info!("fix_smithing2_villager2_ui completed");
        Ok(outcome)
    }
