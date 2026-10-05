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

    /// 旧 `converters/ui/horse.rs`。
    pub mod horse {
        use super::*;

        const HORSE: &str = "assets/minecraft/textures/gui/container/horse.png";

        /// 旧 `scale_factor`：**宽高都必须等于**标准尺寸之一，否则整任务跳过。
        fn scale_factor(width: u32, height: u32) -> Option<u32> {
            match (width, height) {
                (256, 256) => Some(1),
                (512, 512) => Some(2),
                (1024, 1024) => Some(4),
                (2048, 2048) => Some(8),
                _ => None,
            }
        }

        /// 旧实现用 `imageops::overlay` 贴回（**alpha 混合**，不是原始覆盖）。
        fn overlay_at(img: &mut RgbaImage, region: &RgbaImage, x: u32, y: u32) {
            imageops::overlay(img, region, x as i64, y as i64);
        }

        pub fn decl() -> TaskDecl {
            TaskDecl::new("fix_horse_ui", Tier::Surgeon)
                .reads(ScopeSet::exact(HORSE))
                .writes(ScopeSet::exact(HORSE))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(HORSE) {
                crate::log_info!("horse.png not found, skip");
                return Ok(Outcome::default());
            }
            let mut img: RgbaImage = (*tx.image(HORSE)?).clone();
            let (width, height) = img.dimensions();
            let Some(s) = scale_factor(width, height) else {
                crate::log_info!("unsupported horse.png size, skip");
                return Ok(Outcome::default());
            };

            // 步骤 1：把 (7,17)-(25,35) 贴到 (18,220)
            let region =
                imageops::crop_imm(&img, 7 * s, 17 * s, 18 * s, 18 * s).to_image();
            overlay_at(&mut img, &region, 18 * s, 220 * s);

            // 步骤 2：用 (7,16) 的颜色填回刚搬走的区域
            let fill_color = *img.get_pixel(7 * s, 16 * s);
            for y in (17 * s)..(35 * s) {
                for x in (7 * s)..(25 * s) {
                    img.put_pixel(x, y, fill_color);
                }
            }

            // 步骤 3：把 (36,202) 那块 18×18 拷到 (36,220)
            let copy = imageops::crop_imm(&img, 36 * s, 202 * s, 18 * s, 18 * s).to_image();
            overlay_at(&mut img, &copy, 36 * s, 220 * s);

            // 步骤 4：可选覆盖图
            if let Some(dir) = uimage_dir() {
                let overlay_path = dir.join("horse").join(format!("horse_{}.png", width));
                if overlay_path.exists() {
                    let overlay_img = image::open(&overlay_path)
                        .map_err(|e| {
                            AromError::io(format!(
                                "failed to open overlay {}: {}",
                                overlay_path.display(),
                                e
                            ))
                        })?
                        .to_rgba8();
                    imageops::overlay(&mut img, &overlay_img, 0, 0);
                }
            }

            // 旧实现**无论覆盖图是否存在都会重写** horse.png
            tx.put_image(HORSE, &img)?;
            Ok(Outcome {
                changed: 1,
                notes: vec![format!("horse.png fixed in place (scale {s})")],
                ..Outcome::default()
            })
        }
    }

    /// 旧 `converters/ui/sign.rs`。
    pub mod sign {
        use super::*;
        use crate::color::hue::adjust_hue_brightness;

        const ITEM: &str = "assets/minecraft/textures/item";
        const OAK: &str = "assets/minecraft/textures/item/oak_sign.png";
        const SPRUCE: &str = "assets/minecraft/textures/item/spruce_sign.png";

        /// (文件名, 色相, 明度, 饱和) —— 逐条照抄（注意里面**又有一个 `oak_sign.png`**）
        const VARIANTS: [(&str, f32, f32, f32); 11] = [
            ("oak_sign.png", 0.0, 15.0, 0.0),
            ("birch_sign.png", 0.0, 40.0, 0.0),
            ("acacia_sign.png", -23.0, 10.0, 0.0),
            ("dark_oak_sign.png", 0.0, -15.0, 0.0),
            ("jungle_sign.png", -10.0, 4.6, 0.0),
            ("crimson_sign.png", -59.0, -30.0, 0.0),
            ("warped_sign.png", 130.0, -33.0, 0.0),
            ("mangrove_sign.png", -59.0, -10.0, 0.0),
            ("pale_oak_sign.png", 0.0, 30.0, -100.0),
            ("bamboo_sign.png", 25.0, 20.0, 0.0),
            ("cherry_sign.png", -45.0, 30.0, -18.0),
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("fix_sign", Tier::Surgeon)
                .reads(ScopeSet::prefix(ITEM))
                .writes(ScopeSet::prefix(ITEM))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(OAK) {
                crate::log_info!("未找到 oak_sign.png，跳过告示牌处理");
                return Ok(Outcome::default());
            }
            // 旧实现：先删已有的 spruce，再把 oak **改名**过去
            if tx.exists(SPRUCE) {
                tx.remove(SPRUCE)?;
            }
            let base_bytes = tx.read(OAK)?.unwrap_or_default();
            tx.put(SPRUCE, base_bytes)?;
            tx.remove(OAK)?;
            let base: RgbaImage = (*tx.image(SPRUCE)?).clone();

            let mut outcome = Outcome::default();
            for (filename, hue, bright, sat) in VARIANTS {
                let adjusted = adjust_hue_brightness(base.clone(), hue, bright, sat);
                tx.put_image(&format!("{ITEM}/{filename}"), &adjusted)?;
                outcome.changed += 1;
            }
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "fix_horse_ui" => Some((horse::decl(), horse::run)),
            "fix_sign" => Some((sign::decl(), sign::run)),
            _ => None,
        }
    }
