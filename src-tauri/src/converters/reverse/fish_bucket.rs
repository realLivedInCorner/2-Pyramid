use std::path::Path;

use crate::hurray::context::HurrayContext;

pub fn reverse_generate_fish_bucket(ctx: &HurrayContext) -> Result<(), String> {
    let item = ctx.temp_dir().join("assets/minecraft/textures/item");
    for name in &[
        "axolotl_bucket.png",
        "cod_bucket.png",
        "pufferfish_bucket.png",
        "salmon_bucket.png",
        "tropical_fish_bucket.png",
        "tadpole_bucket.png",
    ] {
        let f = item.join(name);
        if f.exists() {
            ctx.defer_remove_file(&f);
        }
    }
    Ok(())
}
