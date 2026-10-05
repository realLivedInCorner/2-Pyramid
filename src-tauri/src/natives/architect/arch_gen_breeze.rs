    use super::*;
    use crate::color::hue::{adjust_hue_brightness, force_hue_saturation};

    const ITEM: &str = "assets/minecraft/textures/item/";
    const BLOCK: &str = "assets/minecraft/textures/block/";
    const MOB: &str = "assets/minecraft/textures/mob_effect/";
    const ENTITY: &str = "assets/minecraft/textures/entity/";

    /// 旧实现的两种染色（相对色相 / 钉色相）。
    enum Tint {
        Shift { h: f32, b: f32, s: f32 },
        Force {
            hue: f32,
            sat: f32,
            v_min: f32,
            v_max: f32,
        },
    }

    /// 旧 `recolor_skip_existing`：源缺失或目标已存在即跳过。
    fn recolor_skip_existing(
        tx: &mut Tx<'_>,
        src: &str,
        dst: &str,
        tint: &Tint,
    ) -> Result<bool, AromError> {
        if !tx.exists(src) || tx.exists(dst) {
            return Ok(false);
        }
        let img: RgbaImage = (*tx.image(src)?).clone();
        let out = match tint {
            Tint::Shift { h, b, s } => adjust_hue_brightness(img, *h, *b, *s),
            Tint::Force {
                hue,
                sat,
                v_min,
                v_max,
            } => force_hue_saturation(img, *hue, 6.0, *sat, *v_min, *v_max),
        };
        tx.put_image(dst, &out)?;
        let meta = format!("{src}.mcmeta");
        if tx.exists(&meta) {
            let bytes = tx.read(&meta)?.unwrap_or_default();
            tx.put(&format!("{dst}.mcmeta"), bytes)?;
        }
        Ok(true)
    }

    /// 候选链的「第一个存在者」。
    fn first_existing<S: AsRef<str>>(tx: &Tx<'_>, candidates: &[S]) -> Option<String> {
        candidates
            .iter()
            .find(|src| tx.exists(src.as_ref()))
            .map(|src| src.as_ref().to_string())
    }

    /// 铜灯泡的色调分组（旧实现用 `contains` 判定，顺序即优先级：oxidized → weathered → exposed → 默认）
    fn bulb_tint(dst: &str, lit: bool) -> Tint {
        let four = |oxidized, weathered, exposed, plain| -> (f32, f32, f32, f32) {
            if dst.contains("oxidized") {
                oxidized
            } else if dst.contains("weathered") {
                weathered
            } else if dst.contains("exposed") {
                exposed
            } else {
                plain
            }
        };
        let (hue, sat, v_min, v_max) = if lit {
            four(
                (145.0, 0.2, 0.45, 0.95),
                (120.0, 0.25, 0.5, 0.95),
                (35.0, 0.3, 0.55, 0.98),
                (18.0, 0.38, 0.55, 0.98),
            )
        } else {
            four(
                (145.0, 0.22, 0.25, 0.55),
                (120.0, 0.28, 0.3, 0.6),
                (35.0, 0.32, 0.35, 0.65),
                (18.0, 0.42, 0.35, 0.7),
            )
        };
        Tint::Force {
            hue,
            sat,
            v_min,
            v_max,
        }
    }

    pub fn decl() -> TaskDecl {
        TaskDecl::new("generate_tricky_trials_breeze", Tier::Architect)
            .reads(ScopeSet::prefix("assets/minecraft/textures"))
            .writes(ScopeSet::prefix("assets/minecraft/textures"))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();
        let emit = |ok: bool, outcome: &mut Outcome| {
            if ok {
                outcome.changed += 1;
            }
        };

        // ── 第 1 批：好做 ──
        emit(
            recolor_skip_existing(
                tx,
                &format!("{ITEM}blaze_rod.png"),
                &format!("{ITEM}breeze_rod.png"),
                &Tint::Shift {
                    h: 185.0,
                    b: -22.0,
                    s: -18.0,
                },
            )?,
            &mut outcome,
        );
        emit(
            recolor_skip_existing(
                tx,
                &format!("{ITEM}snowball.png"),
                &format!("{ITEM}wind_charge.png"),
                &Tint::Force {
                    hue: 222.0,
                    sat: 0.14,
                    v_min: 0.55,
                    v_max: 0.95,
                },
            )?,
            &mut outcome,
        );
        emit(
            recolor_skip_existing(
                tx,
                &format!("{ENTITY}snowball.png"),
                &format!("{ENTITY}projectiles/wind_charge.png"),
                &Tint::Force {
                    hue: 222.0,
                    sat: 0.14,
                    v_min: 0.5,
                    v_max: 0.95,
                },
            )?,
            &mut outcome,
        );
        emit(
            recolor_skip_existing(
                tx,
                &format!("{ITEM}gold_ingot.png"),
                &format!("{ITEM}trial_key.png"),
                &Tint::Force {
                    hue: 22.0,
                    sat: 0.38,
                    v_min: 0.28,
                    v_max: 0.62,
                },
            )?,
            &mut outcome,
        );

        // 不祥试炼钥匙：源是**条件选择**（trial_key 优先，否则 gold_ingot）
        let trial_key = format!("{ITEM}trial_key.png");
        let ominous_src = if tx.exists(&trial_key) {
            trial_key
        } else {
            format!("{ITEM}gold_ingot.png")
        };
        emit(
            recolor_skip_existing(
                tx,
                &ominous_src,
                &format!("{ITEM}ominous_trial_key.png"),
                &Tint::Force {
                    hue: 160.0,
                    sat: 0.16,
                    v_min: 0.18,
                    v_max: 0.45,
                },
            )?,
            &mut outcome,
        );

        // 旋风刷怪蛋：任意已有刷怪蛋 → 蓝灰（第一个存在者胜）
        let eggs = ["chicken", "spider", "cow", "creeper"].map(|n| format!("{ITEM}{n}_spawn_egg.png"));
        if let Some(src) = first_existing(tx, &eggs) {
            emit(
                recolor_skip_existing(
                    tx,
                    &src,
                    &format!("{ITEM}breeze_spawn_egg.png"),
                    &Tint::Force {
                        hue: 230.0,
                        sat: 0.28,
                        v_min: 0.35,
                        v_max: 0.75,
                    },
                )?,
                &mut outcome,
            );
        }

        // 风充能状态图标
        let effects = ["speed", "jump_boost", "absorption"]
            .map(|n| format!("{MOB}{n}.png"));
        if let Some(src) = first_existing(tx, &effects) {
            emit(
                recolor_skip_existing(
                    tx,
                    &src,
                    &format!("{MOB}wind_charged.png"),
                    &Tint::Force {
                        hue: 220.0,
                        sat: 0.28,
                        v_min: 0.45,
                        v_max: 0.9,
                    },
                )?,
                &mut outcome,
            );
        }

        // 铜灯泡族：熄灭（12 个目标）
        let lamp_off = [
            format!("{BLOCK}redstone_lamp.png"),
            format!("{BLOCK}redstone_lamp_off.png"),
        ];
        let copper_base = [
            format!("{BLOCK}copper_block.png"),
            format!("{BLOCK}cut_copper.png"),
        ];
        let off_src = first_existing(tx, &lamp_off).or_else(|| first_existing(tx, &copper_base));
        if let Some(src) = off_src {
            for dst in [
                "copper_bulb.png",
                "copper_bulb_powered.png",
                "exposed_copper_bulb.png",
                "exposed_copper_bulb_powered.png",
                "weathered_copper_bulb.png",
                "weathered_copper_bulb_powered.png",
                "oxidized_copper_bulb.png",
                "oxidized_copper_bulb_powered.png",
                "waxed_copper_bulb.png",
                "waxed_exposed_copper_bulb.png",
                "waxed_weathered_copper_bulb.png",
                "waxed_oxidized_copper_bulb.png",
            ] {
                let tint = bulb_tint(dst, false);
                emit(
                    recolor_skip_existing(tx, &src, &format!("{BLOCK}{dst}"), &tint)?,
                    &mut outcome,
                );
            }
        }

        // 铜灯泡族：点亮（12 个目标）
        let lamp_on = [
            format!("{BLOCK}redstone_lamp_on.png"),
            format!("{BLOCK}redstone_lamp.png"),
        ];
        if let Some(src) = first_existing(tx, &lamp_on) {
            for dst in [
                "copper_bulb_lit.png",
                "copper_bulb_lit_powered.png",
                "exposed_copper_bulb_lit.png",
                "exposed_copper_bulb_lit_powered.png",
                "weathered_copper_bulb_lit.png",
                "weathered_copper_bulb_lit_powered.png",
                "oxidized_copper_bulb_lit.png",
                "oxidized_copper_bulb_lit_powered.png",
                "waxed_copper_bulb_lit.png",
                "waxed_exposed_copper_bulb_lit.png",
                "waxed_weathered_copper_bulb_lit.png",
                "waxed_oxidized_copper_bulb_lit.png",
            ] {
                let tint = bulb_tint(dst, true);
                emit(
                    recolor_skip_existing(tx, &src, &format!("{BLOCK}{dst}"), &tint)?,
                    &mut outcome,
                );
            }
        }

        // ── 第 2 批：中等 ──
        let heavy = [
            format!("{BLOCK}iron_block.png"),
            format!("{BLOCK}deepslate.png"),
            format!("{BLOCK}polished_deepslate.png"),
        ];
        if let Some(src) = first_existing(tx, &heavy) {
            emit(
                recolor_skip_existing(
                    tx,
                    &src,
                    &format!("{BLOCK}heavy_core.png"),
                    &Tint::Force {
                        hue: 220.0,
                        sat: 0.1,
                        v_min: 0.22,
                        v_max: 0.48,
                    },
                )?,
                &mut outcome,
            );
        }

        // 旋风实体：烈焰人两张 → breeze 两张
        for (src, dst) in [
            (format!("{ENTITY}blaze.png"), format!("{ENTITY}breeze/breeze.png")),
            (
                format!("{ENTITY}blaze_blaze.png"),
                format!("{ENTITY}breeze/breeze_eyes.png"),
            ),
        ] {
            emit(
                recolor_skip_existing(
                    tx,
                    &src,
                    &dst,
                    &Tint::Force {
                        hue: 235.0,
                        sat: 0.26,
                        v_min: 0.35,
                        v_max: 0.85,
                    },
                )?,
                &mut outcome,
            );
        }

        // flow 盔甲纹饰模板
        let trims = [
            "armor_trim_smithing_template",
            "silence_armor_trim_smithing_template",
            "bolt_armor_trim_smithing_template",
            "dune_armor_trim_smithing_template",
        ]
        .map(|n| format!("{ITEM}{n}.png"));
        if let Some(src) = first_existing(tx, &trims) {
            emit(
                recolor_skip_existing(
                    tx,
                    &src,
                    &format!("{ITEM}flow_armor_trim_smithing_template.png"),
                    &Tint::Force {
                        hue: 225.0,
                        sat: 0.3,
                        v_min: 0.35,
                        v_max: 0.8,
                    },
                )?,
                &mut outcome,
            );
        }

        // 不祥之瓶
        let bottles = ["potion", "splash_potion", "awkward_potion"]
            .map(|n| format!("{ITEM}{n}.png"));
        if let Some(src) = first_existing(tx, &bottles) {
            emit(
                recolor_skip_existing(
                    tx,
                    &src,
                    &format!("{ITEM}ominous_bottle.png"),
                    &Tint::Force {
                        hue: 280.0,
                        sat: 0.22,
                        v_min: 0.2,
                        v_max: 0.5,
                    },
                )?,
                &mut outcome,
            );
        }

        Ok(outcome)
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "generate_tricky_trials_breeze" => Some((decl(), run)),
            _ => None,
        }
    }
