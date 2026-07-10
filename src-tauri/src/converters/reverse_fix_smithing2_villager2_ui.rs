use std::fs;
use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_fix_smithing2_villager2_ui(ctx: &HurrayContext) -> Result<(), String> {
    let gui = ctx.temp_dir().join("assets/minecraft/textures/gui/container");

    // Defer deletion of generated smithing.png
    let smithing = gui.join("smithing.png");
    if smithing.exists() {
        ctx.defer_remove_file(&smithing);
    }

    // Restore villager.png from backup (part of conversion logic, immediate)
    let backup = gui.join("villager_backup.png");
    let villager = gui.join("villager.png");
    if backup.exists() {
        let _ = fs::rename(&backup, &villager);
    }

    Ok(())
}
