use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_generate_tipped_arrow_images(ctx: &HurrayContext) -> Result<(), String> {
    let items = ctx.temp_dir().join("assets/minecraft/textures/items");
    for name in &["tipped_arrow_base.png", "tipped_arrow_head.png"] {
        let f = items.join(name);
        if f.exists() {
            ctx.defer_remove_file(&f);
        }
    }
    Ok(())
}
