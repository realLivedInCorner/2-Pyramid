//! 版本 → 任务映射表（原 `hurray/scheduler.rs` 的一部分，§9.134 拆出）。
//!
//! **它只是数据**：哪些版本步、每一步按什么顺序跑哪些任务。计划逻辑（`plan`）与执行
//! （按阶段分桶）都在 `scheduler.rs`。
//!
//! **顺序即语义**：同一步内任务顺序决定执行顺序的候选（引擎仍会按阶段重新分桶，
//! 但计划顺序决定了分桶内各任务的相对次序，也决定了驱动 `native_run` 的派发顺序）。

use std::collections::HashMap;

/// 版本对 `(from, to)` → 该步要跑的任务名（**有序**）。
pub type VersionMap = HashMap<(u32, u32), Vec<String>>;

pub struct ConversionMaps {
    pub forward: VersionMap,
    pub reverse: VersionMap,
}

impl ConversionMaps {
    pub fn new() -> Self {
        let mut forward = HashMap::new();
        let mut reverse = HashMap::new();

        // §9.130：`convert_animated_textures` 从**首位**移到这里（`fix_clock_compass` 之后）。
        //
        // 它原本排在这个步骤的第一个。这一改动是让**计划顺序本身更贴近真实数据依赖**：
        // 它是**原位改写**（给 `item/*.png.mcmeta` 补 `frametime` / `interpolate`），
        // 不产生新文件；而 `fix_clock_compass` 要读 `item/clock.png` / `item/compass.png`
        // 的原尺寸来切帧。把"改写 mcmeta"排在"消费原始贴图"之后，顺序上不再有歧义。
        //
        // **如实记录：这一改动本身不足以取消驱动的前置阶段。** §9.129 的偏差
        // （多 105 条 `textures/item/*`、少 6 条原图）实测**逐个单独前置都不够**——
        // 10 个 `EARLY_NATIVES` 一个个试过，没有任何一个能单独保持冻结（只有"Eraser 级
        // 全部 + `EARLY_NATIVES` 全部"一起前置才正确）。因此驱动里的
        // `native_placements` / `Side::Early` 是**必要机制**，不是"批次边界的权宜之计"。
        forward.insert((1, 2), vec!["delete_blockstates_models".to_string(), "generate_tipped_arrow_images".to_string(), "fix_ui_survival".to_string(), "fix_ui_creative".to_string(), "fix_ui_sub_hand".to_string(), "generate_boat".to_string(), "generate_potion_lingering".to_string(), "generate_shulker_box_ui".to_string(), "fix_brewing_stand_ui".to_string(), "fix_clock_compass".to_string(), "convert_animated_textures".to_string(), "overlay_icons".to_string()]);
        forward.insert((2, 3), vec!["generate_shulker_box_ui".to_string(), "delete_horse_folder".to_string(), "fix_horse_ui".to_string()]);
        forward.insert((3, 4), vec!["rename_blocks_items".to_string(), "fix_sign".to_string(), "fix_sign_entities".to_string(), "generate_furnace".to_string(), "fix_machinery_ui".to_string(), "fix_particles".to_string(), "generate_fish_bucket".to_string(), "generate_crossbow".to_string()]);
        forward.insert((4, 5), vec!["process_chest_folder".to_string(), "generate_netherite_block".to_string(), "generate_netherite_ingot".to_string(), "delete_enchanted_item_glint".to_string(), "generate_netherite_tools".to_string(), "generate_netherite_armor_models".to_string(), "generate_smithing_ui".to_string()]);
        forward.insert((5, 6), vec!["delete_font_folder".to_string()]);
        forward.insert((7, 8), vec!["rename_mcpatcher_to_optifine".to_string()]);
        forward.insert((8, 9), vec![]);
        forward.insert((9, 12), vec!["fix_tabs".to_string(), "generate_redwood_cherry_bamboo_planks".to_string()]);
        forward.insert((12, 13), vec!["fix_smithing2_villager2_ui".to_string(), "fix_slider".to_string()]);
        forward.insert((13, 15), vec![]);
        forward.insert((15, 18), vec!["cut_gui".to_string()]);
        forward.insert((18, 22), vec![]);
        forward.insert((22, 32), vec!["adapt_java_shaders".to_string()]);
        forward.insert((32, 34), vec!["generate_tricky_trials_breeze".to_string(), "adapt_java_shaders".to_string()]);
        forward.insert((34, 42), vec!["adapt_java_shaders".to_string()]);
        // 1.17 着色器体系边界（format 7）：6→7 既要生成雪球贴图，也要换着色器体系，
        // 两件事必须写在同一条 insert 里 —— HashMap::insert 是覆盖语义，
        // 拆成两条会让先写的那条静默失效（此前的 generate_snow_bucket 就是这样丢的）。
        forward.insert((6, 7), vec!["generate_snow_bucket".to_string(), "adapt_java_shaders".to_string()]);
        reverse.insert((7, 6), vec!["adapt_java_shaders".to_string()]);
        forward.insert((42, 46), vec!["fix2_horse_ui".to_string(), "fix_armor_models".to_string(), "generate_pale_planks".to_string(), "adapt_java_shaders".to_string()]);
        forward.insert((46, 55), vec!["adapt_java_shaders".to_string()]);
        forward.insert((55, 63), vec![]);
        forward.insert((63, 64), vec![]);
        forward.insert((64, 69), vec!["generate_copper_ingot".to_string(), "generate_copper_block".to_string(), "generate_copper_tools".to_string(), "generate_copper_armor_models".to_string()]);
        forward.insert((69, 75), vec!["adapt_java_shaders".to_string()]);
        forward.insert((75, 84), vec!["adapt_java_shaders".to_string()]);
        forward.insert((84, 88), vec!["adapt_java_shaders".to_string()]);
        forward.insert((88, 97), vec!["generate_poplar_planks".to_string(), "adapt_java_shaders".to_string()]);
        // Bedrock：最新 Java 26.3（97）↔ 1000
        forward.insert((84, 1000), vec!["bedrock_java_to_bedrock".to_string()]);
        forward.insert((88, 1000), vec!["bedrock_java_to_bedrock".to_string()]);
        forward.insert((97, 1000), vec!["bedrock_java_to_bedrock".to_string()]);
        reverse.insert((1000, 97), vec!["bedrock_bedrock_to_java".to_string()]);
        reverse.insert((1000, 88), vec!["bedrock_bedrock_to_java".to_string()]);
        reverse.insert((1000, 84), vec!["bedrock_bedrock_to_java".to_string()]);

        reverse.insert((97, 88), vec!["reverse_generate_poplar_planks".to_string(), "adapt_java_shaders".to_string()]);
        reverse.insert((88, 84), vec!["adapt_java_shaders".to_string()]);
        reverse.insert((84, 75), vec!["adapt_java_shaders".to_string()]);
        reverse.insert((75, 69), vec!["adapt_java_shaders".to_string()]);
        // (69,64) 的铜材质逆变换在下方统一登记，此处不再写空表覆盖
        reverse.insert((64, 63), vec![]);
        reverse.insert((63, 55), vec![]);
        reverse.insert((55, 46), vec!["adapt_java_shaders".to_string()]);
        reverse.insert((46, 42), vec!["reverse_fix_armor_models".to_string(), "reverse_fix2_horse_ui".to_string(), "reverse_generate_pale_planks".to_string(), "adapt_java_shaders".to_string()]);
        reverse.insert((42, 34), vec!["reverse_fix2_horse_ui".to_string(), "adapt_java_shaders".to_string()]);
        reverse.insert((34, 32), vec!["reverse_generate_tricky_trials_breeze".to_string(), "adapt_java_shaders".to_string()]);
        reverse.insert((32, 22), vec!["adapt_java_shaders".to_string()]);
        reverse.insert((22, 18), vec![]);
        reverse.insert((18, 15), vec!["reverse_cut_gui".to_string()]);
        reverse.insert((15, 13), vec![]);
        reverse.insert((13, 12), vec!["reverse_fix_smithing2_villager2_ui".to_string(), "reverse_fix_slider".to_string()]);
        reverse.insert((12, 9), vec!["reverse_generate_redwood_cherry_bamboo_planks".to_string()]);
        reverse.insert((9, 8), vec![]);
        reverse.insert((8, 7), vec!["reverse_rename_mcpatcher_to_optifine".to_string()]);
        // (7,6) 已在着色器边界处登记 adapt_java_shaders，此处不可再 insert 空表覆盖
        reverse.insert((6, 5), vec!["reverse_generate_snow_bucket".to_string()]);
        reverse.insert((69, 64), vec!["reverse_generate_copper_ingot".to_string(), "reverse_generate_copper_block".to_string(), "reverse_generate_copper_tools".to_string(), "reverse_generate_copper_armor_models".to_string()]);
        reverse.insert((5, 4), vec!["reverse_process_chest_folder".to_string(), "reverse_generate_netherite_block".to_string(), "reverse_generate_netherite_ingot".to_string(), "reverse_generate_netherite_tools".to_string(), "reverse_generate_netherite_armor_models".to_string(), "reverse_generate_smithing_ui".to_string()]);
        reverse.insert((4, 3), vec!["reverse_rename_blocks_items".to_string(), "reverse_fix_sign".to_string(), "reverse_fix_sign_entities".to_string(), "reverse_generate_furnace".to_string(), "reverse_fix_machinery_ui".to_string(), "reverse_fix_particles".to_string(), "reverse_generate_fish_bucket".to_string(), "reverse_generate_crossbow".to_string()]);
        reverse.insert((3, 2), vec!["reverse_fix_horse_ui".to_string(), "delete_horse_folder".to_string()]);
        reverse.insert((2, 1), vec!["delete_blockstates_models".to_string(), "reverse_generate_tipped_arrow_images".to_string(), "reverse_fix_ui_survival".to_string(), "reverse_fix_ui_creative".to_string(), "reverse_fix_ui_sub_hand".to_string(), "reverse_generate_boat".to_string(), "reverse_generate_potion_lingering".to_string(), "reverse_generate_shulker_box_ui".to_string(), "reverse_fix_brewing_stand_ui".to_string(), "reverse_fix_clock_compass".to_string(), "reverse_overlay_icons".to_string()]);

        Self { forward, reverse }
    }
}

