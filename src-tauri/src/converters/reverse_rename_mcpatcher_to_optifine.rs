use std::fs;
use std::path::Path;

pub fn reverse_rename_mcpatcher_to_optifine(path: &Path) -> Result<(), String> {
    let optifine = path.join("assets/minecraft/optifine");
    let mcpatcher = path.join("assets/minecraft/mcpatcher");
    if optifine.exists() && !mcpatcher.exists() {
        fs::rename(&optifine, &mcpatcher)
            .map_err(|e| format!("rename optifine->mcpatcher: {}", e))?;
    }
    Ok(())
}
