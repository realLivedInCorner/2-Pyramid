    use super::*;

    macro_rules! defer_pilot {
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
                    defer_remove_if_present(tx, TARGET)
                }
            }
        };
    }

    defer_pilot!(
        shulker_box,
        "reverse_generate_shulker_box_ui",
        "assets/minecraft/textures/gui/container/shulker_box.png"
    );
    defer_pilot!(
        sign_entities,
        "reverse_fix_sign_entities",
        "assets/minecraft/textures/entity/signs"
    );

    pub mod smithing_villager {
        use super::*;

        const GUI: &str = "assets/minecraft/textures/gui/container";
        const SMITHING: &str = "assets/minecraft/textures/gui/container/smithing.png";
        const BACKUP: &str = "assets/minecraft/textures/gui/container/villager_backup.png";
        const VILLAGER: &str = "assets/minecraft/textures/gui/container/villager.png";

        pub fn decl() -> TaskDecl {
            TaskDecl::new("reverse_fix_smithing2_villager2_ui", Tier::Eraser)
                .reads(ScopeSet::prefix(GUI))
                .writes(ScopeSet::prefix(GUI))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            // 立即恢复（旧实现是 fs::rename，非延迟）
            if tx.exists(BACKUP) {
                let bytes = tx.read(BACKUP)?.unwrap_or_default();
                tx.put(VILLAGER, bytes)?;
                tx.remove(BACKUP)?;
                outcome.changed += 1;
                outcome.notes.push("villager_backup -> villager".into());
            }
            // 延迟删除生成的 smithing.png
            outcome
                .deferred_removals
                .extend(defer_remove_if_present(tx, SMITHING)?.deferred_removals);
            Ok(outcome)
        }
    }

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

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "reverse_generate_shulker_box_ui" => Some((shulker_box::decl(), shulker_box::run)),
            "reverse_fix_sign_entities" => Some((sign_entities::decl(), sign_entities::run)),
            "reverse_generate_crossbow" => Some((crossbow::decl(), crossbow::run)),
            "reverse_generate_fish_bucket" => Some((fish_bucket::decl(), fish_bucket::run)),
            "reverse_generate_furnace" => Some((furnace::decl(), furnace::run)),
            "reverse_generate_potion_lingering" => Some((potion_lingering::decl(), potion_lingering::run)),
            "reverse_generate_redwood_cherry_bamboo_planks" => {
                Some((redwood_planks::decl(), redwood_planks::run))
            }
            "reverse_generate_pale_planks" => Some((pale_planks::decl(), pale_planks::run)),
            "reverse_generate_poplar_planks" => Some((poplar_planks::decl(), poplar_planks::run)),
            "reverse_generate_tipped_arrow_images" => {Some((tipped_arrows::decl(), tipped_arrows::run))}
            "reverse_generate_boat" => Some((boat::decl(), boat::run)),
            "reverse_fix_smithing2_villager2_ui" => {
                Some((smithing_villager::decl(), smithing_villager::run))
            }
            _ => None,
        }
    }
