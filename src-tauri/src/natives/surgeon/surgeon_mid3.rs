    use super::*;

    const PARTICLE: &str = "assets/minecraft/textures/particle";
    const PARTICLES: &str = "assets/minecraft/textures/particle/particles.png";

    /// 旧实现的 (row, col) → 输出路径映射；返回 `None` 表示该格丢弃。
    fn tile_target(row: u32, col: u32) -> Option<String> {
        let particle = |name: &str| Some(format!("{PARTICLE}/{name}"));
        match (row, col) {
            (0, c) if c < 8 => particle(&format!("generic_{c}.png")),
            (1, c) if (3..=6).contains(&c) => particle(&format!("splash_{}.png", c - 3)),
            (2, 0) => particle("bubble.png"),
            // 注意：这一格写到 entity/ 目录（旧实现如此）
            (2, 1) => Some("assets/minecraft/textures/entity/fishing_hook.png".to_string()),
            (3, 0) => particle("flame.png"),
            (3, 1) => particle("lava.png"),
            (4, c) if c < 3 => particle(["note.png", "critical_hit.png", "enchanted_hit.png"][c as usize]),
            (5, c) if c < 3 => particle(["heart.png", "angry.png", "glint.png"][c as usize]),
            (7, c) if c < 3 => particle(["drip_hang.png", "drip_fall.png", "drip_land.png"][c as usize]),
            (8, c) if c < 8 => particle(&format!("effect_{c}.png")),
            (9, c) if c < 8 => particle(&format!("spell_{c}.png")),
            (10, c) if c < 8 => particle(&format!("spark_{c}.png")),
            _ => None,
        }
    }

    pub fn decl() -> TaskDecl {
        TaskDecl::new("fix_particles", Tier::Surgeon)
            .reads(ScopeSet::prefix("assets/minecraft/textures"))
            .writes(ScopeSet::prefix("assets/minecraft/textures"))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        if !tx.exists(PARTICLES) {
            crate::log_info!("particles.png not found, skip fix_particles");
            return Ok(Outcome::default());
        }
        let img: RgbaImage = (*tx.image(PARTICLES)?).clone();
        let (w, h) = img.dimensions();
        if w != h || w % 16 != 0 {
            crate::log_info!("particles.png is not square /16, skip split");
            return Ok(Outcome::default());
        }
        let split = w / 16;

        let mut outcome = Outcome::default();
        for row in 0u32..16 {
            for col in 0u32..16 {
                let Some(target) = tile_target(row, col) else {
                    continue;
                };
                let tile = image::imageops::crop_imm(&img, col * split, row * split, split, split)
                    .to_image();
                tx.put_image(&target, &tile)?;
                outcome.changed += 1;
            }
        }

        // 旧实现最后删掉原图
        tx.remove(PARTICLES)?;
        outcome.notes.push("particles.png split and removed".into());
        Ok(outcome)
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "fix_particles" => Some((decl(), run)),
            _ => None,
        }
    }
