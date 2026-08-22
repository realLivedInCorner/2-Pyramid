use std::error::Error;
use std::fs;
use std::path::Path;

use crate::hurray::context::HurrayContext;
use crate::hurray::scheduler::{Scheduler, TaskTier, TaskType};
use crate::hurray::texture::TexturePool;

// ========== 映射表模块导入（仅 pack.py ADJACENT_CONVERSIONS 中的任务） ==========
// Eraser 层
use crate::converters::delete_blockstates_models;
use crate::converters::delete_enchanted_item_glint;
use crate::converters::delete_font_folder;
use crate::converters::delete_horse_folder;
use crate::converters::delete_shaders_folder;
use crate::converters::rename_blocks_items;
use crate::converters::convert_animated_textures;
use crate::converters::rename_mcpatcher_to_optifine;
use crate::converters::process_chest_folder;

// Architect 层 —— generate_*
use crate::converters::generate_boat;
use crate::converters::generate_copper;
use crate::converters::generate_crossbow;
use crate::converters::generate_fish_bucket;
use crate::converters::generate_furnace;
use crate::converters::generate_netherite;
use crate::converters::generate_planks;
use crate::converters::generate_potion_lingering;
use crate::converters::generate_shulker_box_ui;
use crate::converters::generate_smithing_ui;
use crate::converters::generate_snow_bucket;
use crate::converters::generate_tipped_arrow_images;

// Surgeon 层 —— fix_* / overlay_icons / cut_gui
use crate::converters::cut_gui;
use crate::converters::fix2_horse_ui;
use crate::converters::fix_armor_models;
use crate::converters::fix_brewing_stand_ui;
use crate::converters::fix_clock_compass;
use crate::converters::fix_horse_ui;
use crate::converters::fix_machinery_ui;
use crate::converters::fix_particles;
use crate::converters::fix_sign;
use crate::converters::fix_sign_entities;
use crate::converters::fix_slider;
use crate::converters::fix_smithing2_villager2_ui;
use crate::converters::fix_tabs;
use crate::converters::fix_ui_creative;
use crate::converters::fix_ui_sub_hand;
use crate::converters::fix_ui_survival;
use crate::converters::overlay_icons;

// 逆向转换
use crate::converters::reverse_fix_armor_models;
use crate::converters::reverse_fix_brewing_stand_ui;
use crate::converters::reverse_fix_clock_compass;
use crate::converters::reverse_fix_particles;
use crate::converters::reverse_fix_ui_creative;
use crate::converters::reverse_fix_ui_survival;
use crate::converters::reverse_process_chest_folder;
use crate::converters::reverse_rename_blocks_items;
use crate::converters::reverse_cut_gui;
use crate::converters::reverse_fix2_horse_ui;
use crate::converters::reverse_fix_horse_ui;
use crate::converters::reverse_fix_machinery_ui;
use crate::converters::reverse_fix_sign;
use crate::converters::reverse_fix_sign_entities;
use crate::converters::reverse_fix_slider;
use crate::converters::reverse_fix_smithing2_villager2_ui;
use crate::converters::reverse_fix_tabs;
use crate::converters::reverse_fix_ui_sub_hand;
use crate::converters::reverse_generate_boat;
use crate::converters::reverse_generate_copper;
use crate::converters::reverse_generate_crossbow;
use crate::converters::reverse_generate_fish_bucket;
use crate::converters::reverse_generate_furnace;
use crate::converters::reverse_generate_netherite;
use crate::converters::reverse_generate_planks;
use crate::converters::reverse_generate_potion_lingering;
use crate::converters::reverse_generate_shulker_box_ui;
use crate::converters::reverse_generate_smithing_ui;
use crate::converters::reverse_generate_snow_bucket;
use crate::converters::reverse_generate_tipped_arrow_images;
use crate::converters::reverse_overlay_icons;
use crate::converters::reverse_rename_mcpatcher_to_optifine;

