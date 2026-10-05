use std::path::Path;

use crate::hurray::scheduler::Scheduler;

// ============================================================================
// 旧闭包实现（`legacy-oracle` feature，默认关闭）
// ============================================================================
//
// §9.125（M3 收口）：**元数据**（名字 / 并发类型 / 阶段）已移到 `crate::task_registry`，
// 生产路径只依赖那张表。本模块只剩下「旧闭包体」——它们是 `legacy` / `off` 两个对照
// 配置的实现（§9.123 实测：删掉闭包，冻结基线从 4018 条目掉到 3792），
// 因此用 `legacy-oracle` 门控：
//
// | 构建 | 旧闭包 | 88 项元数据 | 用途 |
// |---|---|---|---|
// | 默认（无 feature） | **不编译** | 有 | 生产二进制；`on` 与冻结基线对照 |
// | `--features legacy-oracle` | 编译 | 有 | `legacy` / `off` 对照 + 基线**重生成** |
//
// 那 71 个 `use crate::converters::…` 与闭包体一起被门控，因此**默认构建不含任何旧转换器代码**。
#[cfg(feature = "legacy-oracle")]
mod legacy {
    use std::path::Path;

    use crate::hurray::context::HurrayContext;
    use crate::hurray::scheduler::Scheduler;
    use crate::task_registry;

    // ========== 映射表模块导入（仅 pack.py ADJACENT_CONVERSIONS 中的任务） ==========
    // Eraser 层
    use crate::converters::textures::animated;
    use crate::converters::textures::drop_blockstates_models;
    use crate::converters::textures::drop_enchanted_glint;
    use crate::converters::textures::drop_font;
    use crate::converters::textures::drop_horse;
    use crate::converters::textures::drop_shaders;
    use crate::converters::textures::mcpatcher_to_optifine;
    use crate::converters::textures::rename_blocks;
    use crate::converters::ui::process_chest_folder;

    // Architect 层 —— generate_*
    use crate::converters::textures::boat;
    use crate::converters::textures::breeze;
    use crate::converters::textures::copper;
    use crate::converters::textures::crossbow;
    use crate::converters::textures::fish_bucket;
    use crate::converters::textures::furnace;
    use crate::converters::textures::netherite;
    use crate::converters::textures::planks;
    use crate::converters::textures::potion_lingering;
    use crate::converters::textures::snow_bucket;
    use crate::converters::textures::tipped_arrows;
    use crate::converters::ui::shulker_box;
    use crate::converters::ui::smithing_ui;

    // Surgeon 层 —— fix_* / overlay_icons / cut_gui
    // §9.101：`cut_gui` 的闭包体已换成 `crate::natives::surgeon_cut_gui`，旧模块不再被本文件引用。
    use crate::converters::textures::armor as armor_tex;
    use crate::converters::textures::particles as particles_tex;
    use crate::converters::ui::brewing_stand;
    use crate::converters::ui::clock_compass;
    use crate::converters::ui::creative;
    use crate::converters::ui::horse;
    use crate::converters::ui::horse_v2;
    use crate::converters::ui::machinery;
    use crate::converters::ui::overlay_icons;
    use crate::converters::ui::sign;
    use crate::converters::ui::sign_entities;
    use crate::converters::ui::slider;
    use crate::converters::ui::smithing_villager;
    use crate::converters::ui::sub_hand;
    use crate::converters::ui::survival;
    use crate::converters::ui::tabs;

