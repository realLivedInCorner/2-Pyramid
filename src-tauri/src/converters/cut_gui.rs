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
    use std::fs;
    use tempfile::tempdir;

    /// End-to-end regression test for the 1.20.1 → 1.21.4 inventory bug:
    /// the deferred-cleanup list in GuiSurgeon must NOT delete
    /// `container/inventory.png`, otherwise the post-conversion resource
    /// pack is missing the file the 1.21 client uses to render the
    /// survival/creative inventory background, and the player sees no
    /// inventory GUI.
    ///
    /// We run the production `invoke_conversion` entry point on a copy of
    /// Pika 5K 16x and assert that the conversion finishes without errors
    /// AND that `container/inventory.png` survives.
    #[test]
    fn invoke_conversion_preserves_inventory_png() {
        let rp_root = r"D:\GameTime\.minecraft\versions\1.20.1-OptiFine_I6\resourcepacks\!   §1§b§lPika 5K - 16x";
        if !std::path::Path::new(rp_root).exists() {
            eprintln!("SKIP: Pika 5K not found at {}", rp_root);
            return;
        }

        let temp = tempdir().expect("tempdir");
        // Copy full Pika 5K tree (subdirs included — process_tabs needs
        // `container/creative_inventory/tabs.png`).
        fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
            std::fs::create_dir_all(dst)?;
            for entry in std::fs::read_dir(src)? {
                let entry = entry?;
                let ty = entry.file_type()?;
                let from = entry.path();
                let to = dst.join(entry.file_name());
                if ty.is_dir() {
                    copy_dir_all(&from, &to)?;
                } else {
                    std::fs::copy(&from, &to)?;
                }
            }
            Ok(())
        }
        copy_dir_all(std::path::Path::new(rp_root), temp.path()).expect("copy pika");

        // Production conversion: 1.20.1 (pack_format 15) -> 1.21.4 (pack_format 34)
        crate::invoke_conversion::invoke_conversion(
            temp.path(),
            temp.path(),
            34, // target: 1.21.4
            15, // source: 1.20.1
        ).expect("invoke_conversion 1.20.1 -> 1.21.4");

        // Copy the result to a stable path for diffing with Py 1.0's output
        // (Py 1.0 was run separately and produced D:\temp_rp_test\py_output).
        let dst = std::path::Path::new(r"D:\temp_rp_test\rust_invoke_fixed");
        if dst.exists() { let _ = std::fs::remove_dir_all(dst); }
        std::fs::create_dir_all(dst).expect("mkdir rust_invoke_fixed");
        let temp_path = temp.path();
        copy_dir_all(temp_path, dst).expect("copy rust_invoke_fixed");

        // Assert: `container/inventory.png` MUST survive the conversion.
        // The 1.21 vanilla resource pack still ships an `inventory.png`
        // (256x256 simplified panel), and the client uses the resource
        // pack's file (if present) to render the survival/creative
        // inventory background. Deleting it causes the inventory GUI to
        // disappear after the player loads the converted pack.
        let inv = temp_path.join("assets/minecraft/textures/gui/container/inventory.png");
        assert!(
            inv.exists(),
            "BUG: container/inventory.png was deleted by GuiSurgeon cleanup_files. \
             The 1.21 vanilla resource pack still ships an inventory.png; deleting it \
             causes the post-conversion resource pack to be missing the file the 1.21 \
             client needs to render the survival/creative inventory GUI."
        );

        // Sanity check: the 1.21+ sprite files were actually produced.
        let eff_large = temp_path.join(
            "assets/minecraft/textures/gui/sprites/container/inventory/effect_background_large.png",
        );
        assert!(eff_large.exists(), "sprites/container/inventory/effect_background_large.png should be produced");

        let tab_top = temp_path.join(
            "assets/minecraft/textures/gui/sprites/container/creative_inventory/tab_top_selected_1.png",
        );
        assert!(tab_top.exists(), "sprites/container/creative_inventory/tab_top_selected_1.png should be produced");

        // Hold the tempdir so it doesn't get dropped/cleaned (drops Rust output!)
        std::mem::forget(temp);
    }
}
