    use super::*;
    use crate::color::hue::adjust_hue_brightness;

    /// 旧 `converters/ui/horse_v2.rs`。
    pub mod horse_v2 {
        use super::*;

        const HORSE: &str = "assets/minecraft/textures/gui/sprites/container/horse";
        const SLOT: &str = "assets/minecraft/textures/gui/sprites/container/slot";

        /// (源, 目标) —— 逐条照抄
        const MAPPINGS: [(&str, &str); 3] = [
            ("armor_slot.png", "horse_armor.png"),
            ("llama_armor_slot.png", "llama_armor.png"),
            ("saddle_slot.png", "saddle.png"),
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("fix2_horse_ui", Tier::Surgeon)
                .reads(ScopeSet::prefix(HORSE))
                .writes(ScopeSet::prefix(SLOT))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            // 旧实现：源目录不存在即整任务跳过
            if !tx.has_prefix(HORSE)? {
                crate::log_info!("horse sprites dir not found, skip fix2_horse_ui");
                return Ok(Outcome::default());
            }
            let mut outcome = Outcome::default();
            for (src_name, dest_name) in MAPPINGS {
                let src = format!("{HORSE}/{src_name}");
                if !tx.exists(&src) {
                    continue;
                }
                let bytes = tx.read(&src)?.unwrap_or_default();
                tx.put(&format!("{SLOT}/{dest_name}"), bytes)?;
                outcome.changed += 1;
            }
            Ok(outcome)
        }
    }

    /// 旧 `converters/ui/sign_entities.rs`。
    pub mod sign_entities {
        use super::*;

        const ENTITY: &str = "assets/minecraft/textures/entity";
        const SIGN: &str = "assets/minecraft/textures/entity/sign.png";
        const SIGNS: &str = "assets/minecraft/textures/entity/signs";

        /// (目标名, 色相, 明度, 饱和) —— 逐条照抄（含 `pale_oak` 的 -100 饱和）
        const VARIANTS: [(&str, f32, f32, f32); 11] = [
            ("oak.png", 0.0, 15.0, 0.0),
            ("birch.png", 0.0, 40.0, 0.0),
            ("acacia.png", -23.0, 10.0, 0.0),
            ("dark_oak.png", 0.0, -15.0, 0.0),
            ("jungle.png", -10.0, 4.6, 0.0),
            ("crimson.png", -59.0, -30.0, 0.0),
            ("warped.png", 130.0, -33.0, 0.0),
            ("mangrove.png", -59.0, -10.0, 0.0),
            ("pale_oak.png", 0.0, 30.0, -100.0),
            ("bamboo.png", 25.0, 20.0, 0.0),
            ("cherry.png", -45.0, 30.0, -18.0),
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("fix_sign_entities", Tier::Surgeon)
                .reads(ScopeSet::prefix(ENTITY))
                .writes(ScopeSet::prefix(ENTITY))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(SIGN) {
                crate::log_info!("sign.png not found in entity/, skip fix_sign_entities");
                return Ok(Outcome::default());
            }
            let base: RgbaImage = (*tx.image(SIGN)?).clone();

            let mut outcome = Outcome::default();
            for (filename, hue, bright, sat) in VARIANTS {
                let adjusted = adjust_hue_brightness(base.clone(), hue, bright, sat);
                tx.put_image(&format!("{SIGNS}/{filename}"), &adjusted)?;
                outcome.changed += 1;
            }

            // 旧实现：原图**改名**为 spruce（云杉 = 原图本身）；目标已存在时改为直接删源
            let spruce = format!("{SIGNS}/spruce.png");
            let bytes = tx.read(SIGN)?.unwrap_or_default();
            if tx.exists(&spruce) {
                tx.remove(SIGN)?;
            } else {
                tx.put(&spruce, bytes)?;
                tx.remove(SIGN)?;
            }
            outcome.notes.push("sign.png -> signs/spruce.png".into());
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "fix2_horse_ui" => Some((horse_v2::decl(), horse_v2::run)),
            "fix_sign_entities" => Some((sign_entities::decl(), sign_entities::run)),
            _ => None,
        }
    }
