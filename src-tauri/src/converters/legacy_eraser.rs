use crate::hurray::context::HurrayContext;
use crate::hurray::engine::HurrayEngine;
use crate::hurray::scheduler::{TaskTier, TaskType};
use crate::{log_info, log_warn};

/// Consolidated cleanup operations from legacy delete_* scripts.
/// All deletions are deferred to the end of the conversion pipeline.
pub struct LegacyEraser;

impl LegacyEraser {
    pub fn run_cleanup(ctx: &HurrayContext) -> Result<(), String> {
        let base = ctx.temp_dir();
        let targets = [
            "assets/minecraft/blockstates",
            "assets/minecraft/models",
            "assets/minecraft/font",
            "assets/minecraft/textures/entity/horse",
            "assets/minecraft/shaders",
            "assets/minecraft/textures/misc/enchanted_item_glint.png",
        ];

        for target in targets {
            let full_path = base.join(target);
            if !full_path.exists() {
                continue;
            }

            if full_path.is_dir() {
                log_info!("legacy eraser deferring dir {}", full_path.display());
                ctx.defer_remove_dir(&full_path);
            } else {
                log_info!("legacy eraser deferring file {}", full_path.display());
                ctx.defer_remove_file(&full_path);
            }

        }

        Ok(())
    }

    pub fn register(engine: &mut HurrayEngine) {
        log_warn!("registering legacy eraser bundle into tier Eraser");
        engine.register_task(
            "legacy_eraser_cleanup",
            TaskType::Exclusive,
            TaskTier::Eraser,
            Self::run_cleanup,
        );
    }
}
