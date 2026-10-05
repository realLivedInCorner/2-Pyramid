    use super::*;
    use crate::scale_factor::determine_scale_factor;

    const TABS: &str =
        "assets/minecraft/textures/gui/container/creative_inventory/tabs.png";

    /// 旧 `extract_region`：越界处留透明（`get_pixel_checked` 失败即保持默认 0）。
    fn extract(img: &RgbaImage, coords: (u32, u32, u32, u32)) -> RgbaImage {
        let (x1, y1, x2, y2) = coords;
        let mut region = RgbaImage::new(x2.saturating_sub(x1), y2.saturating_sub(y1));
        for y in 0..region.height() {
            for x in 0..region.width() {
                if let Some(px) = img.get_pixel_checked(x1 + x, y1 + y) {
                    region.put_pixel(x, y, *px);
                }
            }
        }
        region
    }

    /// 旧 `move_region` / `copy_and_paste_region`（两者实现相同）：逐像素覆盖，越界跳过。
    fn paste(img: &mut RgbaImage, region: &RgbaImage, dst: (u32, u32)) {
        let (dx, dy) = dst;
        for y in 0..region.height() {
            for x in 0..region.width() {
                if let Some(px) = region.get_pixel_checked(x, y) {
                    if let Some(dst_px) = img.get_pixel_mut_checked(dx + x, dy + y) {
                        *dst_px = *px;
                    }
                }
            }
        }
    }

    pub fn decl() -> TaskDecl {
        TaskDecl::new("fix_tabs", Tier::Surgeon)
            .reads(ScopeSet::exact(TABS))
            .writes(ScopeSet::exact(TABS))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        if !tx.exists(TABS) {
            crate::log_info!("tabs.png not found, skip");
            return Ok(Outcome::default());
        }
        let mut img: RgbaImage = (*tx.image(TABS)?).clone();
        let (width, height) = img.dimensions();
        let (s, _exact) = determine_scale_factor(width, height);
        let sc = |x: u32, y: u32| (x * s, y * s);

        // 步骤 1：(168,0)-(196,128) 右移 14
        let src = {
            let (x1, y1) = sc(168, 0);
            let (x2, y2) = sc(196, 128);
            (x1, y1, x2, y2)
        };
        let region = extract(&img, src);
        paste(&mut img, &region, (src.0 + 14 * s, src.1));

        // 步骤 2：六组左移
        for ((x1, y1, x2, y2), shift) in [
            ((15u32, 0u32, 41u32, 128u32), 2u32),
            ((43, 0, 69, 128), 4),
            ((71, 0, 97, 128), 6),
            ((99, 0, 125, 128), 8),
            ((127, 0, 153, 128), 10),
            ((155, 0, 168, 128), 12),
        ] {
            let (sx1, sy1) = sc(x1, y1);
            let (sx2, sy2) = sc(x2, y2);
            let region = extract(&img, (sx1, sy1, sx2, sy2));
            let dest = (sx1.saturating_sub(shift * s), sy1);
            paste(&mut img, &region, dest);
        }

        // 步骤 3：(0,0)-(26,128) 拷到 (156,0)
        let (x2, y2) = sc(26, 128);
        let region = extract(&img, (0, 0, x2, y2));
        let dest = sc(156, 0);
        paste(&mut img, &region, dest);

        tx.put_image(TABS, &img)?;
        Ok(Outcome {
            changed: 1,
            notes: vec![format!("tabs.png rewritten (scale {s})")],
            ..Outcome::default()
        })
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "fix_tabs" => Some((decl(), run)),
            _ => None,
        }
    }
