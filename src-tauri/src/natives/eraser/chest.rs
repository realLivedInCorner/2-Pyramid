    use super::*;

    pub const CHEST: &str = "assets/minecraft/textures/entity/chest";
    const SINGLE: [&str; 4] = ["ender.png", "normal.png", "trapped.png", "christmas.png"];
    const DOUBLE: [&str; 3] = [
        "normal_double.png",
        "trapped_double.png",
        "christmas_double.png",
    ];

    pub fn decl() -> TaskDecl {
        TaskDecl::new("process_chest_folder", Tier::Eraser)
            .reads(ScopeSet::prefix(CHEST))
            .writes(ScopeSet::prefix(CHEST))
            .exclusive(true)
    }

    /// 旧实现的缩放表：宽度 → 缩放倍数（不支持的尺寸跳过）。
    pub(super) fn scale_for_single(width: u32) -> Option<u32> {
        match width {
            64 => Some(1),
            128 => Some(2),
            256 => Some(4),
            512 => Some(8),
            1024 => Some(16),
            _ => None,
        }
    }

    fn scale_for_double(width: u32, height: u32) -> Option<u32> {
        match (width, height) {
            (128, 64) => Some(1),
            (256, 128) => Some(2),
            (512, 256) => Some(4),
            (1024, 512) => Some(8),
            _ => None,
        }
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        use crate::chest_region as legacy;

        if !tx.has_prefix(CHEST)? {
            return Ok(Outcome::default());
        }
        let mut outcome = Outcome::default();

        for name in SINGLE {
            let path = format!("{CHEST}/{name}");
            if !tx.exists(&path) {
                continue;
            }
            let mut img: RgbaImage = (*tx.image(&path)?).clone();
            let Some(s) = scale_for_single(img.width()) else {
                outcome.skipped += 1;
                continue;
            };
            let sb = |x1: u32, y1: u32, x2: u32, y2: u32| (x1 * s, y1 * s, x2 * s, y2 * s);

            for (a, b) in [
                (sb(14, 0, 28, 14), sb(28, 0, 42, 14)),
                (sb(14, 14, 28, 19), sb(42, 14, 56, 19)),
                (sb(14, 19, 28, 33), sb(28, 19, 42, 33)),
                (sb(14, 33, 28, 43), sb(42, 33, 56, 43)),
            ] {
                legacy::swap_and_mirror(&mut img, a, b).map_err(AromError::internal)?;
            }
            for b in [
                sb(14, 0, 28, 14),
                sb(28, 0, 42, 14),
                sb(0, 14, 14, 19),
                sb(28, 14, 42, 19),
                sb(14, 19, 28, 33),
                sb(28, 19, 42, 33),
                sb(0, 33, 14, 43),
                sb(28, 33, 42, 43),
            ] {
                legacy::mirror_region(&mut img, b);
            }
            tx.put_image(&path, &img)?;
            outcome.changed += 1;
        }

        for name in DOUBLE {
            let path = format!("{CHEST}/{name}");
            if !tx.exists(&path) {
                continue;
            }
            let img: RgbaImage = (*tx.image(&path)?).clone();
            let Some(s) = scale_for_double(img.width(), img.height()) else {
                outcome.skipped += 1;
                continue;
            };
            let prefix = if name.contains("christmas") {
                "christmas"
            } else if name.contains("normal") {
                "normal"
            } else {
                "trapped"
            };
            let mut left = RgbaImage::new(64 * s, 64 * s);
            let mut right = RgbaImage::new(64 * s, 64 * s);
            legacy::generate_double_chest_images(&mut left, &mut right, &img, s);
            tx.put_image(&format!("{CHEST}/{prefix}_left.png"), &left)?;
            tx.put_image(&format!("{CHEST}/{prefix}_right.png"), &right)?;
            outcome.changed += 1;
            outcome.notes.push(format!("{name} -> {prefix}_left/_right"));
        }

        Ok(outcome)
    }
