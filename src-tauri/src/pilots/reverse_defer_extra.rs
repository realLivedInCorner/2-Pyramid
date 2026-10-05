    use super::*;

    /// 旧 `reverse_fix2_horse_ui`：延迟删 3 个 gui sprite 槽位贴图。
    pub mod horse_ui_slots {
        use super::*;

        const TARGETS: [&str; 3] = [
            "assets/minecraft/textures/gui/sprites/container/slot/horse_armor.png",
            "assets/minecraft/textures/gui/sprites/container/slot/llama_armor.png",
            "assets/minecraft/textures/gui/sprites/container/slot/saddle.png",
        ];

        pub fn decl() -> TaskDecl {
            let mut scope = ScopeSet::none();
            for path in TARGETS {
                scope = scope.union(&ScopeSet::exact(path));
            }
            TaskDecl::new("reverse_fix2_horse_ui", Tier::Eraser)
                .writes(scope)
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for path in TARGETS {
                outcome
                    .deferred_removals
                    .extend(defer_remove_if_present(tx, path)?.deferred_removals);
            }
            Ok(outcome)
        }
    }

    /// 旧 `reverse_generate_tricky_trials_breeze`：删除 1.21 旋风系生成物（36 个路径），
    /// 每个路径**连同 `.png.mcmeta` 附属**各登记一次（旧实现逐条如此）。
    pub mod breeze {
        use super::*;

        const TARGETS: [&str; 36] = [
            "assets/minecraft/textures/item/breeze_rod.png",
            "assets/minecraft/textures/item/wind_charge.png",
            "assets/minecraft/textures/item/trial_key.png",
            "assets/minecraft/textures/item/ominous_trial_key.png",
            "assets/minecraft/textures/item/breeze_spawn_egg.png",
            "assets/minecraft/textures/entity/projectiles/wind_charge.png",
            "assets/minecraft/textures/mob_effect/wind_charged.png",
            "assets/minecraft/textures/block/copper_bulb.png",
            "assets/minecraft/textures/block/copper_bulb_lit.png",
            "assets/minecraft/textures/block/copper_bulb_powered.png",
            "assets/minecraft/textures/block/copper_bulb_lit_powered.png",
            "assets/minecraft/textures/block/exposed_copper_bulb.png",
            "assets/minecraft/textures/block/exposed_copper_bulb_lit.png",
            "assets/minecraft/textures/block/exposed_copper_bulb_powered.png",
            "assets/minecraft/textures/block/exposed_copper_bulb_lit_powered.png",
            "assets/minecraft/textures/block/weathered_copper_bulb.png",
            "assets/minecraft/textures/block/weathered_copper_bulb_lit.png",
            "assets/minecraft/textures/block/weathered_copper_bulb_powered.png",
            "assets/minecraft/textures/block/weathered_copper_bulb_lit_powered.png",
            "assets/minecraft/textures/block/oxidized_copper_bulb.png",
            "assets/minecraft/textures/block/oxidized_copper_bulb_lit.png",
            "assets/minecraft/textures/block/oxidized_copper_bulb_powered.png",
            "assets/minecraft/textures/block/oxidized_copper_bulb_lit_powered.png",
            "assets/minecraft/textures/block/waxed_copper_bulb.png",
            "assets/minecraft/textures/block/waxed_copper_bulb_lit.png",
            "assets/minecraft/textures/block/waxed_exposed_copper_bulb.png",
            "assets/minecraft/textures/block/waxed_exposed_copper_bulb_lit.png",
            "assets/minecraft/textures/block/waxed_weathered_copper_bulb.png",
            "assets/minecraft/textures/block/waxed_weathered_copper_bulb_lit.png",
            "assets/minecraft/textures/block/waxed_oxidized_copper_bulb.png",
            "assets/minecraft/textures/block/waxed_oxidized_copper_bulb_lit.png",
            "assets/minecraft/textures/block/heavy_core.png",
            "assets/minecraft/textures/entity/breeze/breeze.png",
            "assets/minecraft/textures/entity/breeze/breeze_eyes.png",
            "assets/minecraft/textures/item/flow_armor_trim_smithing_template.png",
            "assets/minecraft/textures/item/ominous_bottle.png",
        ];

        pub fn decl() -> TaskDecl {
            let mut scope = ScopeSet::none();
            for path in TARGETS {
                scope = scope.union(&ScopeSet::exact(path));
            }
            TaskDecl::new("reverse_generate_tricky_trials_breeze", Tier::Eraser)
                .writes(scope)
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for base in TARGETS {
                outcome
                    .deferred_removals
                    .extend(defer_remove_if_present(tx, base)?.deferred_removals);
                let meta = format!("{base}.mcmeta");
                outcome
                    .deferred_removals
                    .extend(defer_remove_if_present(tx, &meta)?.deferred_removals);
            }
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "reverse_fix2_horse_ui" => Some((horse_ui_slots::decl(), horse_ui_slots::run)),
            "reverse_generate_tricky_trials_breeze" => Some((breeze::decl(), breeze::run)),
            _ => None,
        }
    }
