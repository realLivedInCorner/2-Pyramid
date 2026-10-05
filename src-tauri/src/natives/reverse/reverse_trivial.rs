    use super::*;

    macro_rules! noop_pilot {
        ($m:ident, $task:literal, $tier:expr) => {
            pub mod $m {
                use super::*;
                pub fn decl() -> TaskDecl {
                    TaskDecl::new($task, $tier).exclusive(true)
                }
                pub fn run(_tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
                    Ok(Outcome::default())
                }
            }
        };
    }

    macro_rules! drop_pilot {
        ($m:ident, $task:literal, $path:literal) => {
            pub mod $m {
                use super::*;
                pub const TARGET: &str = $path;
                pub fn decl() -> TaskDecl {
                    TaskDecl::new($task, Tier::Eraser)
                        .writes(ScopeSet::exact(TARGET))
                        .exclusive(true)
                }
                pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
                    // 旧实现是 `defer_remove_file/dir`：删除**延迟到清理点**，
                    // 否则更晚的任务看不到该文件（§9.23）
                    defer_remove_if_present(tx, TARGET)
                }
            }
        };
    }

    noop_pilot!(cut_gui, "reverse_cut_gui", Tier::Surgeon);
    noop_pilot!(horse, "reverse_fix_horse_ui", Tier::Surgeon);
    noop_pilot!(overlay_icons, "reverse_overlay_icons", Tier::Surgeon);
    noop_pilot!(sub_hand, "reverse_fix_ui_sub_hand", Tier::Surgeon);

    drop_pilot!(
        snow_bucket,
        "reverse_generate_snow_bucket",
        "assets/minecraft/textures/item/powder_snow_bucket.png"
    );
    drop_pilot!(
        smithing_ui,
        "reverse_generate_smithing_ui",
        "assets/minecraft/textures/gui/container/smithing.png"
    );
    drop_pilot!(
        slider,
        "reverse_fix_slider",
        "assets/minecraft/textures/gui/slider.png"
    );

    macro_rules! defer_list_pilot {
        ($m:ident, $task:literal, [$($p:literal),*]) => {
            pub mod $m {
                use super::*;
                pub const TARGETS: [&str; 0 $(+ { let _ = $p; 1 })*] = [$($p),*];
                pub fn decl() -> TaskDecl {
                    let mut scope = ScopeSet::none();
                    $( scope = scope.union(&ScopeSet::exact($p)); )*
                    TaskDecl::new($task, Tier::Eraser).writes(scope).exclusive(true)
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
        };
    }

    defer_list_pilot!(
        tipped_arrows,
        "reverse_generate_tipped_arrow_images",
        [
            "assets/minecraft/textures/items/tipped_arrow_base.png",
            "assets/minecraft/textures/items/tipped_arrow_head.png"
        ]
    );

    /// 旧 `reverse_generate_boat`：延迟删 5 个船变体 + **有守卫**的立即改名（`boat.png` 不存在才改）。
    pub mod boat {
        use super::*;

        const ITEMS: &str = "assets/minecraft/textures/items";
        const VARIANTS: [&str; 5] = [
            "oak_boat.png",
            "birch_boat.png",
            "acacia_boat.png",
            "dark_oak_boat.png",
            "jungle_boat.png",
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("reverse_generate_boat", Tier::Eraser)
                .reads(ScopeSet::prefix(ITEMS))
                .writes(ScopeSet::prefix(ITEMS))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for name in VARIANTS {
                outcome
                    .deferred_removals
                    .extend(defer_remove_if_present(tx, &format!("{ITEMS}/{name}"))?.deferred_removals);
            }
            let spruce = format!("{ITEMS}/spruce_boat.png");
            let boat = format!("{ITEMS}/boat.png");
            // 旧实现带守卫：目标已存在就不改名（不覆盖）
            if tx.exists(&spruce) && !tx.exists(&boat) {
                let bytes = tx.read(&spruce)?.unwrap_or_default();
                tx.put(&boat, bytes)?;
                tx.remove(&spruce)?;
                outcome.changed += 1;
                outcome.notes.push("spruce_boat -> boat".into());
            }
            Ok(outcome)
        }
    }

    defer_list_pilot!(
        crossbow,
        "reverse_generate_crossbow",
        [
            "assets/minecraft/textures/item/crossbow_standby.png",
            "assets/minecraft/textures/item/crossbow_pulling_0.png",
            "assets/minecraft/textures/item/crossbow_pulling_1.png",
            "assets/minecraft/textures/item/crossbow_pulling_2.png",
            "assets/minecraft/textures/item/crossbow_arrow.png",
            "assets/minecraft/textures/item/crossbow_firework.png"
        ]
    );

    defer_list_pilot!(
        fish_bucket,
        "reverse_generate_fish_bucket",
        [
            "assets/minecraft/textures/item/axolotl_bucket.png",
            "assets/minecraft/textures/item/cod_bucket.png",
            "assets/minecraft/textures/item/pufferfish_bucket.png",
            "assets/minecraft/textures/item/salmon_bucket.png",
            "assets/minecraft/textures/item/tropical_fish_bucket.png",
            "assets/minecraft/textures/item/tadpole_bucket.png"
        ]
    );

    /// 带 `.png.mcmeta` 附属的延迟删除（planks 家族的旧实现：本体与附属各删一次）。
    macro_rules! defer_with_meta_pilot {
        ($m:ident, $task:literal, [$($p:literal),*]) => {
            pub mod $m {
                use super::*;
                pub const TARGETS: [&str; 0 $(+ { let _ = $p; 1 })*] = [$($p),*];
                pub fn decl() -> TaskDecl {
                    let mut scope = ScopeSet::none();
                    $( scope = scope.union(&ScopeSet::exact($p)); )*
                    TaskDecl::new($task, Tier::Eraser).writes(scope).exclusive(true)
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
        };
    }

    defer_list_pilot!(
        furnace,
        "reverse_generate_furnace",
        [
            "assets/minecraft/textures/gui/container/blast_furnace.png",
            "assets/minecraft/textures/gui/container/smoker.png"
        ]
    );

    defer_list_pilot!(
        potion_lingering,
        "reverse_generate_potion_lingering",
        [
            "assets/minecraft/textures/items/lingering_potion.png",
            "assets/minecraft/textures/items/lingering_potion.png.mcmeta",
            "assets/minecraft/textures/items/potion_bottle_lingering.png",
            "assets/minecraft/textures/items/potion_bottle_lingering.png.mcmeta"
        ]
    );

    defer_with_meta_pilot!(
        redwood_planks,
        "reverse_generate_redwood_cherry_bamboo_planks",
        [
            "assets/minecraft/textures/block/mangrove_planks.png",
            "assets/minecraft/textures/block/cherry_planks.png",
            "assets/minecraft/textures/block/bamboo_planks.png",
            "assets/minecraft/textures/block/mangrove_log.png",
            "assets/minecraft/textures/block/mangrove_log_top.png",
            "assets/minecraft/textures/block/cherry_log.png",
            "assets/minecraft/textures/block/cherry_log_top.png",
            "assets/minecraft/textures/block/bamboo_block.png",
            "assets/minecraft/textures/block/bamboo_block_top.png",
            "assets/minecraft/textures/block/bamboo_mosaic.png"
        ]
    );

    defer_with_meta_pilot!(
        pale_planks,
        "reverse_generate_pale_planks",
        [
            "assets/minecraft/textures/block/pale_oak_planks.png",
            "assets/minecraft/textures/block/pale_oak_log.png",
            "assets/minecraft/textures/block/pale_oak_log_top.png"
        ]
    );

    defer_with_meta_pilot!(
        poplar_planks,
        "reverse_generate_poplar_planks",
        [
            "assets/minecraft/textures/block/poplar_planks.png",
            "assets/minecraft/textures/block/poplar_log.png",
            "assets/minecraft/textures/block/poplar_log_top.png",
            "assets/minecraft/textures/block/stripped_poplar_log.png",
            "assets/minecraft/textures/block/stripped_poplar_log_top.png",
            "assets/minecraft/textures/block/poplar_door_top.png",
            "assets/minecraft/textures/block/poplar_door_bottom.png",
            "assets/minecraft/textures/block/poplar_trapdoor.png",
            "assets/minecraft/textures/block/poplar_shelf.png",
            "assets/minecraft/textures/block/poplar_sapling.png",
            "assets/minecraft/textures/block/poplar_sign.png",
            "assets/minecraft/textures/block/poplar_hanging_sign.png",
            "assets/minecraft/textures/block/red_poplar_leaves.png",
            "assets/minecraft/textures/block/orange_poplar_leaves.png",
            "assets/minecraft/textures/block/yellow_poplar_leaves.png",
            "assets/minecraft/textures/item/poplar_sign.png",
            "assets/minecraft/textures/item/poplar_hanging_sign.png",
            "assets/minecraft/textures/item/poplar_door.png",
            "assets/minecraft/textures/item/poplar_boat.png",
            "assets/minecraft/textures/item/poplar_chest_boat.png",
            "assets/minecraft/textures/entity/boat/poplar.png",
            "assets/minecraft/textures/entity/chest_boat/poplar.png"
        ]
    );

    /// 任务名 → (声明, 实现)。驱动按名字派发。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "reverse_cut_gui" => Some((cut_gui::decl(), cut_gui::run)),
            "reverse_fix_horse_ui" => Some((horse::decl(), horse::run)),
            "reverse_overlay_icons" => Some((overlay_icons::decl(), overlay_icons::run)),
            "reverse_fix_ui_sub_hand" => Some((sub_hand::decl(), sub_hand::run)),
            "reverse_generate_snow_bucket" => Some((snow_bucket::decl(), snow_bucket::run)),
            "reverse_generate_smithing_ui" => Some((smithing_ui::decl(), smithing_ui::run)),
            "reverse_fix_slider" => Some((slider::decl(), slider::run)),
            _ => None,
        }
    }
