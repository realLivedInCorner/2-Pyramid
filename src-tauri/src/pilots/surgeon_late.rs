    use super::*;

    const ARMOR_SRC: &str = "assets/minecraft/textures/models/armor";
    const HUMANOID: &str = "assets/minecraft/textures/entity/equipment/humanoid";
    const LEGGINGS: &str = "assets/minecraft/textures/entity/equipment/humanoid_leggings";

    /// (源文件名, 目标名) —— 逐条照抄（layer_1 组）
    const LAYER1: [(&str, &str); 8] = [
        ("chainmail_layer_1.png", "chainmail.png"),
        ("diamond_layer_1.png", "diamond.png"),
        ("iron_layer_1.png", "iron.png"),
        ("gold_layer_1.png", "gold.png"),
        ("leather_layer_1.png", "leather.png"),
        ("leather_layer_1_overlay.png", "leather_overlay.png"),
        ("netherite_layer_1.png", "netherite.png"),
        ("copper_layer_1.png", "copper.png"),
    ];

    /// (源文件名, 目标名) —— layer_2 组
    const LAYER2: [(&str, &str); 8] = [
        ("chainmail_layer_2.png", "chainmail.png"),
        ("diamond_layer_2.png", "diamond.png"),
        ("iron_layer_2.png", "iron.png"),
        ("gold_layer_2.png", "gold.png"),
        ("leather_layer_2.png", "leather.png"),
        ("leather_layer_2_overlay.png", "leather_overlay.png"),
        ("netherite_layer_2.png", "netherite.png"),
        ("copper_layer_2.png", "copper.png"),
    ];

    pub fn decl() -> TaskDecl {
        // 注意两点（都踩过）：
        // ① **删除也是写入**（层里是 Tombstone），源目录必须一并声明，否则 `strict_scopes` 报错；
        // ② `TaskDecl::writes()` 是**覆盖**不是累加——多个范围要用 `ScopeSet::with_prefix` 组合。
        TaskDecl::new("fix_armor_models", Tier::Surgeon)
            .reads(ScopeSet::prefix(ARMOR_SRC))
            .writes(
                ScopeSet::prefix(ARMOR_SRC)
                    .with_prefix("assets/minecraft/textures/entity/equipment"),
            )
            .exclusive(true)
    }

    /// 旧实现是 `fs::rename`（**覆盖**目标），这里等价为「读字节 → 写目标 → 删源」。
    fn move_one(
        tx: &mut Tx<'_>,
        src_name: &str,
        dest_dir: &str,
        dest_name: &str,
        outcome: &mut Outcome,
    ) -> Result<(), AromError> {
        let src = format!("{ARMOR_SRC}/{src_name}");
        if !tx.exists(&src) {
            return Ok(());
        }
        let bytes = tx.read(&src)?.unwrap_or_default();
        tx.put(&format!("{dest_dir}/{dest_name}"), bytes)?;
        tx.remove(&src)?;
        outcome.changed += 1;
        Ok(())
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        if !tx.has_prefix(ARMOR_SRC)? {
            crate::log_info!("armor models dir not found, skip");
            return Ok(Outcome::default());
        }
        let mut outcome = Outcome::default();
        for (src, dest) in LAYER1 {
            move_one(tx, src, HUMANOID, dest, &mut outcome)?;
        }
        for (src, dest) in LAYER2 {
            move_one(tx, src, LEGGINGS, dest, &mut outcome)?;
        }
        Ok(outcome)
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "fix_armor_models" => Some((decl(), run)),
            _ => None,
        }
    }
