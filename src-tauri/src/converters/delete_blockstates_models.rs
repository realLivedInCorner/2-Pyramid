use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn delete_blockstates_models(ctx: &HurrayContext) -> Result<(), String> {
    let base = ctx.temp_dir();
    let blockstates = base.join("assets/minecraft/blockstates");
    let models = base.join("assets/minecraft/models");

    for target in [&blockstates, &models] {
        if target.exists() {
            crate::log_info!("deferring cleanup of {}", target.display());
            if target.is_dir() {
                ctx.defer_remove_dir(target);
            } else {
                ctx.defer_remove_file(target);
            }
        }
    }

    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "delete_blockstates_models",
        crate::hurray::scheduler::TaskType::Exclusive,
        crate::hurray::scheduler::TaskTier::Eraser,
        |context| delete_blockstates_models(context),
    );
}
