    use super::rename_blocks_tables::{BLOCK_PAIRS, ITEM_PAIRS, PROCESS_BLOCK_PAIRS};
    use super::*;
    use crate::color::utils::{hsv_to_rgba, rgb_to_hsv};
    use image::{Rgba, RgbaImage};

    pub(super) const TEXTURES: &str = "assets/minecraft/textures";
    pub(super) const ITEMS: &str = "assets/minecraft/textures/items";
    pub(super) const ITEM: &str = "assets/minecraft/textures/item";
    pub(super) const BLOCKS: &str = "assets/minecraft/textures/blocks";
    pub(super) const BLOCK: &str = "assets/minecraft/textures/block";

    pub fn decl() -> TaskDecl {
        TaskDecl::new("rename_blocks_items", Tier::Eraser)
            .reads(ScopeSet::prefix(TEXTURES))
            .writes(ScopeSet::prefix(TEXTURES))
            .exclusive(true)
    }

    fn child_of(prefix: &str, path: &str) -> String {
        let rest = path
            .strip_prefix(prefix)
            .unwrap_or("")
            .trim_start_matches('/');
        rest.to_string()
    }

    /// 旧 `merge_or_rename_dir`：目录合并或改名；返回是否有改动。
    ///
    /// 两种情况都用「逐文件读→写→删源」，**刻意不产生改名规则**：规则与 tombstone 在同层
    /// 求值时互相干扰（已实测两处），而混合运行驱动还要把规则镜像回 workdir（尚未支持）。
    /// 代价是这些文件失去「原始压缩字节透传」，但序列化策略一致，产物字节不受影响。
    pub(super) fn merge_or_rename_dir(tx: &mut Tx<'_>, from: &str, to: &str) -> Result<bool, AromError> {
        if !tx.has_prefix(from)? {
            return Ok(false);
        }
        let files: Vec<String> = tx
            .list(from)?
            .into_iter()
            .filter(|res| !res.is_dir)
            .map(|res| res.path)
            .collect();
        for path in files {
            move_path(tx, &path, &format!("{to}/{}", child_of(from, &path)))?;
        }
        // 源目录（此时已空）整段删除；目标侧已有的、源里没有的文件保持不动
        tx.remove(from)?;
        Ok(true)
    }

    /// 「把 `from` 移到 `to`」：读源内容写到目标（覆盖已存在者），再删掉源。
    ///
    /// 旧实现是「先删目标、再 `fs::rename`」。这里**不能**照抄成「tombstone + 改名规则」：
    /// tombstone 的优先级高于同层规则，会把规则的目标路径一起否掉——真实包上恰好有
    /// 9 个「源名与目标名同时存在」的贴图，它们的**目标名会凭空消失**。改用读→写→删，
    /// 与磁盘语义逐条对应，且不依赖规则与 tombstone 的求值顺序。
    fn move_path(tx: &mut Tx<'_>, from: &str, to: &str) -> Result<(), AromError> {
        let bytes = tx.read(from)?.unwrap_or_default();
        tx.put(to, bytes)?;
        tx.remove(from)?;
        Ok(())
    }

    /// 旧 `rename_with_mcmeta`：png 改名 + 顺带搬同名 `.png.mcmeta`。
    pub(super) fn rename_with_mcmeta(
        tx: &mut Tx<'_>,
        dir: &str,
        old: &str,
        new: &str,
    ) -> Result<bool, AromError> {
        let old_path = format!("{dir}/{old}");
        let new_path = format!("{dir}/{new}");
        if old_path == new_path || !tx.exists(&old_path) {
            return Ok(false);
        }
        move_path(tx, &old_path, &new_path)?;

        let old_meta = format!("{old_path}.mcmeta");
        if tx.exists(&old_meta) {
            move_path(tx, &old_meta, &format!("{new_path}.mcmeta"))?;
        }
        Ok(true)
    }

    /// 旧 `process_blocks::rename_items`：两遍（png+附属 → 裸 `{name}.mcmeta`）。
    fn rename_items(
        tx: &mut Tx<'_>,
        dir: &str,
        pairs: &[(&str, &str)],
    ) -> Result<(), AromError> {
        for (old, new) in pairs {
            let old_path = format!("{dir}/{old}");
            let new_path = format!("{dir}/{new}");
            if !tx.exists(&old_path) {
                continue;
            }
            move_path(tx, &old_path, &new_path)?;

            let old_meta = format!("{old_path}.mcmeta");
            if tx.exists(&old_meta) {
                move_path(tx, &old_meta, &format!("{new_path}.mcmeta"))?;
            }
        }
        for (old, new) in pairs {
            let old_meta = format!("{dir}/{old}.mcmeta");
            let new_meta = format!("{dir}/{new}.mcmeta");
            if tx.exists(&old_meta) && !tx.exists(&new_meta) {
                move_path(tx, &old_meta, &new_meta)?;
            }
        }
        Ok(())
    }

    /// 旧 `process_blocks::process_block_image`：复制一份后做 HSV 调整（源文件保留）。
    fn process_block_image(
        tx: &mut Tx<'_>,
        block: &str,
        file: &str,
        new_name: &str,
        hue_shift: f32,
        brightness_adjust: f32,
        saturation_adjust: f32,
    ) -> Result<(), AromError> {
        let src = format!("{block}/{file}");
        if !tx.exists(&src) {
            return Ok(());
        }
        let mut img: RgbaImage = (*tx.image(&src)?).clone();

        let hue_normalized = hue_shift / 360.0;
        let brightness_factor = brightness_adjust / 100.0;
        let saturation_factor = saturation_adjust / 100.0;
        for pixel in img.pixels_mut() {
            let a = pixel[3];
            let (mut h, mut s, mut v) = rgb_to_hsv(pixel[0], pixel[1], pixel[2]);
            h = (h + hue_normalized).rem_euclid(1.0);
            s = (s + saturation_factor).clamp(0.0, 1.0);
            v = (v + brightness_factor).clamp(0.0, 1.0);
            *pixel = hsv_to_rgba(h, s, v, a);
        }

        let dst = format!("{block}/{new_name}");
        tx.put_image(&dst, &img)?;
        let src_meta = format!("{src}.mcmeta");
        if tx.exists(&src_meta) {
            let bytes = tx.read(&src_meta)?.unwrap_or_default();
            tx.put(&format!("{dst}.mcmeta"), bytes)?;
        }
        Ok(())
    }

    /// 旧 `process_redstone_dust_cross_image`：只留两条对角线（5..=11），其余透明。
    fn process_redstone_dust_cross_image(tx: &mut Tx<'_>, block: &str) -> Result<(), AromError> {
        let cross = format!("{block}/redstone_dust_cross.png");
        if !tx.exists(&cross) {
            return Ok(());
        }
        let mut img: RgbaImage = (*tx.image(&cross)?).clone();
        if img.dimensions() != (16, 16) {
            return Ok(());
        }
        for x in 0..16 {
            for y in 0..16 {
                let diag1 = x == y && (5..=11).contains(&x);
                let diag2 = x + y == 16 && (5..=11).contains(&x);
                if !(diag1 || diag2) {
                    img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                }
            }
        }
        tx.put_image(&format!("{block}/red_dust_dot.png"), &img)?;
        Ok(())
    }

    /// 旧 `process_redstone_dust_line_image`：旋转 90°/270° 得到 line0 / line1。
    fn process_redstone_dust_line_image(tx: &mut Tx<'_>, block: &str) -> Result<(), AromError> {
        let line = format!("{block}/redstone_dust_line.png");
        if !tx.exists(&line) {
            return Ok(());
        }
        let img: RgbaImage = (*tx.image(&line)?).clone();
        let line_0 = image::imageops::rotate90(&img);
        let line_1 = image::imageops::rotate270(&img);
        tx.put_image(&format!("{block}/redstone_dust_line0.png"), &line_0)?;
        tx.put_image(&format!("{block}/redstone_dust_line1.png"), &line_1)?;
        Ok(())
    }

    /// 旧 `process_blocks::change_white_to_yellow`。
    fn change_white_to_yellow(img: &mut RgbaImage) {
        for pixel in img.pixels_mut() {
            if pixel[3] == 0 {
                continue;
            }
            if (180..=255).contains(&pixel[0])
                && (180..=255).contains(&pixel[1])
                && (180..=255).contains(&pixel[2])
            {
                pixel[0] = 255;
                pixel[1] = 255;
                pixel[2] = 0;
            }
        }
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();

        // 1) items → item、blocks → block
        if merge_or_rename_dir(tx, ITEMS, ITEM)? {
            outcome.changed += 1;
            outcome.notes.push("items -> item".into());
        }
        if merge_or_rename_dir(tx, BLOCKS, BLOCK)? {
            outcome.changed += 1;
            outcome.notes.push("blocks -> block".into());
        }

        // 2) 旧 `rename_blocks.rs` 的两张表
        for (old, new) in ITEM_PAIRS {
            if rename_with_mcmeta(tx, ITEM, old, new)? {
                outcome.changed += 1;
            }
        }
        for (old, new) in BLOCK_PAIRS {
            if rename_with_mcmeta(tx, BLOCK, old, new)? {
                outcome.changed += 1;
            }
        }

        // 3) `process_blocks::rename_and_process_blocks(&block_path, false)`
        rename_items(tx, BLOCK, &PROCESS_BLOCK_PAIRS)?;
        process_redstone_dust_cross_image(tx, BLOCK)?;
        process_redstone_dust_line_image(tx, BLOCK)?;
        process_block_image(tx, BLOCK, "oak_planks.png", "warped_planks.png", 130.0, -33.0, 0.0)?;
        process_block_image(tx, BLOCK, "oak_planks.png", "crimson_planks.png", -59.0, -30.0, 0.0)?;

        for ore in [
            "coal_ore",
            "iron_ore",
            "gold_ore",
            "diamond_ore",
            "emerald_ore",
            "redstone_ore",
            "lapis_ore",
        ] {
            process_block_image(
                tx,
                BLOCK,
                &format!("{ore}.png"),
                &format!("deepslate_{ore}.png"),
                0.0,
                -20.0,
                0.0,
            )?;
            if ore == "redstone_ore" {
                process_block_image(tx, BLOCK, "redstone_ore.png", "copper_ore.png", 26.0, 0.0, 0.0)?;
                process_block_image(
                    tx,
                    BLOCK,
                    "copper_ore.png",
                    "deepslate_copper_ore.png",
                    0.0,
                    -20.0,
                    0.0,
                )?;
            }
        }

        let quartz = format!("{BLOCK}/nether_quartz_ore.png");
        if tx.exists(&quartz) {
            let mut gold: RgbaImage = (*tx.image(&quartz)?).clone();
            change_white_to_yellow(&mut gold);
            tx.put_image(&format!("{BLOCK}/nether_gold_ore.png"), &gold)?;
            outcome.changed += 1;
        }

        Ok(outcome)
    }
