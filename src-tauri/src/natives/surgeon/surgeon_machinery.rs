    use super::*;
    use image::imageops;

    const CONTAINER: &str = "assets/minecraft/textures/gui/container";

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

    /// 旧的 `paste_region`：**原始覆盖**、越界跳过（不用 `copy_from`，它越界会报错）。
    fn paste(img: &mut RgbaImage, region: &RgbaImage, dx: u32, dy: u32) {
        for y in 0..region.height() {
            for x in 0..region.width() {
                if let Some(px) = region.get_pixel_checked(x, y) {
                    if let Some(dst) = img.get_pixel_mut_checked(dx + x, dy + y) {
                        *dst = *px;
                    }
                }
            }
        }
    }

    /// 旧的 `process_ui_from_shulker`。
    #[allow(clippy::too_many_arguments)]
    fn ui_from_shulker(
        tx: &mut Tx<'_>,
        out_name: &str,
        paste_positions: &[(u32, u32)],
        overlay_subdir: &str,
        overlay_prefix: &str,
        paste_anvil: bool,
    ) -> Result<bool, AromError> {
        let shulker = format!("{CONTAINER}/shulker_box.png");
        if !tx.exists(&shulker) {
            crate::log_info!("shulker_box.png not found, skip {out_name}");
            return Ok(false);
        }
        let mut img: RgbaImage = (*tx.image(&shulker)?).clone();
        let (width, height) = img.dimensions();
        let s = match (width, height) {
            (256, 256) => 1,
            (512, 512) => 2,
            (1024, 1024) => 4,
            (2048, 2048) => 8,
            _ => {
                crate::log_info!("unsupported shulker_box size {width}x{height}, skip {out_name}");
                return Ok(false);
            }
        };

        // 填 cover_box（色取自 (5,4)）
        let fill = *img.get_pixel(5 * s, 4 * s);
        for y in (16 * s)..(72 * s) {
            for x in (6 * s)..(170 * s) {
                img.put_pixel(x, y, fill);
            }
        }

        // 18×18 区域贴到各位置（原始覆盖）
        let region = imageops::crop_imm(&img, 7 * s, 83 * s, 18 * s, 18 * s).to_image();
        for (px, py) in paste_positions {
            paste(&mut img, &region, px * s, py * s);
        }

        // 可选 UImage 覆盖图（**alpha 混合**，与上面的原始覆盖不同）
        if let Some(dir) = uimage_dir() {
            let overlay_path = dir
                .join(overlay_subdir)
                .join(format!("{overlay_prefix}_{width}.png"));
            if overlay_path.exists() {
                let overlay = image::open(&overlay_path)
                    .map_err(|e| {
                        AromError::io(format!("failed to open overlay: {e}"))
                    })?
                    .to_rgba8();
                imageops::overlay(&mut img, &overlay, 0, 0);
            }
        }

        // 可选贴 anvil 区域
        if paste_anvil {
            let anvil = format!("{CONTAINER}/anvil.png");
            if tx.exists(&anvil) {
                let anvil_img: RgbaImage = (*tx.image(&anvil)?).clone();
                let resized = if anvil_img.dimensions() != (width, width) {
                    imageops::resize(&anvil_img, width, width, imageops::FilterType::Nearest)
                } else {
                    anvil_img
                };
                let crop = imageops::crop_imm(&resized, 176 * s, 0, 28 * s, 21 * s).to_image();
                imageops::overlay(&mut img, &crop, (176 * s) as i64, 0);
            }
        }

        tx.put_image(&format!("{CONTAINER}/{out_name}"), &img)?;
        Ok(true)
    }

    /// 旧的 `process_villager2_machinery`。
    fn villager2_machinery(tx: &mut Tx<'_>) -> Result<bool, AromError> {
        let villager = format!("{CONTAINER}/villager.png");
        if !tx.exists(&villager) {
            crate::log_info!("villager.png not found, skip villager2 machinery");
            return Ok(false);
        }
        let img: RgbaImage = (*tx.image(&villager)?).clone();
        let (width, height) = img.dimensions();
        if width != height {
            crate::log_info!("villager.png is not square, skip villager2 machinery");
            return Ok(false);
        }
        let s = match width {
            256 => 1,
            512 => 2,
            1024 => 4,
            2048 => 8,
            _ => {
                crate::log_info!("unsupported villager.png size, skip");
                return Ok(false);
            }
        };

        let new_w = width * 2;
        let new_h = height;
        let mut out = RgbaImage::new(new_w, new_h);

        // 贴 (0,0)-(240,166) 到 (100s, 0)（alpha 混合，旧实现用 overlay）
        let cropped = imageops::crop_imm(&img, 0, 0, 240 * s, 166 * s).to_image();
        imageops::overlay(&mut out, &cropped, (100 * s) as i64, 0);

        // 可选 UImage villager2 覆盖图
        if let Some(dir) = uimage_dir() {
            let overlay_path = dir
                .join("villager2")
                .join(format!("villager2_{}.png", 256 * s));
            if overlay_path.exists() {
                let overlay = image::open(&overlay_path)
                    .map_err(|e| AromError::io(format!("failed to open overlay: {e}")))?
                    .to_rgba8();
                imageops::overlay(&mut out, &overlay, 0, 0);
            }
        }

        // 填 (186,24)-(208,39)，色取自 (185,17)
        let c1 = *out.get_pixel(185 * s, 17 * s);
        for y in (24 * s)..(39 * s) {
            for x in (186 * s)..(208 * s) {
                out.put_pixel(x, y, c1);
            }
        }

        // 把 (133,48)-(242,76) 上移 16s（区域裁剪自**当前**图，再覆盖贴回）
        let mv = imageops::crop_imm(&out, 133 * s, 48 * s, 109 * s, 28 * s).to_image();
        imageops::overlay(&mut out, &mv, (133 * s) as i64, (32 * s) as i64);

        // 填 (133,60)-(242,76)，色取自 (132,60)
        let c2 = *out.get_pixel(132 * s, 60 * s);
        for y in (60 * s)..(76 * s) {
            for x in (133 * s)..(242 * s) {
                out.put_pixel(x, y, c2);
            }
        }

        // (0,166)-(110,198) 置透明
        for y in (166 * s)..(198 * s) {
            for x in 0..(110 * s) {
                out.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
            }
        }

        // 可选贴 anvil（缩放到**新图**尺寸）
        let anvil = format!("{CONTAINER}/anvil.png");
        if tx.exists(&anvil) {
            let anvil_img: RgbaImage = (*tx.image(&anvil)?).clone();
            let resized = if anvil_img.dimensions() != (new_w, new_h) {
                imageops::resize(&anvil_img, new_w, new_h, imageops::FilterType::Nearest)
            } else {
                anvil_img
            };
            let crop = imageops::crop_imm(&resized, 176 * s, 0, 28 * s, 21 * s).to_image();
            imageops::overlay(&mut out, &crop, (176 * s) as i64, 0);
        }

        // **先把原图备份**（已存在则跳过备份），再写回 villager.png
        let backup = format!("{CONTAINER}/villager_backup.png");
        if !tx.exists(&backup) {
            let bytes = tx.read(&villager)?.unwrap_or_default();
            tx.put(&backup, bytes)?;
        }
        tx.put_image(&villager, &out)?;
        Ok(true)
    }

    pub fn decl() -> TaskDecl {
        TaskDecl::new("fix_machinery_ui", Tier::Surgeon)
            .reads(ScopeSet::prefix(CONTAINER))
            .writes(ScopeSet::prefix(CONTAINER))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        if !tx.has_prefix(CONTAINER)? {
            return Ok(Outcome::default());
        }
        let mut outcome = Outcome::default();
        // 顺序照抄旧实现
        for (out_name, positions, subdir, prefix, anvil) in [
            (
                "grindstone.png",
                &[(48u32, 18u32), (128, 33), (48, 39)][..],
                "grindstone",
                "grindstone",
                true,
            ),
            (
                "cartography_table.png",
                &[(14u32, 51u32), (144, 38), (14, 14)][..],
                "cartography_table",
                "cartography_table",
                false,
            ),
            (
                "stonecutter.png",
                &[(19u32, 32u32), (142, 32)][..],
                "stonecutter",
                "stonecutter",
                false,
            ),
            (
                "loom.png",
                &[(12u32, 25u32), (32, 25), (22, 44), (142, 56)][..],
                "loom",
                "loom",
                false,
            ),
        ] {
            if ui_from_shulker(tx, out_name, positions, subdir, prefix, anvil)? {
                outcome.changed += 1;
            }
        }
        if villager2_machinery(tx)? {
            outcome.changed += 1;
            outcome.notes.push("villager.png -> double width + backup".into());
        }
        Ok(outcome)
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "fix_machinery_ui" => Some((decl(), run)),
            _ => None,
        }
    }
