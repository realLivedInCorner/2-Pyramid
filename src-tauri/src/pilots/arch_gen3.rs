    use super::*;
    use image::imageops;

    /// 与 `arch_gen2` 同一个解析函数（含「找不到就在用户文档建默认目录」的副作用）。
    fn uimage_dir() -> Option<std::path::PathBuf> {
        match crate::image_utils::get_uimage_path() {
            Ok(p) => Some(p),
            Err(e) => {
                crate::log_info!("UImage path not available: {}", e);
                None
            }
        }
    }

    fn overlay_pair(base: &RgbaImage, overlay: &RgbaImage) -> RgbaImage {
        let mut combined = base.clone();
        imageops::overlay(&mut combined, overlay, 0, 0);
        combined
    }

    /// 旧 `converters/textures/crossbow.rs`。
    pub mod crossbow {
        use super::*;

        const ITEM: &str = "assets/minecraft/textures/item";

        /// 旧实现的 (尺寸, 文件名) 映射；**顺序即语义**（按 bow 的宽度找基准图）
        const SIZE_TO_NAME: [(u32, &str); 5] = [
            (16, "crossbow_16.png"),
            (32, "crossbow_32.png"),
            (64, "crossbow_64.png"),
            (128, "crossbow_128.png"),
            (256, "crossbow_256.png"),
        ];

        /// 旧实现的拉弓配对表——注意 `bow_pulling_2` 出现两次，对应四个输出。
        const BOW_PULLING: [&str; 4] = [
            "bow_pulling_0.png",
            "bow_pulling_1.png",
            "bow_pulling_2.png",
            "bow_pulling_2.png",
        ];
        const CROSSBOW_OUT: [&str; 4] = [
            "crossbow_pulling_0.png",
            "crossbow_pulling_1.png",
            "crossbow_pulling_2.png",
            "crossbow_arrow.png",
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_crossbow", Tier::Architect)
                .reads(ScopeSet::prefix(ITEM))
                .writes(ScopeSet::prefix(ITEM))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            // 旧实现：**先解析 UImage**，失败即整任务返回（连 bow 分支都不看）
            let Some(dir) = uimage_dir() else {
                crate::log_info!("UImage path not available, skip crossbow generation");
                return Ok(Outcome::default());
            };
            let crossbow_dir = dir.join("crossbow");
            let base_of = |width: u32| -> Option<std::path::PathBuf> {
                SIZE_TO_NAME
                    .iter()
                    .find(|(size, _)| *size == width)
                    .map(|(_, name)| crossbow_dir.join(name))
            };

            let mut outcome = Outcome::default();

            let bow = format!("{ITEM}/bow.png");
            if tx.exists(&bow) {
                let bow_img: RgbaImage = (*tx.image(&bow)?).clone();
                if let Some(base_path) = base_of(bow_img.width()) {
                    if base_path.exists() {
                        let base_img = image::open(&base_path)
                            .map_err(|e| {
                                AromError::io(format!(
                                    "failed to open {}: {}",
                                    base_path.display(),
                                    e
                                ))
                            })?
                            .to_rgba8();
                        let standby = overlay_pair(&base_img, &bow_img);
                        tx.put_image(&format!("{ITEM}/crossbow_standby.png"), &standby)?;
                        outcome.changed += 1;
                    }
                }
            }

            let pulling0 = format!("{ITEM}/bow_pulling_0.png");
            if tx.exists(&pulling0) {
                let sample: RgbaImage = (*tx.image(&pulling0)?).clone();
                if let Some(base_path) = base_of(sample.width()) {
                    if base_path.exists() {
                        let base_img = image::open(&base_path)
                            .map_err(|e| {
                                AromError::io(format!(
                                    "failed to open {}: {}",
                                    base_path.display(),
                                    e
                                ))
                            })?
                            .to_rgba8();
                        for (bow_file, crossbow_file) in
                            BOW_PULLING.iter().zip(CROSSBOW_OUT.iter())
                        {
                            let bow_path = format!("{ITEM}/{bow_file}");
                            if !tx.exists(&bow_path) {
                                continue;
                            }
                            let bow_img: RgbaImage = (*tx.image(&bow_path)?).clone();
                            let output_path = format!("{ITEM}/{crossbow_file}");
                            tx.put_image(&output_path, &overlay_pair(&base_img, &bow_img))?;
                            outcome.changed += 1;

                            if *crossbow_file == "crossbow_arrow.png" {
                                // 旧实现：先把上一步写出的 crossbow_arrow **拷成**
                                // crossbow_firework，再看 firework 覆盖图是否存在
                                let firework_path = format!("{ITEM}/crossbow_firework.png");
                                let arrow_img: RgbaImage = (*tx.image(&output_path)?).clone();
                                let mut firework_img = arrow_img.clone();
                                tx.put_image(&firework_path, &firework_img)?;
                                let overlay_path = crossbow_dir.join(format!(
                                    "crossbow_firework_{}.png",
                                    sample.width()
                                ));
                                if overlay_path.exists() {
                                    let mut overlay_img = image::open(&overlay_path)
                                        .map_err(|e| {
                                            AromError::io(format!(
                                                "failed to open {}: {}",
                                                overlay_path.display(),
                                                e
                                            ))
                                        })?
                                        .to_rgba8();
                                    if overlay_img.dimensions() != firework_img.dimensions() {
                                        overlay_img = imageops::resize(
                                            &overlay_img,
                                            firework_img.width(),
                                            firework_img.height(),
                                            imageops::FilterType::Triangle,
                                        );
                                    }
                                    imageops::overlay(&mut firework_img, &overlay_img, 0, 0);
                                    tx.put_image(&firework_path, &firework_img)?;
                                }
                            }
                        }
                    }
                }
            }

            Ok(outcome)
        }
    }

    /// 旧 `converters/textures/tipped_arrows.rs`。
    pub mod tipped_arrows {
        use super::*;

        /// 源是**1.9 路径**（旧实现写死 `textures/items`），照抄
        const ITEMS_LEGACY: &str = "assets/minecraft/textures/items";

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_tipped_arrow_images", Tier::Architect)
                .reads(ScopeSet::prefix(ITEMS_LEGACY))
                .writes(ScopeSet::prefix(ITEMS_LEGACY))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let arrow = format!("{ITEMS_LEGACY}/arrow.png");
            if !tx.exists(&arrow) {
                crate::log_info!("arrow.png not found, skip tipped arrow generation");
                return Ok(Outcome::default());
            }
            let base: RgbaImage = (*tx.image(&arrow)?).clone();
            let size = base.width();

            let Some(dir) = uimage_dir() else {
                crate::log_info!("UImage path not available, skip tipped arrow generation");
                return Ok(Outcome::default());
            };
            let head_path = dir
                .join("tipped_arrow_head")
                .join(format!("tipped_arrow_head_{}.png", size));
            if !head_path.exists() {
                crate::log_info!("tipped arrow head not found: {}", head_path.display());
                return Ok(Outcome::default());
            }
            let head = image::open(&head_path)
                .map_err(|e| {
                    AromError::io(format!("failed to open {}: {}", head_path.display(), e))
                })?
                .to_rgba8();

            // 旧实现：按**较短者**停止（`zip` 语义），把头部不透明处的底图 alpha 清掉
            let mut base_out = base.clone();
            let limit = (
                base_out.width().min(head.width()),
                base_out.height().min(head.height()),
            );
            for y in 0..limit.1 {
                for x in 0..limit.0 {
                    if head.get_pixel(x, y)[3] > 0 {
                        base_out.get_pixel_mut(x, y)[3] = 0;
                    }
                }
            }
            tx.put_image(&format!("{ITEMS_LEGACY}/tipped_arrow_base.png"), &base_out)?;

            // 头部贴图是**拷贝**（旧实现 `fs::copy`），等价为原字节写入
            let head_bytes = std::fs::read(&head_path)
                .map_err(|e| AromError::io(format!("read {}: {e}", head_path.display())))?;
            tx.put(&format!("{ITEMS_LEGACY}/tipped_arrow_head.png"), head_bytes)?;

            Ok(Outcome {
                changed: 2,
                notes: vec!["tipped_arrow_base.png + tipped_arrow_head.png".into()],
                ..Outcome::default()
            })
        }
    }

    /// 旧 `converters/textures/snow_bucket.rs`。
    pub mod snow_bucket {
        use super::*;

        const ITEMS: &str = "assets/minecraft/textures/item";

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_snow_bucket", Tier::Architect)
                .reads(ScopeSet::prefix(ITEMS))
                .writes(ScopeSet::prefix(ITEMS))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let milk = format!("{ITEMS}/milk_bucket.png");
            if !tx.exists(&milk) {
                return Ok(Outcome::default());
            }
            let base: RgbaImage = (*tx.image(&milk)?).clone();
            let (width, height) = base.dimensions();
            if width != height || width == 0 {
                return Ok(Outcome::default());
            }

            let powder = format!("{ITEMS}/powder_snow_bucket.png");
            // 旧实现先 fs::copy，再（可选）叠加覆盖图
            let mut bucket = base.clone();
            tx.put_image(&powder, &bucket)?;

            match uimage_dir() {
                Some(dir) => {
                    let overlay_path = dir
                        .join("powder_snow_bucket")
                        .join(format!("powder_snow_bucket_{}.png", width));
                    if overlay_path.exists() {
                        let mut overlay = image::open(&overlay_path)
                            .map_err(|e| {
                                AromError::io(format!(
                                    "failed to open {}: {}",
                                    overlay_path.display(),
                                    e
                                ))
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
                        tx.put_image(&powder, &bucket)?;
                    }
                }
                None => crate::log_info!("UImage path not available, skip snow bucket overlay"),
            }

            Ok(Outcome {
                changed: 1,
                notes: vec!["milk_bucket.png -> powder_snow_bucket.png".into()],
                ..Outcome::default()
            })
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "generate_crossbow" => Some((crossbow::decl(), crossbow::run)),
            "generate_tipped_arrow_images" => {
                Some((tipped_arrows::decl(), tipped_arrows::run))
            }
            "generate_snow_bucket" => Some((snow_bucket::decl(), snow_bucket::run)),
            _ => None,
        }
    }
