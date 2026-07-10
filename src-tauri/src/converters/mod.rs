// Converter modules and re-exports.
// 所有 pub use 必须对齐 pack.py ADJACENT_CONVERSIONS 映射表

pub use fix_sign_entities::fix_sign_entities;
pub use rename_mcpatcher_to_optifine::rename_mcpatcher_to_optifine;

pub mod adjust_hue_brightness;
pub mod anims_folder_conversion;
pub mod blockstate_adapter;
pub mod convert_old_texture_paths;
pub mod convert_sound_files;
pub mod color_utils;
pub mod cut_gui;
pub mod delete_blockstates_models;
pub mod delete_enchanted_item_glint;
pub mod delete_font_folder;
pub mod delete_horse_folder;
pub mod delete_shaders_folder;
pub mod fix2_horse_ui;
pub mod fix_armor_models;
pub mod fix_alpha_layers_in_textures;
pub mod fix_brewing_stand_ui;
pub mod fix_clock_compass;
pub mod fix_horse_ui;
pub mod fix_machinery_ui;
pub mod fix_particles;
pub mod fix_sign;
pub mod fix_sign_entities;
pub mod fix_slider;
pub mod fix_smithing2_villager2_ui;
pub mod fix_tabs;
pub mod fix_ui_creative;
pub mod fix_ui_sub_hand;
pub mod fix_ui_survival;
pub mod gui_surgeon;
pub mod generate_boat;
pub mod generate_furnace;
pub mod generate_copper;
pub mod generate_crossbow;
pub mod generate_fish_bucket;
pub mod generate_netherite;
pub mod generate_planks;
pub mod generate_smithing_ui;
pub mod generate_snow_bucket;
pub mod generate_potion_lingering;
pub mod generate_shulker_box_ui;
pub mod generate_tipped_arrow_images;
pub mod legacy_eraser;
pub mod legacy_processor;
pub mod main_converter;
pub mod overlay_icons;
pub mod process_chest_folder;
pub mod rename_blocks_items;
pub mod rename_and_process_blocks;
pub mod rename_mcpatcher_to_optifine;
pub mod scale_factor;
pub mod reverse_fix_armor_models;
pub mod reverse_fix_brewing_stand_ui;
pub mod reverse_fix_clock_compass;
pub mod reverse_fix_particles;
pub mod reverse_fix_ui_creative;
pub mod reverse_fix_ui_survival;
pub mod reverse_process_chest_folder;
pub mod reverse_rename_blocks_items;

// New reverse modules
pub mod reverse_cut_gui;
pub mod reverse_fix2_horse_ui;
pub mod reverse_fix_horse_ui;
pub mod reverse_fix_machinery_ui;
pub mod reverse_fix_sign;
pub mod reverse_fix_sign_entities;
pub mod reverse_fix_slider;
pub mod reverse_fix_smithing2_villager2_ui;
pub mod reverse_fix_tabs;
pub mod reverse_fix_ui_sub_hand;
pub mod reverse_generate_boat;
pub mod reverse_generate_copper;
pub mod reverse_generate_crossbow;
pub mod reverse_generate_fish_bucket;
pub mod reverse_generate_furnace;
pub mod reverse_generate_netherite;
pub mod reverse_generate_planks;
pub mod reverse_generate_potion_lingering;
pub mod reverse_generate_shulker_box_ui;
pub mod reverse_generate_smithing_ui;
pub mod reverse_generate_snow_bucket;
pub mod reverse_generate_tipped_arrow_images;
pub mod reverse_overlay_icons;
pub mod reverse_rename_mcpatcher_to_optifine;
pub mod version_converter;
pub mod zip;

// ── UImage 路径解析（委托给 resource_resolver）───────────────────────

/// 通过 Tauri 资源 API 解析 UImage 路径并缓存（委托 resource_resolver）
pub fn set_uimage_path_from_app(app: &tauri::AppHandle) {
    crate::resource_resolver::cache_resource_from_app(app, "UImage");
}

/// 获取 UImage 资源目录（多策略查找，找不到则在用户文档创建默认目录）
pub fn get_uimage_path() -> Result<std::path::PathBuf, String> {
    // 1. 优先通过 resource_resolver 的多策略解析
    match crate::resource_resolver::resolve_resource_dir("UImage", crate::resource_resolver::uimage_validator()) {
        Ok(p) => return Ok(p),
        Err(e) => crate::log_info!("UImage resolve_resource_dir failed: {}", e),
    }

    // 2. 回退到用户文档目录
    if let Ok(user_dir) = crate::overlay::user_data_root_dir() {
        let fallback = user_dir.join("UImage");
        if fallback.exists() {
            crate::log_info!("UImage found in user data dir: {}", fallback.display());
            return Ok(fallback);
        }
    }

    // 3. 最后手段：在用户文档目录创建默认 UImage
    let default_path = crate::overlay::user_data_root_dir()
        .map(|dir| dir.join("UImage"))
        .unwrap_or_else(|_| {
            std::env::current_dir()
                .map(|dir| dir.join("UImage"))
                .unwrap_or_else(|_| std::path::PathBuf::from("UImage"))
        });

    crate::log_info!("creating default UImage in user data: {}", default_path.display());
    std::fs::create_dir_all(&default_path).map_err(|e| {
        format!("UImage directory is missing and default directory creation failed: {}", e)
    })?;
    let _ = std::fs::write(default_path.join("README.txt"),
        "UImage directory for Resource Pack Converter.\nPut required UI image assets here.\n");
    Ok(default_path)
}

