    use super::*;

    const GUI: &str = "assets/minecraft/textures/gui/container";
    const FURNACE: &str = "assets/minecraft/textures/gui/container/furnace.png";
    const BLAST: &str = "assets/minecraft/textures/gui/container/blast_furnace.png";
    const SMOKER: &str = "assets/minecraft/textures/gui/container/smoker.png";

    pub mod furnace {
        use super::*;

        pub fn decl() -> TaskDecl {
            // 阶段与活注册表一致：Architect（生成类任务）
            TaskDecl::new("generate_furnace", Tier::Architect)
                .writes(ScopeSet::prefix(GUI))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(FURNACE) {
                return Ok(Outcome::default());
            }
            let bytes = tx.read(FURNACE)?.unwrap_or_default();
            tx.put(BLAST, bytes.clone())?;
            tx.put(SMOKER, bytes)?;
            Ok(Outcome {
                changed: 2,
                notes: vec!["furnace.png -> blast_furnace.png / smoker.png".into()],
                ..Outcome::default()
            })
        }
    }

    /// `generate_boat`：由 `items/boat.png` 生成 5 个色相/明度变体（复用
    /// `converters::color::hue::adjust_hue_brightness`，与旧实现同一个函数），
    /// 最后把 `boat.png` **改名**为 `spruce_boat.png`（源消失——这一步很容易漏）。
    pub mod boat {
        use super::*;
        use crate::color::hue::adjust_hue_brightness;

        const ITEMS: &str = "assets/minecraft/textures/items";
        const BOAT: &str = "assets/minecraft/textures/items/boat.png";
        const SPRUCE: &str = "assets/minecraft/textures/items/spruce_boat.png";

        /// (输出名, 色相, 明度, 饱和度) —— 逐条照抄旧实现
        const VARIANTS: [(&str, f32, f32, f32); 5] = [
            ("oak_boat.png", 0.0, 15.0, 0.0),
            ("birch_boat.png", 0.0, 40.0, 0.0),
            ("acacia_boat.png", -23.0, 10.0, 0.0),
            ("dark_oak_boat.png", 0.0, -15.0, 0.0),
            ("jungle_boat.png", -10.0, 4.6, 0.0),
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_boat", Tier::Architect)
                .reads(ScopeSet::prefix(ITEMS))
                .writes(ScopeSet::prefix(ITEMS))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(BOAT) {
                return Ok(Outcome::default());
            }
            let base: RgbaImage = (*tx.image(BOAT)?).clone();
            let mut outcome = Outcome::default();
            for (name, hue, brightness, saturation) in VARIANTS {
                let variant = adjust_hue_brightness(base.clone(), hue, brightness, saturation);
                tx.put_image(&format!("{ITEMS}/{name}"), &variant)?;
                outcome.changed += 1;
            }
            // 旧实现：先删已存在的 spruce，再把 boat.png **改名**过去
            if tx.exists(SPRUCE) {
                tx.remove(SPRUCE)?;
            }
            let bytes = tx.read(BOAT)?.unwrap_or_default();
            tx.put(SPRUCE, bytes)?;
            tx.remove(BOAT)?;
            outcome.notes.push("boat.png -> spruce_boat.png".into());
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "generate_furnace" => Some((furnace::decl(), furnace::run)),
            "generate_boat" => Some((boat::decl(), boat::run)),
            _ => None,
        }
    }
