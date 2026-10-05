    use super::*;

    const CONTAINER: &str = "assets/minecraft/textures/gui/container";
    const GENERIC: &str = "assets/minecraft/textures/gui/container/generic_54.png";

    /// 旧 `determine_scale_factor`：返回最接近 `max(w,h)` 的 256 的倍数。
    fn determine_scale_factor(width: u32, height: u32) -> u32 {
        let candidates = [1u32, 2, 4, 8];
        let image_size = width.max(height);
        let mut best = candidates[0];
        let mut best_delta = (candidates[0] * 256).abs_diff(image_size);
        for &candidate in &candidates[1..] {
            let delta = (candidate * 256).abs_diff(image_size);
            if delta < best_delta {
                best = candidate;
                best_delta = delta;
            }
        }
        best
    }

    pub fn decl() -> TaskDecl {
        TaskDecl::new("generate_shulker_box_ui", Tier::Architect)
            .reads(ScopeSet::prefix(CONTAINER))
            .writes(ScopeSet::prefix(CONTAINER))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        if !tx.exists(GENERIC) {
            return Ok(Outcome::default());
        }
        let img: RgbaImage = (*tx.image(GENERIC)?).clone();
        let (width, height) = img.dimensions();
        if width == 0 || height == 0 {
            return Ok(Outcome::default());
        }

        let s = determine_scale_factor(width, height);
        let mut new_img = img.clone();

        let x_max = 176 * s;
        let clear_start = 71 * s;
        let clear_end = 127 * s;
        for x in 0..x_max {
            for y in clear_start..clear_end {
                if x < width && y < height {
                    new_img.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
                }
            }
        }

        let move_start = 127 * s;
        let move_end = 222 * s;
        let move_delta = 56 * s;
        for x in 0..x_max {
            for y in move_start..move_end {
                if x < width && y < height {
                    let new_y = y.saturating_sub(move_delta);
                    if new_y < height {
                        let pixel = img.get_pixel(x, y);
                        new_img.put_pixel(x, new_y, *pixel);
                    }
                    new_img.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
                }
            }
        }

        tx.mkdir(CONTAINER)?;
        tx.put_image(&format!("{CONTAINER}/shulker_box.png"), &new_img)?;
        Ok(Outcome {
            changed: 1,
            notes: vec!["generic_54.png -> shulker_box.png".into()],
            ..Outcome::default()
        })
    }
