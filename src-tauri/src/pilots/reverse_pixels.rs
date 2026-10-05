    use super::*;
    use crate::image_utils::paste_region;
    use image::imageops;

    /// 旧实现的缩放表：宽度 → 倍数，其他尺寸跳过。
    fn scale_of(width: u32) -> Option<u32> {
        match width {
            256 => Some(1),
            512 => Some(2),
            1024 => Some(4),
            2048 => Some(8),
            _ => None,
        }
    }

    pub mod brewing_stand_ui {
        use super::*;

        const PATH: &str = "assets/minecraft/textures/gui/container/brewing_stand.png";

        pub fn decl() -> TaskDecl {
            TaskDecl::new("reverse_fix_brewing_stand_ui", Tier::Surgeon)
                .writes(ScopeSet::exact(PATH))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(PATH) {
                return Ok(Outcome::default());
            }
            let mut img: RgbaImage = (*tx.image(PATH)?).clone();
            let Some(s) = scale_of(img.width()) else {
                return Ok(Outcome {
                    skipped: 1,
                    ..Outcome::default()
                });
            };
            let fill = *img.get_pixel(7 * s, 4 * s);
            for y in (43 * s)..(49 * s) {
                for x in (41 * s)..(79 * s) {
                    img.put_pixel(x, y, fill);
                }
            }
            for y in (14 * s)..(43 * s) {
                for x in (14 * s)..(55 * s) {
                    img.put_pixel(x, y, fill);
                }
            }
            let region = imageops::crop_imm(&img, 55 * s, 50 * s, (119 - 55) * s, (75 - 50) * s)
                .to_image();
            paste_region(&mut img, &region, 55 * s, 45 * s).map_err(AromError::internal)?;
            for y in (70 * s)..(75 * s) {
                for x in (55 * s)..(119 * s) {
                    img.put_pixel(x, y, fill);
                }
            }
            tx.put_image(PATH, &img)?;
            Ok(Outcome {
                changed: 1,
                ..Outcome::default()
            })
        }
    }

    pub mod ui_creative {
        use super::*;

        const PATH: &str =
            "assets/minecraft/textures/gui/container/creative_inventory/tab_inventory.png";

        pub fn decl() -> TaskDecl {
            TaskDecl::new("reverse_fix_ui_creative", Tier::Surgeon)
                .writes(ScopeSet::exact(PATH))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(PATH) {
                return Ok(Outcome::default());
            }
            let mut img: RgbaImage = (*tx.image(PATH)?).clone();
            let Some(s) = scale_of(img.width()) else {
                return Ok(Outcome {
                    skipped: 1,
                    ..Outcome::default()
                });
            };
            let scaled = |c: u32| c * s;
            let fill = *img.get_pixel(scaled(164), scaled(27));
            for y in scaled(19)..scaled(37) {
                for x in scaled(34)..scaled(52) {
                    img.put_pixel(x, y, fill);
                }
            }
            let src_w = scaled(129) - scaled(51);
            let src_h = scaled(53);
            let region = imageops::crop_imm(&img, scaled(51), 0, src_w, src_h).to_image();
            paste_region(&mut img, &region, scaled(6), 0).map_err(AromError::internal)?;
            for y in 0..scaled(53) {
                for x in scaled(84)..scaled(129) {
                    img.put_pixel(x, y, fill);
                }
            }
            tx.put_image(PATH, &img)?;
            Ok(Outcome {
                changed: 1,
                ..Outcome::default()
            })
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "reverse_fix_brewing_stand_ui" => {
                Some((brewing_stand_ui::decl(), brewing_stand_ui::run))
            }
            "reverse_fix_ui_creative" => Some((ui_creative::decl(), ui_creative::run)),
            _ => None,
        }
    }
