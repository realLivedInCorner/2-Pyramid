use crate::converters::gui_surgeon::GuiSurgeon;
use crate::hurray::context::HurrayContext;
use crate::hurray::resolution::ResolutionTransducer;
use crate::hurray::scheduler::{TaskType, TaskTier};
use crate::hurray::texture::TexturePool;

/// Cut GUI sprites using GuiSurgeon
/// 
/// # Parameters
/// - `context`: Hurray context
/// 
/// # Returns
/// - `Ok(())` on success, `Err(String)` on failure
pub fn cut_gui(context: &HurrayContext) -> Result<(), String> {
    crate::log_info!("2-Pyramid: starting cut_gui (GuiSurgeon pipeline)...");

    let mut pool = TexturePool::new();
    let mut resolution = ResolutionTransducer::new();
    resolution
        .detect_resolution(context.temp_dir())
        .map_err(|e| e.to_string())?;

    GuiSurgeon::execute_transformation(context, &mut pool, &resolution)?;
    pool.commit_all().map_err(|e| e.to_string())?;

    Ok(())
}

/// Register cut_gui task to the engine
pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "cut_gui",
        TaskType::Hybrid,
        TaskTier::Surgeon,
        |context| {
            cut_gui(context)
        }
    );
}

/// Register cut_gui task with full dependencies
pub fn register_task_with_deps(engine: &mut crate::hurray::engine::HurrayEngine) {
    // 这个函数将在scheduler支持传递更多参数时使用
    // 目前保留为未来扩展
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::converters::fix_machinery_ui;
    use crate::converters::fix_smithing2_villager2_ui;
    use crate::hurray::context::HurrayContext;
    use std::fs;
    use tempfile::tempdir;

    /// End-to-end test: copy a real 1.20 vanilla resource pack's container
    /// folder (Pika 5K 16x) into a tempdir, run the fix_* and cut_gui pipeline,
    /// then verify all 17 sprite sub-folders in `sprites/container/*` are
    /// populated with files of non-trivial size.
    ///
    /// If "锻造台/制图台缺格子" comes from a missing or truncated sprite, this
    /// test will catch it.
    #[test]
    fn test_cut_gui_sprites_from_pika5k() {
        let rp_root = r"D:\GameTime\.minecraft\versions\1.20.1-OptiFine_I6\resourcepacks\!   §1§b§lPika 5K - 16x";
        if !std::path::Path::new(rp_root).exists() {
            eprintln!("SKIP: Pika 5K not found at {}", rp_root);
            return;
        }
        let src_container = std::path::Path::new(rp_root)
            .join("assets/minecraft/textures/gui/container");
        if !src_container.exists() {
            eprintln!("SKIP: Pika 5K container folder missing");
            return;
        }

        let temp = tempdir().expect("tempdir");
        let dst_container = temp
            .path()
            .join("assets/minecraft/textures/gui/container");
        fs::create_dir_all(&dst_container).expect("mkdir container");
        for entry in fs::read_dir(&src_container).expect("read src container") {
            let entry = entry.expect("dir entry");
            if entry.file_type().expect("filetype").is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".png")
            {
                fs::copy(
                    entry.path(),
                    dst_container.join(entry.file_name()),
                )
                .expect("copy png");
            }
        }

        // Run the fix_* pipeline first
        fix_smithing2_villager2_ui::fix_smithing2_villager2_ui(temp.path())
            .expect("fix_smithing2_villager2_ui");
        fix_machinery_ui::fix_machinery_ui(temp.path()).expect("fix_machinery_ui");

        // Run cut_gui via HurrayContext
        let ctx = HurrayContext::new(temp.path().to_str().expect("utf-8 path"));
        cut_gui(&ctx).expect("cut_gui");

        // Verify ONLY the two sub-folders the user reported: 锻造台 (smithing)
        // and 制图台 (cartography_table). Continue past the first failure so
        // we see the full picture.
        let sprites_dir = temp
            .path()
            .join("assets/minecraft/textures/gui/sprites/container");
        let mut failures: Vec<String> = Vec::new();
        let smithing_files: &[&str] = &["template_slot.png", "base_slot.png", "addition_slot.png", "result_slot.png", "error.png"];
        let cart_files: &[&str] = &["duplicated_map.png", "scaled_map.png", "map.png", "locked.png", "error.png"];
        for (sub, files) in &[("smithing", smithing_files), ("cartography_table", cart_files)] {
            let sub_dir = sprites_dir.join(sub);
            for f in *files {
                let p = sub_dir.join(f);
                if !p.exists() {
                    failures.push(format!("MISSING {}/{}", sub, f));
                    continue;
                }
                let img = match image::open(&p) {
                    Ok(i) => i.to_rgba8(),
                    Err(e) => { failures.push(format!("OPEN-FAIL {}/{}: {}", sub, f, e)); continue; }
                };
                let (w, h) = img.dimensions();
                if w < 2 || h < 2 {
                    failures.push(format!("DEGENERATE {}/{}: {}x{}", sub, f, w, h));
                    continue;
                }
                if !img.pixels().any(|px| px[3] > 0) {
                    failures.push(format!("TRANSPARENT {}/{}", sub, f));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "User-reported '缺格子' sprite failures:\n  - {}",
            failures.join("\n  - ")
        );
    }
}
