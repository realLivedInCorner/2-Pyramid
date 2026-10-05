    use super::*;
    use crate::color::hue::{adjust_hue_brightness, force_hue_saturation};

    const BLOCK: &str = "assets/minecraft/textures/block";

    /// 旧 `process_block_image`：`blocks_path` 下的 `source` → `target`。
    fn recolor_block(
        tx: &mut Tx<'_>,
        source: &str,
        target: &str,
        hue: f32,
        brightness: f32,
        saturation: f32,
    ) -> Result<bool, AromError> {
        recolor_rel(
            tx,
            &format!("{BLOCK}/{source}"),
            &format!("{BLOCK}/{target}"),
            hue,
            brightness,
            saturation,
        )
    }

    /// 旧 `recolor_rel`：任意路径（相对包根）的 recolor，源存在才做。
    fn recolor_rel(
        tx: &mut Tx<'_>,
        source: &str,
        target: &str,
        hue: f32,
        brightness: f32,
        saturation: f32,
    ) -> Result<bool, AromError> {
        if !tx.exists(source) {
            return Ok(false);
        }
        let img: RgbaImage = (*tx.image(source)?).clone();
        let adjusted = adjust_hue_brightness(img, hue, brightness, saturation);
        tx.put_image(target, &adjusted)?;
        let meta = format!("{source}.mcmeta");
        if tx.exists(&meta) {
            let bytes = tx.read(&meta)?.unwrap_or_default();
            tx.put(&format!("{target}.mcmeta"), bytes)?;
        }
        Ok(true)
    }

    /// 旧 `leaves` 闭包：钉色相 + 固定饱和区间（**不拷贝源**，源缺失即跳过）。
    fn leaves(
        tx: &mut Tx<'_>,
        source: &str,
        target: &str,
        hue: f32,
        sat: f32,
    ) -> Result<bool, AromError> {
        let src = format!("{BLOCK}/{source}");
        if !tx.exists(&src) {
            return Ok(false);
        }
        let img: RgbaImage = (*tx.image(&src)?).clone();
        let out = force_hue_saturation(img, hue, 6.0, sat, 0.22, 0.88);
        let dst = format!("{BLOCK}/{target}");
        tx.put_image(&dst, &out)?;
        let meta = format!("{src}.mcmeta");
        if tx.exists(&meta) {
            let bytes = tx.read(&meta)?.unwrap_or_default();
            tx.put(&format!("{dst}.mcmeta"), bytes)?;
        }
        Ok(true)
    }

    /// 旧 `generate_redwood_cherry_bamboo_planks`：1.19 mangrove / 1.20 cherry+bamboo。
    pub mod redwood_cherry_bamboo {
        use super::*;

        /// (源, 目标, 色相, 明度, 饱和) —— 逐条照抄旧实现的 10 次调用
        const JOBS: [(&str, &str, f32, f32, f32); 10] = [
            ("oak_planks.png", "mangrove_planks.png", -59.0, -15.0, 0.0),
            ("oak_planks.png", "cherry_planks.png", -45.0, 45.0, -18.0),
            ("oak_planks.png", "bamboo_planks.png", 25.0, 20.0, 0.0),
            ("oak_log.png", "mangrove_log.png", -59.0, -15.0, 0.0),
            ("oak_log_top.png", "mangrove_log_top.png", -59.0, -15.0, 0.0),
            ("oak_log.png", "cherry_log.png", -45.0, 45.0, -18.0),
            ("oak_log_top.png", "cherry_log_top.png", -45.0, 45.0, -18.0),
            ("oak_log.png", "bamboo_block.png", 25.0, 20.0, 0.0),
            ("oak_log_top.png", "bamboo_block_top.png", 25.0, 20.0, 0.0),
            ("oak_planks.png", "bamboo_mosaic.png", 25.0, 15.0, 0.0),
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_redwood_cherry_bamboo_planks", Tier::Architect)
                .reads(ScopeSet::prefix(BLOCK))
                .writes(ScopeSet::prefix(BLOCK))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for (source, target, hue, brightness, saturation) in JOBS {
                if recolor_block(tx, source, target, hue, brightness, saturation)? {
                    outcome.changed += 1;
                }
            }
            Ok(outcome)
        }
    }

    /// 旧 `generate_pale_planks`：1.21.4 苍白橡木（3 个文件）。
    pub mod pale {
        use super::*;

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_pale_planks", Tier::Architect)
                .reads(ScopeSet::prefix(BLOCK))
                .writes(ScopeSet::prefix(BLOCK))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for (source, target) in [
                ("oak_planks.png", "pale_oak_planks.png"),
                ("oak_log.png", "pale_oak_log.png"),
                ("oak_log_top.png", "pale_oak_log_top.png"),
            ] {
                if recolor_block(tx, source, target, 0.0, 30.0, -100.0)? {
                    outcome.changed += 1;
                }
            }
            Ok(outcome)
        }
    }

    /// 旧 `generate_poplar_planks`：26.3 杨树全套（原木/木板/家具/树叶）。
    pub mod poplar {
        use super::*;

        /// 木板与家具用的 jungle 参数 / 树皮用的 oak 参数（顺序即语义）
        const JUNGLE: (f32, f32, f32) = (-1.0, -4.0, -35.0);
        const OAK: (f32, f32, f32) = (-7.0, -5.0, -36.0);

        /// (jungle 源, oak 回退源, 目标) —— 相对包根，逐条照抄
        const PREFER_JUNGLE_THEN_OAK: [(&str, &str, &str); 9] = [
            (
                "assets/minecraft/textures/item/jungle_sign.png",
                "assets/minecraft/textures/item/oak_sign.png",
                "assets/minecraft/textures/item/poplar_sign.png",
            ),
            (
                "assets/minecraft/textures/item/jungle_hanging_sign.png",
                "assets/minecraft/textures/item/oak_hanging_sign.png",
                "assets/minecraft/textures/item/poplar_hanging_sign.png",
            ),
            (
                "assets/minecraft/textures/item/jungle_door.png",
                "assets/minecraft/textures/item/oak_door.png",
                "assets/minecraft/textures/item/poplar_door.png",
            ),
            (
                "assets/minecraft/textures/item/jungle_boat.png",
                "assets/minecraft/textures/item/oak_boat.png",
                "assets/minecraft/textures/item/poplar_boat.png",
            ),
            (
                "assets/minecraft/textures/item/jungle_chest_boat.png",
                "assets/minecraft/textures/item/oak_chest_boat.png",
                "assets/minecraft/textures/item/poplar_chest_boat.png",
            ),
            (
                "assets/minecraft/textures/block/jungle_sign.png",
                "assets/minecraft/textures/block/oak_sign.png",
                "assets/minecraft/textures/block/poplar_sign.png",
            ),
            (
                "assets/minecraft/textures/block/jungle_hanging_sign.png",
                "assets/minecraft/textures/block/oak_hanging_sign.png",
                "assets/minecraft/textures/block/poplar_hanging_sign.png",
            ),
            (
                "assets/minecraft/textures/entity/boat/jungle.png",
                "assets/minecraft/textures/entity/boat/oak.png",
                "assets/minecraft/textures/entity/boat/poplar.png",
            ),
            (
                "assets/minecraft/textures/entity/chest_boat/jungle.png",
                "assets/minecraft/textures/entity/chest_boat/oak.png",
                "assets/minecraft/textures/entity/chest_boat/poplar.png",
            ),
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_poplar_planks", Tier::Architect)
                .reads(ScopeSet::prefix("assets/minecraft/textures"))
                .writes(ScopeSet::prefix("assets/minecraft/textures"))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();

            // ── 原木 / 木板 ──
            for (source, target, hue, brightness, saturation) in [
                ("jungle_planks.png", "poplar_planks.png", -1.0, -4.0, -35.0),
                ("jungle_log_top.png", "poplar_log_top.png", -3.0, -5.0, -33.0),
                ("oak_log.png", "poplar_log.png", -7.0, -12.0, -8.0),
                (
                    "stripped_oak_log.png",
                    "stripped_poplar_log.png",
                    -7.0,
                    -5.0,
                    -36.0,
                ),
                (
                    "stripped_oak_log_top.png",
                    "stripped_poplar_log_top.png",
                    -10.0,
                    -3.0,
                    -37.0,
                ),
            ] {
                if recolor_block(tx, source, target, hue, brightness, saturation)? {
                    outcome.changed += 1;
                }
            }

            // ── 家具：优先 jungle，缺失回退 oak（回退用 oak 参数）──
            for (src, dst) in [
                ("jungle_door_top.png", "poplar_door_top.png"),
                ("jungle_door_bottom.png", "poplar_door_bottom.png"),
                ("jungle_trapdoor.png", "poplar_trapdoor.png"),
                ("jungle_shelf.png", "poplar_shelf.png"),
                ("jungle_sapling.png", "poplar_sapling.png"),
            ] {
                if tx.exists(&format!("{BLOCK}/{src}")) {
                    let (h, b, s) = JUNGLE;
                    if recolor_block(tx, src, dst, h, b, s)? {
                        outcome.changed += 1;
                    }
                } else {
                    let oak = src.replace("jungle_", "oak_");
                    let (h, b, s) = OAK;
                    if recolor_block(tx, &oak, dst, h, b, s)? {
                        outcome.changed += 1;
                    }
                }
            }

            // ── 物品图标 / 实体船：优先 jungle，缺失再 oak ──
            for (jungle, oak, target) in PREFER_JUNGLE_THEN_OAK {
                let (h, b, s) = JUNGLE;
                if tx.exists(jungle) {
                    if recolor_rel(tx, jungle, target, h, b, s)? {
                        outcome.changed += 1;
                    }
                } else {
                    let (h, b, s) = OAK;
                    if recolor_rel(tx, oak, target, h, b, s)? {
                        outcome.changed += 1;
                    }
                }
            }

            // ── 树叶：钉色相（原版参考 红 ~8° / 橙 ~28° / 黄 ~45°）──
            for (target, hue, sat) in [
                ("red_poplar_leaves.png", 8.0, 0.72),
                ("orange_poplar_leaves.png", 28.0, 0.80),
                ("yellow_poplar_leaves.png", 45.0, 0.78),
            ] {
                if leaves(tx, "oak_leaves.png", target, hue, sat)? {
                    outcome.changed += 1;
                }
            }

            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "generate_redwood_cherry_bamboo_planks" => Some((
                redwood_cherry_bamboo::decl(),
                redwood_cherry_bamboo::run,
            )),
            "generate_pale_planks" => Some((pale::decl(), pale::run)),
            "generate_poplar_planks" => Some((poplar::decl(), poplar::run)),
            _ => None,
        }
    }
