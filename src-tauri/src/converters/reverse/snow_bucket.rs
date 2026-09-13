use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_generate_snow_bucket(ctx: &HurrayContext) -> Result<(), String> {
    let f = ctx.temp_dir().join("assets/minecraft/textures/item/powder_snow_bucket.png");
    if f.exists() {
        ctx.defer_remove_file(&f);
    }
    Ok(())
}
