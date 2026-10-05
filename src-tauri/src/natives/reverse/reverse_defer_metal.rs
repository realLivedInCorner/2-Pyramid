    use super::*;

    const ITEM: &str = "assets/minecraft/textures/item";
    const BLOCK: &str = "assets/minecraft/textures/block";
    const ARMOR: &str = "assets/minecraft/textures/models/armor";

    /// 显式路径表：逐条延迟删除（表里显式写了 `.mcmeta` 的照原样删）。
    fn defer_paths(tx: &Tx<'_>, paths: &[&str]) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();
        for path in paths {
            outcome
                .deferred_removals
                .extend(defer_remove_if_present(tx, path)?.deferred_removals);
        }
        Ok(outcome)
    }

    /// 名字表 + 附属：本体与 `{name}.mcmeta` 各删一次（与旧实现逐条对应）。
    fn defer_names_with_sidecar(
        tx: &Tx<'_>,
        dir: &str,
        names: &[&str],
    ) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();
        for name in names {
            let path = format!("{dir}/{name}");
            outcome
                .deferred_removals
                .extend(defer_remove_if_present(tx, &path)?.deferred_removals);
            let meta = format!("{path}.mcmeta");
            outcome
                .deferred_removals
                .extend(defer_remove_if_present(tx, &meta)?.deferred_removals);
        }
        Ok(outcome)
    }

    fn decl_for(task: &str, scope: ScopeSet) -> TaskDecl {
        TaskDecl::new(task, Tier::Eraser)
            .writes(scope)
            .exclusive(true)
    }

    macro_rules! metal_pilot {
        ($m:ident, $task:literal, $scope:expr, $body:expr) => {
            pub mod $m {
                use super::*;
                pub fn decl() -> TaskDecl {
                    decl_for($task, $scope)
                }
                pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
                    #[allow(clippy::redundant_closure_call)]
                    ($body)(tx)
                }
            }
        };
    }

    metal_pilot!(
        copper_ingot,
        "reverse_generate_copper_ingot",
        ScopeSet::prefix(ITEM),
        |tx: &mut Tx<'_>| defer_paths(
            tx,
            &[
                "assets/minecraft/textures/item/copper_ingot.png",
                "assets/minecraft/textures/item/copper_ingot.png.mcmeta",
            ]
        )
    );

    metal_pilot!(
        copper_block,
        "reverse_generate_copper_block",
        ScopeSet::prefix(BLOCK),
        |tx: &mut Tx<'_>| defer_names_with_sidecar(
            tx,
            BLOCK,
            &[
                "copper_block.png",
                "exposed_copper.png",
                "weathered_copper.png",
                "oxidized_copper.png",
            ]
        )
    );

    metal_pilot!(
        copper_tools,
        "reverse_generate_copper_tools",
        ScopeSet::prefix(ITEM),
        |tx: &mut Tx<'_>| defer_names_with_sidecar(
            tx,
            ITEM,
            &[
                "copper_sword.png",
                "copper_helmet.png",
                "copper_chestplate.png",
                "copper_leggings.png",
                "copper_boots.png",
                "copper_axe.png",
                "copper_pickaxe.png",
                "copper_shovel.png",
                "copper_hoe.png",
                "copper_horse_armor.png",
            ]
        )
    );

    metal_pilot!(
        copper_armor_models,
        "reverse_generate_copper_armor_models",
        ScopeSet::prefix(ARMOR),
        |tx: &mut Tx<'_>| defer_paths(
            tx,
            &[
                "assets/minecraft/textures/models/armor/copper_layer_1.png",
                "assets/minecraft/textures/models/armor/copper_layer_2.png",
            ]
        )
    );

    metal_pilot!(
        netherite_block,
        "reverse_generate_netherite_block",
        ScopeSet::prefix(BLOCK),
        |tx: &mut Tx<'_>| defer_paths(
            tx,
            &[
                "assets/minecraft/textures/block/netherite_block.png",
                "assets/minecraft/textures/block/netherite_block.png.mcmeta",
            ]
        )
    );

    metal_pilot!(
        netherite_ingot,
        "reverse_generate_netherite_ingot",
        ScopeSet::prefix(ITEM),
        |tx: &mut Tx<'_>| defer_paths(
            tx,
            &[
                "assets/minecraft/textures/item/netherite_ingot.png",
                "assets/minecraft/textures/item/netherite_ingot.png.mcmeta",
            ]
        )
    );

    metal_pilot!(
        netherite_tools,
        "reverse_generate_netherite_tools",
        ScopeSet::prefix(ITEM),
        |tx: &mut Tx<'_>| defer_names_with_sidecar(
            tx,
            ITEM,
            &[
                "netherite_sword.png",
                "netherite_helmet.png",
                "netherite_chestplate.png",
                "netherite_leggings.png",
                "netherite_boots.png",
                "netherite_axe.png",
                "netherite_pickaxe.png",
                "netherite_shovel.png",
                "netherite_hoe.png",
                "spectral_arrow.png",
            ]
        )
    );

    metal_pilot!(
        netherite_armor_models,
        "reverse_generate_netherite_armor_models",
        ScopeSet::prefix(ARMOR),
        |tx: &mut Tx<'_>| defer_paths(
            tx,
            &[
                "assets/minecraft/textures/models/armor/netherite_layer_1.png",
                "assets/minecraft/textures/models/armor/netherite_layer_2.png",
            ]
        )
    );

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "reverse_generate_copper_ingot" => Some((copper_ingot::decl(), copper_ingot::run)),
            "reverse_generate_copper_block" => Some((copper_block::decl(), copper_block::run)),
            "reverse_generate_copper_tools" => Some((copper_tools::decl(), copper_tools::run)),
            "reverse_generate_copper_armor_models" => {
                Some((copper_armor_models::decl(), copper_armor_models::run))
            }
            "reverse_generate_netherite_block" => {
                Some((netherite_block::decl(), netherite_block::run))
            }
            "reverse_generate_netherite_ingot" => {
                Some((netherite_ingot::decl(), netherite_ingot::run))
            }
            "reverse_generate_netherite_tools" => {
                Some((netherite_tools::decl(), netherite_tools::run))
            }
            "reverse_generate_netherite_armor_models" => Some((
                netherite_armor_models::decl(),
                netherite_armor_models::run,
            )),
            _ => None,
        }
    }
