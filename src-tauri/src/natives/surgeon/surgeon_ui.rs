    use super::*;
    use crate::scale_factor::determine_scale_factor;

    const WIDGETS: &str = "assets/minecraft/textures/gui/widgets.png";
    const TAB_INVENTORY: &str =
        "assets/minecraft/textures/gui/container/creative_inventory/tab_inventory.png";

    /// 旧 `extract_region`：越界处留透明。
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

    /// 旧 `copy_and_paste_region`：逐像素覆盖，越界跳过、越界读取跳过。
    fn copy_paste(img: &mut RgbaImage, src: (u32, u32, u32, u32), dst: (u32, u32)) {
        let region = extract(img, src);
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

    /// 旧 `fill_region`：越界跳过。
    fn fill(img: &mut RgbaImage, region: (u32, u32, u32, u32), color: image::Rgba<u8>) {
        let (x1, y1, x2, y2) = region;
        for y in y1..y2 {
            for x in x1..x2 {
                if let Some(px) = img.get_pixel_mut_checked(x, y) {
                    *px = color;
                }
            }
        }
    }

    /// 旧 `converters/ui/sub_hand.rs`。
    pub mod sub_hand {
        use super::*;

        pub fn decl() -> TaskDecl {
            TaskDecl::new("fix_ui_sub_hand", Tier::Surgeon)
                .reads(ScopeSet::exact(WIDGETS))
                .writes(ScopeSet::exact(WIDGETS))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(WIDGETS) {
                crate::log_info!("widgets.png not found, skip");
                return Ok(Outcome::default());
            }
            let mut img: RgbaImage = (*tx.image(WIDGETS)?).clone();
            let (width, height) = img.dimensions();
            let (s, _exact) = determine_scale_factor(width, height);

            let src = (1 * s, 23 * s, 23 * s, 45 * s);
            copy_paste(&mut img, src, (24 * s, 23 * s));
            copy_paste(&mut img, src, (60 * s, 23 * s));

            tx.put_image(WIDGETS, &img)?;
            Ok(Outcome {
                changed: 1,
                notes: vec![format!("widgets.png offhand patch (scale {s})")],
                ..Outcome::default()
            })
        }
    }

    /// 旧 `converters/ui/creative.rs`。
    pub mod creative {
        use super::*;

        pub fn decl() -> TaskDecl {
            TaskDecl::new("fix_ui_creative", Tier::Surgeon)
                .reads(ScopeSet::exact(TAB_INVENTORY))
                .writes(ScopeSet::exact(TAB_INVENTORY))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(TAB_INVENTORY) {
                crate::log_info!("tab_inventory.png missing, skip");
                return Ok(Outcome::default());
            }
            let mut img: RgbaImage = (*tx.image(TAB_INVENTORY)?).clone();
            let (width, height) = img.dimensions();
            let (s, _exact) = determine_scale_factor(width, height);

            // 步骤 1：(6,0)-(84,53) 拷到 (51,0)
            copy_paste(&mut img, (6 * s, 0, 84 * s, 53 * s), (51 * s, 0));

            // 步骤 2：用 **(164,27)**（第一步之后的图）的颜色填左区 (6,0)-(53,53)
            let fill_color = img
                .get_pixel_checked(164 * s, 27 * s)
                .copied()
                .unwrap_or(image::Rgba([0, 0, 0, 0]));
            fill(&mut img, (6 * s, 0, 53 * s, 53 * s), fill_color);

            // 步骤 3：(53,5) 起 18×18 拷到 (34,19)（源同样取自被前面步骤改过的图）
            copy_paste(
                &mut img,
                (53 * s, 5 * s, 71 * s, 23 * s),
                (34 * s, 19 * s),
            );

            tx.put_image(TAB_INVENTORY, &img)?;
            Ok(Outcome {
                changed: 1,
                notes: vec![format!("tab_inventory.png adjusted (scale {s})")],
                ..Outcome::default()
            })
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "fix_ui_sub_hand" => Some((sub_hand::decl(), sub_hand::run)),
            "fix_ui_creative" => Some((creative::decl(), creative::run)),
            _ => None,
        }
    }
