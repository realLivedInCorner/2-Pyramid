    use super::*;
    use image::imageops;

    /// `UImage` 目录：与旧实现同一个解析函数（含「找不到就在用户文档建默认目录」的副作用）。
    fn uimage_dir() -> Option<std::path::PathBuf> {
        match crate::image_utils::get_uimage_path() {
            Ok(p) => Some(p),
            // 旧实现这里只打日志、**不中止**；调用方各自决定「没有覆盖图时怎么办」。
            Err(e) => {
                crate::log_info!("UImage path not available: {}", e);
                None
            }
        }
    }

    /// 旧 `converters/ui/smithing_ui.rs`：由 `gui/container/anvil.png` 生成 `smithing.png`。
    pub mod smithing {
        use super::*;

        const CONTAINER: &str = "assets/minecraft/textures/gui/container";
        const ANVIL: &str = "assets/minecraft/textures/gui/container/anvil.png";

        /// 旧 `scale_factor`：只认 256/512/1024/2048，其余**整任务跳过**。
        fn scale_factor(size: u32) -> Option<u32> {
            match size {
                256 => Some(1),
                512 => Some(2),
                1024 => Some(4),
                2048 => Some(8),
                _ => None,
            }
        }

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_smithing_ui", Tier::Architect)
                .reads(ScopeSet::prefix(CONTAINER))
                .writes(ScopeSet::prefix(CONTAINER))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(ANVIL) {
                return Ok(Outcome::default());
            }
            let mut img: RgbaImage = (*tx.image(ANVIL)?).clone();
            let (width, height) = img.dimensions();
            // 非方形 → 跳过（旧实现 return Ok(())）
            if width != height {
                return Ok(Outcome::default());
            }
            let factor = match scale_factor(width) {
                Some(f) => f,
                None => return Ok(Outcome::default()),
            };

            // 取一个采样像素色填满覆盖框（旧实现：逐像素 put_pixel）
            let fill_color = *img.get_pixel(5 * factor, 4 * factor);
            for x in (10 * factor)..(169 * factor) {
                for y in (5 * factor)..(37 * factor) {
                    img.put_pixel(x, y, fill_color);
                }
            }

            // 覆盖图：**存在才**叠加；UImage 解析失败时只打日志（不中止）
            match uimage_dir() {
                Some(dir) => {
                    let overlay_path = dir
                        .join("smithing")
                        .join(format!("smithing_{}.png", width));
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
                        imageops::overlay(&mut img, &overlay, 0, 0);
                    }
                }
                None => crate::log_info!("UImage path not available, skip smithing overlay"),
            }

            // 透明框：静态坐标，**不乘 factor**（旧实现如此）
            let transparent = image::Rgba([0, 0, 0, 0]);
            for x in 0..110 {
                for y in 166..198 {
                    img.put_pixel(x, y, transparent);
                }
            }

            tx.mkdir(CONTAINER)?;
            tx.put_image(&format!("{CONTAINER}/smithing.png"), &img)?;
            Ok(Outcome {
                changed: 1,
                notes: vec!["anvil.png -> smithing.png".into()],
                ..Outcome::default()
            })
        }
    }

    /// 旧 `converters/textures/fish_bucket.rs`：`item/water_bucket.png` → 6 个鱼桶。
    pub mod fish_bucket {
        use super::*;

        const ITEMS: &str = "assets/minecraft/textures/item";
        const WATER: &str = "assets/minecraft/textures/item/water_bucket.png";

        /// 逐条照抄旧实现的顺序
        const FISH: [&str; 6] = [
            "axolotl",
            "cod",
            "pufferfish",
            "salmon",
            "tropical_fish",
            "tadpole",
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_fish_bucket", Tier::Architect)
                .reads(ScopeSet::prefix(ITEMS))
                .writes(ScopeSet::prefix(ITEMS))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(WATER) {
                return Ok(Outcome::default());
            }
            let base: RgbaImage = (*tx.image(WATER)?).clone();
            let (width, height) = base.dimensions();
            if width != height || width == 0 {
                return Ok(Outcome::default());
            }

            // 旧实现：`get_uimage_path().map(|p| p.join("water_bucket")).ok()`
            // ——解析失败时**每个**鱼桶都 continue（水桶拷贝仍然发生）
            let overlay_dir = uimage_dir().map(|p| p.join("water_bucket"));

            let mut outcome = Outcome::default();
            for fish in FISH {
                let output_path = format!("{ITEMS}/{fish}_bucket.png");
                // 旧实现是 fs::copy：逐字节复制已解码的源（同一张 RGBA 重新编码）
                tx.put_image(&output_path, &base)?;
                outcome.changed += 1;

                let dir = match &overlay_dir {
                    Some(d) => d,
                    None => continue,
                };
                let overlay_path = dir.join(format!("{}_bucket_{}.png", fish, width));
                if !overlay_path.exists() {
                    continue;
                }
                let mut bucket: RgbaImage = (*tx.image(&output_path)?).clone();
                let mut overlay = image::open(&overlay_path)
                    .map_err(|e| {
                        AromError::io(format!("failed to open {}: {}", overlay_path.display(), e))
                    })?
                    .to_rgba8();
                if overlay.dimensions() != bucket.dimensions() {
                    overlay = imageops::resize(
                        &overlay,
                        bucket.width(),
                        bucket.height(),
                        imageops::FilterType::Triangle,
                    );
                }
                imageops::overlay(&mut bucket, &overlay, 0, 0);
                tx.put_image(&output_path, &bucket)?;
            }
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    ///
    /// 两个任务都已派发。`generate_smithing_ui` 需要**放在旧批次之前**
    /// （计划里它早于 Surgeon 的 `fix_smithing2_villager2_ui`，后者会重新派生并覆盖
    /// `container/smithing.png`），这条由驱动里的 `EARLY_NATIVES` 名单落实，见 §9.52。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "generate_smithing_ui" => Some((smithing::decl(), smithing::run)),
            "generate_fish_bucket" => Some((fish_bucket::decl(), fish_bucket::run)),
            _ => None,
        }
    }
