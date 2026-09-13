use std::fs;
use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_fix_machinery_ui(ctx: &HurrayContext) -> Result<(), String> {
    let gui = ctx.temp_dir().join("assets/minecraft/textures/gui/container");

    // Defer deletion of generated machinery UIs
    for name in &["grindstone.png", "cartography_table.png", "stonecutter.png", "loom.png"] {
        let f = gui.join(name);
        if f.exists() {
            ctx.defer_remove_file(&f);
        }
    }

    // Restore villager.png from backup if available (part of conversion logic, immediate)
    let backup = gui.join("villager_backup.png");
    let villager = gui.join("villager.png");
    if backup.exists() {
        let _ = fs::rename(&backup, &villager);
    }

    Ok(())
}