    // 逆向转换（与 ui/textures 同名，用 rev_ 前缀）
    use crate::converters::reverse::armor as rev_armor;
    use crate::converters::reverse::boat as rev_boat;
    use crate::converters::reverse::brewing_stand as rev_brewing_stand;
    use crate::converters::reverse::breeze as rev_breeze;
    use crate::converters::reverse::chest_folder as rev_chest_folder;
    use crate::converters::reverse::clock_compass as rev_clock_compass;
    use crate::converters::reverse::copper as rev_copper;
    use crate::converters::reverse::creative as rev_creative;
    use crate::converters::reverse::crossbow as rev_crossbow;
    use crate::converters::reverse::cut_gui as rev_cut_gui;
    use crate::converters::reverse::fish_bucket as rev_fish_bucket;
    use crate::converters::reverse::furnace as rev_furnace;
    use crate::converters::reverse::horse as rev_horse;
    use crate::converters::reverse::horse_v2 as rev_horse_v2;
    use crate::converters::reverse::machinery as rev_machinery;
    use crate::converters::reverse::mcpatcher_to_optifine as rev_mcpatcher_to_optifine;
    use crate::converters::reverse::netherite as rev_netherite;
    use crate::converters::reverse::overlay_icons as rev_overlay_icons;
    use crate::converters::reverse::particles as rev_particles;
    use crate::converters::reverse::planks as rev_planks;
    use crate::converters::reverse::potion_lingering as rev_potion_lingering;
    use crate::converters::reverse::rename_blocks as rev_rename_blocks;
    use crate::converters::reverse::shulker_box as rev_shulker_box;
    use crate::converters::reverse::sign as rev_sign;
    use crate::converters::reverse::sign_entities as rev_sign_entities;
    use crate::converters::reverse::slider as rev_slider;
    use crate::converters::reverse::smithing_ui as rev_smithing_ui;
    use crate::converters::reverse::smithing_villager as rev_smithing_villager;
    use crate::converters::reverse::snow_bucket as rev_snow_bucket;
    use crate::converters::reverse::sub_hand as rev_sub_hand;
    use crate::converters::reverse::survival as rev_survival;
    use crate::converters::reverse::tabs as rev_tabs;
    use crate::converters::reverse::tipped_arrows as rev_tipped_arrows;

    use crate::{log_debug, log_info};

    /// 关掉 shaders 适配开关时打印的信息（与旧实现逐字一致）。
    pub(super) fn log_adapt_shaders_disabled() {
        log_info!("adapt_java_shaders disabled by user (experimental)");
    }

    /// 收尾日志（与旧实现逐字一致）。
    pub(super) fn log_all_registered() {
        log_debug!("all mapping table tasks registered");
    }

