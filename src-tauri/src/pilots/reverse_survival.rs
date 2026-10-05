    use super::*;
    use crate::image_utils::paste_region;
    use image::{imageops, Rgba};

    const INVENTORY: &str = "assets/minecraft/textures/gui/container/inventory.png";
    const MOB_EFFECT: &str = "assets/minecraft/textures/mob_effect";

    const EFFECTS: [&str; 19] = [
        "speed.png",
        "slowness.png",
        "haste.png",
        "mining_fatigue.png",
        "strength.png",
        "weakness.png",
        "poison.png",
        "regeneration.png",
        "invisibility.png",
        "hunger.png",
        "jump_boost.png",
        "nausea.png",
        "night_vision.png",
        "blindness.png",
        "resistance.png",
        "fire_resistance.png",
        "water_breathing.png",
        "wither.png",
        "absorption.png",
    ];

    pub fn decl() -> TaskDecl {
        TaskDecl::new("reverse_fix_ui_survival", Tier::Surgeon)
            .reads(ScopeSet::prefix("assets/minecraft/textures"))
            .writes(ScopeSet::prefix("assets/minecraft/textures"))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        if !tx.exists(INVENTORY) {
            return Ok(Outcome::default());
        }
        let mut img: RgbaImage = (*tx.image(INVENTORY)?).clone();
        let s = match img.width() {
            256 => 1,
            512 => 2,
            1024 => 4,
            2048 => 8,
            _ => {
                return Ok(Outcome {
                    skipped: 1,
                    ..Outcome::default()
                })
            }
        };
        let scaled = |c: u32| c * s;

        // 步骤 1：清空底部区域为透明
        for y in scaled(198)..scaled(254) {
            for x in 0..scaled(144) {
                img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }

        // 步骤 2：把 mob_effect 图标缩放后贴回
        if tx.has_prefix(MOB_EFFECT)? {
            let icon_size = scaled(18);
            for (i, name) in EFFECTS.iter().enumerate() {
                let path = format!("{MOB_EFFECT}/{name}");
                if !tx.exists(&path) {
                    continue;
                }
                let effect: RgbaImage = (*tx.image(&path)?).clone();
                let resized = if effect.dimensions() != (icon_size, icon_size) {
                    imageops::resize(&effect, icon_size, icon_size, imageops::FilterType::Lanczos3)
                } else {
                    effect
                };
                let row = (i / 8) as u32;
                let col = (i % 8) as u32;
                paste_region(
                    &mut img,
                    &resized,
                    col * icon_size + scaled(0),
                    row * icon_size + scaled(198),
                )
                .map_err(AromError::internal)?;
            }
        }

        // 步骤 3：用 (90,10) 的颜色填 (76,61)-(94,79)
        let fill = *img.get_pixel(scaled(90), scaled(10));
        for y in scaled(61)..scaled(79) {
            for x in scaled(76)..scaled(94) {
                img.put_pixel(x, y, fill);
            }
        }

        // 步骤 4：把 (96,16)-(172,54) 搬 (-10,+8)
        let move_w = scaled(172) - scaled(96);
        let move_h = scaled(54) - scaled(16);
        let region = imageops::crop_imm(&img, scaled(96), scaled(16), move_w, move_h).to_image();
        let dst_x = ((96i32 - 10) * s as i32) as i64;
        let dst_y = ((16i32 + 8) * s as i32) as i64;
        imageops::overlay(&mut img, &region, dst_x, dst_y);

        // 步骤 5 / 6：填回搬走后的空隙
        for y in scaled(16)..scaled(25) {
            for x in scaled(96)..scaled(172) {
                img.put_pixel(x, y, fill);
            }
        }
        for y in scaled(25)..scaled(54) {
            for x in scaled(161)..scaled(172) {
                img.put_pixel(x, y, fill);
            }
        }

        tx.put_image(INVENTORY, &img)?;
        Ok(Outcome {
            changed: 1,
            ..Outcome::default()
        })
    }
