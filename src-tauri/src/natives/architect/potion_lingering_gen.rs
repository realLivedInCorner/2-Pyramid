    use super::*;
    use image::Rgba;

    const ITEMS: &str = "assets/minecraft/textures/items";
    const TARGETS: [(&str, &str); 2] = [
        ("potion.png", "lingering_potion.png"),
        ("potion_bottle_drinkable.png", "potion_bottle_lingering.png"),
    ];

    /// 旧 `apply_top_third_transparency` / `..._region`：把每个 `width×width` 方格的
    /// **上三分之一**置为全透明（`y_offset` 为方格起点）。
    fn top_third(img: &mut RgbaImage, width: u32, y_offset: u32) {
        let cutoff = width / 3;
        for y in 0..width {
            for x in 0..width {
                if y < cutoff {
                    img.put_pixel(x, y_offset + y, Rgba([0, 0, 0, 0]));
                }
            }
        }
    }

    pub fn decl() -> TaskDecl {
        TaskDecl::new("generate_potion_lingering", Tier::Architect)
            .reads(ScopeSet::prefix(ITEMS))
            .writes(ScopeSet::prefix(ITEMS))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();
        for (original, new_name) in TARGETS {
            let original_path = format!("{ITEMS}/{original}");
            if !tx.exists(&original_path) {
                continue;
            }
            let mut img: RgbaImage = (*tx.image(&original_path)?).clone();
            let (width, height) = img.dimensions();
            if width == 0 || height == 0 {
                continue;
            }
            if width == height {
                top_third(&mut img, width, 0);
            } else if height % width == 0 {
                for square in 0..(height / width) {
                    top_third(&mut img, width, square * width);
                }
            } else {
                continue;
            }
            tx.put_image(&format!("{ITEMS}/{new_name}"), &img)?;
            let meta = format!("{original_path}.mcmeta");
            if tx.exists(&meta) {
                let bytes = tx.read(&meta)?.unwrap_or_default();
                tx.put(&format!("{ITEMS}/{new_name}.mcmeta"), bytes)?;
            }
            outcome.changed += 1;
        }
        Ok(outcome)
    }
