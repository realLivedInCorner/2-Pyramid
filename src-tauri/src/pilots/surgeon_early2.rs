    use super::*;
    use image::imageops;

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

    /// 旧 `converters/ui/overlay_icons.rs`。
    pub mod overlay_icons {
        use super::*;

        const ICONS: &str = "assets/minecraft/textures/gui/icons.png";

        /// 旧 `alpha_paste`：以覆盖图 alpha 为蒙版做**线性混合**（不是 `imageops::overlay`）。
        fn alpha_paste(base: &mut RgbaImage, overlay: &RgbaImage, dest_x: u32, dest_y: u32) {
            let (base_w, base_h) = base.dimensions();
            let (overlay_w, overlay_h) = overlay.dimensions();
            for y in 0..overlay_h {
                let target_y = dest_y + y;
                if target_y >= base_h {
                    continue;
                }
                for x in 0..overlay_w {
                    let target_x = dest_x + x;
                    if target_x >= base_w {
                        continue;
                    }
                    let src_pixel = overlay.get_pixel(x, y);
                    let alpha = src_pixel[3] as f64 / 255.0;
                    if alpha > 0.0 {
                        let dst_pixel = base.get_pixel(target_x, target_y);
                        let blended = [
                            ((1.0 - alpha) * dst_pixel[0] as f64 + alpha * src_pixel[0] as f64) as u8,
                            ((1.0 - alpha) * dst_pixel[1] as f64 + alpha * src_pixel[1] as f64) as u8,
                            ((1.0 - alpha) * dst_pixel[2] as f64 + alpha * src_pixel[2] as f64) as u8,
                            dst_pixel[3].max(src_pixel[3]),
                        ];
                        base.put_pixel(target_x, target_y, image::Rgba(blended));
                    }
                }
            }
        }

        pub fn decl() -> TaskDecl {
            TaskDecl::new("overlay_icons", Tier::Surgeon)
                .reads(ScopeSet::exact(ICONS))
                .writes(ScopeSet::exact(ICONS))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(ICONS) {
                crate::log_info!("icons.png not found, skip");
                return Ok(Outcome::default());
            }
            let mut base: RgbaImage = (*tx.image(ICONS)?).clone();
            let (width, height) = base.dimensions();
            if width != height {
                crate::log_info!("icons.png is not square, skip");
                return Ok(Outcome::default());
            }
            let overlay_filename = match width {
                256 => "icons_256.png",
                512 => "icons_512.png",
                1024 => "icons_1024.png",
                2048 => "icons_2048.png",
                _ => {
                    crate::log_info!("unsupported icons.png size, skip");
                    return Ok(Outcome::default());
                }
            };

            let mut changed = 0usize;
            match uimage_dir() {
                Some(dir) => {
                    let overlay_path = dir.join("icons").join(overlay_filename);
                    if overlay_path.exists() {
                        let overlay = image::open(&overlay_path)
                            .map_err(|e| {
                                AromError::io(format!(
                                    "failed to open {}: {}",
                                    overlay_path.display(),
                                    e
                                ))
                            })?
                            .to_rgba8();
                        alpha_paste(&mut base, &overlay, 0, 0);
                        changed = 1;
                    }
                }
                None => crate::log_info!("UImage path not available, skip overlay_icons"),
            }

            // 旧实现**无论是否叠加都会重写** icons.png（即使没装覆盖图）——照抄这个行为
            tx.put_image(ICONS, &base)?;
            Ok(Outcome {
                changed,
                notes: vec![format!("icons.png rewritten (overlay applied: {changed})")],
                ..Outcome::default()
            })
        }
    }

    /// 旧 `converters/ui/brewing_stand.rs`。
    pub mod brewing_stand {
        use super::*;

        const CONTAINER: &str = "assets/minecraft/textures/gui/container";
        const SHULKER: &str = "assets/minecraft/textures/gui/container/shulker_box.png";
        const BREWING: &str = "assets/minecraft/textures/gui/container/brewing_stand.png";

        pub fn decl() -> TaskDecl {
            TaskDecl::new("fix_brewing_stand_ui", Tier::Surgeon)
                .reads(ScopeSet::prefix(CONTAINER))
                .writes(ScopeSet::prefix(CONTAINER))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(SHULKER) {
                crate::log_info!("shulker_box.png not found, skip brewing stand generation");
                return Ok(Outcome::default());
            }
            let mut img: RgbaImage = (*tx.image(SHULKER)?).clone();
            let (width, height) = img.dimensions();
            if width != height {
                crate::log_info!("shulker_box.png is not square, skip");
                return Ok(Outcome::default());
            }
            let s = match width {
                256 => 1,
                512 => 2,
                1024 => 4,
                2048 => 8,
                _ => {
                    crate::log_info!("unsupported shulker_box.png size, skip");
                    return Ok(Outcome::default());
                }
            };

            let fill_color = *img.get_pixel(5 * s, 4 * s);
            for y in (16 * s)..(72 * s) {
                for x in (6 * s)..(170 * s) {
                    img.put_pixel(x, y, fill_color);
                }
            }

            let region = imageops::crop_imm(&img, 7 * s, 83 * s, 18 * s, 18 * s).to_image();
            for (px, py) in [
                (16 * s, 16 * s),
                (78 * s, 16 * s),
                (55 * s, 50 * s),
                (78 * s, 57 * s),
                (101 * s, 50 * s),
            ] {
                // 旧实现用 `crate::image_utils::paste_region`（原始覆盖，非 alpha 混合），
                // 且**失败只记录不中止**；这里按同样的「越界即跳过」语义实现。
                for y in 0..region.height() {
                    for x in 0..region.width() {
                        let (dx, dy) = (px + x, py + y);
                        if dx < img.width() && dy < img.height() {
                            let px_val = *region.get_pixel(x, y);
                            img.put_pixel(dx, dy, px_val);
                        }
                    }
                }
            }

            match uimage_dir() {
                Some(dir) => {
                    let overlay_path = dir
                        .join("brewing_stand")
                        .join(format!("brewing_stand_{}.png", width));
                    if overlay_path.exists() {
                        let overlay = image::open(&overlay_path)
                            .map_err(|e| {
                                AromError::io(format!(
                                    "failed to open overlay {}: {}",
                                    overlay_path.display(),
                                    e
                                ))
                            })?
                            .to_rgba8();
                        imageops::overlay(&mut img, &overlay, 0, 0);
                    }
                }
                None => crate::log_info!("UImage path not available, skip brewing_stand overlay"),
            }

            tx.put_image(BREWING, &img)?;
            Ok(Outcome {
                changed: 1,
                notes: vec!["shulker_box.png -> brewing_stand.png".into()],
                ..Outcome::default()
            })
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "overlay_icons" => Some((overlay_icons::decl(), overlay_icons::run)),
            "fix_brewing_stand_ui" => Some((brewing_stand::decl(), brewing_stand::run)),
            _ => None,
        }
    }
