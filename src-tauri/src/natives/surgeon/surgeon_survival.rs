    use super::*;
    use image::imageops;

    const INVENTORY: &str = "assets/minecraft/textures/gui/container/inventory.png";
    const MOB_EFFECT: &str = "assets/minecraft/textures/mob_effect";
    const SPRITES: &str = "assets/minecraft/textures/gui/sprites/container/inventory";

    /// 旧 `determine_scale_factor`（返回 `f32`；不支持的尺寸 → 整任务跳过）。
    fn scale_factor(width: u32, height: u32) -> Option<f32> {
        match (width, height) {
            (256, 256) => Some(1.0),
            (512, 512) => Some(2.0),
            (1024, 1024) => Some(4.0),
            (2048, 2048) => Some(8.0),
            _ => None,
        }
    }

    /// 旧 `fill_rect_solid`：边界裁剪到图内。
    fn fill_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: image::Rgba<u8>) {
        let (iw, ih) = img.dimensions();
        for yy in y..(y + h).min(ih) {
            for xx in x..(x + w).min(iw) {
                img.put_pixel(xx, yy, color);
            }
        }
    }

    /// 旧 `move_region`（含「用目标位置的颜色擦源区」这个怪癖）。
    fn move_region(
        img: &mut RgbaImage,
        x1: u32,
        y1: u32,
        x2: u32,
        y2: u32,
        dx: i32,
        dy: i32,
        s: f32,
    ) {
        let sc = |c: u32| (c as f32 * s) as u32;
        let (sx, sy) = (sc(x1), sc(y1));
        let (sw, sh) = (sc(x2 - x1), sc(y2 - y1));
        let region = imageops::crop_imm(img, sx, sy, sw, sh).to_image();

        let tx = ((x1 as i32 + dx).max(0) as f32 * s) as u32;
        let ty = ((y1 as i32 + dy).max(0) as f32 * s) as u32;

        // 旧实现：取**目标位置**（此时仍是原图背景）的颜色，填掉源区，再贴过去
        let bg = *img.get_pixel(tx, ty);
        fill_rect(img, sx, sy, sw, sh, bg);
        // 贴上用 `paste_region`（原始覆盖、失败仅记录）
        for y in 0..region.height() {
            for x in 0..region.width() {
                if let Some(px) = region.get_pixel_checked(x, y) {
                    if let Some(dst) = img.get_pixel_mut_checked(tx + x, ty + y) {
                        *dst = *px;
                    }
                }
            }
        }
    }

    /// 旧 `fill_region`：色取自 (cx,cy)，范围 (x1,y1)-(x2,y2)。
    fn fill_region(
        img: &mut RgbaImage,
        x1: u32,
        y1: u32,
        x2: u32,
        y2: u32,
        cx: u32,
        cy: u32,
        s: f32,
    ) {
        let sc = |c: u32| (c as f32 * s) as u32;
        let color = *img.get_pixel(sc(cx), sc(cy));
        for y in sc(y1)..sc(y2) {
            for x in sc(x1)..sc(x2) {
                if let Some(p) = img.get_pixel_mut_checked(x, y) {
                    *p = color;
                }
            }
        }
    }

    /// 旧 `copy_paste`：**原始覆盖**（等价 `Image.paste`，不做 alpha 混合）。
    fn copy_paste(
        img: &mut RgbaImage,
        x1: u32,
        y1: u32,
        x2: u32,
        y2: u32,
        tx: u32,
        ty: u32,
        s: f32,
    ) {
        let sc = |c: u32| (c as f32 * s) as u32;
        let region =
            imageops::crop_imm(img, sc(x1), sc(y1), sc(x2 - x1), sc(y2 - y1)).to_image();
        for y in 0..region.height() {
            for x in 0..region.width() {
                if let Some(px) = region.get_pixel_checked(x, y) {
                    if let Some(dst) = img.get_pixel_mut_checked(sc(tx) + x, sc(ty) + y) {
                        *dst = *px;
                    }
                }
            }
        }
    }

    pub fn decl() -> TaskDecl {
        // 写三处（container/、mob_effect/、gui/sprites/…）——**必须用 `with_prefix` 组合**：
        // `TaskDecl::writes()` 是覆盖不是累加（§9.66 的教训）。
        TaskDecl::new("fix_ui_survival", Tier::Surgeon)
            .reads(
                ScopeSet::prefix("assets/minecraft/textures/gui")
                    .with_prefix("assets/minecraft/textures/mob_effect"),
            )
            .writes(
                ScopeSet::prefix(MOB_EFFECT)
                    .with_prefix("assets/minecraft/textures/gui"),
            )
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        if !tx.exists(INVENTORY) {
            return Ok(Outcome::default());
        }
        let mut img: RgbaImage = (*tx.image(INVENTORY)?).clone();
        let (width, height) = img.dimensions();
        let Some(s) = scale_factor(width, height) else {
            crate::log_info!("fix_ui_survival skipped: unsupported size");
            return Ok(Outcome::default());
        };
        let sc = |c: u32| (c as f32 * s) as u32;

        // ── 步骤 1：抽 19 个状态图标 ──
        let icon = sc(18);
        let rows: [&[&str]; 3] = [
            &[
                "speed.png",
                "slowness.png",
                "haste.png",
                "mining_fatigue.png",
                "strength.png",
                "weakness.png",
                "poison.png",
                "regeneration.png",
            ],
            &[
                "invisibility.png",
                "hunger.png",
                "jump_boost.png",
                "nausea.png",
                "night_vision.png",
                "blindness.png",
                "resistance.png",
                "fire_resistance.png",
            ],
            &["water_breathing.png", "wither.png", "absorption.png"],
        ];
        let mut changed = 0usize;
        for (row_idx, row) in rows.iter().enumerate() {
            for (col_idx, name) in row.iter().enumerate() {
                let x = sc(col_idx as u32 * 18);
                let y = sc(198 + row_idx as u32 * 18);
                let tile = imageops::crop_imm(&img, x, y, icon, icon).to_image();
                tx.put_image(&format!("{MOB_EFFECT}/{name}"), &tile)?;
                changed += 1;
            }
        }

        // ── 步骤 2：搬移 + 两处填充 + 一块拷贝 ──
        move_region(&mut img, 86, 24, 162, 62, 10, -8, s);
        fill_region(&mut img, 75, 6, 96, 80, 90, 10, s);
        fill_region(&mut img, 96, 54, 162, 62, 90, 10, s);
        copy_paste(&mut img, 152, 26, 172, 46, 75, 60, s);

        // ── 步骤 3：可选 UImage 模板叠加 ──
        let template_name = match width {
            256 => Some("inventory_256.png"),
            512 => Some("inventory_512.png"),
            1024 => Some("inventory_1024.png"),
            2048 => Some("inventory_2048.png"),
            _ => None,
        };
        if let Some(name) = template_name {
            if let Ok(dir) = crate::image_utils::get_uimage_path() {
                let template = dir.join("inventory").join(name);
                if template.exists() {
                    let overlay = image::open(&template)
                        .map_err(|e| AromError::io(format!("failed to open template: {e}")))?
                        .to_rgba8();
                    let resized = if overlay.dimensions() != img.dimensions() {
                        imageops::resize(&overlay, width, height, imageops::FilterType::Lanczos3)
                    } else {
                        overlay
                    };
                    imageops::overlay(&mut img, &resized, 0, 0);
                }
            }
        }

        // ── 步骤 4：抽两张药水背景 sprite ──
        for ((x1, y1, x2, y2), name) in [
            ((0u32, 166u32, 120u32, 198u32), "effect_background_large.png"),
            ((0, 198, 32, 230), "effect_background_small.png"),
        ] {
            let cropped = imageops::crop_imm(
                &img,
                sc(x1),
                sc(y1),
                sc(x2 - x1),
                sc(y2 - y1),
            )
            .to_image();
            tx.put_image(&format!("{SPRITES}/{name}"), &cropped)?;
            changed += 1;
        }

        tx.put_image(INVENTORY, &img)?;
        changed += 1;
        Ok(Outcome {
            changed,
            notes: vec![format!("inventory.png fixed in place (scale {s})")],
            ..Outcome::default()
        })
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "fix_ui_survival" => Some((decl(), run)),
            _ => None,
        }
    }
