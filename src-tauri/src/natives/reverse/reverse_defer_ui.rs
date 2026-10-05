    use super::*;

    const ITEM: &str = "assets/minecraft/textures/item";
    const GUI: &str = "assets/minecraft/textures/gui/container";

    fn defer_paths(tx: &Tx<'_>, paths: &[&str]) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();
        for path in paths {
            outcome
                .deferred_removals
                .extend(defer_remove_if_present(tx, path)?.deferred_removals);
        }
        Ok(outcome)
    }

    /// 立即「恢复」：读源、写目标（**覆盖**，旧实现无守卫）、删源。
    fn restore(tx: &mut Tx<'_>, from: &str, to: &str) -> Result<bool, AromError> {
        if !tx.exists(from) {
            return Ok(false);
        }
        let bytes = tx.read(from)?.unwrap_or_default();
        tx.put(to, bytes)?;
        tx.remove(from)?;
        Ok(true)
    }

    pub mod sign {
        use super::*;

        const VARIANTS: [&str; 11] = [
            "oak_sign.png",
            "birch_sign.png",
            "acacia_sign.png",
            "dark_oak_sign.png",
            "jungle_sign.png",
            "crimson_sign.png",
            "warped_sign.png",
            "mangrove_sign.png",
            "pale_oak_sign.png",
            "bamboo_sign.png",
            "cherry_sign.png",
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("reverse_fix_sign", Tier::Eraser)
                .reads(ScopeSet::prefix(ITEM))
                .writes(ScopeSet::prefix(ITEM))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let paths: Vec<String> = VARIANTS.iter().map(|n| format!("{ITEM}/{n}")).collect();
            let refs: Vec<&str> = paths.iter().map(|p| p.as_str()).collect();
            let mut outcome = defer_paths(tx, &refs)?;
            // 顺序照抄旧实现：先登记删除，再做立即改名（`oak_sign.png` 同时在列表里，
            // 清理点仍会把它删掉——这是旧实现的真实行为，不能"顺手修正"）
            if restore(tx, &format!("{ITEM}/spruce_sign.png"), &format!("{ITEM}/oak_sign.png"))? {
                outcome.changed += 1;
                outcome.notes.push("spruce_sign -> oak_sign".into());
            }
            Ok(outcome)
        }
    }

    pub mod machinery {
        use super::*;

        const GENERATED: [&str; 4] = [
            "grindstone.png",
            "cartography_table.png",
            "stonecutter.png",
            "loom.png",
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("reverse_fix_machinery_ui", Tier::Eraser)
                .reads(ScopeSet::prefix(GUI))
                .writes(ScopeSet::prefix(GUI))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let paths: Vec<String> = GENERATED.iter().map(|n| format!("{GUI}/{n}")).collect();
            let refs: Vec<&str> = paths.iter().map(|p| p.as_str()).collect();
            let mut outcome = defer_paths(tx, &refs)?;
            if restore(
                tx,
                &format!("{GUI}/villager_backup.png"),
                &format!("{GUI}/villager.png"),
            )? {
                outcome.changed += 1;
                outcome.notes.push("villager_backup -> villager".into());
            }
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "reverse_fix_sign" => Some((sign::decl(), sign::run)),
            "reverse_fix_machinery_ui" => Some((machinery::decl(), machinery::run)),
            _ => None,
        }
    }
