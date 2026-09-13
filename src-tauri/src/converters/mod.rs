// Converter modules grouped by domain.
// ui/       GUI & HUD fixes
// textures/ block/item texture generation, rename, delete
// reverse/  undo counterparts of forward converters
// color/    HSV helpers
// audio/    sound conversion
// shaders/  Java shader adaptation
// bedrock/  Java <-> Bedrock pipeline

pub use ui::sign_entities::fix_sign_entities;
pub use textures::mcpatcher_to_optifine::rename_mcpatcher_to_optifine;

pub mod audio;
pub mod bedrock;
pub mod blockstate_adapter;
pub mod color;
pub mod legacy_eraser;
pub mod legacy_processor;
pub mod main_converter;
pub mod reverse;
pub mod scale_factor;
pub mod shaders;
pub mod textures;
pub mod ui;
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

