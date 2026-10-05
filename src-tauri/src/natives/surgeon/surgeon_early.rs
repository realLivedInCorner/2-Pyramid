    use super::*;
    use crate::scale_factor::determine_scale_factor;

    const ITEMS_LEGACY: &str = "assets/minecraft/textures/items";

    /// 旧 `copy_and_paste_region`：逐像素覆盖，**越界跳过**（`get_*_checked`）。
    fn copy_and_paste(src: &RgbaImage, dest: &mut RgbaImage, src_box: (u32, u32, u32, u32), dest_pt: (u32, u32)) {
        let (sx, sy, ex, ey) = src_box;
        let (dx, dy) = dest_pt;
        for y in 0..(ey.saturating_sub(sy)) {
            for x in 0..(ex.saturating_sub(sx)) {
                if let Some(p) = src.get_pixel_checked(sx + x, sy + y) {
                    if let Some(d) = dest.get_pixel_mut_checked(dx + x, dy + y) {
                        *d = *p;
                    }
                }
            }
        }
    }

    /// 旧 `converters/ui/slider.rs`。
    pub mod slider {
        use super::*;

        const WIDGETS: &str = "assets/minecraft/textures/gui/widgets.png";
        const SLIDER: &str = "assets/minecraft/textures/gui/slider.png";

        pub fn decl() -> TaskDecl {
            TaskDecl::new("fix_slider", Tier::Surgeon)
                .reads(ScopeSet::exact(WIDGETS))
                .writes(ScopeSet::exact(SLIDER))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(WIDGETS) {
                crate::log_info!("widgets.png not found, skip slider");
                return Ok(Outcome::default());
            }
            let img: RgbaImage = (*tx.image(WIDGETS)?).clone();
            let (width, height) = img.dimensions();
            let (s, _exact) = determine_scale_factor(width, height);

            // 旧实现：目标是**全透明**的同尺寸新图
            let mut slider_img = RgbaImage::new(width, height);
            let sc = |x: u32, y: u32| (x * s, y * s);
            let (x1, y1) = sc(0, 46);
            let (x2, y2) = sc(200, 66);
            copy_and_paste(&img, &mut slider_img, (x1, y1, x2, y2), (0, 0));
            let (x1, y1) = sc(0, 46);
            let (x2, y2) = sc(200, 106);
            let (dx, dy) = sc(0, 20);
            copy_and_paste(&img, &mut slider_img, (x1, y1, x2, y2), (dx, dy));

            // 旧实现**不创建目录**（`widgets.png` 存在即意味着 `gui/` 已在包里），
            // 因此这里也不能写目录条目——多写一条会让「声明范围」契约当场报错（实测）。
            tx.put_image(SLIDER, &slider_img)?;
            Ok(Outcome {
                changed: 1,
                notes: vec![format!("widgets.png -> slider.png (scale {s})")],
                ..Outcome::default()
            })
        }
    }

    /// 旧 `converters/ui/clock_compass.rs`。
    pub mod clock_compass {
        use super::*;

        pub fn decl() -> TaskDecl {
            TaskDecl::new("fix_clock_compass", Tier::Surgeon)
                .reads(ScopeSet::prefix(ITEMS_LEGACY))
                .writes(ScopeSet::prefix(ITEMS_LEGACY))
                .exclusive(true)
        }

        /// 旧 `split_image`：纵向均分抽帧 → 写 `{prefix}_{NN}.png` → 删原图与 `.mcmeta`。
        fn split(
            tx: &mut Tx<'_>,
            image_rel: &str,
            prefix: &str,
            retain_num: u32,
        ) -> Result<usize, AromError> {
            let img: RgbaImage = (*tx.image(image_rel)?).clone();
            let (img_width, img_height) = img.dimensions();
            if img_width == 0 {
                return Err(AromError::io(format!("invalid image width 0 for {image_rel}")));
            }
            let num_splits = img_height / img_width;
            let split_height = img_height / num_splits.max(1);

            let indices: Vec<u32> = if num_splits > retain_num {
                let step = num_splits as f64 / retain_num as f64;
                (0..retain_num)
                    .map(|i| ((i as f64) * step) as u32)
                    .map(|idx| idx.min(num_splits - 1))
                    .collect()
            } else {
                (0..num_splits).collect()
            };

            let mut written = 0usize;
            for (j, &i) in indices.iter().enumerate() {
                let y = i * split_height;
                let cropped = image::imageops::crop_imm(&img, 0, y, img_width, split_height).to_image();
                tx.put_image(&format!("{ITEMS_LEGACY}/{prefix}_{j:02}.png"), &cropped)?;
                written += 1;
            }

            // 旧实现随后**删原图**（含 `.mcmeta` 附属）
            tx.remove(image_rel)?;
            let mcmeta = format!("{image_rel}.mcmeta");
            if tx.exists(&mcmeta) {
                tx.remove(&mcmeta)?;
            }
            Ok(written)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for (name, prefix, retain) in [
                ("clock.png", "clock", 64u32),
                ("compass.png", "compass", 32u32),
            ] {
                let rel = format!("{ITEMS_LEGACY}/{name}");
                if !tx.exists(&rel) {
                    crate::log_info!("{name} not found, skip");
                    continue;
                }
                outcome.changed += split(tx, &rel, prefix, retain)?;
            }
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "fix_slider" => Some((slider::decl(), slider::run)),
            "fix_clock_compass" => Some((clock_compass::decl(), clock_compass::run)),
            _ => None,
        }
    }
