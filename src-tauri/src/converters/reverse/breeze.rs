use crate::hurray::context::HurrayContext;

/// reverse(34→32)：删除 1.21 旋风系生成物。
pub fn reverse_generate_tricky_trials_breeze(ctx: &HurrayContext) -> Result<(), String> {
    let root = ctx.temp_dir();
    let names = [
        // 第 1 批
        "assets/minecraft/textures/item/breeze_rod.png",
        "assets/minecraft/textures/item/wind_charge.png",
        "assets/minecraft/textures/item/trial_key.png",
        "assets/minecraft/textures/item/ominous_trial_key.png",
        "assets/minecraft/textures/item/breeze_spawn_egg.png",
        "assets/minecraft/textures/entity/projectiles/wind_charge.png",
        "assets/minecraft/textures/mob_effect/wind_charged.png",
        // copper bulb 族
        "assets/minecraft/textures/block/copper_bulb.png",
        "assets/minecraft/textures/block/copper_bulb_lit.png",
        "assets/minecraft/textures/block/copper_bulb_powered.png",
        "assets/minecraft/textures/block/copper_bulb_lit_powered.png",
        "assets/minecraft/textures/block/exposed_copper_bulb.png",
        "assets/minecraft/textures/block/exposed_copper_bulb_lit.png",
        "assets/minecraft/textures/block/exposed_copper_bulb_powered.png",
        "assets/minecraft/textures/block/exposed_copper_bulb_lit_powered.png",
        "assets/minecraft/textures/block/weathered_copper_bulb.png",
        "assets/minecraft/textures/block/weathered_copper_bulb_lit.png",
        "assets/minecraft/textures/block/weathered_copper_bulb_powered.png",
        "assets/minecraft/textures/block/weathered_copper_bulb_lit_powered.png",
        "assets/minecraft/textures/block/oxidized_copper_bulb.png",
        "assets/minecraft/textures/block/oxidized_copper_bulb_lit.png",
        "assets/minecraft/textures/block/oxidized_copper_bulb_powered.png",
        "assets/minecraft/textures/block/oxidized_copper_bulb_lit_powered.png",
        "assets/minecraft/textures/block/waxed_copper_bulb.png",
        "assets/minecraft/textures/block/waxed_copper_bulb_lit.png",
        "assets/minecraft/textures/block/waxed_exposed_copper_bulb.png",
        "assets/minecraft/textures/block/waxed_exposed_copper_bulb_lit.png",
        "assets/minecraft/textures/block/waxed_weathered_copper_bulb.png",
        "assets/minecraft/textures/block/waxed_weathered_copper_bulb_lit.png",
        "assets/minecraft/textures/block/waxed_oxidized_copper_bulb.png",
        "assets/minecraft/textures/block/waxed_oxidized_copper_bulb_lit.png",
        // 第 2 批
        "assets/minecraft/textures/block/heavy_core.png",
        "assets/minecraft/textures/entity/breeze/breeze.png",
        "assets/minecraft/textures/entity/breeze/breeze_eyes.png",
        "assets/minecraft/textures/item/flow_armor_trim_smithing_template.png",
        "assets/minecraft/textures/item/ominous_bottle.png",
    ];
    for name in names {
        let p = root.join(name);
        if p.exists() {
            ctx.defer_remove_file(&p);
        }
        let m = p.with_extension("png.mcmeta");
        if m.exists() {
            ctx.defer_remove_file(&m);
        }
    }
    Ok(())
}