    /// 把 88 个任务的旧闭包按**元数据表的顺序**注册进调度器。
    ///
    /// 为什么在这里 `match` 名字而不是把闭包放进 `task_registry`：
    /// 元数据表要能被生产路径使用，就不能携带任何旧转换器的符号。名字是两张表之间的
    /// 唯一接口，`match` 里缺一项会在**编译期**报 `unreachable` 之外的问题——
    /// 运行时则由「注册结果与元数据表逐项一致」的用例（`task_registry_matches_metadata`）兜住。
    pub(super) fn register(
        scheduler: &mut Scheduler,
        target_path: &Path,
        target_version: u32,
        source_version: u32,
        run_gui_surgeon: bool,
        fix_alpha_layers: bool,
        adapt_shaders: bool,
    ) {
        let _ = (target_path, target_version, source_version, run_gui_surgeon);

        // ============================================================
        // 以下任务注册严格对齐 pack.py ADJACENT_CONVERSIONS 映射表
        // ============================================================

        if fix_alpha_layers {
            crate::converters::textures::alpha_layers::register_scheduler_task(scheduler);
        }

        // 与元数据表**同一顺序**（顺序即语义：同阶段内的执行顺序取决于它）。
        for meta in task_registry::REGISTRY {
            let name = meta.name;
            let task_type = meta.task_type.clone();
            let tier = meta.tier;
            scheduler.register_task(name, task_type, tier, move |ctx: &HurrayContext| {
                match name {
                    // ── Eraser 层：删除旧结构，必须串行 ──
                    //
                    // 动态贴图 mcmeta 升级：必须在 rename_blocks_items（items → item 归一）
                    // 之后执行——同层按注册顺序串行。老版 {"animation": {}} 的 .png.mcmeta
                    // 按同名 png 尺寸推导帧数，改写为 { frametime, interpolate } 高版本格式。
                    "convert_animated_textures" => {
                        let temp_dir = ctx.temp_dir();
                        animated::convert_animated_textures(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "rename_blocks_items" => {
                        let temp_dir = ctx.temp_dir();
                        rename_blocks::rename_blocks_items(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "delete_blockstates_models" => drop_blockstates_models::delete_blockstates_models(ctx)
                        .map_err(|e| e.to_string()),
                    "delete_horse_folder" => drop_horse::delete_horse_folder(ctx)
                        .map_err(|e| e.to_string()),
                    "delete_enchanted_item_glint" => drop_enchanted_glint::delete_enchanted_item_glint(ctx)
                        .map_err(|e| e.to_string()),
                    "delete_shaders_folder" => drop_shaders::delete_shaders_folder(ctx)
                        .map_err(|e| e.to_string()),
                    "delete_font_folder" => drop_font::delete_font_folder(ctx)
                        .map_err(|e| e.to_string()),
                    "process_chest_folder" => {
                        let temp_dir = ctx.temp_dir();
                        process_chest_folder::process_chest_folder(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "rename_mcpatcher_to_optifine" => {
                        let temp_dir = ctx.temp_dir();
                        mcpatcher_to_optifine::rename_mcpatcher_to_optifine(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "reverse_rename_blocks_items" => {
                        let temp_dir = ctx.temp_dir();
                        rev_rename_blocks::reverse_rename_blocks_items(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "reverse_process_chest_folder" => {
                        let temp_dir = ctx.temp_dir();
                        rev_chest_folder::reverse_process_chest_folder(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }

                    // ── Architect 层：生成新资源，可并行 ──
                    "generate_tipped_arrow_images" => tipped_arrows::generate_tipped_arrow_images(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_boat" => boat::generate_boat(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_potion_lingering" => potion_lingering::generate_potion_lingering(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_shulker_box_ui" => shulker_box::generate_shulker_box_ui(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_furnace" => furnace::generate_furnace(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_fish_bucket" => fish_bucket::generate_fish_bucket(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_crossbow" => crossbow::generate_crossbow(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_netherite_block" => netherite::generate_netherite_block(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_netherite_ingot" => netherite::generate_netherite_ingot(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_netherite_tools" => netherite::generate_netherite_tools(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_netherite_armor_models" => netherite::generate_netherite_armor_models(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_copper_ingot" => copper::generate_copper_ingot(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_copper_block" => copper::generate_copper_block(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_copper_tools" => copper::generate_copper_tools(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_copper_armor_models" => copper::generate_copper_armor_models(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_snow_bucket" => snow_bucket::generate_snow_bucket(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_smithing_ui" => smithing_ui::generate_smithing_ui(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_redwood_cherry_bamboo_planks" => planks::generate_redwood_cherry_bamboo_planks(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_pale_planks" => planks::generate_pale_planks(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_poplar_planks" => planks::generate_poplar_planks(ctx.temp_dir())
                        .map_err(|e| e.to_string()),
                    "generate_tricky_trials_breeze" => breeze::generate_tricky_trials_breeze(ctx.temp_dir())
                        .map_err(|e| e.to_string()),

                    // ── Surgeon 层：修改已有资源（Hybrid：并行内部安全操作 + 串行独占操作） ──
                    "fix_clock_compass" => clock_compass::fix_clock_compass(ctx)
                        .map_err(|e| e.to_string()),
                    "fix_brewing_stand_ui" => {
                        let temp_dir = ctx.temp_dir();
                        brewing_stand::fix_brewing_stand_ui(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "fix_particles" => {
                        let temp_dir = ctx.temp_dir();
                        particles_tex::fix_particles(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "fix_sign" => sign::fix_sign(ctx)
                        .map_err(|e| e.to_string()),
                    "fix_sign_entities" => {
                        let temp_dir = ctx.temp_dir();
                        sign_entities::fix_sign_entities(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "fix_ui_creative" => creative::fix_ui_creative(ctx)
                        .map_err(|e| e.to_string()),
                    "fix_ui_sub_hand" => sub_hand::fix_ui_sub_hand(ctx)
                        .map_err(|e| e.to_string()),
                    "fix_ui_survival" => survival::fix_ui_survival(ctx)
                        .map_err(|e| e.to_string()),
                    "fix_armor_models" => {
                        let temp_dir = ctx.temp_dir();
                        armor_tex::fix_armor_models(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "fix_horse_ui" => {
                        let temp_dir = ctx.temp_dir();
                        horse::fix_horse_ui(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "fix2_horse_ui" => {
                        let temp_dir = ctx.temp_dir();
                        horse_v2::fix2_horse_ui(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "fix_machinery_ui" => {
                        let temp_dir = ctx.temp_dir();
                        machinery::fix_machinery_ui(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "fix_tabs" => {
                        let temp_dir = ctx.temp_dir();
                        tabs::fix_tabs(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "fix_slider" => {
                        let temp_dir = ctx.temp_dir();
                        slider::fix_slider(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "fix_smithing2_villager2_ui" => {
                        let temp_dir = ctx.temp_dir();
                        smithing_villager::fix_smithing2_villager2_ui(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "overlay_icons" => overlay_icons::overlay_icons(ctx)
                        .map_err(|e| e.to_string()),
                    "cut_gui" => {
                        // **§9.101：闭包体换成原生实现**（`natives::surgeon_cut_gui`）。
                        //
                        // 这里只换实现、**不动位置**——仍由 `run_named` 在计划的 `(15,18)` 调用。
                        // §9.100 的实测教训：`cut_gui` 一旦被挪出旧批次（哪怕只是挪到批次末尾）产物就会少
                        // 3 个 sprite（`sprites/container/slot/{horse_armor,llama_armor,saddle}.png`），
                        // 因为它的输入 `gui/container/*.png` 在那些时刻的状态不同（§9.50 的同一规律）。
                        //
                        // 旧闭包体是 `cut_gui::cut_gui(ctx)`——一个 14 行的薄包装，内容与
                        // `surgeon_cut_gui::run_in_workdir` 逐句相同（含 `TexturePool` / `ResolutionTransducer`
                        // 的构造与 `commit_all`），因此**行为等价**；由 `native_switch` 的相对闸门把关。
                        crate::natives::surgeon_cut_gui::run_in_workdir(ctx, Path::new(ctx.temp_dir()))
                            .map_err(|e| e.to_string())
                    }

                    // 逆向 Surgeon
                    "reverse_fix_armor_models" => {
                        let temp_dir = ctx.temp_dir();
                        rev_armor::reverse_fix_armor_models(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "reverse_fix_brewing_stand_ui" => {
                        let temp_dir = ctx.temp_dir();
                        rev_brewing_stand::reverse_fix_brewing_stand_ui(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "reverse_fix_clock_compass" => rev_clock_compass::reverse_fix_clock_compass(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_fix_particles" => rev_particles::reverse_fix_particles(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_fix_ui_creative" => {
                        let temp_dir = ctx.temp_dir();
                        rev_creative::reverse_fix_ui_creative(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }
                    "reverse_fix_ui_survival" => {
                        let temp_dir = ctx.temp_dir();
                        rev_survival::reverse_fix_ui_survival(Path::new(temp_dir))
                            .map_err(|e| e.to_string())
                    }

                    // ── 新增逆向任务（reverse generate_*：删除生成物） ──
                    "reverse_generate_boat" => rev_boat::reverse_generate_boat(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_potion_lingering" => rev_potion_lingering::reverse_generate_potion_lingering(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_shulker_box_ui" => rev_shulker_box::reverse_generate_shulker_box_ui(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_furnace" => rev_furnace::reverse_generate_furnace(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_netherite_block" => rev_netherite::reverse_generate_netherite_block(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_netherite_ingot" => rev_netherite::reverse_generate_netherite_ingot(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_netherite_tools" => rev_netherite::reverse_generate_netherite_tools(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_netherite_armor_models" => rev_netherite::reverse_generate_netherite_armor_models(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_copper_ingot" => rev_copper::reverse_generate_copper_ingot(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_copper_block" => rev_copper::reverse_generate_copper_block(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_copper_tools" => rev_copper::reverse_generate_copper_tools(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_copper_armor_models" => rev_copper::reverse_generate_copper_armor_models(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_smithing_ui" => rev_smithing_ui::reverse_generate_smithing_ui(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_crossbow" => rev_crossbow::reverse_generate_crossbow(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_fish_bucket" => rev_fish_bucket::reverse_generate_fish_bucket(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_snow_bucket" => rev_snow_bucket::reverse_generate_snow_bucket(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_tipped_arrow_images" => rev_tipped_arrows::reverse_generate_tipped_arrow_images(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_redwood_cherry_bamboo_planks" => rev_planks::reverse_generate_redwood_cherry_bamboo_planks(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_pale_planks" => rev_planks::reverse_generate_pale_planks(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_poplar_planks" => rev_planks::reverse_generate_poplar_planks(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_generate_tricky_trials_breeze" => rev_breeze::reverse_generate_tricky_trials_breeze(ctx)
                        .map_err(|e| e.to_string()),

                    // reverse rename
                    "reverse_rename_mcpatcher_to_optifine" => {
                        rev_mcpatcher_to_optifine::reverse_rename_mcpatcher_to_optifine(Path::new(ctx.temp_dir()))
                            .map_err(|e| e.to_string())
                    }

                    // reverse fix_*（删除生成物或空操作）
                    "reverse_fix_sign" => rev_sign::reverse_fix_sign(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_fix_sign_entities" => rev_sign_entities::reverse_fix_sign_entities(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_fix_slider" => rev_slider::reverse_fix_slider(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_fix_tabs" => rev_tabs::reverse_fix_tabs(Path::new(ctx.temp_dir()))
                        .map_err(|e| e.to_string()),
                    "reverse_fix_horse_ui" => rev_horse::reverse_fix_horse_ui(Path::new(ctx.temp_dir()))
                        .map_err(|e| e.to_string()),
                    "reverse_fix2_horse_ui" => rev_horse_v2::reverse_fix2_horse_ui(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_fix_machinery_ui" => rev_machinery::reverse_fix_machinery_ui(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_fix_ui_sub_hand" => rev_sub_hand::reverse_fix_ui_sub_hand(Path::new(ctx.temp_dir()))
                        .map_err(|e| e.to_string()),
                    "reverse_fix_smithing2_villager2_ui" => rev_smithing_villager::reverse_fix_smithing2_villager2_ui(ctx)
                        .map_err(|e| e.to_string()),
                    "reverse_overlay_icons" => rev_overlay_icons::reverse_overlay_icons(Path::new(ctx.temp_dir()))
                        .map_err(|e| e.to_string()),
                    "reverse_cut_gui" => rev_cut_gui::reverse_cut_gui(Path::new(ctx.temp_dir()))
                        .map_err(|e| e.to_string()),

                    other => Err(format!(
                        "legacy closure missing for registered task `{other}`（元数据表与闭包表漂移）"
                    )),
                }
            });
        }

        // ── 开关驱动的附加注册（顺序与旧实现一致：`fix_alpha_layers` 在 Eraser 段之后、
        //    `adapt_java_shaders` 在 Eraser 段之内）──
        if adapt_shaders {
            crate::converters::shaders::java::register_scheduler_task(scheduler);
        } else {
            log_adapt_shaders_disabled();
        }

        log_all_registered();
    }
}

/// 注册全部转换任务（**这是「谁跑什么」的单一来源**）。
///
/// 分两段：
///
/// 1. **元数据**（总是执行）：按 [`crate::task_registry::REGISTRY`] 的顺序把
///    `(名字, 并发类型, 阶段)` 登记进调度器。生产驱动 `native_run` 用
///    [`Scheduler::task_tier`] 决定每个任务的**放置侧**，因此这一段**必须**在默认构建里跑；
///    它不引用任何旧转换器代码。
/// 2. **旧闭包实现**（仅 `legacy-oracle`）：`legacy` / `off` 两个对照配置的实现体，
///    也是冻结基线的**重生成**能力（§9.123）。
///
/// 参数即闭包会捕获的全部上下文——注册发生在 `HurrayContext` 创建之前，因此不含 context。
#[allow(clippy::too_many_arguments)]
pub fn register_tasks(
    scheduler: &mut Scheduler,
    target_path: &Path,
    target_version: u32,
    source_version: u32,
    run_gui_surgeon: bool,
    fix_alpha_layers: bool,
    adapt_shaders: bool,
) {
    init_meta_tasks(scheduler);

    #[cfg(feature = "legacy-oracle")]
    legacy::register(
        scheduler,
        target_path,
        target_version,
        source_version,
        run_gui_surgeon,
        fix_alpha_layers,
        adapt_shaders,
    );

    // 默认构建（无 feature）下这些参数只用于元数据段，此处显式忽略以免告警。
    #[cfg(not(feature = "legacy-oracle"))]
    {
        install_native_defaults(scheduler);

        let _ = (
            target_path,
            target_version,
            source_version,
            run_gui_surgeon,
            fix_alpha_layers,
            adapt_shaders,
        );
    }
}

/// **默认构建下的「原生默认实现」**（§9.125）。
///
/// 旧闭包被门控后，88 个名字仍然要能被执行——因为它们**本来就是生产要跑的任务**。
/// 绝大多数名字在生产配置（`NativeSwitches::all()`）下由驱动直接派发到
/// `natives::*`，**不经过**注册表闭包；唯一例外是
/// [`CUT_GUI`](crate::native_run) `cut_gui`：
///
/// §9.100/§9.102 定下它**必须留在旧批次的 `(15,18)` 槽位**，因此驱动是把它当
/// 「适配层任务」交给注册表闭包跑的（`native_run.rs` 的 `run_named` 路径）。
/// 默认构建里若给它装一个空实现，产物会**少 3 个 sprite**
/// （`gui/sprites/container/slot/{horse_armor,llama_armor,saddle}.png`）——
/// 这不是推测：交接文档记录过同样的 3 项缺失，本轮也实测复现（4015 条目 / `0xc07855d73488424d`）。
/// 批次后的直连步骤**不能**替代它：§9.90/§9.91 实测「两者各自必需」。
///
/// 因此这里把它接到**原生实现** `natives::surgeon_cut_gui::run_in_workdir`
/// （§9.101 起闭包体就是它，逐句相同），于是默认构建既不缺 sprite，也不含旧转换器代码。
#[cfg(not(feature = "legacy-oracle"))]
fn install_native_defaults(scheduler: &mut Scheduler) {
    use crate::hurray::context::HurrayContext;

    let meta = crate::task_registry::lookup("cut_gui")
        .expect("`cut_gui` 必须在元数据表里（本函数依赖它的阶段）");
    let task_type = meta.task_type.clone();
    scheduler.register_task(
        "cut_gui",
        task_type,
        meta.tier,
        |ctx: &HurrayContext| {
            crate::natives::surgeon_cut_gui::run_in_workdir(ctx, std::path::Path::new(ctx.temp_dir()))
                .map_err(|e| e.to_string())
        },
    );
}

/// 与开关无关、也**不属于** [`crate::task_registry::REGISTRY`] 的注册项。
///
/// 它们不是「88 项旧闭包」的一部分：Bedrock 边任务（j2b/b2j）是**生产功能**，
/// 注册项与实现都在 `bedrock_convert`（其阶段同样登记在 `task_registry::AUXILIARY`）。
///
/// 顺序与旧实现一致：Bedrock 两项仍是最后注册的。
fn init_meta_tasks(scheduler: &mut Scheduler) {
    use crate::log_debug;

    // 基岩 ↔ Java 结构转换：逻辑在 `bedrock_convert/*`，此处仅注册到 Scheduler
    crate::bedrock_convert::register_tasks(scheduler);

    log_debug!("all mapping table tasks registered");
}
