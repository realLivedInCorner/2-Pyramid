    use super::*;
    use crate::color::utils::{
        adjust_copper_color, apply_netherite_transform, apply_spectral_arrow_transform,
    };

    const ITEM: &str = "assets/minecraft/textures/item";
    const BLOCK: &str = "assets/minecraft/textures/block";
    const ARMOR: &str = "assets/minecraft/textures/models/armor";

    /// 旧实现的 `fs::copy(src, dst)` + 后缀附属：源存在**才**拷贝 `.png.mcmeta`。
    fn write_with_sidecar(
        tx: &mut Tx<'_>,
        src: &str,
        dst: &str,
        img: &RgbaImage,
    ) -> Result<(), AromError> {
        tx.put_image(dst, img)?;
        if let Some(dir) = parent_of(dst) {
            tx.mkdir(dir)?;
        }
        let meta = format!("{src}.mcmeta");
        if tx.exists(&meta) {
            let bytes = tx.read(&meta)?.unwrap_or_default();
            tx.put(&format!("{dst}.mcmeta"), bytes)?;
        }
        Ok(())
    }

    fn parent_of(path: &str) -> Option<&str> {
        path.rfind('/').map(|i| &path[..i])
    }

    /// 取第一张存在的候选源（旧实现的 for-else 回退链），返回 `(源路径, 图像)`。
    fn first_available(tx: &Tx<'_>, candidates: &[String]) -> Result<Option<(String, RgbaImage)>, AromError> {
        for candidate in candidates {
            if tx.exists(candidate) {
                let img: RgbaImage = (*tx.image(candidate)?).clone();
                return Ok(Some((candidate.clone(), img)));
            }
        }
        Ok(None)
    }

    /// 旧 `converters/textures/copper.rs`（4 个任务）。
    pub mod copper {
        use super::*;

        /// 工具族的回退链（逐条照抄旧实现的顺序）
        const TOOL_MATERIALS: [&str; 4] = ["diamond", "gold", "stone", "netherite"];
        const TOOL_ITEMS: [&str; 10] = [
            "sword",
            "helmet",
            "chestplate",
            "leggings",
            "boots",
            "axe",
            "pickaxe",
            "shovel",
            "hoe",
            "horse_armor",
        ];
        const ARMOR_MATERIALS: [&str; 4] = ["diamond", "gold", "chainmail", "leather"];
        const ARMOR_FILES: [&str; 2] = ["layer_1", "layer_2"];

        pub fn ingot_decl() -> TaskDecl {
            TaskDecl::new("generate_copper_ingot", Tier::Architect)
                .reads(ScopeSet::prefix(ITEM))
                .writes(ScopeSet::prefix(ITEM))
                .exclusive(true)
        }

        pub fn ingot(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let src = format!("{ITEM}/iron_ingot.png");
            if !tx.exists(&src) {
                return Ok(Outcome::default());
            }
            let dst = format!("{ITEM}/copper_ingot.png");
            let mut img: RgbaImage = (*tx.image(&src)?).clone();
            adjust_copper_color(&mut img);
            write_with_sidecar(tx, &src, &dst, &img)?;
            Ok(Outcome {
                changed: 1,
                notes: vec![format!("iron_ingot.png -> copper_ingot.png")],
                ..Outcome::default()
            })
        }

        pub fn block_decl() -> TaskDecl {
            TaskDecl::new("generate_copper_block", Tier::Architect)
                .reads(ScopeSet::prefix(BLOCK))
                .writes(ScopeSet::prefix(BLOCK))
                .exclusive(true)
        }

        pub fn block(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let src = format!("{BLOCK}/iron_block.png");
            if !tx.exists(&src) {
                return Ok(Outcome::default());
            }
            let dst = format!("{BLOCK}/copper_block.png");
            let mut copper: RgbaImage = (*tx.image(&src)?).clone();
            adjust_copper_color(&mut copper);
            write_with_sidecar(tx, &src, &dst, &copper)?;

            // 三个阶段变体：每个都是「铜块像素再按固定配方混色」，逐条照抄
            let mix = |img: &RgbaImage, r: (f32, f32), g: (f32, f32), b: (f32, f32)| {
                let mut out = img.clone();
                for pixel in out.pixels_mut() {
                    if pixel[3] == 0 {
                        continue;
                    }
                    pixel[0] = (pixel[0] as f32 * r.0 + r.1).round().clamp(0.0, 255.0) as u8;
                    pixel[1] = (pixel[1] as f32 * g.0 + g.1).round().clamp(0.0, 255.0) as u8;
                    pixel[2] = (pixel[2] as f32 * b.0 + b.1).round().clamp(0.0, 255.0) as u8;
                }
                out
            };
            let exposed = mix(&copper, (0.8, 20.0), (0.7, 54.0), (0.6, 64.0));
            let weathered = mix(&copper, (0.6, 28.0), (0.5, 95.0), (0.4, 108.0));
            let mut oxidized = copper.clone();
            for pixel in oxidized.pixels_mut() {
                if pixel[3] == 0 {
                    continue;
                }
                pixel[0] = 50;
                pixel[1] = 210;
                pixel[2] = 210;
            }
            for (name, img) in [
                ("exposed_copper.png", &exposed),
                ("weathered_copper.png", &weathered),
                ("oxidized_copper.png", &oxidized),
            ] {
                tx.put_image(&format!("{BLOCK}/{name}"), img)?;
            }
            // 旧实现的附属：只有 `iron_block.png.mcmeta` 存在时才写这 4 个
            let meta = format!("{src}.mcmeta");
            if tx.exists(&meta) {
                let bytes = tx.read(&meta)?.unwrap_or_default();
                for name in [
                    "copper_block.png",
                    "exposed_copper.png",
                    "weathered_copper.png",
                    "oxidized_copper.png",
                ] {
                    tx.put(&format!("{BLOCK}/{name}.mcmeta"), bytes.clone())?;
                }
            }
            Ok(Outcome {
                changed: 4,
                notes: vec!["iron_block.png -> copper_block + 3 oxidation stages".into()],
                ..Outcome::default()
            })
        }

        pub fn tools_decl() -> TaskDecl {
            TaskDecl::new("generate_copper_tools", Tier::Architect)
                .reads(ScopeSet::prefix(ITEM))
                .writes(ScopeSet::prefix(ITEM))
                .exclusive(true)
        }

        pub fn tools(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for item in TOOL_ITEMS {
                let mut candidates = vec![format!("{ITEM}/iron_{item}.png")];
                candidates.extend(
                    TOOL_MATERIALS
                        .iter()
                        .map(|material| format!("{ITEM}/{material}_{item}.png")),
                );
                let Some((src, mut img)) = first_available(tx, &candidates)? else {
                    continue;
                };
                adjust_copper_color(&mut img);
                let dst = format!("{ITEM}/copper_{item}.png");
                write_with_sidecar(tx, &src, &dst, &img)?;
                outcome.changed += 1;
            }
            Ok(outcome)
        }

        pub fn armor_decl() -> TaskDecl {
            TaskDecl::new("generate_copper_armor_models", Tier::Architect)
                .reads(ScopeSet::prefix(ARMOR))
                .writes(ScopeSet::prefix(ARMOR))
                .exclusive(true)
        }

        pub fn armor(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for layer in ARMOR_FILES {
                let mut candidates = vec![format!("{ARMOR}/iron_{layer}.png")];
                candidates.extend(
                    ARMOR_MATERIALS
                        .iter()
                        .map(|material| format!("{ARMOR}/{material}_{layer}.png")),
                );
                let Some((_src, mut img)) = first_available(tx, &candidates)? else {
                    continue;
                };
                adjust_copper_color(&mut img);
                // 旧实现的 armor 分支**不拷贝 mcmeta**（与 tools 不同），照抄
                tx.put_image(&format!("{ARMOR}/copper_{layer}.png"), &img)?;
                outcome.changed += 1;
            }
            Ok(outcome)
        }
    }

    /// 旧 `converters/textures/netherite.rs`（4 个任务）。
    pub mod netherite {
        use super::*;

        const TOOL_ITEMS: [&str; 9] = [
            "sword",
            "helmet",
            "chestplate",
            "leggings",
            "boots",
            "axe",
            "pickaxe",
            "shovel",
            "hoe",
        ];
        const ARMOR_FILES: [&str; 2] = ["layer_1", "layer_2"];

        pub fn block_decl() -> TaskDecl {
            TaskDecl::new("generate_netherite_block", Tier::Architect)
                .reads(ScopeSet::prefix(BLOCK))
                .writes(ScopeSet::prefix(BLOCK))
                .exclusive(true)
        }

        pub fn block(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let src = format!("{BLOCK}/diamond_block.png");
            if !tx.exists(&src) {
                return Ok(Outcome::default());
            }
            let dst = format!("{BLOCK}/netherite_block.png");
            let mut img: RgbaImage = (*tx.image(&src)?).clone();
            apply_netherite_transform(&mut img);
            write_with_sidecar(tx, &src, &dst, &img)?;
            Ok(Outcome {
                changed: 1,
                notes: vec!["diamond_block.png -> netherite_block.png".into()],
                ..Outcome::default()
            })
        }

        pub fn ingot_decl() -> TaskDecl {
            TaskDecl::new("generate_netherite_ingot", Tier::Architect)
                .reads(ScopeSet::prefix(ITEM))
                .writes(ScopeSet::prefix(ITEM))
                .exclusive(true)
        }

        pub fn ingot(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let src = format!("{ITEM}/gold_ingot.png");
            if !tx.exists(&src) {
                return Ok(Outcome::default());
            }
            let dst = format!("{ITEM}/netherite_ingot.png");
            let mut img: RgbaImage = (*tx.image(&src)?).clone();
            apply_netherite_transform(&mut img);
            write_with_sidecar(tx, &src, &dst, &img)?;
            Ok(Outcome {
                changed: 1,
                notes: vec!["gold_ingot.png -> netherite_ingot.png".into()],
                ..Outcome::default()
            })
        }

        pub fn tools_decl() -> TaskDecl {
            TaskDecl::new("generate_netherite_tools", Tier::Architect)
                .reads(ScopeSet::prefix(ITEM))
                .writes(ScopeSet::prefix(ITEM))
                .exclusive(true)
        }

        pub fn tools(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for item in TOOL_ITEMS {
                let src = format!("{ITEM}/diamond_{item}.png");
                if !tx.exists(&src) {
                    continue;
                }
                let mut img: RgbaImage = (*tx.image(&src)?).clone();
                apply_netherite_transform(&mut img);
                let dst = format!("{ITEM}/netherite_{item}.png");
                write_with_sidecar(tx, &src, &dst, &img)?;
                outcome.changed += 1;
            }
            // 同任务里附带 `arrow.png` → `spectral_arrow.png`（旧实现如此，不另立任务）
            let arrow = format!("{ITEM}/arrow.png");
            if tx.exists(&arrow) {
                let mut img: RgbaImage = (*tx.image(&arrow)?).clone();
                apply_spectral_arrow_transform(&mut img);
                let dst = format!("{ITEM}/spectral_arrow.png");
                write_with_sidecar(tx, &arrow, &dst, &img)?;
                outcome.changed += 1;
                outcome.notes.push("arrow.png -> spectral_arrow.png".into());
            }
            Ok(outcome)
        }

        pub fn armor_decl() -> TaskDecl {
            TaskDecl::new("generate_netherite_armor_models", Tier::Architect)
                .reads(ScopeSet::prefix(ARMOR))
                .writes(ScopeSet::prefix(ARMOR))
                .exclusive(true)
        }

        pub fn armor(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for layer in ARMOR_FILES {
                let src = format!("{ARMOR}/diamond_{layer}.png");
                if !tx.exists(&src) {
                    continue;
                }
                let mut img: RgbaImage = (*tx.image(&src)?).clone();
                apply_netherite_transform(&mut img);
                // 旧实现这一支同样**不拷贝 mcmeta**
                tx.put_image(&format!("{ARMOR}/netherite_{layer}.png"), &img)?;
                outcome.changed += 1;
            }
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "generate_copper_ingot" => Some((copper::ingot_decl(), copper::ingot)),
            "generate_copper_block" => Some((copper::block_decl(), copper::block)),
            "generate_copper_tools" => Some((copper::tools_decl(), copper::tools)),
            "generate_copper_armor_models" => Some((copper::armor_decl(), copper::armor)),
            "generate_netherite_block" => Some((netherite::block_decl(), netherite::block)),
            "generate_netherite_ingot" => Some((netherite::ingot_decl(), netherite::ingot)),
            "generate_netherite_tools" => Some((netherite::tools_decl(), netherite::tools)),
            "generate_netherite_armor_models" => {
                Some((netherite::armor_decl(), netherite::armor))
            }
            _ => None,
        }
    }