pub fn invoke_conversion(
    target_path: &Path,
    work_dir: &Path,
    target_version: u32,
    source_version: u32,
) -> Result<(), Box<dyn Error>> {
    use crate::{log_info, log_debug, log_warn};
    log_info!("==============================");
    log_info!("2-Pyramid DTD engine start");
    log_info!("source pack_format = {}", source_version);
    log_info!("target pack_format = {}", target_version);
    log_info!("work_dir = {}", work_dir.display());
    log_info!("==============================");

    if !work_dir.exists() {
        log_info!("create work_dir: {}", work_dir.display());
        fs::create_dir_all(work_dir)?;
    }

    let mut scheduler = Scheduler::new();

    // ============================================================
    // 以下任务注册严格对齐 pack.py ADJACENT_CONVERSIONS 映射表
    // ============================================================

    // ── Eraser 层：删除旧结构，必须串行 ──
    scheduler.register_task("rename_blocks_items", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        let temp_dir = ctx.temp_dir();
        rename_blocks_items::rename_blocks_items(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    // 动态贴图 mcmeta 升级：必须在 rename_blocks_items（items → item 归一）
    // 之后执行——同层按注册顺序串行。老版 {"animation": {}} 的 .png.mcmeta
    // 按同名 png 尺寸推导帧数，改写为 { frametime, interpolate } 高版本格式。
    scheduler.register_task("convert_animated_textures", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        let temp_dir = ctx.temp_dir();
        convert_animated_textures::convert_animated_textures(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("delete_blockstates_models", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        delete_blockstates_models::delete_blockstates_models(ctx)
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("delete_horse_folder", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        delete_horse_folder::delete_horse_folder(ctx)
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("delete_enchanted_item_glint", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        delete_enchanted_item_glint::delete_enchanted_item_glint(ctx)
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("delete_shaders_folder", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        delete_shaders_folder::delete_shaders_folder(ctx)
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("delete_font_folder", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        delete_font_folder::delete_font_folder(ctx)
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("process_chest_folder", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        let temp_dir = ctx.temp_dir();
        process_chest_folder::process_chest_folder(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("rename_mcpatcher_to_optifine", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        let temp_dir = ctx.temp_dir();
        rename_mcpatcher_to_optifine::rename_mcpatcher_to_optifine(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    // 逆向 Eraser
    scheduler.register_task("reverse_rename_blocks_items", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        let temp_dir = ctx.temp_dir();
        reverse_rename_blocks_items::reverse_rename_blocks_items(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("reverse_process_chest_folder", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        let temp_dir = ctx.temp_dir();
        reverse_process_chest_folder::reverse_process_chest_folder(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    // ── Architect 层：生成新资源，可并行 ──
    scheduler.register_task("generate_tipped_arrow_images", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_tipped_arrow_images::generate_tipped_arrow_images(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_boat", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_boat::generate_boat(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_potion_lingering", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_potion_lingering::generate_potion_lingering(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_shulker_box_ui", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_shulker_box_ui::generate_shulker_box_ui(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_furnace", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_furnace::generate_furnace(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_fish_bucket", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_fish_bucket::generate_fish_bucket(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_crossbow", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_crossbow::generate_crossbow(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_netherite_block", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_netherite::generate_netherite_block(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_netherite_ingot", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_netherite::generate_netherite_ingot(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_netherite_tools", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_netherite::generate_netherite_tools(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_netherite_armor_models", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_netherite::generate_netherite_armor_models(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_copper_ingot", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_copper::generate_copper_ingot(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_copper_block", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_copper::generate_copper_block(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_copper_tools", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_copper::generate_copper_tools(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_copper_armor_models", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_copper::generate_copper_armor_models(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_snow_bucket", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_snow_bucket::generate_snow_bucket(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_smithing_ui", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_smithing_ui::generate_smithing_ui(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_redwood_cherry_bamboo_planks", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_planks::generate_redwood_cherry_bamboo_planks(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("generate_pale_planks", TaskType::Parallel, TaskTier::Architect, |ctx| {
        generate_planks::generate_pale_planks(ctx.temp_dir())
            .map_err(|e| e.to_string())
    });

    // ── Surgeon 层：修改已有资源，Hybrid（并行内部安全操作 + 串行独占操作） ──
    scheduler.register_task("fix_clock_compass", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        fix_clock_compass::fix_clock_compass(ctx)
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("fix_brewing_stand_ui", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        let temp_dir = ctx.temp_dir();
        fix_brewing_stand_ui::fix_brewing_stand_ui(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("fix_particles", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        let temp_dir = ctx.temp_dir();
        fix_particles::fix_particles(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("fix_sign", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        fix_sign::fix_sign(ctx)
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("fix_sign_entities", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        let temp_dir = ctx.temp_dir();
        fix_sign_entities::fix_sign_entities(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("fix_ui_creative", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        fix_ui_creative::fix_ui_creative(ctx)
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("fix_ui_sub_hand", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        fix_ui_sub_hand::fix_ui_sub_hand(ctx)
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("fix_ui_survival", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        fix_ui_survival::fix_ui_survival(ctx)
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("fix_armor_models", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        let temp_dir = ctx.temp_dir();
        fix_armor_models::fix_armor_models(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("fix_horse_ui", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        let temp_dir = ctx.temp_dir();
        fix_horse_ui::fix_horse_ui(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("fix2_horse_ui", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        let temp_dir = ctx.temp_dir();
        fix2_horse_ui::fix2_horse_ui(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("fix_machinery_ui", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        let temp_dir = ctx.temp_dir();
        fix_machinery_ui::fix_machinery_ui(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("fix_tabs", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        let temp_dir = ctx.temp_dir();
        fix_tabs::fix_tabs(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("fix_slider", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        let temp_dir = ctx.temp_dir();
        fix_slider::fix_slider(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("fix_smithing2_villager2_ui", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        let temp_dir = ctx.temp_dir();
        fix_smithing2_villager2_ui::fix_smithing2_villager2_ui(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("overlay_icons", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        overlay_icons::overlay_icons(ctx)
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("cut_gui", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        cut_gui::cut_gui(ctx)
            .map_err(|e| e.to_string())
    });

    // 逆向 Surgeon
    scheduler.register_task("reverse_fix_armor_models", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        let temp_dir = ctx.temp_dir();
        reverse_fix_armor_models::reverse_fix_armor_models(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("reverse_fix_brewing_stand_ui", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        let temp_dir = ctx.temp_dir();
        reverse_fix_brewing_stand_ui::reverse_fix_brewing_stand_ui(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("reverse_fix_clock_compass", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        reverse_fix_clock_compass::reverse_fix_clock_compass(ctx)
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("reverse_fix_particles", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        reverse_fix_particles::reverse_fix_particles(ctx)
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("reverse_fix_ui_creative", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        let temp_dir = ctx.temp_dir();
        reverse_fix_ui_creative::reverse_fix_ui_creative(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    scheduler.register_task("reverse_fix_ui_survival", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        let temp_dir = ctx.temp_dir();
        reverse_fix_ui_survival::reverse_fix_ui_survival(Path::new(temp_dir))
            .map_err(|e| e.to_string())
    });

    // ── 新增逆向任务 ──

    // reverse generate_* (delete generated files)
    scheduler.register_task("reverse_generate_boat", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_boat::reverse_generate_boat(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_potion_lingering", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_potion_lingering::reverse_generate_potion_lingering(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_shulker_box_ui", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_shulker_box_ui::reverse_generate_shulker_box_ui(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_furnace", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_furnace::reverse_generate_furnace(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_netherite_block", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_netherite::reverse_generate_netherite_block(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_netherite_ingot", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_netherite::reverse_generate_netherite_ingot(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_netherite_tools", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_netherite::reverse_generate_netherite_tools(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_netherite_armor_models", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_netherite::reverse_generate_netherite_armor_models(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_copper_ingot", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_copper::reverse_generate_copper_ingot(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_copper_block", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_copper::reverse_generate_copper_block(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_copper_tools", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_copper::reverse_generate_copper_tools(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_copper_armor_models", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_copper::reverse_generate_copper_armor_models(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_smithing_ui", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_smithing_ui::reverse_generate_smithing_ui(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_crossbow", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_crossbow::reverse_generate_crossbow(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_fish_bucket", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_fish_bucket::reverse_generate_fish_bucket(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_snow_bucket", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_snow_bucket::reverse_generate_snow_bucket(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_tipped_arrow_images", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_tipped_arrow_images::reverse_generate_tipped_arrow_images(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_redwood_cherry_bamboo_planks", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_planks::reverse_generate_redwood_cherry_bamboo_planks(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_generate_pale_planks", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_generate_planks::reverse_generate_pale_planks(ctx)
            .map_err(|e| e.to_string())
    });

    // reverse rename
    scheduler.register_task("reverse_rename_mcpatcher_to_optifine", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_rename_mcpatcher_to_optifine::reverse_rename_mcpatcher_to_optifine(Path::new(ctx.temp_dir()))
            .map_err(|e| e.to_string())
    });

    // reverse fix_* (delete generated or no-op)
    scheduler.register_task("reverse_fix_sign", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_fix_sign::reverse_fix_sign(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_fix_sign_entities", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_fix_sign_entities::reverse_fix_sign_entities(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_fix_slider", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_fix_slider::reverse_fix_slider(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_fix_tabs", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        reverse_fix_tabs::reverse_fix_tabs(Path::new(ctx.temp_dir()))
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_fix_horse_ui", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        reverse_fix_horse_ui::reverse_fix_horse_ui(Path::new(ctx.temp_dir()))
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_fix2_horse_ui", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_fix2_horse_ui::reverse_fix2_horse_ui(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_fix_machinery_ui", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_fix_machinery_ui::reverse_fix_machinery_ui(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_fix_ui_sub_hand", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        reverse_fix_ui_sub_hand::reverse_fix_ui_sub_hand(Path::new(ctx.temp_dir()))
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_fix_smithing2_villager2_ui", TaskType::Exclusive, TaskTier::Eraser, |ctx| {
        reverse_fix_smithing2_villager2_ui::reverse_fix_smithing2_villager2_ui(ctx)
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_overlay_icons", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        reverse_overlay_icons::reverse_overlay_icons(Path::new(ctx.temp_dir()))
            .map_err(|e| e.to_string())
    });
    scheduler.register_task("reverse_cut_gui", TaskType::Hybrid, TaskTier::Surgeon, |ctx| {
        reverse_cut_gui::reverse_cut_gui(Path::new(ctx.temp_dir()))
            .map_err(|e| e.to_string())
    });

    log_debug!("all mapping table tasks registered");

    // ── 通过 Engine 执行 ──
    log_info!("engine execute DTD scheduler");
    let mut texture_pool = TexturePool::new();
    let work_dir_str = work_dir.to_str().unwrap_or_else(|| {
        log_warn!("work_dir contains invalid UTF-8, using lossy representation");
        ""
    });
    let context = HurrayContext::new(work_dir_str);
    let pack_name = target_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("pack");
    context.set_data("pack_name", pack_name);

    scheduler.execute_version_conversion(&context, &mut texture_pool, source_version, target_version)?;

    // Only run GuiSurgeon for target versions >= 34 (Java 1.21+)
    // which use sprite-based UI. Pre-1.21 versions rely on atlas-based
    // UI (inventory.png, etc.) — running GuiSurgeon would delete those
    // atlas files and cause Minecraft to fall back to vanilla defaults.
    if target_version >= 34 {
        let mut resolution = crate::hurray::resolution::ResolutionTransducer::new();
        let _ = resolution.detect_resolution(work_dir);
        crate::converters::gui_surgeon::GuiSurgeon::execute_transformation(
            &context,
            &mut texture_pool,
            &resolution,
        ).map_err(|e| format!("GuiSurgeon failed: {}", e))?;
    }

    // ── Execute all deferred file/directory cleanup at the very end ──
    // All cleanup operations (Eraser deletions, GuiSurgeon atlas cleanup,
    // reverse generate/fix deletions, etc.) are registered during conversion
    // and executed here in one batch, ensuring no file is deleted before
    // every conversion task has had a chance to read or modify it.
    log_info!("executing deferred cleanup...");
    context.execute_cleanup().map_err(|e| format!("cleanup failed: {}", e))?;
    log_info!("deferred cleanup complete");

    log_info!("==============================");
    log_info!("conversion finished");
    log_info!("output dir: {}", target_path.display());
    log_info!("==============================");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_invoke_conversion() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let textures_path = temp_dir.path().join("assets/minecraft/textures");
        fs::create_dir_all(&textures_path).expect("Failed to create test directory structure");

        let result = invoke_conversion(temp_dir.path(), temp_dir.path(), 46, 1);
        assert!(result.is_ok());
    }
}
