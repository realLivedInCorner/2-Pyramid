    use super::*;
    use crate::image_utils::paste_region;

    const ITEMS: &str = "assets/minecraft/textures/items";
    const PARTICLE: &str = "assets/minecraft/textures/particle";
    const ENTITY: &str = "assets/minecraft/textures/entity";

    /// 旧 `merge_images`：按给定顺序纵向拼接；成功拼进去的源文件登记延迟删除。
    fn merge_images(
        tx: &mut Tx<'_>,
        sources: &[String],
        output: &str,
    ) -> Result<bool, AromError> {
        let mut images: Vec<RgbaImage> = Vec::new();
        let mut merged_sources: Vec<String> = Vec::new();
        for path in sources {
            if !tx.exists(path) {
                continue;
            }
            images.push((*tx.image(path)?).clone());
            merged_sources.push(path.clone());
        }
        if images.is_empty() {
            return Ok(false);
        }
        let max_width = images.iter().map(|i| i.width()).max().unwrap_or(0);
        let total_height: u32 = images.iter().map(|i| i.height()).sum();
        let mut merged = RgbaImage::new(max_width, total_height);
        let mut y_offset = 0u32;
        for img in &images {
            paste_region(&mut merged, img, 0, y_offset).map_err(AromError::internal)?;
            y_offset += img.height();
        }
        tx.put_image(output, &merged)?;
        // 源文件**不在这里删**：旧实现是 `defer_remove_file`（清理点统一执行），
        // 由调用方登记 `deferred_removals`。立即删会让更晚的任务看不到它们（§9.23）。
        let _ = &merged_sources;
        Ok(true)
    }

    pub mod clock_compass {
        use super::*;

        const MCMETA: &[u8] = br#"{"animation":{}}"#;

        pub fn decl() -> TaskDecl {
            TaskDecl::new("reverse_fix_clock_compass", Tier::Surgeon)
                .reads(ScopeSet::prefix(ITEMS))
                .writes(ScopeSet::prefix(ITEMS))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.has_prefix(ITEMS)? {
                return Ok(Outcome::default());
            }
            let mut outcome = Outcome::default();
            for (prefix, count, output) in [
                ("compass", 32u32, "compass.png"),
                ("clock", 64u32, "clock.png"),
            ] {
                let sources: Vec<String> = (0..count)
                    .map(|i| format!("{ITEMS}/{prefix}_{i:02}.png"))
                    .collect();
                if merge_images(tx, &sources, &format!("{ITEMS}/{output}"))? {
                    tx.put(&format!("{ITEMS}/{output}.mcmeta"), MCMETA.to_vec())?;
                    for path in &sources {
                        outcome
                            .deferred_removals
                            .extend(defer_remove_if_present(tx, path)?.deferred_removals);
                    }
                    outcome.changed += 1;
                }
            }
            Ok(outcome)
        }
    }

    pub mod particles {
        use super::*;

        /// 旧实现的「文件名 → (行, 列)」表（顺序固定；旧实现用 HashMap，取 split_size 时
        /// 依赖迭代顺序，但所有瓦片尺寸相同，因此结果一致）。
        fn positions() -> Vec<(String, (u32, u32))> {
            let mut out: Vec<(String, (u32, u32))> = Vec::new();
            for c in 0..8u32 {
                out.push((format!("generic_{c}.png"), (0, c)));
            }
            for c in 0..4u32 {
                out.push((format!("splash_{c}.png"), (1, c + 3)));
            }
            out.push(("bubble.png".into(), (2, 0)));
            out.push(("fishing_hook.png".into(), (2, 1)));
            out.push(("flame.png".into(), (3, 0)));
            out.push(("lava.png".into(), (3, 1)));
            for (i, n) in ["note.png", "critical_hit.png", "enchanted_hit.png"]
                .iter()
                .enumerate()
            {
                out.push(((*n).into(), (4, i as u32)));
            }
            for (i, n) in ["heart.png", "angry.png", "glint.png"].iter().enumerate() {
                out.push(((*n).into(), (5, i as u32)));
            }
            for (i, n) in ["drip_hang.png", "drip_fall.png", "drip_land.png"]
                .iter()
                .enumerate()
            {
                out.push(((*n).into(), (7, i as u32)));
            }
            for c in 0..8u32 {
                out.push((format!("effect_{c}.png"), (8, c)));
            }
            for c in 0..8u32 {
                out.push((format!("spell_{c}.png"), (9, c)));
            }
            for c in 0..8u32 {
                out.push((format!("spark_{c}.png"), (10, c)));
            }
            out
        }

        pub fn decl() -> TaskDecl {
            TaskDecl::new("reverse_fix_particles", Tier::Surgeon)
                .reads(ScopeSet::prefix("assets/minecraft/textures"))
                .writes(ScopeSet::prefix("assets/minecraft/textures"))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.has_prefix(PARTICLE)? {
                return Ok(Outcome::default());
            }
            let output = format!("{PARTICLE}/particles.png");
            if tx.exists(&output) {
                // 旧实现：已存在就跳过
                return Ok(Outcome::default());
            }
            let table = positions();

            // split_size 取「第一个能打开的瓦片」的宽度（旧实现同）
            let mut split_size = 0u32;
            for (name, _) in &table {
                for dir in [PARTICLE, ENTITY] {
                    let path = format!("{dir}/{name}");
                    if tx.exists(&path) {
                        split_size = tx.image(&path)?.width();
                        break;
                    }
                }
                if split_size > 0 {
                    break;
                }
            }
            if split_size == 0 {
                return Ok(Outcome {
                    skipped: 1,
                    ..Outcome::default()
                });
            }

            let mut merged = RgbaImage::new(16 * split_size, 16 * split_size);
            let mut outcome = Outcome::default();
            for (name, (row, col)) in &table {
                let mut chosen: Option<String> = None;
                for dir in [PARTICLE, ENTITY] {
                    let path = format!("{dir}/{name}");
                    if tx.exists(&path) {
                        chosen = Some(path);
                        break;
                    }
                }
                let Some(path) = chosen else { continue };
                let img: RgbaImage = (*tx.image(&path)?).clone();
                paste_region(&mut merged, &img, col * split_size, row * split_size)
                    .map_err(AromError::internal)?;
                outcome
                    .deferred_removals
                    .extend(defer_remove_if_present(tx, &path)?.deferred_removals);
            }
            tx.put_image(&output, &merged)?;
            outcome.changed += 1;
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "reverse_fix_clock_compass" => Some((clock_compass::decl(), clock_compass::run)),
            "reverse_fix_particles" => Some((particles::decl(), particles::run)),
            _ => None,
        }
    }
