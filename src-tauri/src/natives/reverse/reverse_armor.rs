    use super::*;

    const HUMAN: &str = "assets/minecraft/textures/entity/equipment/humanoid";
    const LEGGINGS: &str = "assets/minecraft/textures/entity/equipment/humanoid_leggings";
    const ARMOR: &str = "assets/minecraft/textures/models/armor";

    const LAYER1: [(&str, &str); 8] = [
        ("chainmail.png", "chainmail_layer_1.png"),
        ("diamond.png", "diamond_layer_1.png"),
        ("iron.png", "iron_layer_1.png"),
        ("gold.png", "gold_layer_1.png"),
        ("leather.png", "leather_layer_1.png"),
        ("leather_overlay.png", "leather_layer_1_overlay.png"),
        ("netherite.png", "netherite_layer_1.png"),
        ("copper.png", "copper_layer_1.png"),
    ];
    const LAYER2: [(&str, &str); 8] = [
        ("chainmail.png", "chainmail_layer_2.png"),
        ("diamond.png", "diamond_layer_2.png"),
        ("iron.png", "iron_layer_2.png"),
        ("gold.png", "gold_layer_2.png"),
        ("leather.png", "leather_layer_2.png"),
        ("leather_overlay.png", "leather_layer_2_overlay.png"),
        ("netherite.png", "netherite_layer_2.png"),
        ("copper.png", "copper_layer_2.png"),
    ];

    /// 旧实现逐条 `fs::rename`：**无守卫、覆盖**。
    fn move_file(tx: &mut Tx<'_>, from: &str, to: &str) -> Result<bool, AromError> {
        if !tx.exists(from) {
            return Ok(false);
        }
        let bytes = tx.read(from)?.unwrap_or_default();
        tx.put(to, bytes)?;
        tx.remove(from)?;
        Ok(true)
    }

    pub fn decl() -> TaskDecl {
        TaskDecl::new("reverse_fix_armor_models", Tier::Surgeon)
            .reads(ScopeSet::prefix("assets/minecraft/textures"))
            .writes(ScopeSet::prefix("assets/minecraft/textures"))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();
        for (src, dest) in LAYER1 {
            if move_file(tx, &format!("{HUMAN}/{src}"), &format!("{ARMOR}/{dest}"))? {
                outcome.changed += 1;
            }
        }
        for (src, dest) in LAYER2 {
            if move_file(tx, &format!("{LEGGINGS}/{src}"), &format!("{ARMOR}/{dest}"))? {
                outcome.changed += 1;
            }
        }
        Ok(outcome)
    }
