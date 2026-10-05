//! M1 试点：三个低风险转换器的 A-ROM 原生实现，配双轨对照。
//!
//! 目的不是「把这三个任务迁完」，而是**证明新契约能表达真任务**：三个试点刻意覆盖三种机制——
//!
//! | 试点 | 机制 | 旧实现 | A-ROM 表达 |
//! |---|---|---|---|
//! | [`drop_font`] | 整棵子树删除 | `ctx.defer_remove_dir` + 收尾清理 | `tx.remove(prefix)`（层里的 Tombstone，子树语义已由层保证） |
//! | [`old_paths`] | 复制而不动源 | `fs::copy` | `tx.alias(from, to)`（零字节复制） |
//! | [`animated`] | 读 JSON + 按 PNG 尺寸推导 + 写回 | `fs::read_to_string` / `fs::write` | `tx.text()` + `tx.image()` + `tx.put()`（写入进层，可回滚） |
//!
//! **旧实现原样保留**：本模块只是并行新增，双轨对照用例同时跑两边并比对最终产物
//! （旧：解压 → 跑旧函数 → 重打包；新：`Pack` → 跑试点 → 序列化 → 容器级闸门）。
//! 注册切换属 M2，本模块不参与生产路径。

use std::sync::Arc;

use image::RgbaImage;

use crate::arom::{AromError, ScopeSet, TaskDecl, Tier, Tx};

/// 任务执行结果（可观测性最小集）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    pub changed: usize,
    pub skipped: usize,
    pub notes: Vec<String>,
    /// **延迟删除**：旧实现用 `defer_remove_file/dir` 登记、在全局清理点统一执行；
    /// 原生任务同样只登记，由驱动在清理点应用（见 `native_run`）。
    pub deferred_removals: Vec<String>,
}

/// **延迟删除**：只登记路径，由驱动在清理点统一 tombstone——复刻旧实现 `defer_remove_*` 的时机。
///
/// 立即删除会让**更晚**的任务看不到文件（实测：反向改名因源文件已被删而搬不动东西，§9.23）。
fn defer_remove_if_present(tx: &Tx<'_>, path: &str) -> Result<Outcome, AromError> {
    if !tx.has_prefix(path)? {
        return Ok(Outcome::default());
    }
    Ok(Outcome {
        deferred_removals: vec![path.to_string()],
        notes: vec![format!("defer removal of {path}")],
        ..Outcome::default()
    })
}

/// 整棵子树删除（对应旧 `delete_font_folder`）。
pub mod drop_font {
    use super::*;

    pub const TARGET: &str = "assets/minecraft/font";

    pub fn decl() -> TaskDecl {
        TaskDecl::new("delete_font_folder", Tier::Eraser)
            .writes(ScopeSet::prefix(TARGET))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        // 注意：容器里**可能没有**这个目录的显式条目（目录由文件隐含）。
        // 旧实现查的是解压后的文件系统（目录必然存在），所以这里用 `has_prefix`。
        if !tx.has_prefix(TARGET)? {
            return Ok(Outcome::default());
        }
        tx.remove(TARGET)?;
        Ok(Outcome {
            changed: 1,
            notes: vec![format!("removed {TARGET}")],
            ..Outcome::default()
        })
    }
}

/// 「存在就整段删除」类任务的共同实现。
///
/// 旧实现一律是 `path.exists()` + `defer_remove_file/dir`；这里用 `has_prefix`，
/// 以覆盖「目录只由文件隐含、没有显式条目」的情况（理由同 `drop_font`）。
fn remove_if_present(tx: &mut Tx<'_>, path: &str) -> Result<Outcome, AromError> {
    if !tx.has_prefix(path)? {
        return Ok(Outcome::default());
    }
    tx.remove(path)?;
    Ok(Outcome {
        changed: 1,
        notes: vec![format!("removed {path}")],
        ..Outcome::default()
    })
}

/// 旧 `delete_horse_folder`（`converters/textures/drop_horse.rs`）。
pub mod drop_horse {
    use super::*;

    pub const TARGET: &str = "assets/minecraft/textures/entity/horse";

    pub fn decl() -> TaskDecl {
        TaskDecl::new("delete_horse_folder", Tier::Eraser)
            .writes(ScopeSet::prefix(TARGET))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        remove_if_present(tx, TARGET)
    }
}

/// 旧 `delete_shaders_folder`（`converters/textures/drop_shaders.rs`）。
pub mod drop_shaders {
    use super::*;

    pub const TARGET: &str = "assets/minecraft/shaders";

    pub fn decl() -> TaskDecl {
        TaskDecl::new("delete_shaders_folder", Tier::Eraser)
            .writes(ScopeSet::prefix(TARGET))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        remove_if_present(tx, TARGET)
    }
}

/// 旧 `delete_enchanted_item_glint`（`converters/textures/drop_enchanted_glint.rs`）——删单个文件。
pub mod drop_glint {
    use super::*;

    pub const TARGET: &str = "assets/minecraft/textures/misc/enchanted_item_glint.png";

    pub fn decl() -> TaskDecl {
        TaskDecl::new("delete_enchanted_item_glint", Tier::Eraser)
            .writes(ScopeSet::exact(TARGET))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        remove_if_present(tx, TARGET)
    }
}

/// 旧 `delete_blockstates_models`（`converters/textures/drop_blockstates_models.rs`）——一次删两个目录。
pub mod drop_blockstates {
    use super::*;

    pub const TARGETS: [&str; 2] = ["assets/minecraft/blockstates", "assets/minecraft/models"];

    pub fn decl() -> TaskDecl {
        TaskDecl::new("delete_blockstates_models", Tier::Eraser)
            .writes(ScopeSet::prefix(TARGETS[0]).union(&ScopeSet::prefix(TARGETS[1])))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();
        for target in TARGETS {
            let one = remove_if_present(tx, target)?;
            outcome.changed += one.changed;
            outcome.notes.extend(one.notes);
        }
        Ok(outcome)
    }
}

mod rename_blocks_tables;

/// 旧 `rename_blocks_items`（`converters/textures/rename_blocks.rs`）**连同**它在末尾调用的
/// `process_blocks::rename_and_process_blocks(&block_path, false)`——两者是同一条注册任务，
/// 必须一起迁移，否则产物不同。
///
/// 语义要点（逐条对应旧实现）：
/// 1. `items → item`、`blocks → block`：目标不存在则整体改名；目标已存在则**逐文件移动、
///    同名时源覆盖目标**，最后删掉源目录（旧的 `merge_or_rename_dir`）；
/// 2. 旧表的 154 条重命名（`rename_with_mcmeta`：顺带搬 `.png.mcmeta`）；
/// 3. `rename_items` 的两遍：先搬 png（连带 `.png.mcmeta`），再搬「不带 .png」的 `{name}.mcmeta`；
/// 4. 红石粉十字/线贴图派生、木板与矿石的色相/明度派生、`nether_gold_ore` 的白→黄。
///
/// 派生图走 `tx.put_image`，它与旧实现的 `img.save()` 是同一条编码路径
/// （`DynamicImage::write_to(.., Png)`），因此产物字节可期一致；这一点由闸门实测把关。
pub mod rename_blocks {
    use super::rename_blocks_tables::{BLOCK_PAIRS, ITEM_PAIRS, PROCESS_BLOCK_PAIRS};
    use super::*;
    use crate::converters::color::utils::{hsv_to_rgba, rgb_to_hsv};
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
}

/// 旧 `process_chest_folder`（`converters/ui/process_chest_folder.rs`，**ui 模块第一个迁移任务**）。
///
/// 纯图像几何变换：单胸按宽度定缩放做 4 组「交换+镜像」与 8 组镜像；双胸用旧实现的
/// overlay 表生成 `{prefix}_left.png` / `{prefix}_right.png`。旧实现里的纯函数
/// （`swap_and_mirror` / `mirror_region` / `generate_double_chest_images`）**直接复用**
/// （放开到 `pub(crate)`），避免把这张几十行的 overlay 表抄第二遍。
pub mod chest {
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
}

/// 旧 `reverse_process_chest_folder`（`converters/reverse/chest_folder.rs`）——**反向转换的第一个任务**。
///
/// 它是正向 `process_chest_folder` 的镜像版：同一张缩放表、同样 4 组「交换+镜像」与 8 组镜像，
/// 但**不处理双胸**（反向只把单胸的变换倒回去）。纯函数同样直接复用：交换用反向模块自己的
/// `swap_and_mirror`，镜像用正向模块的 `mirror_region`（两者语义逐字相同：先水平再垂直翻转后 overlay）。
pub mod chest_reverse {
    use super::*;

    pub const CHEST: &str = "assets/minecraft/textures/entity/chest";
    const SINGLE: [&str; 4] = ["ender.png", "normal.png", "trapped.png", "christmas.png"];

    pub fn decl() -> TaskDecl {
        TaskDecl::new("reverse_process_chest_folder", Tier::Eraser)
            .reads(ScopeSet::prefix(CHEST))
            .writes(ScopeSet::prefix(CHEST))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        

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
            let Some(s) = super::chest::scale_for_single(img.width()) else {
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
                crate::chest_region::swap_and_mirror(&mut img, a, b)
                    .map_err(AromError::internal)?;
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
                crate::chest_region::mirror_region(&mut img, b);
            }
            tx.put_image(&path, &img)?;
            outcome.changed += 1;
        }
        Ok(outcome)
    }
}

/// 反向侧的**空操作**与**删单路径**两类任务，成批迁移。
///
/// - 空操作类：旧实现有明确文档说明「无法从产物反推原状」（`cut_gui` 抽走了图集、`horse_ui`
///   与 `overlay_icons` 把外部贴图永久合成进去、`sub_hand` 同理），因此旧实现就是 `Ok(())`；
///   原生实现同样什么都不做，声明里**读写都为空**（精确反映事实）。
/// - 删单路径类：与正向的 `drop_*` 同构，直接复用 `remove_if_present`。
pub mod reverse_trivial {
    use super::*;

    macro_rules! noop_pilot {
        ($m:ident, $task:literal, $tier:expr) => {
            pub mod $m {
                use super::*;
                pub fn decl() -> TaskDecl {
                    TaskDecl::new($task, $tier).exclusive(true)
                }
                pub fn run(_tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
                    Ok(Outcome::default())
                }
            }
        };
    }

    macro_rules! drop_pilot {
        ($m:ident, $task:literal, $path:literal) => {
            pub mod $m {
                use super::*;
                pub const TARGET: &str = $path;
                pub fn decl() -> TaskDecl {
                    TaskDecl::new($task, Tier::Eraser)
                        .writes(ScopeSet::exact(TARGET))
                        .exclusive(true)
                }
                pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
                    // 旧实现是 `defer_remove_file/dir`：删除**延迟到清理点**，
                    // 否则更晚的任务看不到该文件（§9.23）
                    defer_remove_if_present(tx, TARGET)
                }
            }
        };
    }

    noop_pilot!(cut_gui, "reverse_cut_gui", Tier::Surgeon);
    noop_pilot!(horse, "reverse_fix_horse_ui", Tier::Surgeon);
    noop_pilot!(overlay_icons, "reverse_overlay_icons", Tier::Surgeon);
    noop_pilot!(sub_hand, "reverse_fix_ui_sub_hand", Tier::Surgeon);

    drop_pilot!(
        snow_bucket,
        "reverse_generate_snow_bucket",
        "assets/minecraft/textures/item/powder_snow_bucket.png"
    );
    drop_pilot!(
        smithing_ui,
        "reverse_generate_smithing_ui",
        "assets/minecraft/textures/gui/container/smithing.png"
    );
    drop_pilot!(
        slider,
        "reverse_fix_slider",
        "assets/minecraft/textures/gui/slider.png"
    );

    macro_rules! defer_list_pilot {
        ($m:ident, $task:literal, [$($p:literal),*]) => {
            pub mod $m {
                use super::*;
                pub const TARGETS: [&str; 0 $(+ { let _ = $p; 1 })*] = [$($p),*];
                pub fn decl() -> TaskDecl {
                    let mut scope = ScopeSet::none();
                    $( scope = scope.union(&ScopeSet::exact($p)); )*
                    TaskDecl::new($task, Tier::Eraser).writes(scope).exclusive(true)
                }
                pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
                    let mut outcome = Outcome::default();
                    for path in TARGETS {
                        outcome
                            .deferred_removals
                            .extend(defer_remove_if_present(tx, path)?.deferred_removals);
                    }
                    Ok(outcome)
                }
            }
        };
    }

    defer_list_pilot!(
        tipped_arrows,
        "reverse_generate_tipped_arrow_images",
        [
            "assets/minecraft/textures/items/tipped_arrow_base.png",
            "assets/minecraft/textures/items/tipped_arrow_head.png"
        ]
    );

    /// 旧 `reverse_generate_boat`：延迟删 5 个船变体 + **有守卫**的立即改名（`boat.png` 不存在才改）。
    pub mod boat {
        use super::*;

        const ITEMS: &str = "assets/minecraft/textures/items";
        const VARIANTS: [&str; 5] = [
            "oak_boat.png",
            "birch_boat.png",
            "acacia_boat.png",
            "dark_oak_boat.png",
            "jungle_boat.png",
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("reverse_generate_boat", Tier::Eraser)
                .reads(ScopeSet::prefix(ITEMS))
                .writes(ScopeSet::prefix(ITEMS))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for name in VARIANTS {
                outcome
                    .deferred_removals
                    .extend(defer_remove_if_present(tx, &format!("{ITEMS}/{name}"))?.deferred_removals);
            }
            let spruce = format!("{ITEMS}/spruce_boat.png");
            let boat = format!("{ITEMS}/boat.png");
            // 旧实现带守卫：目标已存在就不改名（不覆盖）
            if tx.exists(&spruce) && !tx.exists(&boat) {
                let bytes = tx.read(&spruce)?.unwrap_or_default();
                tx.put(&boat, bytes)?;
                tx.remove(&spruce)?;
                outcome.changed += 1;
                outcome.notes.push("spruce_boat -> boat".into());
            }
            Ok(outcome)
        }
    }

    defer_list_pilot!(
        crossbow,
        "reverse_generate_crossbow",
        [
            "assets/minecraft/textures/item/crossbow_standby.png",
            "assets/minecraft/textures/item/crossbow_pulling_0.png",
            "assets/minecraft/textures/item/crossbow_pulling_1.png",
            "assets/minecraft/textures/item/crossbow_pulling_2.png",
            "assets/minecraft/textures/item/crossbow_arrow.png",
            "assets/minecraft/textures/item/crossbow_firework.png"
        ]
    );

    defer_list_pilot!(
        fish_bucket,
        "reverse_generate_fish_bucket",
        [
            "assets/minecraft/textures/item/axolotl_bucket.png",
            "assets/minecraft/textures/item/cod_bucket.png",
            "assets/minecraft/textures/item/pufferfish_bucket.png",
            "assets/minecraft/textures/item/salmon_bucket.png",
            "assets/minecraft/textures/item/tropical_fish_bucket.png",
            "assets/minecraft/textures/item/tadpole_bucket.png"
        ]
    );

    /// 带 `.png.mcmeta` 附属的延迟删除（planks 家族的旧实现：本体与附属各删一次）。
    macro_rules! defer_with_meta_pilot {
        ($m:ident, $task:literal, [$($p:literal),*]) => {
            pub mod $m {
                use super::*;
                pub const TARGETS: [&str; 0 $(+ { let _ = $p; 1 })*] = [$($p),*];
                pub fn decl() -> TaskDecl {
                    let mut scope = ScopeSet::none();
                    $( scope = scope.union(&ScopeSet::exact($p)); )*
                    TaskDecl::new($task, Tier::Eraser).writes(scope).exclusive(true)
                }
                pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
                    let mut outcome = Outcome::default();
                    for base in TARGETS {
                        outcome
                            .deferred_removals
                            .extend(defer_remove_if_present(tx, base)?.deferred_removals);
                        let meta = format!("{base}.mcmeta");
                        outcome
                            .deferred_removals
                            .extend(defer_remove_if_present(tx, &meta)?.deferred_removals);
                    }
                    Ok(outcome)
                }
            }
        };
    }

    defer_list_pilot!(
        furnace,
        "reverse_generate_furnace",
        [
            "assets/minecraft/textures/gui/container/blast_furnace.png",
            "assets/minecraft/textures/gui/container/smoker.png"
        ]
    );

    defer_list_pilot!(
        potion_lingering,
        "reverse_generate_potion_lingering",
        [
            "assets/minecraft/textures/items/lingering_potion.png",
            "assets/minecraft/textures/items/lingering_potion.png.mcmeta",
            "assets/minecraft/textures/items/potion_bottle_lingering.png",
            "assets/minecraft/textures/items/potion_bottle_lingering.png.mcmeta"
        ]
    );

    defer_with_meta_pilot!(
        redwood_planks,
        "reverse_generate_redwood_cherry_bamboo_planks",
        [
            "assets/minecraft/textures/block/mangrove_planks.png",
            "assets/minecraft/textures/block/cherry_planks.png",
            "assets/minecraft/textures/block/bamboo_planks.png",
            "assets/minecraft/textures/block/mangrove_log.png",
            "assets/minecraft/textures/block/mangrove_log_top.png",
            "assets/minecraft/textures/block/cherry_log.png",
            "assets/minecraft/textures/block/cherry_log_top.png",
            "assets/minecraft/textures/block/bamboo_block.png",
            "assets/minecraft/textures/block/bamboo_block_top.png",
            "assets/minecraft/textures/block/bamboo_mosaic.png"
        ]
    );

    defer_with_meta_pilot!(
        pale_planks,
        "reverse_generate_pale_planks",
        [
            "assets/minecraft/textures/block/pale_oak_planks.png",
            "assets/minecraft/textures/block/pale_oak_log.png",
            "assets/minecraft/textures/block/pale_oak_log_top.png"
        ]
    );

    defer_with_meta_pilot!(
        poplar_planks,
        "reverse_generate_poplar_planks",
        [
            "assets/minecraft/textures/block/poplar_planks.png",
            "assets/minecraft/textures/block/poplar_log.png",
            "assets/minecraft/textures/block/poplar_log_top.png",
            "assets/minecraft/textures/block/stripped_poplar_log.png",
            "assets/minecraft/textures/block/stripped_poplar_log_top.png",
            "assets/minecraft/textures/block/poplar_door_top.png",
            "assets/minecraft/textures/block/poplar_door_bottom.png",
            "assets/minecraft/textures/block/poplar_trapdoor.png",
            "assets/minecraft/textures/block/poplar_shelf.png",
            "assets/minecraft/textures/block/poplar_sapling.png",
            "assets/minecraft/textures/block/poplar_sign.png",
            "assets/minecraft/textures/block/poplar_hanging_sign.png",
            "assets/minecraft/textures/block/red_poplar_leaves.png",
            "assets/minecraft/textures/block/orange_poplar_leaves.png",
            "assets/minecraft/textures/block/yellow_poplar_leaves.png",
            "assets/minecraft/textures/item/poplar_sign.png",
            "assets/minecraft/textures/item/poplar_hanging_sign.png",
            "assets/minecraft/textures/item/poplar_door.png",
            "assets/minecraft/textures/item/poplar_boat.png",
            "assets/minecraft/textures/item/poplar_chest_boat.png",
            "assets/minecraft/textures/entity/boat/poplar.png",
            "assets/minecraft/textures/entity/chest_boat/poplar.png"
        ]
    );

    /// 任务名 → (声明, 实现)。驱动按名字派发。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "reverse_cut_gui" => Some((cut_gui::decl(), cut_gui::run)),
            "reverse_fix_horse_ui" => Some((horse::decl(), horse::run)),
            "reverse_overlay_icons" => Some((overlay_icons::decl(), overlay_icons::run)),
            "reverse_fix_ui_sub_hand" => Some((sub_hand::decl(), sub_hand::run)),
            "reverse_generate_snow_bucket" => Some((snow_bucket::decl(), snow_bucket::run)),
            "reverse_generate_smithing_ui" => Some((smithing_ui::decl(), smithing_ui::run)),
            "reverse_fix_slider" => Some((slider::decl(), slider::run)),
            _ => None,
        }
    }
}

/// 旧 `reverse_rename_mcpatcher_to_optifine`（`converters/reverse/mcpatcher_to_optifine.rs`）：
/// 把 `optifine/` 改回 `mcpatcher/`，**仅当目标不存在**（旧实现带守卫，不合并）。
pub mod mcpatcher_optifine_reverse {
    use super::rename_blocks::merge_or_rename_dir;
    use super::*;

    const OPTIFINE: &str = "assets/minecraft/optifine";
    const MCPATCHER: &str = "assets/minecraft/mcpatcher";

    pub fn decl() -> TaskDecl {
        TaskDecl::new("reverse_rename_mcpatcher_to_optifine", Tier::Eraser)
            .reads(ScopeSet::prefix("assets/minecraft"))
            .writes(ScopeSet::prefix("assets/minecraft"))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        if !tx.has_prefix(OPTIFINE)? || tx.has_prefix(MCPATCHER)? {
            return Ok(Outcome::default());
        }
        merge_or_rename_dir(tx, OPTIFINE, MCPATCHER)?;
        Ok(Outcome {
            changed: 1,
            notes: vec![format!("{OPTIFINE} -> {MCPATCHER}")],
            ..Outcome::default()
        })
    }
}

/// 反向「延迟删除」批次的第二批（复用 §9.24 的延迟删除机制）。
///
/// 三个任务都只有字面路径、无循环：
/// - `reverse_generate_shulker_box_ui`：延迟删一个 gui 文件；
/// - `reverse_fix_sign_entities`：延迟删一整棵 `entity/signs` 子树；
/// - `reverse_fix_smithing2_villager2_ui`：**立即**把 `villager_backup.png` 改名回 `villager.png`
///   （旧实现是 `fs::rename`，不是延迟），再延迟删 `smithing.png`。
pub mod reverse_defer {
    use super::*;

    macro_rules! defer_pilot {
        ($m:ident, $task:literal, $path:literal) => {
            pub mod $m {
                use super::*;
                pub const TARGET: &str = $path;
                pub fn decl() -> TaskDecl {
                    TaskDecl::new($task, Tier::Eraser)
                        .writes(ScopeSet::exact(TARGET))
                        .exclusive(true)
                }
                pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
                    defer_remove_if_present(tx, TARGET)
                }
            }
        };
    }

    defer_pilot!(
        shulker_box,
        "reverse_generate_shulker_box_ui",
        "assets/minecraft/textures/gui/container/shulker_box.png"
    );
    defer_pilot!(
        sign_entities,
        "reverse_fix_sign_entities",
        "assets/minecraft/textures/entity/signs"
    );

    pub mod smithing_villager {
        use super::*;

        const GUI: &str = "assets/minecraft/textures/gui/container";
        const SMITHING: &str = "assets/minecraft/textures/gui/container/smithing.png";
        const BACKUP: &str = "assets/minecraft/textures/gui/container/villager_backup.png";
        const VILLAGER: &str = "assets/minecraft/textures/gui/container/villager.png";

        pub fn decl() -> TaskDecl {
            TaskDecl::new("reverse_fix_smithing2_villager2_ui", Tier::Eraser)
                .reads(ScopeSet::prefix(GUI))
                .writes(ScopeSet::prefix(GUI))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            // 立即恢复（旧实现是 fs::rename，非延迟）
            if tx.exists(BACKUP) {
                let bytes = tx.read(BACKUP)?.unwrap_or_default();
                tx.put(VILLAGER, bytes)?;
                tx.remove(BACKUP)?;
                outcome.changed += 1;
                outcome.notes.push("villager_backup -> villager".into());
            }
            // 延迟删除生成的 smithing.png
            outcome
                .deferred_removals
                .extend(defer_remove_if_present(tx, SMITHING)?.deferred_removals);
            Ok(outcome)
        }
    }

    macro_rules! defer_list_pilot {
        ($m:ident, $task:literal, [$($p:literal),*]) => {
            pub mod $m {
                use super::*;
                pub const TARGETS: [&str; 0 $(+ { let _ = $p; 1 })*] = [$($p),*];
                pub fn decl() -> TaskDecl {
                    let mut scope = ScopeSet::none();
                    $( scope = scope.union(&ScopeSet::exact($p)); )*
                    TaskDecl::new($task, Tier::Eraser).writes(scope).exclusive(true)
                }
                pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
                    let mut outcome = Outcome::default();
                    for path in TARGETS {
                        outcome
                            .deferred_removals
                            .extend(defer_remove_if_present(tx, path)?.deferred_removals);
                    }
                    Ok(outcome)
                }
            }
        };
    }

    defer_list_pilot!(
        tipped_arrows,
        "reverse_generate_tipped_arrow_images",
        [
            "assets/minecraft/textures/items/tipped_arrow_base.png",
            "assets/minecraft/textures/items/tipped_arrow_head.png"
        ]
    );

    /// 旧 `reverse_generate_boat`：延迟删 5 个船变体 + **有守卫**的立即改名（`boat.png` 不存在才改）。
    pub mod boat {
        use super::*;

        const ITEMS: &str = "assets/minecraft/textures/items";
        const VARIANTS: [&str; 5] = [
            "oak_boat.png",
            "birch_boat.png",
            "acacia_boat.png",
            "dark_oak_boat.png",
            "jungle_boat.png",
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("reverse_generate_boat", Tier::Eraser)
                .reads(ScopeSet::prefix(ITEMS))
                .writes(ScopeSet::prefix(ITEMS))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for name in VARIANTS {
                outcome
                    .deferred_removals
                    .extend(defer_remove_if_present(tx, &format!("{ITEMS}/{name}"))?.deferred_removals);
            }
            let spruce = format!("{ITEMS}/spruce_boat.png");
            let boat = format!("{ITEMS}/boat.png");
            // 旧实现带守卫：目标已存在就不改名（不覆盖）
            if tx.exists(&spruce) && !tx.exists(&boat) {
                let bytes = tx.read(&spruce)?.unwrap_or_default();
                tx.put(&boat, bytes)?;
                tx.remove(&spruce)?;
                outcome.changed += 1;
                outcome.notes.push("spruce_boat -> boat".into());
            }
            Ok(outcome)
        }
    }

    defer_list_pilot!(
        crossbow,
        "reverse_generate_crossbow",
        [
            "assets/minecraft/textures/item/crossbow_standby.png",
            "assets/minecraft/textures/item/crossbow_pulling_0.png",
            "assets/minecraft/textures/item/crossbow_pulling_1.png",
            "assets/minecraft/textures/item/crossbow_pulling_2.png",
            "assets/minecraft/textures/item/crossbow_arrow.png",
            "assets/minecraft/textures/item/crossbow_firework.png"
        ]
    );

    defer_list_pilot!(
        fish_bucket,
        "reverse_generate_fish_bucket",
        [
            "assets/minecraft/textures/item/axolotl_bucket.png",
            "assets/minecraft/textures/item/cod_bucket.png",
            "assets/minecraft/textures/item/pufferfish_bucket.png",
            "assets/minecraft/textures/item/salmon_bucket.png",
            "assets/minecraft/textures/item/tropical_fish_bucket.png",
            "assets/minecraft/textures/item/tadpole_bucket.png"
        ]
    );

    /// 带 `.png.mcmeta` 附属的延迟删除（planks 家族的旧实现：本体与附属各删一次）。
    macro_rules! defer_with_meta_pilot {
        ($m:ident, $task:literal, [$($p:literal),*]) => {
            pub mod $m {
                use super::*;
                pub const TARGETS: [&str; 0 $(+ { let _ = $p; 1 })*] = [$($p),*];
                pub fn decl() -> TaskDecl {
                    let mut scope = ScopeSet::none();
                    $( scope = scope.union(&ScopeSet::exact($p)); )*
                    TaskDecl::new($task, Tier::Eraser).writes(scope).exclusive(true)
                }
                pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
                    let mut outcome = Outcome::default();
                    for base in TARGETS {
                        outcome
                            .deferred_removals
                            .extend(defer_remove_if_present(tx, base)?.deferred_removals);
                        let meta = format!("{base}.mcmeta");
                        outcome
                            .deferred_removals
                            .extend(defer_remove_if_present(tx, &meta)?.deferred_removals);
                    }
                    Ok(outcome)
                }
            }
        };
    }

    defer_list_pilot!(
        furnace,
        "reverse_generate_furnace",
        [
            "assets/minecraft/textures/gui/container/blast_furnace.png",
            "assets/minecraft/textures/gui/container/smoker.png"
        ]
    );

    defer_list_pilot!(
        potion_lingering,
        "reverse_generate_potion_lingering",
        [
            "assets/minecraft/textures/items/lingering_potion.png",
            "assets/minecraft/textures/items/lingering_potion.png.mcmeta",
            "assets/minecraft/textures/items/potion_bottle_lingering.png",
            "assets/minecraft/textures/items/potion_bottle_lingering.png.mcmeta"
        ]
    );

    defer_with_meta_pilot!(
        redwood_planks,
        "reverse_generate_redwood_cherry_bamboo_planks",
        [
            "assets/minecraft/textures/block/mangrove_planks.png",
            "assets/minecraft/textures/block/cherry_planks.png",
            "assets/minecraft/textures/block/bamboo_planks.png",
            "assets/minecraft/textures/block/mangrove_log.png",
            "assets/minecraft/textures/block/mangrove_log_top.png",
            "assets/minecraft/textures/block/cherry_log.png",
            "assets/minecraft/textures/block/cherry_log_top.png",
            "assets/minecraft/textures/block/bamboo_block.png",
            "assets/minecraft/textures/block/bamboo_block_top.png",
            "assets/minecraft/textures/block/bamboo_mosaic.png"
        ]
    );

    defer_with_meta_pilot!(
        pale_planks,
        "reverse_generate_pale_planks",
        [
            "assets/minecraft/textures/block/pale_oak_planks.png",
            "assets/minecraft/textures/block/pale_oak_log.png",
            "assets/minecraft/textures/block/pale_oak_log_top.png"
        ]
    );

    defer_with_meta_pilot!(
        poplar_planks,
        "reverse_generate_poplar_planks",
        [
            "assets/minecraft/textures/block/poplar_planks.png",
            "assets/minecraft/textures/block/poplar_log.png",
            "assets/minecraft/textures/block/poplar_log_top.png",
            "assets/minecraft/textures/block/stripped_poplar_log.png",
            "assets/minecraft/textures/block/stripped_poplar_log_top.png",
            "assets/minecraft/textures/block/poplar_door_top.png",
            "assets/minecraft/textures/block/poplar_door_bottom.png",
            "assets/minecraft/textures/block/poplar_trapdoor.png",
            "assets/minecraft/textures/block/poplar_shelf.png",
            "assets/minecraft/textures/block/poplar_sapling.png",
            "assets/minecraft/textures/block/poplar_sign.png",
            "assets/minecraft/textures/block/poplar_hanging_sign.png",
            "assets/minecraft/textures/block/red_poplar_leaves.png",
            "assets/minecraft/textures/block/orange_poplar_leaves.png",
            "assets/minecraft/textures/block/yellow_poplar_leaves.png",
            "assets/minecraft/textures/item/poplar_sign.png",
            "assets/minecraft/textures/item/poplar_hanging_sign.png",
            "assets/minecraft/textures/item/poplar_door.png",
            "assets/minecraft/textures/item/poplar_boat.png",
            "assets/minecraft/textures/item/poplar_chest_boat.png",
            "assets/minecraft/textures/entity/boat/poplar.png",
            "assets/minecraft/textures/entity/chest_boat/poplar.png"
        ]
    );

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "reverse_generate_shulker_box_ui" => Some((shulker_box::decl(), shulker_box::run)),
            "reverse_fix_sign_entities" => Some((sign_entities::decl(), sign_entities::run)),
            "reverse_generate_crossbow" => Some((crossbow::decl(), crossbow::run)),
            "reverse_generate_fish_bucket" => Some((fish_bucket::decl(), fish_bucket::run)),
            "reverse_generate_furnace" => Some((furnace::decl(), furnace::run)),
            "reverse_generate_potion_lingering" => Some((potion_lingering::decl(), potion_lingering::run)),
            "reverse_generate_redwood_cherry_bamboo_planks" => {
                Some((redwood_planks::decl(), redwood_planks::run))
            }
            "reverse_generate_pale_planks" => Some((pale_planks::decl(), pale_planks::run)),
            "reverse_generate_poplar_planks" => Some((poplar_planks::decl(), poplar_planks::run)),
            "reverse_generate_tipped_arrow_images" => {Some((tipped_arrows::decl(), tipped_arrows::run))}
            "reverse_generate_boat" => Some((boat::decl(), boat::run)),
            "reverse_fix_smithing2_villager2_ui" => {
                Some((smithing_villager::decl(), smithing_villager::run))
            }
            _ => None,
        }
    }
}

/// 旧 `reverse_rename_blocks_items`（`converters/reverse/rename_blocks.rs`）。
///
/// 与正向的差异（逐条对应旧实现）：
/// 1. 目录改名的**方向相反**且**不做合并**——只有目标目录不存在时才整体改名；
/// 2. 一张**只作用于 `items/`** 的 128 对反向表（脚本抽取），全部走同一套 `rename_with_mcmeta`；
/// 3. **不调用** `process_blocks::rename_and_process_blocks`（正向才调）。
pub mod rename_blocks_reverse {
    use super::rename_blocks::{BLOCK, BLOCKS, ITEM, ITEMS, TEXTURES};
    use super::rename_blocks_tables::REVERSE_PAIRS;
    use super::*;

    pub fn decl() -> TaskDecl {
        TaskDecl::new("reverse_rename_blocks_items", Tier::Eraser)
            .reads(ScopeSet::prefix(TEXTURES))
            .writes(ScopeSet::prefix(TEXTURES))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();
        if tx.has_prefix(ITEM)? && !tx.has_prefix(ITEMS)? {
            super::rename_blocks::merge_or_rename_dir(tx, ITEM, ITEMS)?;
            outcome.changed += 1;
            outcome.notes.push("item -> items".into());
        }
        if tx.has_prefix(BLOCK)? && !tx.has_prefix(BLOCKS)? {
            super::rename_blocks::merge_or_rename_dir(tx, BLOCK, BLOCKS)?;
            outcome.changed += 1;
            outcome.notes.push("block -> blocks".into());
        }
        for (old, new) in REVERSE_PAIRS {
            if super::rename_blocks::rename_with_mcmeta(tx, ITEMS, old, new)? {
                outcome.changed += 1;
            }
        }
        Ok(outcome)
    }
}

/// 旧贴图路径复制（对应旧 `convert_old_texture_paths`）。
pub mod old_paths {
    use super::*;

    /// 与旧实现逐字对应的映射表。
    pub const MAPPINGS: [(&str, &str); 2] = [
        ("assets/minecraft/terrain.png", "assets/minecraft/block.png"),
        ("assets/minecraft/gui/items.png", "assets/minecraft/item.png"),
    ];

    pub fn decl() -> TaskDecl {
        let mut reads = ScopeSet::none();
        let mut writes = ScopeSet::none();
        for (old, new) in MAPPINGS {
            reads = reads.with_exact(old);
            writes = writes.with_exact(new);
        }
        TaskDecl::new("convert_old_texture_paths", Tier::Eraser)
            .reads(reads)
            .writes(writes)
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let mut changed = 0;
        let mut notes = Vec::new();
        for (old, new) in MAPPINGS {
            if tx.exists(old) {
                tx.alias(old, new)?;
                changed += 1;
                notes.push(format!("{old} -> {new}"));
            }
        }
        Ok(Outcome {
            changed,
            notes,
            ..Outcome::default()
        })
    }
}

/// 动画贴图 mcmeta 升级（对应旧 `convert_animated_textures`）。
pub mod animated {
    use super::*;

    pub const DIRS: [&str; 2] = [
        "assets/minecraft/textures/item",
        "assets/minecraft/textures/items",
    ];

    pub fn decl() -> TaskDecl {
        let scope = ScopeSet::prefix(DIRS[0]).union(&ScopeSet::prefix(DIRS[1]));
        // 阶段与活注册表一致：`invoke_conversion.rs` 把 `convert_animated_textures`
        // 登记为 `TaskType::Exclusive` / `TaskTier::Eraser`。
        TaskDecl::new("convert_animated_textures", Tier::Eraser)
            .reads(scope.clone())
            .writes(scope)
            .exclusive(true)
    }

    /// 由贴图尺寸推导帧数；语义与旧实现逐条一致（正方形 1 帧、条带取整除、否则跳过）。
    pub fn frames_for(w: u32, h: u32) -> Option<u32> {
        if w == h {
            return Some(1);
        }
        if w > h && w % h == 0 {
            return Some(w / h);
        }
        if h > w && h % w == 0 {
            return Some(h / w);
        }
        None
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        // 先读后写：`list` / `exists` 返回 owned 数据，收集完再改
        let mut targets: Vec<(String, String)> = Vec::new();
        for dir in DIRS {
            for res in tx.list(dir)? {
                if res.is_dir {
                    continue;
                }
                let lower = res.path.to_ascii_lowercase();
                if !lower.ends_with(".png.mcmeta") {
                    continue;
                }
                let png = res.path[..res.path.len() - ".mcmeta".len()].to_string();
                if tx.exists(&png) {
                    targets.push((res.path.clone(), png));
                }
            }
        }

        let mut outcome = Outcome::default();
        for (mcmeta, png) in targets {
            match upgrade_one(tx, &mcmeta, &png) {
                Ok(true) => outcome.changed += 1,
                Ok(false) => outcome.skipped += 1,
                Err(e) => {
                    // 与旧实现一致：单条失败只记日志，不中断整个任务
                    outcome.skipped += 1;
                    outcome.notes.push(format!("skip {mcmeta}: {e}"));
                }
            }
        }
        Ok(outcome)
    }

    fn upgrade_one(tx: &mut Tx<'_>, mcmeta: &str, png: &str) -> Result<bool, AromError> {
        let text = tx.text(mcmeta)?;
        let trimmed = text.trim_start_matches('\u{feff}');
        let mut data: serde_json::Value = serde_json::from_str(trimmed)
            .map_err(|e| AromError::View(format!("{mcmeta}: invalid JSON: {e}")))?;

        let Some(anim) = data.get_mut("animation").and_then(|a| a.as_object_mut()) else {
            return Ok(false);
        };
        if anim.contains_key("frametime") {
            return Ok(false);
        }

        let (w, h) = {
            let img = tx.image(png)?;
            (img.width(), img.height())
        };
        let frames = frames_for(w, h).ok_or_else(|| {
            AromError::View(format!("{png}: 无法从尺寸推导帧数 ({w}x{h})"))
        })?;

        anim.insert("frametime".into(), serde_json::json!(frames));
        anim.insert("interpolate".into(), serde_json::json!(true));
        let pretty = serde_json::to_string_pretty(&data).map_err(AromError::internal)?;
        tx.put(mcmeta, pretty.into_bytes())?;
        Ok(true)
    }
}

/// 目录整体改名（对应旧 `rename_mcpatcher_to_optifine`）：用前缀规则，不物化子条目。
pub mod mcpatcher_optifine {
    use super::*;

    pub const FROM: &str = "assets/minecraft/mcpatcher";
    pub const TO: &str = "assets/minecraft/optifine";

    pub fn decl() -> TaskDecl {
        TaskDecl::new("rename_mcpatcher_to_optifine", Tier::Eraser)
            .reads(ScopeSet::prefix(FROM))
            .writes(ScopeSet::prefix(TO))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        // 与旧实现一致：源不存在 → 跳过；目标已存在 → 跳过（不合并）
        if !tx.has_prefix(FROM)? {
            return Ok(Outcome::default());
        }
        if tx.has_prefix(TO)? {
            return Ok(Outcome {
                skipped: 1,
                notes: vec![format!("{TO} already exists, skip rename")],
                ..Outcome::default()
            });
        }
        tx.rename_dir(FROM, TO)?;
        Ok(Outcome {
            changed: 1,
            notes: vec![format!("{FROM} -> {TO}")],
            ..Outcome::default()
        })
    }
}

/// 全部试点：`(名称, 声明, 执行体)`，按阶段顺序排列（与旧管线一致）。
pub type PilotFn = fn(&mut Tx<'_>) -> Result<Outcome, AromError>;

pub fn all() -> Vec<(&'static str, TaskDecl, PilotFn)> {
    vec![
        ("drop_font", drop_font::decl(), drop_font::run as PilotFn),
        (
            "drop_blockstates",
            drop_blockstates::decl(),
            drop_blockstates::run as PilotFn,
        ),
        ("drop_horse", drop_horse::decl(), drop_horse::run as PilotFn),
        (
            "drop_shaders",
            drop_shaders::decl(),
            drop_shaders::run as PilotFn,
        ),
        ("drop_glint", drop_glint::decl(), drop_glint::run as PilotFn),
        (
            "rename_blocks",
            rename_blocks::decl(),
            rename_blocks::run as PilotFn,
        ),
        ("old_paths", old_paths::decl(), old_paths::run as PilotFn),
        (
            "mcpatcher_optifine",
            mcpatcher_optifine::decl(),
            mcpatcher_optifine::run as PilotFn,
        ),
        ("animated", animated::decl(), animated::run as PilotFn),
    ]
}

/// 只读视图里的一次性读（试点的公共小工具）。
pub fn read_dimensions(tx: &Tx<'_>, path: &str) -> Result<(u32, u32), AromError> {
    let img: Arc<RgbaImage> = tx.image(path)?;
    Ok((img.width(), img.height()))
}

/// 反向「延迟删除」的第三批：手写模块（不复用 `reverse_defer` 内部的宏，
/// 因为 `macro_rules!` 只在定义它的模块及其子模块可见）。
pub mod reverse_defer_extra {
    use super::*;

    /// 旧 `reverse_fix2_horse_ui`：延迟删 3 个 gui sprite 槽位贴图。
    pub mod horse_ui_slots {
        use super::*;

        const TARGETS: [&str; 3] = [
            "assets/minecraft/textures/gui/sprites/container/slot/horse_armor.png",
            "assets/minecraft/textures/gui/sprites/container/slot/llama_armor.png",
            "assets/minecraft/textures/gui/sprites/container/slot/saddle.png",
        ];

        pub fn decl() -> TaskDecl {
            let mut scope = ScopeSet::none();
            for path in TARGETS {
                scope = scope.union(&ScopeSet::exact(path));
            }
            TaskDecl::new("reverse_fix2_horse_ui", Tier::Eraser)
                .writes(scope)
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for path in TARGETS {
                outcome
                    .deferred_removals
                    .extend(defer_remove_if_present(tx, path)?.deferred_removals);
            }
            Ok(outcome)
        }
    }

    /// 旧 `reverse_generate_tricky_trials_breeze`：删除 1.21 旋风系生成物（36 个路径），
    /// 每个路径**连同 `.png.mcmeta` 附属**各登记一次（旧实现逐条如此）。
    pub mod breeze {
        use super::*;

        const TARGETS: [&str; 36] = [
            "assets/minecraft/textures/item/breeze_rod.png",
            "assets/minecraft/textures/item/wind_charge.png",
            "assets/minecraft/textures/item/trial_key.png",
            "assets/minecraft/textures/item/ominous_trial_key.png",
            "assets/minecraft/textures/item/breeze_spawn_egg.png",
            "assets/minecraft/textures/entity/projectiles/wind_charge.png",
            "assets/minecraft/textures/mob_effect/wind_charged.png",
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
            "assets/minecraft/textures/block/heavy_core.png",
            "assets/minecraft/textures/entity/breeze/breeze.png",
            "assets/minecraft/textures/entity/breeze/breeze_eyes.png",
            "assets/minecraft/textures/item/flow_armor_trim_smithing_template.png",
            "assets/minecraft/textures/item/ominous_bottle.png",
        ];

        pub fn decl() -> TaskDecl {
            let mut scope = ScopeSet::none();
            for path in TARGETS {
                scope = scope.union(&ScopeSet::exact(path));
            }
            TaskDecl::new("reverse_generate_tricky_trials_breeze", Tier::Eraser)
                .writes(scope)
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for base in TARGETS {
                outcome
                    .deferred_removals
                    .extend(defer_remove_if_present(tx, base)?.deferred_removals);
                let meta = format!("{base}.mcmeta");
                outcome
                    .deferred_removals
                    .extend(defer_remove_if_present(tx, &meta)?.deferred_removals);
            }
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "reverse_fix2_horse_ui" => Some((horse_ui_slots::decl(), horse_ui_slots::run)),
            "reverse_generate_tricky_trials_breeze" => Some((breeze::decl(), breeze::run)),
            _ => None,
        }
    }
}

/// 反向「延迟删除」的第四批：铜/下界合金族共 8 个任务（都是路径表）。
pub mod reverse_defer_metal {
    use super::*;

    const ITEM: &str = "assets/minecraft/textures/item";
    const BLOCK: &str = "assets/minecraft/textures/block";
    const ARMOR: &str = "assets/minecraft/textures/models/armor";

    /// 显式路径表：逐条延迟删除（表里显式写了 `.mcmeta` 的照原样删）。
    fn defer_paths(tx: &Tx<'_>, paths: &[&str]) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();
        for path in paths {
            outcome
                .deferred_removals
                .extend(defer_remove_if_present(tx, path)?.deferred_removals);
        }
        Ok(outcome)
    }

    /// 名字表 + 附属：本体与 `{name}.mcmeta` 各删一次（与旧实现逐条对应）。
    fn defer_names_with_sidecar(
        tx: &Tx<'_>,
        dir: &str,
        names: &[&str],
    ) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();
        for name in names {
            let path = format!("{dir}/{name}");
            outcome
                .deferred_removals
                .extend(defer_remove_if_present(tx, &path)?.deferred_removals);
            let meta = format!("{path}.mcmeta");
            outcome
                .deferred_removals
                .extend(defer_remove_if_present(tx, &meta)?.deferred_removals);
        }
        Ok(outcome)
    }

    fn decl_for(task: &str, scope: ScopeSet) -> TaskDecl {
        TaskDecl::new(task, Tier::Eraser)
            .writes(scope)
            .exclusive(true)
    }

    macro_rules! metal_pilot {
        ($m:ident, $task:literal, $scope:expr, $body:expr) => {
            pub mod $m {
                use super::*;
                pub fn decl() -> TaskDecl {
                    decl_for($task, $scope)
                }
                pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
                    #[allow(clippy::redundant_closure_call)]
                    ($body)(tx)
                }
            }
        };
    }

    metal_pilot!(
        copper_ingot,
        "reverse_generate_copper_ingot",
        ScopeSet::prefix(ITEM),
        |tx: &mut Tx<'_>| defer_paths(
            tx,
            &[
                "assets/minecraft/textures/item/copper_ingot.png",
                "assets/minecraft/textures/item/copper_ingot.png.mcmeta",
            ]
        )
    );

    metal_pilot!(
        copper_block,
        "reverse_generate_copper_block",
        ScopeSet::prefix(BLOCK),
        |tx: &mut Tx<'_>| defer_names_with_sidecar(
            tx,
            BLOCK,
            &[
                "copper_block.png",
                "exposed_copper.png",
                "weathered_copper.png",
                "oxidized_copper.png",
            ]
        )
    );

    metal_pilot!(
        copper_tools,
        "reverse_generate_copper_tools",
        ScopeSet::prefix(ITEM),
        |tx: &mut Tx<'_>| defer_names_with_sidecar(
            tx,
            ITEM,
            &[
                "copper_sword.png",
                "copper_helmet.png",
                "copper_chestplate.png",
                "copper_leggings.png",
                "copper_boots.png",
                "copper_axe.png",
                "copper_pickaxe.png",
                "copper_shovel.png",
                "copper_hoe.png",
                "copper_horse_armor.png",
            ]
        )
    );

    metal_pilot!(
        copper_armor_models,
        "reverse_generate_copper_armor_models",
        ScopeSet::prefix(ARMOR),
        |tx: &mut Tx<'_>| defer_paths(
            tx,
            &[
                "assets/minecraft/textures/models/armor/copper_layer_1.png",
                "assets/minecraft/textures/models/armor/copper_layer_2.png",
            ]
        )
    );

    metal_pilot!(
        netherite_block,
        "reverse_generate_netherite_block",
        ScopeSet::prefix(BLOCK),
        |tx: &mut Tx<'_>| defer_paths(
            tx,
            &[
                "assets/minecraft/textures/block/netherite_block.png",
                "assets/minecraft/textures/block/netherite_block.png.mcmeta",
            ]
        )
    );

    metal_pilot!(
        netherite_ingot,
        "reverse_generate_netherite_ingot",
        ScopeSet::prefix(ITEM),
        |tx: &mut Tx<'_>| defer_paths(
            tx,
            &[
                "assets/minecraft/textures/item/netherite_ingot.png",
                "assets/minecraft/textures/item/netherite_ingot.png.mcmeta",
            ]
        )
    );

    metal_pilot!(
        netherite_tools,
        "reverse_generate_netherite_tools",
        ScopeSet::prefix(ITEM),
        |tx: &mut Tx<'_>| defer_names_with_sidecar(
            tx,
            ITEM,
            &[
                "netherite_sword.png",
                "netherite_helmet.png",
                "netherite_chestplate.png",
                "netherite_leggings.png",
                "netherite_boots.png",
                "netherite_axe.png",
                "netherite_pickaxe.png",
                "netherite_shovel.png",
                "netherite_hoe.png",
                "spectral_arrow.png",
            ]
        )
    );

    metal_pilot!(
        netherite_armor_models,
        "reverse_generate_netherite_armor_models",
        ScopeSet::prefix(ARMOR),
        |tx: &mut Tx<'_>| defer_paths(
            tx,
            &[
                "assets/minecraft/textures/models/armor/netherite_layer_1.png",
                "assets/minecraft/textures/models/armor/netherite_layer_2.png",
            ]
        )
    );

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "reverse_generate_copper_ingot" => Some((copper_ingot::decl(), copper_ingot::run)),
            "reverse_generate_copper_block" => Some((copper_block::decl(), copper_block::run)),
            "reverse_generate_copper_tools" => Some((copper_tools::decl(), copper_tools::run)),
            "reverse_generate_copper_armor_models" => {
                Some((copper_armor_models::decl(), copper_armor_models::run))
            }
            "reverse_generate_netherite_block" => {
                Some((netherite_block::decl(), netherite_block::run))
            }
            "reverse_generate_netherite_ingot" => {
                Some((netherite_ingot::decl(), netherite_ingot::run))
            }
            "reverse_generate_netherite_tools" => {
                Some((netherite_tools::decl(), netherite_tools::run))
            }
            "reverse_generate_netherite_armor_models" => Some((
                netherite_armor_models::decl(),
                netherite_armor_models::run,
            )),
            _ => None,
        }
    }
}

/// 反向「延迟删除」的第五批：`sign` 与 `machinery`（都是「延迟列表 + 立即改名」）。
///
/// 与 `boat` 的差别：这里的改名**没有目标守卫**（旧实现直接 `fs::rename`，会覆盖），
/// 因此原生实现也照抄「覆盖」，不擅自加守卫。
pub mod reverse_defer_ui {
    use super::*;

    const ITEM: &str = "assets/minecraft/textures/item";
    const GUI: &str = "assets/minecraft/textures/gui/container";

    fn defer_paths(tx: &Tx<'_>, paths: &[&str]) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();
        for path in paths {
            outcome
                .deferred_removals
                .extend(defer_remove_if_present(tx, path)?.deferred_removals);
        }
        Ok(outcome)
    }

    /// 立即「恢复」：读源、写目标（**覆盖**，旧实现无守卫）、删源。
    fn restore(tx: &mut Tx<'_>, from: &str, to: &str) -> Result<bool, AromError> {
        if !tx.exists(from) {
            return Ok(false);
        }
        let bytes = tx.read(from)?.unwrap_or_default();
        tx.put(to, bytes)?;
        tx.remove(from)?;
        Ok(true)
    }

    pub mod sign {
        use super::*;

        const VARIANTS: [&str; 11] = [
            "oak_sign.png",
            "birch_sign.png",
            "acacia_sign.png",
            "dark_oak_sign.png",
            "jungle_sign.png",
            "crimson_sign.png",
            "warped_sign.png",
            "mangrove_sign.png",
            "pale_oak_sign.png",
            "bamboo_sign.png",
            "cherry_sign.png",
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("reverse_fix_sign", Tier::Eraser)
                .reads(ScopeSet::prefix(ITEM))
                .writes(ScopeSet::prefix(ITEM))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let paths: Vec<String> = VARIANTS.iter().map(|n| format!("{ITEM}/{n}")).collect();
            let refs: Vec<&str> = paths.iter().map(|p| p.as_str()).collect();
            let mut outcome = defer_paths(tx, &refs)?;
            // 顺序照抄旧实现：先登记删除，再做立即改名（`oak_sign.png` 同时在列表里，
            // 清理点仍会把它删掉——这是旧实现的真实行为，不能"顺手修正"）
            if restore(tx, &format!("{ITEM}/spruce_sign.png"), &format!("{ITEM}/oak_sign.png"))? {
                outcome.changed += 1;
                outcome.notes.push("spruce_sign -> oak_sign".into());
            }
            Ok(outcome)
        }
    }

    pub mod machinery {
        use super::*;

        const GENERATED: [&str; 4] = [
            "grindstone.png",
            "cartography_table.png",
            "stonecutter.png",
            "loom.png",
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("reverse_fix_machinery_ui", Tier::Eraser)
                .reads(ScopeSet::prefix(GUI))
                .writes(ScopeSet::prefix(GUI))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let paths: Vec<String> = GENERATED.iter().map(|n| format!("{GUI}/{n}")).collect();
            let refs: Vec<&str> = paths.iter().map(|p| p.as_str()).collect();
            let mut outcome = defer_paths(tx, &refs)?;
            if restore(
                tx,
                &format!("{GUI}/villager_backup.png"),
                &format!("{GUI}/villager.png"),
            )? {
                outcome.changed += 1;
                outcome.notes.push("villager_backup -> villager".into());
            }
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "reverse_fix_sign" => Some((sign::decl(), sign::run)),
            "reverse_fix_machinery_ui" => Some((machinery::decl(), machinery::run)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arom::{write_zip, Pack, SafeLimits, SerializeOptions};
    use crate::converters::pack_diff::diff_containers;
    use std::io::Write as _;
    use std::path::{Path, PathBuf};

    fn png(size: (u32, u32)) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(size.0, size.1, image::Rgba([200, 30, 30, 255]));
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .expect("encode png");
        buf
    }

    /// 夹具刻意覆盖：被删的目录、被复制的两张旧贴图、待升级/已升级/非动画三种 mcmeta、无关文件。
    fn write_fixture(path: &Path) {
        let file = std::fs::File::create(path).expect("create fixture");
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::FileOptions::default();
        let mut add = |name: &str, body: Vec<u8>| {
            zip.start_file(name, opts).expect("start");
            zip.write_all(&body).expect("write");
        };

        add("pack.mcmeta", br#"{"pack":{"pack_format":34}}"#.to_vec());
        // 试点 1：整棵子树
        add("assets/minecraft/font/default.json", b"{}".to_vec());
        add("assets/minecraft/font/extra/deep.json", b"{}".to_vec());
        // 试点 1b–1e：另外四种「存在即删」（两目录 / 目录 / 目录 / 单文件）
        add("assets/minecraft/blockstates/oak.json", b"{}".to_vec());
        add("assets/minecraft/models/item/x.json", b"{}".to_vec());
        add(
            "assets/minecraft/textures/entity/horse/horse_brown.png",
            png((16, 16)),
        );
        add("assets/minecraft/shaders/core/x.fsh", b"void main(){}".to_vec());
        add(
            "assets/minecraft/textures/misc/enchanted_item_glint.png",
            png((16, 16)),
        );
        // 试点 1f：大改名任务（items/blocks 合并 + 两张重命名表 + 图像派生）
        //   这里刻意让 `item/` 与 `blocks/`+`block/` **同时存在**，以覆盖「合并」分支
        //   （真实包就是这种形态；只覆盖「纯改名」分支曾让一个真 bug 溜过去）。
        add("assets/minecraft/textures/items/gold_sword.png", png((16, 16)));
        add(
            "assets/minecraft/textures/items/gold_sword.png.mcmeta",
            br#"{"x":1}"#.to_vec(),
        );
        add("assets/minecraft/textures/item/gold_sword.png", png((8, 8)));
        add("assets/minecraft/textures/item/keep_me.png", png((8, 8)));
        add("assets/minecraft/textures/blocks/planks_oak.png", png((16, 16)));
        add("assets/minecraft/textures/block/keep_me_too.png", png((8, 8)));
        // 现代名的「目标名已存在」情形（真实包就是这样：`blocks/` 里全是现代名，
        // 重命名表的**目标**名因此天然已存在）——最小复现用。
        add("assets/minecraft/textures/blocks/dark_oak_planks.png", png((16, 16)));
        add("assets/minecraft/textures/blocks/farmland.png", png((16, 16)));
        add(
            "assets/minecraft/textures/blocks/redstone_dust_cross.png",
            png((16, 16)),
        );
        add("assets/minecraft/textures/blocks/coal_ore.png", png((16, 16)));
        add("assets/minecraft/textures/blocks/nether_quartz_ore.png", png((16, 16)));
        // 试点 2：旧贴图路径
        add("assets/minecraft/terrain.png", png((16, 16)));
        add("assets/minecraft/gui/items.png", png((16, 32)));
        // 试点 3：目录改名
        add("assets/minecraft/mcpatcher/cit/a.properties", b"a=1".to_vec());
        add("assets/minecraft/mcpatcher/cit/deep/b.properties", b"b=2".to_vec());
        // 试点 3：动画 mcmeta
        add("assets/minecraft/textures/item/water.png", png((32, 64)));
        add(
            "assets/minecraft/textures/item/water.png.mcmeta",
            br#"{"animation": {}}"#.to_vec(),
        );
        add("assets/minecraft/textures/item/stone.png", png((16, 16)));
        add(
            "assets/minecraft/textures/item/stone.png.mcmeta",
            br#"{"animation": {"frametime": 3}}"#.to_vec(),
        );
        add("assets/minecraft/textures/item/tool.png", png((16, 16)));
        add(
            "assets/minecraft/textures/item/tool.png.mcmeta",
            br#"{"texture": "x"}"#.to_vec(),
        );
        // 无关文件（验证透传）
        add("assets/minecraft/lang/zh_cn.json", br#"{"a":"b"}"#.to_vec());
        zip.finish().expect("finish");
    }

    /// 真实包上**暂不参与**双轨对照的迁移试点（附原因）。
    ///
    /// `rename_blocks`：本轮定位到两个原因——① 驱动把层写回 workdir 时**没有应用改名规则**
    /// （旧任务于是在 `items/`/`blocks/` 旧布局上工作）；② 补上之后试点本身在真实包上仍有分歧
    /// （`dark_oak_planks.png` / `farmland.png`）。因此它仍不派发、也不参与真实包对照，详见细则 §9.14。
    const REAL_PACK_SKIP: [&str; 0] = [];

    fn old_path_output(fixture: &Path, tmp: &Path, skip: &[&str]) -> PathBuf {
        use crate::converters::textures::{
            animated, drop_blockstates_models, drop_enchanted_glint, drop_font, drop_horse,
            drop_shaders, old_paths,
        };
        use crate::hurray::context::HurrayContext;

        let work = tmp.join("work");
        std::fs::create_dir_all(&work).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            fixture.to_str().expect("utf8"),
            work.to_str().expect("utf8"),
        )
        .expect("extract");

        // Eraser 阶段（与活注册表同序：四个删除任务 → 字体 → 大改名 → 改名；延迟清理一次执行）
        let ctx = HurrayContext::new(work.to_str().expect("utf8"));
        drop_blockstates_models::delete_blockstates_models(&ctx).expect("drop blockstates");
        drop_horse::delete_horse_folder(&ctx).expect("drop horse");
        drop_shaders::delete_shaders_folder(&ctx).expect("drop shaders");
        drop_enchanted_glint::delete_enchanted_item_glint(&ctx).expect("drop glint");
        drop_font::delete_font_folder(&ctx).expect("delete_font_folder");
        ctx.execute_cleanup().expect("cleanup");
        if !skip.contains(&"rename_blocks") {
            crate::converters::textures::rename_blocks::rename_blocks_items(&work)
                .expect("rename blocks/items");
        }
        old_paths::convert_old_texture_paths(&work).expect("old paths");
        crate::converters::textures::mcpatcher_to_optifine::rename_mcpatcher_to_optifine(&work)
            .expect("rename mcpatcher");

        // Surgeon 阶段
        animated::convert_animated_textures(&work).expect("animated");

        let out = tmp.join("old.zip");
        crate::converters::zip::repack_resource_pack(
            work.to_str().expect("utf8"),
            out.to_str().expect("utf8"),
        )
        .expect("repack");
        out
    }

    fn new_path_output(fixture: &Path, tmp: &Path, skip: &[&str]) -> (PathBuf, Vec<Outcome>) {
        let mut pack = Pack::open_zip(fixture, &SafeLimits::preserving_current(), None).expect("open");
        let mut outcomes = Vec::new();

        for (name, _decl, run) in all() {
            if skip.contains(&name) {
                // 占位保持下标稳定，便于断言按序对应
                outcomes.push(Outcome::default());
                continue;
            }
            let layer = {
                let mut tx = pack.tx(name);
                outcomes.push(run(&mut tx).expect("pilot run"));
                tx.into_layer()
            };
            pack.commit(layer);
        }
        let out = tmp.join("new.zip");
        let view = pack.view();
        write_zip(&pack, &view, &out, &SerializeOptions::default()).expect("serialize");
        (out, outcomes)
    }

    fn assert_equivalent(old: &Path, new: &Path) -> crate::converters::pack_diff::PackDiffReport {
        let report = diff_containers(old, new).expect("diff");
        assert_eq!(report.blocking, 0, "内容差异：{:?}", report.diffs);
        assert_eq!(
            report.container_entry_set_blocking, 0,
            "条目集合差异：{:?}",
            report.container
        );
        assert_eq!(
            report.container_byte_only, 0,
            "容器字节属性差异：{:?}",
            report.container
        );
        assert!(report.passed(true), "严格模式必须通过：{:?}", report.container);
        report
    }

    #[test]
    fn pilots_match_the_old_implementations() {
        let skip: &[&str] = &[];
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("fixture.zip");
        write_fixture(&fixture);

        let old = old_path_output(&fixture, tmp.path(), skip);
        let (new, outcomes) = new_path_output(&fixture, tmp.path(), skip);

        assert_equivalent(&old, &new);
        // all() 的顺序：drop_font, drop_blockstates, drop_horse, drop_shaders, drop_glint,
        //                rename_blocks, old_paths, mcpatcher_optifine, animated
        assert_eq!(outcomes[0].changed, 1, "font 目录应被删除：{outcomes:?}");
        assert_eq!(outcomes[1].changed, 2, "blockstates 与 models 各删一个：{outcomes:?}");
        assert_eq!(outcomes[2].changed, 1, "horse 目录应被删除：{outcomes:?}");
        assert_eq!(outcomes[3].changed, 1, "shaders 目录应被删除：{outcomes:?}");
        assert_eq!(outcomes[4].changed, 1, "glint 单文件应被删除：{outcomes:?}");
        assert_eq!(
            outcomes[5].changed, 5,
            "items 合并 + blocks 合并 + 物品改名 + 方块改名 + nether_gold 派生：{outcomes:?}"
        );
        assert_eq!(outcomes[6].changed, 2, "两张旧贴图应被复制：{outcomes:?}");
        assert_eq!(outcomes[7].changed, 1, "mcpatcher 应被改名：{outcomes:?}");
        assert_eq!(outcomes[8].changed, 1, "只有 water 需要升级：{outcomes:?}");
        assert_eq!(
            outcomes[8].skipped, 3,
            "夹具共 4 个 mcmeta：water 升级，stone/tool/改名后的 golden_sword 各跳过一条：{outcomes:?}"
        );
    }

    #[test]
    fn pilot_effects_are_visible_in_the_new_output() {
        let skip: &[&str] = &[];
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("fixture.zip");
        write_fixture(&fixture);
        let (new, _) = new_path_output(&fixture, tmp.path(), skip);

        let pack = Pack::open_zip(&new, &SafeLimits::preserving_current(), None).expect("reopen");
        let view = pack.view();

        assert!(view.resolve("assets/minecraft/font").is_none(), "font 子树已删");
        assert!(
            view.resolve("assets/minecraft/blockstates").is_none(),
            "blockstates 子树已删"
        );
        assert!(view.resolve("assets/minecraft/models").is_none(), "models 子树已删");
        assert!(
            view.resolve("assets/minecraft/textures/entity/horse").is_none(),
            "horse 子树已删"
        );
        assert!(view.resolve("assets/minecraft/shaders").is_none(), "shaders 子树已删");
        assert!(
            view.resolve("assets/minecraft/textures/misc/enchanted_item_glint.png")
                .is_none(),
            "glint 文件已删"
        );
        // 大改名任务：目录合并 + 重命名 + 图像派生
        assert!(
            view.resolve("assets/minecraft/textures/items").is_none(),
            "items 已并入 item"
        );
        assert!(
            view.resolve("assets/minecraft/textures/item/golden_sword.png").is_some(),
            "重命名后的物品贴图"
        );
        assert!(
            view.resolve("assets/minecraft/textures/item/gold_sword.png").is_none(),
            "旧名消失"
        );
        assert!(
            view.resolve("assets/minecraft/textures/item/golden_sword.png.mcmeta").is_some(),
            "附属 mcmeta 跟着搬"
        );
        assert!(
            view.resolve("assets/minecraft/textures/block/deepslate_coal_ore.png").is_some(),
            "矿石派生图"
        );
        assert!(
            view.resolve("assets/minecraft/textures/block/red_dust_dot.png").is_some(),
            "红石粉十字派生"
        );
        assert!(
            view.resolve("assets/minecraft/textures/block/nether_gold_ore.png").is_some(),
            "白→黄派生"
        );
        assert!(
            view.resolve("assets/minecraft/textures/block/warped_planks.png").is_some(),
            "色相派生"
        );
        // 合并分支：源覆盖目标、目标独有者保留
        assert!(
            view.resolve("assets/minecraft/textures/item/keep_me.png").is_some(),
            "目标独有文件必须保留"
        );
        assert!(
            view.resolve("assets/minecraft/textures/block/keep_me_too.png").is_some(),
            "目标独有文件必须保留（block）"
        );
        assert!(view.resolve("assets/minecraft/block.png").is_some(), "复制产物存在");
        assert!(view.resolve("assets/minecraft/terrain.png").is_some(), "Copy 不动源文件");
        assert!(view.resolve("assets/minecraft/item.png").is_some());
        assert!(
            view.resolve("assets/minecraft/mcpatcher").is_none(),
            "Move：源目录消失"
        );
        assert!(
            view.resolve("assets/minecraft/optifine/cit/deep/b.properties").is_some(),
            "改名后子树跟着走"
        );

        let meta: serde_json::Value = view
            .json("assets/minecraft/textures/item/water.png.mcmeta")
            .expect("json");
        assert_eq!(meta["animation"]["frametime"], serde_json::json!(2));
        assert_eq!(meta["animation"]["interpolate"], serde_json::json!(true));

        // 现代名 + 目标名已存在：不得被删掉（真实包形态的最小复现点）
        assert!(
            view.resolve("assets/minecraft/textures/block/dark_oak_planks.png")
                .is_some(),
            "已存在的目标名必须保留：dark_oak_planks"
        );
        assert!(
            view.resolve("assets/minecraft/textures/block/farmland.png").is_some(),
            "已存在的目标名必须保留：farmland"
        );
    }

    #[test]
    /// ui 模块第一个迁移任务的专项双轨对照：旧 `process_chest_folder` vs A-ROM 原生实现。
    ///
    /// 不动 `all()` 的下标（那是别的用例的依赖），单独用胸口贴图夹具做闸门对照。
    #[test]
    fn chest_pilot_matches_the_old_implementation() {
        use crate::converters::pack_diff::diff_containers;

        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("chest.zip");
        {
            let file = std::fs::File::create(&fixture).expect("create");
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::FileOptions::default();
            let mut add = |name: &str, body: Vec<u8>| {
                zip.start_file(name, opts).expect("start");
                zip.write_all(&body).expect("write");
            };
            add("pack.mcmeta", br#"{"pack":{"pack_format":34}}"#.to_vec());
            add(
                "assets/minecraft/textures/entity/chest/normal.png",
                png((64, 64)),
            );
            add(
                "assets/minecraft/textures/entity/chest/normal_double.png",
                png((128, 64)),
            );
            add(
                "assets/minecraft/textures/entity/chest/ender.png",
                png((32, 32)),
            );
            zip.finish().expect("finish");
        }

        let work = tmp.path().join("chest_legacy");
        std::fs::create_dir_all(&work).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            fixture.to_str().expect("utf8"),
            work.to_str().expect("utf8"),
        )
        .expect("extract");
        crate::converters::ui::process_chest_folder::process_chest_folder(&work).expect("legacy");
        let legacy_out = tmp.path().join("chest_legacy.zip");
        crate::converters::zip::repack_resource_pack(
            work.to_str().expect("utf8"),
            legacy_out.to_str().expect("utf8"),
        )
        .expect("repack");

        let mut pack =
            Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None).expect("open");
        let outcome = {
            let mut tx = pack.tx("process_chest_folder");
            let outcome = chest::run(&mut tx).expect("pilot");
            let layer = tx.into_layer();
            pack.commit(layer);
            outcome
        };
        assert_eq!(outcome.changed, 2, "一张单胸 + 一张双胸：{outcome:?}");
        assert_eq!(outcome.skipped, 1, "32×32 不支持应跳过：{outcome:?}");

        let native_out = tmp.path().join("chest_native.zip");
        {
            let view = pack.view();
            write_zip(&pack, &view, &native_out, &SerializeOptions::default()).expect("write");
        }

        let report = diff_containers(&legacy_out, &native_out).expect("diff");
        assert_eq!(report.blocking, 0, "内容差异：{:?}", report.diffs);
        assert_eq!(
            report.container_entry_set_blocking, 0,
            "条目集合差异：{:?}",
            report.container
        );
        assert_eq!(
            report.container_byte_only, 0,
            "压缩方法/字节差异：{:?}",
            report.container
        );
    }

    /// 反向转换第一个任务的专项双轨对照：旧 `reverse_process_chest_folder` vs A-ROM 原生实现。
    #[test]
    fn reverse_chest_pilot_matches_the_old_implementation() {
        use crate::converters::pack_diff::diff_containers;

        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("chest_rev.zip");
        {
            let file = std::fs::File::create(&fixture).expect("create");
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::FileOptions::default();
            let mut add = |name: &str, body: Vec<u8>| {
                zip.start_file(name, opts).expect("start");
                zip.write_all(&body).expect("write");
            };
            add("pack.mcmeta", br#"{"pack":{"pack_format":34}}"#.to_vec());
            add(
                "assets/minecraft/textures/entity/chest/normal.png",
                png((64, 64)),
            );
            add(
                "assets/minecraft/textures/entity/chest/ender.png",
                png((32, 32)),
            );
            zip.finish().expect("finish");
        }

        let work = tmp.path().join("chest_rev_legacy");
        std::fs::create_dir_all(&work).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            fixture.to_str().expect("utf8"),
            work.to_str().expect("utf8"),
        )
        .expect("extract");
        crate::converters::reverse::chest_folder::reverse_process_chest_folder(&work)
            .expect("legacy reverse");
        let legacy_out = tmp.path().join("chest_rev_legacy.zip");
        crate::converters::zip::repack_resource_pack(
            work.to_str().expect("utf8"),
            legacy_out.to_str().expect("utf8"),
        )
        .expect("repack");

        let mut pack =
            Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None).expect("open");
        let outcome = {
            let mut tx = pack.tx("reverse_process_chest_folder");
            let outcome = chest_reverse::run(&mut tx).expect("pilot");
            let layer = tx.into_layer();
            pack.commit(layer);
            outcome
        };
        assert_eq!(outcome.changed, 1, "只有 64×64 的 normal 应被处理：{outcome:?}");
        assert_eq!(outcome.skipped, 1, "32×32 不支持应跳过：{outcome:?}");

        let native_out = tmp.path().join("chest_rev_native.zip");
        {
            let view = pack.view();
            write_zip(&pack, &view, &native_out, &SerializeOptions::default()).expect("write");
        }

        let report = diff_containers(&legacy_out, &native_out).expect("diff");
        assert_eq!(report.blocking, 0, "内容差异：{:?}", report.diffs);
        assert_eq!(
            report.container_entry_set_blocking, 0,
            "条目集合差异：{:?}",
            report.container
        );
        assert_eq!(
            report.container_byte_only, 0,
            "压缩方法/字节差异：{:?}",
            report.container
        );
    }

    fn frames_derivation_matches_the_old_rule() {
        assert_eq!(animated::frames_for(16, 16), Some(1), "正方形 1 帧");
        assert_eq!(animated::frames_for(32, 64), Some(2), "纵向条带");
        assert_eq!(animated::frames_for(64, 32), Some(2), "横向条带");
        assert_eq!(animated::frames_for(30, 20), None, "不能整除 → 跳过");
    }

    #[test]
    fn declarations_match_the_actual_effects() {
        // 声明的写范围必须真的覆盖各试点写下的路径（否则调试断言会当场报错）
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("fixture.zip");
        write_fixture(&fixture);

        let pack = Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None).expect("open");
        for (name, decl, run) in all() {
            let mut tx = pack.tx(name);
            run(&mut tx).expect("run");
            for path in tx.layer().writes().keys() {
                decl.check_write(path).unwrap_or_else(|e| panic!("{e}"));
            }
        }
    }

    /// 真实包上的等价性（默认忽略）：
    /// `AROM_REAL_PACK=<包路径> cargo test --lib pilots -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn pilots_match_the_old_implementations_on_a_real_pack() {
        let skip = &REAL_PACK_SKIP;
        let Ok(src) = std::env::var("AROM_REAL_PACK") else {
            println!("AROM_REAL_PACK 未设置，跳过");
            return;
        };
        let fixture = std::path::PathBuf::from(&src);
        assert!(fixture.is_file(), "不是文件：{}", fixture.display());
        let tmp = tempfile::tempdir().expect("tempdir");

        let old = old_path_output(&fixture, tmp.path(), skip);
        let (new, outcomes) = new_path_output(&fixture, tmp.path(), skip);
        let report = assert_equivalent(&old, &new);
        println!("source = {}", fixture.display());
        println!("outcomes = {outcomes:?}");
        println!("report = {}", report.summary());
    }

    /// **夹具正题**（默认忽略）：四个任务的源在真实包里走的是「跳过」分支（§9.52/§9.56），
    /// 所以真实包证明不了它们**算得对**。这里自造 1.13+ 路径的源（部分贴图直接取自真实包），
    /// 对**同一份输入**分别跑旧转换器函数与原生实现，逐像素比对。
    ///
    /// 需要真实 `UImage`：解析不到就直接失败（**不许静默跳过**，否则这个用例会变成空跑）。
    #[test]
    #[ignore]
    fn uimage_tasks_match_the_old_implementations_on_a_fixture() {
        let Ok(src) = std::env::var("AROM_REAL_PACK") else {
            println!("AROM_REAL_PACK 未设置，跳过");
            return;
        };
        let real = std::path::PathBuf::from(&src);
        assert!(real.is_file(), "不是文件：{}", real.display());

        // ① UImage 必须可用，否则这四个任务在两边都会跳过，用例失去意义
        let uimage = crate::converters::get_uimage_path().expect("UImage 必须可解析");
        for probe in [
            "crossbow/crossbow_16.png",
            "crossbow/crossbow_firework_16.png",
            "tipped_arrow_head/tipped_arrow_head_16.png",
            "powder_snow_bucket/powder_snow_bucket_16.png",
            "water_bucket/cod_bucket_16.png",
        ] {
            assert!(
                uimage.join(probe).is_file(),
                "UImage 缺少 {probe}（{}）",
                uimage.display()
            );
        }

        // ② 夹具：源取自真实包（bow / bow_pulling_* / arrow / water_bucket / milk_bucket），
        //    路径换成新格式；目标是 16x16，与上面探测的覆盖图同名尺寸对齐。
        let item = "assets/minecraft/textures/item";
        let items_legacy = "assets/minecraft/textures/items";
        let sources: [(&str, &str); 9] = [
            (
                "assets/minecraft/textures/items/bow_standby.png",
                &format!("{item}/bow.png"),
            ),
            (
                "assets/minecraft/textures/items/bow_pulling_0.png",
                &format!("{item}/bow_pulling_0.png"),
            ),
            (
                "assets/minecraft/textures/items/bow_pulling_1.png",
                &format!("{item}/bow_pulling_1.png"),
            ),
            (
                "assets/minecraft/textures/items/bow_pulling_2.png",
                &format!("{item}/bow_pulling_2.png"),
            ),
            ("assets/minecraft/textures/items/arrow.png", &format!("{items_legacy}/arrow.png")),
            (
                "assets/minecraft/textures/items/bucket_water.png",
                &format!("{item}/water_bucket.png"),
            ),
            (
                "assets/minecraft/textures/items/bucket_milk.png",
                &format!("{item}/milk_bucket.png"),
            ),
            // 让 `snow_bucket` 的覆盖图分支也走到（不与 milk 同图，便于区分）
            (
                "assets/minecraft/textures/items/bucket_lava.png",
                &format!("{item}/milk_bucket_overlay_probe.png"),
            ),
            // `shulker_box_ui` 的输入：真实包里有 1.9 路径的 `gui/container/generic_54.png`
            (
                "assets/minecraft/textures/gui/container/generic_54.png",
                "assets/minecraft/textures/gui/container/generic_54.png",
            ),
        ];

        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("uimage_fixture.zip");
        {
            use std::io::Write as _;
            let file = std::fs::File::create(&fixture).expect("create");
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::FileOptions::default();
            let mut add = |name: &str, body: Vec<u8>| {
                zip.start_file(name, opts).expect("start");
                zip.write_all(&body).expect("write");
            };
            add("pack.mcmeta", br#"{"pack":{"pack_format":34}}"#.to_vec());
            for (src_name, dst_name) in sources {
                let bytes = read_zip_entry(&real, src_name)
                    .unwrap_or_else(|| panic!("真实包里没有 {src_name}"));
                add(dst_name, bytes);
            }
            zip.finish().expect("finish");
        }

        // ③ 旧侧：解压后直接调旧转换器函数
        let legacy_dir = tmp.path().join("legacy");
        std::fs::create_dir_all(&legacy_dir).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            fixture.to_str().expect("utf8"),
            legacy_dir.to_str().expect("utf8"),
        )
        .expect("extract");

        // ④ 原生侧：同一个 zip 建 Pack，跑完各任务后物化到目录
        let native_dir = tmp.path().join("native");
        std::fs::create_dir_all(&native_dir).expect("mkdir");
        {
            let mut pack = Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None)
                .expect("open fixture");
            for name in [
                "generate_crossbow",
                "generate_tipped_arrow_images",
                "generate_snow_bucket",
                "generate_fish_bucket",
                "generate_shulker_box_ui",
            ] {
                let (_, _, run) = crate::native_run::native_for_probe(name)
                    .unwrap_or_else(|| panic!("{name} 未在派发表里"));
                let mut tx = pack.tx(name);
                run(&mut tx).expect("native run");
                pack.commit(tx.into_layer());
            }
            let view = pack.view();
            crate::arom::pathview::materialize(&view, &native_dir).expect("materialize");
        }

        // ⑤ 跑旧函数并逐个输出对比
        let cases: [(&str, fn(&std::path::Path) -> Result<(), String>, Vec<String>); 5] = [
            (
                "generate_crossbow",
                crate::converters::textures::crossbow::generate_crossbow,
                [
                    "crossbow_standby",
                    "crossbow_pulling_0",
                    "crossbow_pulling_1",
                    "crossbow_pulling_2",
                    "crossbow_arrow",
                    "crossbow_firework",
                ]
                .iter()
                .map(|n| format!("{item}/{n}.png"))
                .collect(),
            ),
            (
                "generate_tipped_arrow_images",
                crate::converters::textures::tipped_arrows::generate_tipped_arrow_images,
                ["tipped_arrow_base", "tipped_arrow_head"]
                    .iter()
                    .map(|n| format!("{items_legacy}/{n}.png"))
                    .collect(),
            ),
            (
                "generate_snow_bucket",
                crate::converters::textures::snow_bucket::generate_snow_bucket,
                vec![format!("{item}/powder_snow_bucket.png")],
            ),
            (
                "generate_fish_bucket",
                crate::converters::textures::fish_bucket::generate_fish_bucket,
                ["axolotl", "cod", "pufferfish", "salmon", "tropical_fish", "tadpole"]
                    .iter()
                    .map(|n| format!("{item}/{n}_bucket.png"))
                    .collect(),
            ),
            (
                "generate_shulker_box_ui",
                crate::converters::ui::shulker_box::generate_shulker_box_ui,
                vec![format!(
                    "assets/minecraft/textures/gui/container/shulker_box.png"
                )],
            ),
        ];

        let mut problems: Vec<String> = Vec::new();
        for (name, legacy_fn, outputs) in cases {
            legacy_fn(&legacy_dir).unwrap_or_else(|e| panic!("{name} 旧实现失败：{e}"));
            let mut compared = 0usize;
            for rel in &outputs {
                let a = std::fs::read(legacy_dir.join(rel));
                let b = std::fs::read(native_dir.join(rel));
                match (a, b) {
                    (Ok(a), Ok(b)) => {
                        let ia = image::load_from_memory(&a)
                            .expect("decode legacy")
                            .to_rgba8();
                        let ib = image::load_from_memory(&b)
                            .expect("decode native")
                            .to_rgba8();
                        if ia.dimensions() != ib.dimensions() {
                            problems.push(format!(
                                "{name}: {rel} 尺寸不同 {:?} vs {:?}",
                                ia.dimensions(),
                                ib.dimensions()
                            ));
                            continue;
                        }
                        let mut diff = 0usize;
                        let mut worst = 0i32;
                        for (pa, pb) in ia.pixels().zip(ib.pixels()) {
                            if pa.0 != pb.0 {
                                diff += 1;
                                for c in 0..4 {
                                    worst = worst.max((pa.0[c] as i32 - pb.0[c] as i32).abs());
                                }
                            }
                        }
                        if diff > 0 {
                            problems.push(format!(
                                "{name}: {rel} 像素不同 {diff}/{} 最大通道差 {worst}",
                                ia.pixels().len()
                            ));
                        }
                        compared += 1;
                    }
                    (Err(_), Err(_)) => {
                        problems.push(format!("{name}: {rel} 两边都没生成"));
                    }
                    (a, b) => {
                        problems.push(format!(
                            "{name}: {rel} 一侧缺失（旧={} 原生={}）",
                            a.is_ok(),
                            b.is_ok()
                        ));
                    }
                }
            }
            println!("{name}: 比对 {compared}/{} 个产物", outputs.len());
        }

        assert!(problems.is_empty(), "夹具正题差异：{problems:#?}");
    }

    /// **`fix_tabs` 的正题**（默认忽略）。
    ///
    /// 为什么需要单独测：`fix_tabs` 在**本仓的验收路径上根本不出现**——`scheduler` 有一条规则
    /// `from==9 && to==12 && target_version>15` 时**跳过**它，而 1→97 的路径不含 9→12 段。
    /// 也就是说真实包闸门**覆盖不到**它，只能靠这里证明「算法与旧实现一致」。
    #[test]
    #[ignore]
    fn fix_tabs_matches_the_old_implementation_on_a_fixture() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("tabs_fixture.zip");
        let tabs_rel = "assets/minecraft/textures/gui/container/creative_inventory/tabs.png";

        // 造一张 256×256 的 tabs.png：每个 8×8 区块用坐标派生出的确定性颜色，便于比对
        let make_tabs = |size: u32| -> Vec<u8> {
            let mut img = RgbaImage::new(size, size);
            for y in 0..size {
                for x in 0..size {
                    img.put_pixel(
                        x,
                        y,
                        image::Rgba([
                            (x % 251) as u8,
                            (y % 251) as u8,
                            ((x + y) % 251) as u8,
                            if (x + y) % 7 == 0 { 0 } else { 255 },
                        ]),
                    );
                }
            }
            let mut buf = Vec::new();
            image::DynamicImage::ImageRgba8(img)
                .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                .expect("encode");
            buf
        };

        for size in [256u32, 512u32] {
            let fixture = tmp.path().join(format!("tabs_{size}.zip"));
            {
                use std::io::Write as _;
                let file = std::fs::File::create(&fixture).expect("create");
                let mut zip = zip::ZipWriter::new(file);
                let opts = zip::write::FileOptions::default();
                zip.start_file("pack.mcmeta", opts).expect("start");
                zip.write_all(br#"{"pack":{"pack_format":34}}"#).expect("write");
                zip.start_file(tabs_rel, opts).expect("start");
                zip.write_all(&make_tabs(size)).expect("write");
                zip.finish().expect("finish");
            }

            // 旧侧
            let legacy_dir = tmp.path().join(format!("legacy_{size}"));
            std::fs::create_dir_all(&legacy_dir).expect("mkdir");
            crate::converters::zip::extract_resource_pack(
                fixture.to_str().expect("utf8"),
                legacy_dir.to_str().expect("utf8"),
            )
            .expect("extract");
            crate::converters::ui::tabs::fix_tabs(&legacy_dir).expect("legacy fix_tabs");

            // 原生侧
            let native_dir = tmp.path().join(format!("native_{size}"));
            std::fs::create_dir_all(&native_dir).expect("mkdir");
            {
                let mut pack = Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None)
                    .expect("open fixture");
                let (_, _, run) = crate::native_run::native_for_probe("fix_tabs")
                    .expect("native fix_tabs");
                let mut tx = pack.tx("fix_tabs");
                run(&mut tx).expect("native run");
                pack.commit(tx.into_layer());
                crate::arom::pathview::materialize(&pack.view(), &native_dir)
                    .expect("materialize");
            }

            let a = image::open(legacy_dir.join(tabs_rel)).expect("legacy image").to_rgba8();
            let b = image::open(native_dir.join(tabs_rel)).expect("native image").to_rgba8();
            assert_eq!(a.dimensions(), b.dimensions(), "size {size}: 尺寸不同");
            let mut diff = 0usize;
            let mut worst = 0i32;
            for (pa, pb) in a.pixels().zip(b.pixels()) {
                if pa.0 != pb.0 {
                    diff += 1;
                    for c in 0..4 {
                        worst = worst.max((pa.0[c] as i32 - pb.0[c] as i32).abs());
                    }
                }
            }
            println!("fix_tabs size={size}: 差异 {diff} 像素（最大通道差 {worst}）");
            assert_eq!(diff, 0, "size {size}: fix_tabs 与旧实现不一致");
        }
    }

    /// **`fix_smithing2_villager2_ui` 的正题**（默认忽略，§9.87）。
    ///
    /// 两个子过程都比对：`smithing.png`（由 `anvil.png` 派生）与 `villager.png`（原位改写，
    /// 并产生 `villager_backup.png`）。夹具按 256 与 512 两档尺寸各造一份，
    /// 像素用坐标派生的确定性颜色（含 alpha 变体，以覆盖 paste/overlay 的 alpha 语义）。
    #[test]
    #[ignore]
    fn smithing2_villager2_matches_the_old_implementation_on_a_fixture() {
        let container = "assets/minecraft/textures/gui/container";
        let anvil_rel = format!("{container}/anvil.png");
        let smithing_rel = format!("{container}/smithing.png");
        let villager_rel = format!("{container}/villager.png");
        let backup_rel = format!("{container}/villager_backup.png");

        let make_png = |size: u32, seed: u32| -> Vec<u8> {
            let mut img = RgbaImage::new(size, size);
            for y in 0..size {
                for x in 0..size {
                    img.put_pixel(
                        x,
                        y,
                        image::Rgba([
                            ((x + seed) % 251) as u8,
                            ((y + seed * 3) % 251) as u8,
                            ((x + y + seed * 7) % 251) as u8,
                            if (x + y + seed) % 7 == 0 { 0 } else { 255 },
                        ]),
                    );
                }
            }
            let mut buf = Vec::new();
            image::DynamicImage::ImageRgba8(img)
                .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                .expect("encode");
            buf
        };

        let tmp = tempfile::tempdir().expect("tempdir");
        for size in [256u32, 512u32] {
            let fixture = tmp.path().join(format!("sv_{size}.zip"));
            {
                use std::io::Write as _;
                let file = std::fs::File::create(&fixture).expect("create");
                let mut zip = zip::ZipWriter::new(file);
                let opts = zip::write::FileOptions::default();
                zip.start_file("pack.mcmeta", opts).expect("start");
                zip.write_all(br#"{"pack":{"pack_format":34}}"#).expect("write");
                zip.start_file(&anvil_rel, opts).expect("start");
                zip.write_all(&make_png(size, 1)).expect("write");
                zip.start_file(&villager_rel, opts).expect("start");
                zip.write_all(&make_png(size, 2)).expect("write");
                zip.finish().expect("finish");
            }

            // 旧侧
            let legacy_dir = tmp.path().join(format!("legacy_{size}"));
            std::fs::create_dir_all(&legacy_dir).expect("mkdir");
            crate::converters::zip::extract_resource_pack(
                fixture.to_str().expect("utf8"),
                legacy_dir.to_str().expect("utf8"),
            )
            .expect("extract");
            crate::converters::ui::smithing_villager::fix_smithing2_villager2_ui(&legacy_dir)
                .expect("legacy fix_smithing2_villager2_ui");

            // 原生侧
            let native_dir = tmp.path().join(format!("native_{size}"));
            std::fs::create_dir_all(&native_dir).expect("mkdir");
            {
                let mut pack = Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None)
                    .expect("open fixture");
                let (_, _, run) = crate::native_run::native_for_probe("fix_smithing2_villager2_ui")
                    .expect("native fix_smithing2_villager2_ui");
                let mut tx = pack.tx("fix_smithing2_villager2_ui");
                run(&mut tx).expect("native run");
                pack.commit(tx.into_layer());
                crate::arom::pathview::materialize(&pack.view(), &native_dir)
                    .expect("materialize");
            }

            for rel in [&smithing_rel, &villager_rel, &backup_rel] {
                let lp = legacy_dir.join(rel);
                let np = native_dir.join(rel);
                if lp.exists() != np.exists() {
                    panic!(
                        "size {size} {rel}: 存在性不同（旧 {} / 原生 {}）",
                        lp.exists(),
                        np.exists()
                    );
                }
                if !lp.exists() {
                    continue;
                }
                let a = image::open(&lp).expect("legacy image").to_rgba8();
                let b = image::open(&np).expect("native image").to_rgba8();
                assert_eq!(a.dimensions(), b.dimensions(), "size {size} {rel}: 尺寸不同");
                let mut diff = 0usize;
                let mut worst = 0i32;
                for (pa, pb) in a.pixels().zip(b.pixels()) {
                    if pa.0 != pb.0 {
                        diff += 1;
                        for c in 0..4 {
                            worst = worst.max((pa.0[c] as i32 - pb.0[c] as i32).abs());
                        }
                    }
                }
                println!("size={size} {rel}: 差异 {diff} 像素（最大通道差 {worst}）");
                assert_eq!(diff, 0, "size {size} {rel}: 与旧实现不一致");
            }
        }
    }

    /// **`gui_surgeon_tx` 阶段 1 的正题**（默认忽略，§9.105）。
    ///
    /// 只比对 **`SPRITE_MAP` 主循环**的产出（Late 那批 sprite）。
    /// 做法：造一份含 15 张 `gui/container/*.png` 的夹具（256×256，坐标派生色），
    /// **旧侧**跑完整的 `GuiSurgeon::execute_transformation`（它会产出主循环 + 7 个 `process_*`），
    /// **原生侧**只跑 `cut_sprite_map`。断言：**主循环的那 74 个 target 逐个逐字节相同**。
    ///
    /// 用"逐个比对原生产出"而不是"比对整个 sprites 树"，是因为原生侧**刻意只做了主循环**；
    /// 比对整棵树会把"尚未移植的 7 个过程"误报成差异。
    #[test]
    #[ignore]
    fn gui_surgeon_sprite_map_matches_the_old_implementation_on_a_fixture() {
        let container = "assets/minecraft/textures/gui/container";
        // 与 legacy `SPRITE_MAP` 的 source_name 去重结果一致（15 个）。
        const SOURCES: [&str; 15] = [
            "anvil", "beacon", "blast_furnace", "brewing_stand", "cartography_table",
            "enchanting_table", "furnace", "grindstone", "horse", "inventory", "loom",
            "smithing", "smoker", "stonecutter", "villager2",
        ];

        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("gui_fixture.zip");
        {
            use std::io::Write as _;
            let f = std::fs::File::create(&fixture).expect("create");
            let mut zip = zip::ZipWriter::new(f);
            let opts = zip::write::FileOptions::default();
            zip.start_file("pack.mcmeta", opts).expect("start");
            zip.write_all(br#"{"pack":{"pack_format":34}}"#).expect("write");
            for name in SOURCES {
                // 256×256：与 base_width=256/512 的缩放路径都对得上（512 档会取 256/512=0.5）
                let mut img = RgbaImage::new(256, 256);
                for y in 0..256u32 {
                    for x in 0..256u32 {
                        img.put_pixel(
                            x,
                            y,
                            image::Rgba([
                                (x % 251) as u8,
                                (y % 251) as u8,
                                ((x * 3 + y * 7) % 251) as u8,
                                if (x + y) % 11 == 0 { 0 } else { 255 },
                            ]),
                        );
                    }
                }
                let mut buf = Vec::new();
                image::DynamicImage::ImageRgba8(img)
                    .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                    .expect("encode");
                zip.start_file(format!("{container}/{name}.png"), opts).expect("start");
                zip.write_all(&buf).expect("write");
            }
            // `save_slices` 的两个直接源图（不在 container/ 下，而在 gui/ 下）
            for name in ["resource_packs", "server_selection"] {
                let mut img = RgbaImage::new(256, 256);
                for y in 0..256u32 {
                    for x in 0..256u32 {
                        img.put_pixel(
                            x,
                            y,
                            image::Rgba([
                                ((x * 5 + y) % 251) as u8,
                                ((y * 3 + 11) % 251) as u8,
                                ((x + y * 2) % 251) as u8,
                                if (x * y) % 13 == 0 { 0 } else { 255 },
                            ]),
                        );
                    }
                }
                let mut buf = Vec::new();
                image::DynamicImage::ImageRgba8(img)
                    .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                    .expect("encode");
                zip.start_file(format!("assets/minecraft/textures/gui/{name}.png"), opts)
                    .expect("start");
                zip.write_all(&buf).expect("write");
            }
            // `process_slider` / `process_title` / `process_widgets` 的直接源图
            for rel in [
                "assets/minecraft/textures/gui/slider.png",
                "assets/minecraft/textures/gui/title/minecraft.png",
                "assets/minecraft/textures/gui/widgets.png",
                "assets/minecraft/textures/gui/container/creative_inventory/tabs.png",
                "assets/minecraft/textures/gui/icons.png",
            ] {
                let mut img = RgbaImage::new(256, 256);
                for y in 0..256u32 {
                    for x in 0..256u32 {
                        img.put_pixel(
                            x,
                            y,
                            image::Rgba([
                                ((x * 7 + y * 2) % 251) as u8,
                                ((y * 5 + 3) % 251) as u8,
                                ((x * 2 + y * 9) % 251) as u8,
                                if (x + y * 3) % 17 == 0 { 0 } else { 255 },
                            ]),
                        );
                    }
                }
                let mut buf = Vec::new();
                image::DynamicImage::ImageRgba8(img)
                    .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                    .expect("encode");
                zip.start_file(rel, opts).expect("start");
                zip.write_all(&buf).expect("write");
            }
            zip.finish().expect("finish");
        }

        // 旧侧：解压 → 完整 GuiSurgeon（它磁盘读写）
        let legacy_dir = tmp.path().join("legacy");
        std::fs::create_dir_all(&legacy_dir).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            fixture.to_str().expect("utf8"),
            legacy_dir.to_str().expect("utf8"),
        )
        .expect("extract");
        {
            let ctx = crate::hurray::context::HurrayContext::new(
                legacy_dir.to_str().expect("utf8"),
            );
            let mut pool = crate::hurray::texture::TexturePool::new();
            let mut res = crate::hurray::resolution::ResolutionTransducer::new();
            let _ = res.detect_resolution(&legacy_dir);
            crate::converters::ui::gui_surgeon::GuiSurgeon::execute_transformation(
                &ctx, &mut pool, &res,
            )
            .expect("legacy GuiSurgeon");
        }

        // 原生侧：Pack → cut_sprite_map → 物化
        let native_dir = tmp.path().join("native");
        std::fs::create_dir_all(&native_dir).expect("mkdir");
        let written = {
            let mut pack = Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None)
                .expect("open fixture");
            let mut tx = pack.tx("gui_surgeon_tx");
            let mut n = crate::pilots::gui_surgeon_tx::cut_sprite_map(&mut tx).expect("native cut");
            // §9.105 阶段 2：`save_slices` 的两个直接调用者。
            n += crate::pilots::gui_surgeon_tx::process_resource_packs(&mut tx).expect("resource_packs");
            n += crate::pilots::gui_surgeon_tx::process_server_selection(&mut tx).expect("server_selection");
            n += crate::pilots::gui_surgeon_tx::process_slider(&mut tx).expect("slider");
            n += crate::pilots::gui_surgeon_tx::process_title(&mut tx).expect("title");
            n += crate::pilots::gui_surgeon_tx::process_widgets(&mut tx).expect("widgets");
            n += crate::pilots::gui_surgeon_tx::process_tabs(&mut tx).expect("tabs");
            n += crate::pilots::gui_surgeon_tx::process_icons(&mut tx).expect("icons");
            pack.commit(tx.into_layer());
            crate::arom::pathview::materialize(&pack.view(), &native_dir).expect("materialize");
            n
        };
        println!("原生已移植部分写出 {written} 个 sprite");

        // 非空转：必须真的写出一批（源图齐全时不至于是 0）
        assert!(written > 0, "原生侧没有写出任何 sprite —— 夹具或实现有问题");

        // **清理清单的两条硬约束**（§9.96/§9.111）：
        // ① 20 项；② **绝不含 `container/inventory.png`**——1.21 客户端仍需它渲染背包背景，
        //    删掉会让生存/创造背包 GUI 消失（旧注释专门写了这段）。
        let cl = crate::pilots::gui_surgeon_tx::cleanup_list();
        assert_eq!(cl.len(), 20, "清理清单应为 20 项");
        assert!(
            !cl.iter().any(|p| p.ends_with("container/inventory.png")),
            "清理清单**不得**包含 container/inventory.png（会让背包 GUI 消失）"
        );

        // 逐条比对：原生**写出的每个路径**都必须与旧侧同路径文件逐像素相同。
        //
        // 比对范围 = `sprites/` 整棵子树 **+ `process_title` 会就地回写的源文件**
        // （`gui/title/minecraft.png`）。后者不在 `sprites/` 下，因此必须单独列出——
        // 否则会出现「写出 96、比对 95」，而那 1 个差异恰恰是最需要验证的（就地写语义）。
        let mut targets: Vec<std::path::PathBuf> = Vec::new();
        let sprites_root = native_dir.join("assets/minecraft/textures/gui/sprites");
        let mut stack = vec![sprites_root.clone()];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    targets.push(p);
                }
            }
        }
        targets.push(native_dir.join("assets/minecraft/textures/gui/title/minecraft.png"));

        let mut checked = 0usize;
        let mut problems: Vec<String> = Vec::new();
        for p in &targets {
            let rel = p.strip_prefix(&native_dir).expect("rel").to_string_lossy().replace('\\', "/");
            let lp = legacy_dir.join(&rel);
            if !lp.exists() {
                problems.push(format!("{rel}: 原生有、旧侧没有"));
                continue;
            }
            let a = std::fs::read(&lp).expect("read legacy");
            let b = std::fs::read(p).expect("read native");
            if a != b {
                // 逐像素给出差异量级（PNG 字节可能因编码不同而不同，故再看像素）
                let ia = image::load_from_memory(&a).map(|i| i.to_rgba8());
                let ib = image::load_from_memory(&b).map(|i| i.to_rgba8());
                match (ia, ib) {
                    (Ok(ia), Ok(ib)) if ia.dimensions() == ib.dimensions() => {
                        let diff = ia.pixels().zip(ib.pixels()).filter(|(x, y)| x.0 != y.0).count();
                        if diff > 0 {
                            problems.push(format!("{rel}: 像素不同 {diff} 个"));
                        }
                    }
                    _ => problems.push(format!("{rel}: 尺寸不同或解码失败")),
                }
            }
            checked += 1;
        }
        println!("逐条比对 {checked} 个路径（含就地回写的源文件）");
        assert!(problems.is_empty(), "已移植部分与旧实现不一致：{problems:#?}");
        // **比对数量 vs 写出数量**：`written` 数的是**写操作次数**，`checked` 数的是**磁盘上的文件**。
        // 旧实现里有**幂等的重复写**（`process_widgets` 对 `language.png` 写了两次），
        // 因此 `checked` 可以**小于** `written`，但**绝不能**小太多——
        // 用 `written - 少量重复` 兜底，既能接受已知的幂等重复，又能抓住"大片段没写出去"。
        assert!(
            checked <= written && checked + 8 >= written,
            "比对数量与写出数量差距过大：checked={checked} written={written}（预期只差少量幂等重复写）"
        );
        assert!(checked > 100, "比对的文件太少（{checked}），疑似大片未写出");

        // **§9.111：顶层 `run` 的资源守恒自检**。
        //
        // `run` = 8 个步骤 + 把清理清单里**此刻仍存在**的文件登记为延迟删除。
        // 夹具里那些文件都在，故 `run` 之后：
        //   写操作数 = written + 清理项数（`Tx` 里删除算写：`Slot::Tombstone`）
        //   文件数   = 上面比对的 `checked`（删除不产生文件）
        // 这条自检能在**不依赖旧实现**的前提下抓住"run 漏调了某一步或漏登记了删除"。
        {
            let mut pack = Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None)
                .expect("open fixture");
            let outcome = {
                let mut tx = pack.tx("gui_surgeon_tx_run");
                let o = crate::pilots::gui_surgeon_tx::run(&mut tx).expect("native run");
                pack.commit(tx.into_layer());
                o
            };
            println!(
                "顶层 run：changed={} deferred_removals={}",
                outcome.changed,
                outcome.deferred_removals.len()
            );
            assert_eq!(
                outcome.changed, written,
                "run 的写操作数应与逐步调用之和一致（漏调了某一步？）"
            );
            assert_eq!(
                outcome.deferred_removals.len(),
                20,
                "夹具里清理清单的 20 个文件都在，应全部登记为延迟删除"
            );
        }
    }

    /// 从 zip 里读一个条目的字节（读不到返回 None）。
    fn read_zip_entry(zip_path: &std::path::Path, name: &str) -> Option<Vec<u8>> {
        use std::io::Read as _;
        let f = std::fs::File::open(zip_path).ok()?;
        let mut archive = zip::ZipArchive::new(f).ok()?;
        let mut file = archive.by_name(name).ok()?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).ok()?;
        Some(buf)
    }

    /// **`fix_clock_compass` 的正题**（默认忽略）。
    ///
    /// 为什么需要单独测：在**生产计划里它是空操作**——旧实现读 `textures/items/{clock,compass}.png`，
    /// 而阶段 3–4 的 `rename_blocks_items` 已经把这两个文件改名到 `item/`，所以源"不存在"→ 整任务跳过。
    /// 真实包闸门因此只证明「我也跳过了」，证明不了**抽帧算法**（这也解释了它为何必须放在后阶段，见 §9.59）。
    ///
    /// 这里自造纵向条带图，对同一份输入分别跑旧函数与原生实现，逐像素 + 逐条目比对。
    #[test]
    #[ignore]
    fn clock_compass_split_matches_the_old_implementation_on_a_fixture() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("split_fixture.zip");

        // clock：8x8 的宽度、高 8*8=64（8 帧）→ retain 64 → 全部保留，无抽取
        // compass：8x8 的宽度、高 8*32=256（32 帧）→ retain 32 → 全部保留
        // 另造一张「帧数多于 retain」的：高度 8*128=1024，retain 64 → 走抽帧分支
        let make_strip = |w: u32, frames: u32| -> Vec<u8> {
            let mut img = RgbaImage::new(w, w * frames);
            for y in 0..img.height() {
                for x in 0..w {
                    let frame = y / w;
                    img.put_pixel(
                        x,
                        y,
                        image::Rgba([(frame % 251) as u8, (x * 7 % 251) as u8, (y % 251) as u8, 255]),
                    );
                }
            }
            let mut buf = Vec::new();
            image::DynamicImage::ImageRgba8(img)
                .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                .expect("encode");
            buf
        };
        {
            use std::io::Write as _;
            let file = std::fs::File::create(&fixture).expect("create");
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::FileOptions::default();
            let mut add = |name: &str, body: Vec<u8>| {
                zip.start_file(name, opts).expect("start");
                zip.write_all(&body).expect("write");
            };
            add("pack.mcmeta", br#"{"pack":{"pack_format":34}}"#.to_vec());
            let items = "assets/minecraft/textures/items";
            add(&format!("{items}/clock.png"), make_strip(8, 8));
            add(&format!("{items}/compass.png"), make_strip(8, 128));
            add(
                &format!("{items}/compass.png.mcmeta"),
                br#"{"animation":{"frametime":1}}"#.to_vec(),
            );
            zip.finish().expect("finish");
        }

        // 旧侧
        let legacy_dir = tmp.path().join("legacy");
        std::fs::create_dir_all(&legacy_dir).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            fixture.to_str().expect("utf8"),
            legacy_dir.to_str().expect("utf8"),
        )
        .expect("extract");
        let ctx = crate::hurray::context::HurrayContext::new(&legacy_dir.to_string_lossy());
        crate::converters::ui::clock_compass::fix_clock_compass(&ctx).expect("legacy split");

        // 原生侧
        let native_dir = tmp.path().join("native");
        std::fs::create_dir_all(&native_dir).expect("mkdir");
        {
            let mut pack = Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None)
                .expect("open fixture");
            let (_, _, run) = crate::native_run::native_for_probe("fix_clock_compass")
                .expect("native fix_clock_compass");
            let mut tx = pack.tx("fix_clock_compass");
            run(&mut tx).expect("native run");
            pack.commit(tx.into_layer());
            crate::arom::pathview::materialize(&pack.view(), &native_dir).expect("materialize");
        }

        // 逐条目比对（含「原图应被删除」这一条）
        let items = "assets/minecraft/textures/items";
        let mut problems = Vec::new();
        let mut checked = 0usize;
        for name in ["clock.png", "clock.png.mcmeta", "compass.png", "compass.png.mcmeta"] {
            let rel = format!("{items}/{name}");
            let a = legacy_dir.join(&rel);
            let b = native_dir.join(&rel);
            if a.exists() != b.exists() {
                problems.push(format!("{rel}: 存在性不同（旧={} 原生={}）", a.exists(), b.exists()));
            }
            checked += 1;
        }
        for (prefix, count) in [("clock", 8usize), ("compass", 64usize)] {
            for j in 0..count {
                let rel = format!("{items}/{prefix}_{j:02}.png");
                let a = std::fs::read(legacy_dir.join(&rel));
                let b = std::fs::read(native_dir.join(&rel));
                match (a, b) {
                    (Ok(a), Ok(b)) => {
                        let ia = image::load_from_memory(&a).expect("decode legacy").to_rgba8();
                        let ib = image::load_from_memory(&b).expect("decode native").to_rgba8();
                        if ia.dimensions() != ib.dimensions() {
                            problems.push(format!("{rel}: 尺寸不同"));
                        } else if ia.pixels().zip(ib.pixels()).any(|(x, y)| x.0 != y.0) {
                            problems.push(format!("{rel}: 像素不同"));
                        }
                        checked += 1;
                    }
                    (Err(_), Err(_)) => {}
                    (a, b) => problems.push(format!(
                        "{rel}: 一侧缺失（旧={} 原生={}）",
                        a.is_ok(),
                        b.is_ok()
                    )),
                }
            }
        }
        println!("fix_clock_compass: 比对 {checked} 个条目");
        assert!(problems.is_empty(), "抽帧夹具差异：{problems:#?}");
    }
}

/// §9.34 的实验（旧实现那一半）：单独串起两个任务，逐步打印目标文件是否存在。
/// 只调用旧的转换器函数，不涉及任何原生实现——先确定**旧侧**的真实语义。
#[cfg(test)]
mod legacy_armor_semantics_tests {
    use std::io::Write as _;

    #[test]
    #[ignore]
    fn legacy_armor_then_netherite_step_by_step() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("armor.zip");
        {
            let file = std::fs::File::create(&fixture).expect("create");
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::FileOptions::default();
            let mut add = |name: &str, body: Vec<u8>| {
                zip.start_file(name, opts).expect("start");
                zip.write_all(&body).expect("write");
            };
            add("pack.mcmeta", br#"{"pack":{"pack_format":34}}"#.to_vec());
            add(
                "assets/minecraft/textures/entity/equipment/humanoid/netherite.png",
                b"humanoid-net".to_vec(),
            );
            add(
                "assets/minecraft/textures/models/armor/netherite_layer_1.png",
                b"preexisting-layer1".to_vec(),
            );
            zip.finish().expect("finish");
        }

        let work = tmp.path().join("legacy_armor_work");
        std::fs::create_dir_all(&work).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            fixture.to_str().expect("utf8"),
            work.to_str().expect("utf8"),
        )
        .expect("extract");

        let layer1 = work.join("assets/minecraft/textures/models/armor/netherite_layer_1.png");
        let humanoid = work.join("assets/minecraft/textures/entity/equipment/humanoid/netherite.png");
        println!("[初始] layer_1 存在={} / humanoid 存在={}", layer1.exists(), humanoid.exists());

        crate::converters::reverse::armor::reverse_fix_armor_models(&work).expect("armor");
        println!("[armor 改名后] layer_1 存在={} / humanoid 存在={}", layer1.exists(), humanoid.exists());

        let ctx = crate::hurray::context::HurrayContext::new(work.to_str().expect("utf8"));
        crate::converters::reverse::netherite::reverse_generate_netherite_armor_models(&ctx)
            .expect("netherite defer");
        println!("[netherite 登记后] layer_1 存在={}（延迟删除尚未执行）", layer1.exists());

        ctx.execute_cleanup().expect("cleanup");
        println!("[清理后] layer_1 存在={}", layer1.exists());
    }
}
/// 反向「像素回退」批次：`brewing_stand_ui` 与 `ui_creative`。
///
/// 两者都是对 GUI 贴图做像素级回退（填色 + 区域搬移），逐行照抄旧实现的坐标与缩放表；
/// `paste_region` 直接复用 `crate::image_utils`（旧实现用的就是它）。
pub mod reverse_pixels {
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
}
/// 反向「合成」批次：`clock_compass`（把逐帧贴图并回一张 + 生成 mcmeta）与
/// `particles`（用瓦片重建图集）。两者的源文件都在**延迟删除**里登记，与旧实现一致。
pub mod reverse_compose {
    use super::*;
    use crate::image_utils::paste_region;

    const ITEMS: &str = "assets/minecraft/textures/items";
    const PARTICLE: &str = "assets/minecraft/textures/particle";
    const ENTITY: &str = "assets/minecraft/textures/entity";

    /// 旧 `merge_images`：按给定顺序纵向拼接；成功拼进去的源文件登记延迟删除。
    fn merge_images(
        tx: &mut Tx<'_>,
        sources: &[String],
        output: &str,
    ) -> Result<bool, AromError> {
        let mut images: Vec<RgbaImage> = Vec::new();
        let mut merged_sources: Vec<String> = Vec::new();
        for path in sources {
            if !tx.exists(path) {
                continue;
            }
            images.push((*tx.image(path)?).clone());
            merged_sources.push(path.clone());
        }
        if images.is_empty() {
            return Ok(false);
        }
        let max_width = images.iter().map(|i| i.width()).max().unwrap_or(0);
        let total_height: u32 = images.iter().map(|i| i.height()).sum();
        let mut merged = RgbaImage::new(max_width, total_height);
        let mut y_offset = 0u32;
        for img in &images {
            paste_region(&mut merged, img, 0, y_offset).map_err(AromError::internal)?;
            y_offset += img.height();
        }
        tx.put_image(output, &merged)?;
        // 源文件**不在这里删**：旧实现是 `defer_remove_file`（清理点统一执行），
        // 由调用方登记 `deferred_removals`。立即删会让更晚的任务看不到它们（§9.23）。
        let _ = &merged_sources;
        Ok(true)
    }

    pub mod clock_compass {
        use super::*;

        const MCMETA: &[u8] = br#"{"animation":{}}"#;

        pub fn decl() -> TaskDecl {
            TaskDecl::new("reverse_fix_clock_compass", Tier::Surgeon)
                .reads(ScopeSet::prefix(ITEMS))
                .writes(ScopeSet::prefix(ITEMS))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.has_prefix(ITEMS)? {
                return Ok(Outcome::default());
            }
            let mut outcome = Outcome::default();
            for (prefix, count, output) in [
                ("compass", 32u32, "compass.png"),
                ("clock", 64u32, "clock.png"),
            ] {
                let sources: Vec<String> = (0..count)
                    .map(|i| format!("{ITEMS}/{prefix}_{i:02}.png"))
                    .collect();
                if merge_images(tx, &sources, &format!("{ITEMS}/{output}"))? {
                    tx.put(&format!("{ITEMS}/{output}.mcmeta"), MCMETA.to_vec())?;
                    for path in &sources {
                        outcome
                            .deferred_removals
                            .extend(defer_remove_if_present(tx, path)?.deferred_removals);
                    }
                    outcome.changed += 1;
                }
            }
            Ok(outcome)
        }
    }

    pub mod particles {
        use super::*;

        /// 旧实现的「文件名 → (行, 列)」表（顺序固定；旧实现用 HashMap，取 split_size 时
        /// 依赖迭代顺序，但所有瓦片尺寸相同，因此结果一致）。
        fn positions() -> Vec<(String, (u32, u32))> {
            let mut out: Vec<(String, (u32, u32))> = Vec::new();
            for c in 0..8u32 {
                out.push((format!("generic_{c}.png"), (0, c)));
            }
            for c in 0..4u32 {
                out.push((format!("splash_{c}.png"), (1, c + 3)));
            }
            out.push(("bubble.png".into(), (2, 0)));
            out.push(("fishing_hook.png".into(), (2, 1)));
            out.push(("flame.png".into(), (3, 0)));
            out.push(("lava.png".into(), (3, 1)));
            for (i, n) in ["note.png", "critical_hit.png", "enchanted_hit.png"]
                .iter()
                .enumerate()
            {
                out.push(((*n).into(), (4, i as u32)));
            }
            for (i, n) in ["heart.png", "angry.png", "glint.png"].iter().enumerate() {
                out.push(((*n).into(), (5, i as u32)));
            }
            for (i, n) in ["drip_hang.png", "drip_fall.png", "drip_land.png"]
                .iter()
                .enumerate()
            {
                out.push(((*n).into(), (7, i as u32)));
            }
            for c in 0..8u32 {
                out.push((format!("effect_{c}.png"), (8, c)));
            }
            for c in 0..8u32 {
                out.push((format!("spell_{c}.png"), (9, c)));
            }
            for c in 0..8u32 {
                out.push((format!("spark_{c}.png"), (10, c)));
            }
            out
        }

        pub fn decl() -> TaskDecl {
            TaskDecl::new("reverse_fix_particles", Tier::Surgeon)
                .reads(ScopeSet::prefix("assets/minecraft/textures"))
                .writes(ScopeSet::prefix("assets/minecraft/textures"))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.has_prefix(PARTICLE)? {
                return Ok(Outcome::default());
            }
            let output = format!("{PARTICLE}/particles.png");
            if tx.exists(&output) {
                // 旧实现：已存在就跳过
                return Ok(Outcome::default());
            }
            let table = positions();

            // split_size 取「第一个能打开的瓦片」的宽度（旧实现同）
            let mut split_size = 0u32;
            for (name, _) in &table {
                for dir in [PARTICLE, ENTITY] {
                    let path = format!("{dir}/{name}");
                    if tx.exists(&path) {
                        split_size = tx.image(&path)?.width();
                        break;
                    }
                }
                if split_size > 0 {
                    break;
                }
            }
            if split_size == 0 {
                return Ok(Outcome {
                    skipped: 1,
                    ..Outcome::default()
                });
            }

            let mut merged = RgbaImage::new(16 * split_size, 16 * split_size);
            let mut outcome = Outcome::default();
            for (name, (row, col)) in &table {
                let mut chosen: Option<String> = None;
                for dir in [PARTICLE, ENTITY] {
                    let path = format!("{dir}/{name}");
                    if tx.exists(&path) {
                        chosen = Some(path);
                        break;
                    }
                }
                let Some(path) = chosen else { continue };
                let img: RgbaImage = (*tx.image(&path)?).clone();
                paste_region(&mut merged, &img, col * split_size, row * split_size)
                    .map_err(AromError::internal)?;
                outcome
                    .deferred_removals
                    .extend(defer_remove_if_present(tx, &path)?.deferred_removals);
            }
            tx.put_image(&output, &merged)?;
            outcome.changed += 1;
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "reverse_fix_clock_compass" => Some((clock_compass::decl(), clock_compass::run)),
            "reverse_fix_particles" => Some((particles::decl(), particles::run)),
            _ => None,
        }
    }
}
/// 反向「像素回退」：`reverse_fix_ui_survival`（六步：透明填充 → 19 个状态图标缩放粘贴
/// → 填充 → 区域搬移 → 两处填充）。坐标、缩放表与顺序逐行照抄旧实现。
pub mod reverse_survival {
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
}
/// 反向 `reverse_fix_armor_models`：把 `entity/equipment/humanoid(_leggings)/*.png` 改名回
/// `models/armor/*_layer_{1,2}.png`（8+8 条，**覆盖**语义，与旧实现逐条对应）。
///
/// 阶段是 **Surgeon**（活注册表：`TaskType::Hybrid` / `TaskTier::Surgeon`）——§9.40 的教训：
/// 声明错阶段会让它被放到 Eraser 段之前，从而被清理点的延迟删除一并带走。
/// 它与仍为旧实现的 `adapt_java_shaders`（同属 Surgeon）在驱动里位置相同，
/// 但两者作用路径不相交（armor 贴图 vs shaders）；安全性由反向整包对照实测。
pub mod reverse_armor {
    use super::*;

    const HUMAN: &str = "assets/minecraft/textures/entity/equipment/humanoid";
    const LEGGINGS: &str = "assets/minecraft/textures/entity/equipment/humanoid_leggings";
    const ARMOR: &str = "assets/minecraft/textures/models/armor";

    const LAYER1: [(&str, &str); 8] = [
        ("chainmail.png", "chainmail_layer_1.png"),
        ("diamond.png", "diamond_layer_1.png"),
        ("iron.png", "iron_layer_1.png"),
        ("gold.png", "gold_layer_1.png"),
        ("leather.png", "leather_layer_1.png"),
        ("leather_overlay.png", "leather_layer_1_overlay.png"),
        ("netherite.png", "netherite_layer_1.png"),
        ("copper.png", "copper_layer_1.png"),
    ];
    const LAYER2: [(&str, &str); 8] = [
        ("chainmail.png", "chainmail_layer_2.png"),
        ("diamond.png", "diamond_layer_2.png"),
        ("iron.png", "iron_layer_2.png"),
        ("gold.png", "gold_layer_2.png"),
        ("leather.png", "leather_layer_2.png"),
        ("leather_overlay.png", "leather_layer_2_overlay.png"),
        ("netherite.png", "netherite_layer_2.png"),
        ("copper.png", "copper_layer_2.png"),
    ];

    /// 旧实现逐条 `fs::rename`：**无守卫、覆盖**。
    fn move_file(tx: &mut Tx<'_>, from: &str, to: &str) -> Result<bool, AromError> {
        if !tx.exists(from) {
            return Ok(false);
        }
        let bytes = tx.read(from)?.unwrap_or_default();
        tx.put(to, bytes)?;
        tx.remove(from)?;
        Ok(true)
    }

    pub fn decl() -> TaskDecl {
        TaskDecl::new("reverse_fix_armor_models", Tier::Surgeon)
            .reads(ScopeSet::prefix("assets/minecraft/textures"))
            .writes(ScopeSet::prefix("assets/minecraft/textures"))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();
        for (src, dest) in LAYER1 {
            if move_file(tx, &format!("{HUMAN}/{src}"), &format!("{ARMOR}/{dest}"))? {
                outcome.changed += 1;
            }
        }
        for (src, dest) in LAYER2 {
            if move_file(tx, &format!("{LEGGINGS}/{src}"), &format!("{ARMOR}/{dest}"))? {
                outcome.changed += 1;
            }
        }
        Ok(outcome)
    }
}
/// 前向 Architect 批次（真生成逻辑，不能表驱动）——逐个移植。
///
/// `generate_furnace`：把 `gui/container/furnace.png` **复制**成 `blast_furnace.png` 与
/// `smoker.png`（旧实现是两次 `fs::copy`：源保留、目标若存在则覆盖）。
pub mod arch_gen {
    use super::*;

    const GUI: &str = "assets/minecraft/textures/gui/container";
    const FURNACE: &str = "assets/minecraft/textures/gui/container/furnace.png";
    const BLAST: &str = "assets/minecraft/textures/gui/container/blast_furnace.png";
    const SMOKER: &str = "assets/minecraft/textures/gui/container/smoker.png";

    pub mod furnace {
        use super::*;

        pub fn decl() -> TaskDecl {
            // 阶段与活注册表一致：Architect（生成类任务）
            TaskDecl::new("generate_furnace", Tier::Architect)
                .writes(ScopeSet::prefix(GUI))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(FURNACE) {
                return Ok(Outcome::default());
            }
            let bytes = tx.read(FURNACE)?.unwrap_or_default();
            tx.put(BLAST, bytes.clone())?;
            tx.put(SMOKER, bytes)?;
            Ok(Outcome {
                changed: 2,
                notes: vec!["furnace.png -> blast_furnace.png / smoker.png".into()],
                ..Outcome::default()
            })
        }
    }

    /// `generate_boat`：由 `items/boat.png` 生成 5 个色相/明度变体（复用
    /// `converters::color::hue::adjust_hue_brightness`，与旧实现同一个函数），
    /// 最后把 `boat.png` **改名**为 `spruce_boat.png`（源消失——这一步很容易漏）。
    pub mod boat {
        use super::*;
        use crate::converters::color::hue::adjust_hue_brightness;

        const ITEMS: &str = "assets/minecraft/textures/items";
        const BOAT: &str = "assets/minecraft/textures/items/boat.png";
        const SPRUCE: &str = "assets/minecraft/textures/items/spruce_boat.png";

        /// (输出名, 色相, 明度, 饱和度) —— 逐条照抄旧实现
        const VARIANTS: [(&str, f32, f32, f32); 5] = [
            ("oak_boat.png", 0.0, 15.0, 0.0),
            ("birch_boat.png", 0.0, 40.0, 0.0),
            ("acacia_boat.png", -23.0, 10.0, 0.0),
            ("dark_oak_boat.png", 0.0, -15.0, 0.0),
            ("jungle_boat.png", -10.0, 4.6, 0.0),
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_boat", Tier::Architect)
                .reads(ScopeSet::prefix(ITEMS))
                .writes(ScopeSet::prefix(ITEMS))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(BOAT) {
                return Ok(Outcome::default());
            }
            let base: RgbaImage = (*tx.image(BOAT)?).clone();
            let mut outcome = Outcome::default();
            for (name, hue, brightness, saturation) in VARIANTS {
                let variant = adjust_hue_brightness(base.clone(), hue, brightness, saturation);
                tx.put_image(&format!("{ITEMS}/{name}"), &variant)?;
                outcome.changed += 1;
            }
            // 旧实现：先删已存在的 spruce，再把 boat.png **改名**过去
            if tx.exists(SPRUCE) {
                tx.remove(SPRUCE)?;
            }
            let bytes = tx.read(BOAT)?.unwrap_or_default();
            tx.put(SPRUCE, bytes)?;
            tx.remove(BOAT)?;
            outcome.notes.push("boat.png -> spruce_boat.png".into());
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "generate_furnace" => Some((furnace::decl(), furnace::run)),
            "generate_boat" => Some((boat::decl(), boat::run)),
            _ => None,
        }
    }
}
/// `generate_potion_lingering`：把 `items/potion.png` / `potion_bottle_drinkable.png`
/// **拷贝**成 lingering 版本，并把「上三分之一」透明化（方形图整幅；纵向条带逐格），
/// 最后连带拷贝 `.png.mcmeta`。模块名带 `_gen` 后缀以区别于反向的同名删除任务。
pub mod potion_lingering_gen {
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
}

/// 前向 Architect 批次（续）：依赖**外部 `UImage` 覆盖图**的两个任务。
///
/// 这两个任务的旧实现都有「资源不可用则跳过」的分支，**移植时逐条照抄**——把跳过
/// 写成照做或报错都会改变产物（§9.44 记的正是这条）：
/// - `generate_smithing_ui`：`UImage/smithing/smithing_{width}.png` **存在才**叠加；解析不到
///   `UImage` 目录时只打日志、继续写出；
/// - `generate_fish_bucket`：水桶**先拷贝**，之后**每个**鱼桶各自独立地判断覆盖图是否存在，
///   存在才叠加（不存在就保留拷贝出来的水桶）。
pub mod arch_gen2 {
    use super::*;
    use image::imageops;

    /// `UImage` 目录：与旧实现同一个解析函数（含「找不到就在用户文档建默认目录」的副作用）。
    fn uimage_dir() -> Option<std::path::PathBuf> {
        match crate::converters::get_uimage_path() {
            Ok(p) => Some(p),
            // 旧实现这里只打日志、**不中止**；调用方各自决定「没有覆盖图时怎么办」。
            Err(e) => {
                crate::log_info!("UImage path not available: {}", e);
                None
            }
        }
    }

    /// 旧 `converters/ui/smithing_ui.rs`：由 `gui/container/anvil.png` 生成 `smithing.png`。
    pub mod smithing {
        use super::*;

        const CONTAINER: &str = "assets/minecraft/textures/gui/container";
        const ANVIL: &str = "assets/minecraft/textures/gui/container/anvil.png";

        /// 旧 `scale_factor`：只认 256/512/1024/2048，其余**整任务跳过**。
        fn scale_factor(size: u32) -> Option<u32> {
            match size {
                256 => Some(1),
                512 => Some(2),
                1024 => Some(4),
                2048 => Some(8),
                _ => None,
            }
        }

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_smithing_ui", Tier::Architect)
                .reads(ScopeSet::prefix(CONTAINER))
                .writes(ScopeSet::prefix(CONTAINER))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(ANVIL) {
                return Ok(Outcome::default());
            }
            let mut img: RgbaImage = (*tx.image(ANVIL)?).clone();
            let (width, height) = img.dimensions();
            // 非方形 → 跳过（旧实现 return Ok(())）
            if width != height {
                return Ok(Outcome::default());
            }
            let factor = match scale_factor(width) {
                Some(f) => f,
                None => return Ok(Outcome::default()),
            };

            // 取一个采样像素色填满覆盖框（旧实现：逐像素 put_pixel）
            let fill_color = *img.get_pixel(5 * factor, 4 * factor);
            for x in (10 * factor)..(169 * factor) {
                for y in (5 * factor)..(37 * factor) {
                    img.put_pixel(x, y, fill_color);
                }
            }

            // 覆盖图：**存在才**叠加；UImage 解析失败时只打日志（不中止）
            match uimage_dir() {
                Some(dir) => {
                    let overlay_path = dir
                        .join("smithing")
                        .join(format!("smithing_{}.png", width));
                    if overlay_path.exists() {
                        let overlay = image::open(&overlay_path)
                            .map_err(|e| {
                                AromError::io(format!(
                                    "failed to open {}: {}",
                                    overlay_path.display(),
                                    e
                                ))
                            })?
                            .to_rgba8();
                        imageops::overlay(&mut img, &overlay, 0, 0);
                    }
                }
                None => crate::log_info!("UImage path not available, skip smithing overlay"),
            }

            // 透明框：静态坐标，**不乘 factor**（旧实现如此）
            let transparent = image::Rgba([0, 0, 0, 0]);
            for x in 0..110 {
                for y in 166..198 {
                    img.put_pixel(x, y, transparent);
                }
            }

            tx.mkdir(CONTAINER)?;
            tx.put_image(&format!("{CONTAINER}/smithing.png"), &img)?;
            Ok(Outcome {
                changed: 1,
                notes: vec!["anvil.png -> smithing.png".into()],
                ..Outcome::default()
            })
        }
    }

    /// 旧 `converters/textures/fish_bucket.rs`：`item/water_bucket.png` → 6 个鱼桶。
    pub mod fish_bucket {
        use super::*;

        const ITEMS: &str = "assets/minecraft/textures/item";
        const WATER: &str = "assets/minecraft/textures/item/water_bucket.png";

        /// 逐条照抄旧实现的顺序
        const FISH: [&str; 6] = [
            "axolotl",
            "cod",
            "pufferfish",
            "salmon",
            "tropical_fish",
            "tadpole",
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_fish_bucket", Tier::Architect)
                .reads(ScopeSet::prefix(ITEMS))
                .writes(ScopeSet::prefix(ITEMS))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(WATER) {
                return Ok(Outcome::default());
            }
            let base: RgbaImage = (*tx.image(WATER)?).clone();
            let (width, height) = base.dimensions();
            if width != height || width == 0 {
                return Ok(Outcome::default());
            }

            // 旧实现：`get_uimage_path().map(|p| p.join("water_bucket")).ok()`
            // ——解析失败时**每个**鱼桶都 continue（水桶拷贝仍然发生）
            let overlay_dir = uimage_dir().map(|p| p.join("water_bucket"));

            let mut outcome = Outcome::default();
            for fish in FISH {
                let output_path = format!("{ITEMS}/{fish}_bucket.png");
                // 旧实现是 fs::copy：逐字节复制已解码的源（同一张 RGBA 重新编码）
                tx.put_image(&output_path, &base)?;
                outcome.changed += 1;

                let dir = match &overlay_dir {
                    Some(d) => d,
                    None => continue,
                };
                let overlay_path = dir.join(format!("{}_bucket_{}.png", fish, width));
                if !overlay_path.exists() {
                    continue;
                }
                let mut bucket: RgbaImage = (*tx.image(&output_path)?).clone();
                let mut overlay = image::open(&overlay_path)
                    .map_err(|e| {
                        AromError::io(format!("failed to open {}: {}", overlay_path.display(), e))
                    })?
                    .to_rgba8();
                if overlay.dimensions() != bucket.dimensions() {
                    overlay = imageops::resize(
                        &overlay,
                        bucket.width(),
                        bucket.height(),
                        imageops::FilterType::Triangle,
                    );
                }
                imageops::overlay(&mut bucket, &overlay, 0, 0);
                tx.put_image(&output_path, &bucket)?;
            }
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    ///
    /// 两个任务都已派发。`generate_smithing_ui` 需要**放在旧批次之前**
    /// （计划里它早于 Surgeon 的 `fix_smithing2_villager2_ui`，后者会重新派生并覆盖
    /// `container/smithing.png`），这条由驱动里的 `EARLY_NATIVES` 名单落实，见 §9.52。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "generate_smithing_ui" => Some((smithing::decl(), smithing::run)),
            "generate_fish_bucket" => Some((fish_bucket::decl(), fish_bucket::run)),
            _ => None,
        }
    }
}

/// `generate_shulker_box_ui`：由 `gui/container/generic_54.png` 派生 `shulker_box.png`。
///
/// 语义逐条照抄（**已完成，但暂不派发**——见 §9.51/§9.52）：
/// 1. `determine_scale_factor`：在 {1,2,4,8} 里取 `candidate*256` 与 `max(w,h)` **最接近**者
///    （`exact` 标志在旧实现里**没被使用**，因此这里也不需要）；
/// 2. 清空 `x ∈ [0, 176s)`、`y ∈ [71s, 127s)`（越界处按 `x<width && y<height` 跳过）；
/// 3. 把 `y ∈ [127s, 222s)` 整体**上移 56s**（`saturating_sub`），并把原区间清空——
///    **所有读取都来自原图**（旧实现读 `img` 写 `new_img`），因此源区与目标区重叠时不会自我覆盖。
///
/// **为什么暂不派发**：它的输入 `generic_54.png` 被**更高阶段**（Surgeon 的 GUI 切片链）的旧任务消费，
/// 而它的输出又要在那之前就位——即它需要「Architect 旧任务之后、Surgeon 旧任务之前」这个**中间位置**。
/// 旧批次不可拆分（§9.42），所以这个位置在 Surgeon 也原生化之前并不存在（§9.51）。
/// 实现与语义已就绪，等 Surgeon 就绪后随该阶段一起验收派发。
pub mod shulker_box_gen {
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
}

/// 前向 Architect 批次（续）：铜族与下界合金族 8 个任务。
///
/// 两个模块（`converters/textures/copper.rs`、`netherite.rs`）的形态一致，逐条照抄的要点：
/// - **源缺失 → 整任务跳过**（`iron_ingot` / `diamond_block` 等基准贴图不在包里时什么都不做）；
/// - **拷贝产物 = 同一张 RGBA 重新编码**，不做 `fs::copy`（`tx.put_image`，与旧实现解码后重存等价）；
/// - **`.mcmeta` 附属**：源有 `{src}.png.mcmeta` 才写 `{dst}.png.mcmeta`（旧实现用 `fs::copy`，
///   失败静默忽略——这里保持「有才写」）；
/// - **回退链顺序**：`copper_tools` 是「iron 优先，否则 diamond→gold→stone→netherite」，
///   `copper_armor_models` 是「iron 优先，否则 diamond→gold→chainmail→leather」——**顺序即产物**。
pub mod arch_gen_metal {
    use super::*;
    use crate::converters::color::utils::{
        adjust_copper_color, apply_netherite_transform, apply_spectral_arrow_transform,
    };

    const ITEM: &str = "assets/minecraft/textures/item";
    const BLOCK: &str = "assets/minecraft/textures/block";
    const ARMOR: &str = "assets/minecraft/textures/models/armor";

    /// 旧实现的 `fs::copy(src, dst)` + 后缀附属：源存在**才**拷贝 `.png.mcmeta`。
    fn write_with_sidecar(
        tx: &mut Tx<'_>,
        src: &str,
        dst: &str,
        img: &RgbaImage,
    ) -> Result<(), AromError> {
        tx.put_image(dst, img)?;
        if let Some(dir) = parent_of(dst) {
            tx.mkdir(dir)?;
        }
        let meta = format!("{src}.mcmeta");
        if tx.exists(&meta) {
            let bytes = tx.read(&meta)?.unwrap_or_default();
            tx.put(&format!("{dst}.mcmeta"), bytes)?;
        }
        Ok(())
    }

    fn parent_of(path: &str) -> Option<&str> {
        path.rfind('/').map(|i| &path[..i])
    }

    /// 取第一张存在的候选源（旧实现的 for-else 回退链），返回 `(源路径, 图像)`。
    fn first_available(tx: &Tx<'_>, candidates: &[String]) -> Result<Option<(String, RgbaImage)>, AromError> {
        for candidate in candidates {
            if tx.exists(candidate) {
                let img: RgbaImage = (*tx.image(candidate)?).clone();
                return Ok(Some((candidate.clone(), img)));
            }
        }
        Ok(None)
    }

    /// 旧 `converters/textures/copper.rs`（4 个任务）。
    pub mod copper {
        use super::*;

        /// 工具族的回退链（逐条照抄旧实现的顺序）
        const TOOL_MATERIALS: [&str; 4] = ["diamond", "gold", "stone", "netherite"];
        const TOOL_ITEMS: [&str; 10] = [
            "sword",
            "helmet",
            "chestplate",
            "leggings",
            "boots",
            "axe",
            "pickaxe",
            "shovel",
            "hoe",
            "horse_armor",
        ];
        const ARMOR_MATERIALS: [&str; 4] = ["diamond", "gold", "chainmail", "leather"];
        const ARMOR_FILES: [&str; 2] = ["layer_1", "layer_2"];

        pub fn ingot_decl() -> TaskDecl {
            TaskDecl::new("generate_copper_ingot", Tier::Architect)
                .reads(ScopeSet::prefix(ITEM))
                .writes(ScopeSet::prefix(ITEM))
                .exclusive(true)
        }

        pub fn ingot(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let src = format!("{ITEM}/iron_ingot.png");
            if !tx.exists(&src) {
                return Ok(Outcome::default());
            }
            let dst = format!("{ITEM}/copper_ingot.png");
            let mut img: RgbaImage = (*tx.image(&src)?).clone();
            adjust_copper_color(&mut img);
            write_with_sidecar(tx, &src, &dst, &img)?;
            Ok(Outcome {
                changed: 1,
                notes: vec![format!("iron_ingot.png -> copper_ingot.png")],
                ..Outcome::default()
            })
        }

        pub fn block_decl() -> TaskDecl {
            TaskDecl::new("generate_copper_block", Tier::Architect)
                .reads(ScopeSet::prefix(BLOCK))
                .writes(ScopeSet::prefix(BLOCK))
                .exclusive(true)
        }

        pub fn block(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let src = format!("{BLOCK}/iron_block.png");
            if !tx.exists(&src) {
                return Ok(Outcome::default());
            }
            let dst = format!("{BLOCK}/copper_block.png");
            let mut copper: RgbaImage = (*tx.image(&src)?).clone();
            adjust_copper_color(&mut copper);
            write_with_sidecar(tx, &src, &dst, &copper)?;

            // 三个阶段变体：每个都是「铜块像素再按固定配方混色」，逐条照抄
            let mix = |img: &RgbaImage, r: (f32, f32), g: (f32, f32), b: (f32, f32)| {
                let mut out = img.clone();
                for pixel in out.pixels_mut() {
                    if pixel[3] == 0 {
                        continue;
                    }
                    pixel[0] = (pixel[0] as f32 * r.0 + r.1).round().clamp(0.0, 255.0) as u8;
                    pixel[1] = (pixel[1] as f32 * g.0 + g.1).round().clamp(0.0, 255.0) as u8;
                    pixel[2] = (pixel[2] as f32 * b.0 + b.1).round().clamp(0.0, 255.0) as u8;
                }
                out
            };
            let exposed = mix(&copper, (0.8, 20.0), (0.7, 54.0), (0.6, 64.0));
            let weathered = mix(&copper, (0.6, 28.0), (0.5, 95.0), (0.4, 108.0));
            let mut oxidized = copper.clone();
            for pixel in oxidized.pixels_mut() {
                if pixel[3] == 0 {
                    continue;
                }
                pixel[0] = 50;
                pixel[1] = 210;
                pixel[2] = 210;
            }
            for (name, img) in [
                ("exposed_copper.png", &exposed),
                ("weathered_copper.png", &weathered),
                ("oxidized_copper.png", &oxidized),
            ] {
                tx.put_image(&format!("{BLOCK}/{name}"), img)?;
            }
            // 旧实现的附属：只有 `iron_block.png.mcmeta` 存在时才写这 4 个
            let meta = format!("{src}.mcmeta");
            if tx.exists(&meta) {
                let bytes = tx.read(&meta)?.unwrap_or_default();
                for name in [
                    "copper_block.png",
                    "exposed_copper.png",
                    "weathered_copper.png",
                    "oxidized_copper.png",
                ] {
                    tx.put(&format!("{BLOCK}/{name}.mcmeta"), bytes.clone())?;
                }
            }
            Ok(Outcome {
                changed: 4,
                notes: vec!["iron_block.png -> copper_block + 3 oxidation stages".into()],
                ..Outcome::default()
            })
        }

        pub fn tools_decl() -> TaskDecl {
            TaskDecl::new("generate_copper_tools", Tier::Architect)
                .reads(ScopeSet::prefix(ITEM))
                .writes(ScopeSet::prefix(ITEM))
                .exclusive(true)
        }

        pub fn tools(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for item in TOOL_ITEMS {
                let mut candidates = vec![format!("{ITEM}/iron_{item}.png")];
                candidates.extend(
                    TOOL_MATERIALS
                        .iter()
                        .map(|material| format!("{ITEM}/{material}_{item}.png")),
                );
                let Some((src, mut img)) = first_available(tx, &candidates)? else {
                    continue;
                };
                adjust_copper_color(&mut img);
                let dst = format!("{ITEM}/copper_{item}.png");
                write_with_sidecar(tx, &src, &dst, &img)?;
                outcome.changed += 1;
            }
            Ok(outcome)
        }

        pub fn armor_decl() -> TaskDecl {
            TaskDecl::new("generate_copper_armor_models", Tier::Architect)
                .reads(ScopeSet::prefix(ARMOR))
                .writes(ScopeSet::prefix(ARMOR))
                .exclusive(true)
        }

        pub fn armor(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for layer in ARMOR_FILES {
                let mut candidates = vec![format!("{ARMOR}/iron_{layer}.png")];
                candidates.extend(
                    ARMOR_MATERIALS
                        .iter()
                        .map(|material| format!("{ARMOR}/{material}_{layer}.png")),
                );
                let Some((_src, mut img)) = first_available(tx, &candidates)? else {
                    continue;
                };
                adjust_copper_color(&mut img);
                // 旧实现的 armor 分支**不拷贝 mcmeta**（与 tools 不同），照抄
                tx.put_image(&format!("{ARMOR}/copper_{layer}.png"), &img)?;
                outcome.changed += 1;
            }
            Ok(outcome)
        }
    }

    /// 旧 `converters/textures/netherite.rs`（4 个任务）。
    pub mod netherite {
        use super::*;

        const TOOL_ITEMS: [&str; 9] = [
            "sword",
            "helmet",
            "chestplate",
            "leggings",
            "boots",
            "axe",
            "pickaxe",
            "shovel",
            "hoe",
        ];
        const ARMOR_FILES: [&str; 2] = ["layer_1", "layer_2"];

        pub fn block_decl() -> TaskDecl {
            TaskDecl::new("generate_netherite_block", Tier::Architect)
                .reads(ScopeSet::prefix(BLOCK))
                .writes(ScopeSet::prefix(BLOCK))
                .exclusive(true)
        }

        pub fn block(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let src = format!("{BLOCK}/diamond_block.png");
            if !tx.exists(&src) {
                return Ok(Outcome::default());
            }
            let dst = format!("{BLOCK}/netherite_block.png");
            let mut img: RgbaImage = (*tx.image(&src)?).clone();
            apply_netherite_transform(&mut img);
            write_with_sidecar(tx, &src, &dst, &img)?;
            Ok(Outcome {
                changed: 1,
                notes: vec!["diamond_block.png -> netherite_block.png".into()],
                ..Outcome::default()
            })
        }

        pub fn ingot_decl() -> TaskDecl {
            TaskDecl::new("generate_netherite_ingot", Tier::Architect)
                .reads(ScopeSet::prefix(ITEM))
                .writes(ScopeSet::prefix(ITEM))
                .exclusive(true)
        }

        pub fn ingot(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let src = format!("{ITEM}/gold_ingot.png");
            if !tx.exists(&src) {
                return Ok(Outcome::default());
            }
            let dst = format!("{ITEM}/netherite_ingot.png");
            let mut img: RgbaImage = (*tx.image(&src)?).clone();
            apply_netherite_transform(&mut img);
            write_with_sidecar(tx, &src, &dst, &img)?;
            Ok(Outcome {
                changed: 1,
                notes: vec!["gold_ingot.png -> netherite_ingot.png".into()],
                ..Outcome::default()
            })
        }

        pub fn tools_decl() -> TaskDecl {
            TaskDecl::new("generate_netherite_tools", Tier::Architect)
                .reads(ScopeSet::prefix(ITEM))
                .writes(ScopeSet::prefix(ITEM))
                .exclusive(true)
        }

        pub fn tools(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for item in TOOL_ITEMS {
                let src = format!("{ITEM}/diamond_{item}.png");
                if !tx.exists(&src) {
                    continue;
                }
                let mut img: RgbaImage = (*tx.image(&src)?).clone();
                apply_netherite_transform(&mut img);
                let dst = format!("{ITEM}/netherite_{item}.png");
                write_with_sidecar(tx, &src, &dst, &img)?;
                outcome.changed += 1;
            }
            // 同任务里附带 `arrow.png` → `spectral_arrow.png`（旧实现如此，不另立任务）
            let arrow = format!("{ITEM}/arrow.png");
            if tx.exists(&arrow) {
                let mut img: RgbaImage = (*tx.image(&arrow)?).clone();
                apply_spectral_arrow_transform(&mut img);
                let dst = format!("{ITEM}/spectral_arrow.png");
                write_with_sidecar(tx, &arrow, &dst, &img)?;
                outcome.changed += 1;
                outcome.notes.push("arrow.png -> spectral_arrow.png".into());
            }
            Ok(outcome)
        }

        pub fn armor_decl() -> TaskDecl {
            TaskDecl::new("generate_netherite_armor_models", Tier::Architect)
                .reads(ScopeSet::prefix(ARMOR))
                .writes(ScopeSet::prefix(ARMOR))
                .exclusive(true)
        }

        pub fn armor(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for layer in ARMOR_FILES {
                let src = format!("{ARMOR}/diamond_{layer}.png");
                if !tx.exists(&src) {
                    continue;
                }
                let mut img: RgbaImage = (*tx.image(&src)?).clone();
                apply_netherite_transform(&mut img);
                // 旧实现这一支同样**不拷贝 mcmeta**
                tx.put_image(&format!("{ARMOR}/netherite_{layer}.png"), &img)?;
                outcome.changed += 1;
            }
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "generate_copper_ingot" => Some((copper::ingot_decl(), copper::ingot)),
            "generate_copper_block" => Some((copper::block_decl(), copper::block)),
            "generate_copper_tools" => Some((copper::tools_decl(), copper::tools)),
            "generate_copper_armor_models" => Some((copper::armor_decl(), copper::armor)),
            "generate_netherite_block" => Some((netherite::block_decl(), netherite::block)),
            "generate_netherite_ingot" => Some((netherite::ingot_decl(), netherite::ingot)),
            "generate_netherite_tools" => Some((netherite::tools_decl(), netherite::tools)),
            "generate_netherite_armor_models" => {
                Some((netherite::armor_decl(), netherite::armor))
            }
            _ => None,
        }
    }
}

/// 前向 Architect 批次（续）：新木种 3 个任务（`converters/textures/planks.rs`）。
///
/// 三个任务共用两种变换，逐条照抄：
/// - **`process_block_image` / `recolor_rel`**：源存在才做，产物是
///   `adjust_hue_brightness(源, h, b, s)`，并且**源有 `.png.mcmeta` 才写附属**；
/// - **`leaves`**（poplar 专用）：读源 → `force_hue_saturation(源, hue, 6.0, sat, 0.22, 0.88)`
///   → 写目标 → **源有 `.png.mcmeta` 才写附属**（注意它**不**先拷贝源）。
///
/// 参数表（色相/明度/饱和度）全部逐字照抄旧实现——它们在注释里都有来源（原版均色差），
/// 改一个数字就会改变产物。
pub mod arch_gen_planks {
    use super::*;
    use crate::converters::color::hue::{adjust_hue_brightness, force_hue_saturation};

    const BLOCK: &str = "assets/minecraft/textures/block";

    /// 旧 `process_block_image`：`blocks_path` 下的 `source` → `target`。
    fn recolor_block(
        tx: &mut Tx<'_>,
        source: &str,
        target: &str,
        hue: f32,
        brightness: f32,
        saturation: f32,
    ) -> Result<bool, AromError> {
        recolor_rel(
            tx,
            &format!("{BLOCK}/{source}"),
            &format!("{BLOCK}/{target}"),
            hue,
            brightness,
            saturation,
        )
    }

    /// 旧 `recolor_rel`：任意路径（相对包根）的 recolor，源存在才做。
    fn recolor_rel(
        tx: &mut Tx<'_>,
        source: &str,
        target: &str,
        hue: f32,
        brightness: f32,
        saturation: f32,
    ) -> Result<bool, AromError> {
        if !tx.exists(source) {
            return Ok(false);
        }
        let img: RgbaImage = (*tx.image(source)?).clone();
        let adjusted = adjust_hue_brightness(img, hue, brightness, saturation);
        tx.put_image(target, &adjusted)?;
        let meta = format!("{source}.mcmeta");
        if tx.exists(&meta) {
            let bytes = tx.read(&meta)?.unwrap_or_default();
            tx.put(&format!("{target}.mcmeta"), bytes)?;
        }
        Ok(true)
    }

    /// 旧 `leaves` 闭包：钉色相 + 固定饱和区间（**不拷贝源**，源缺失即跳过）。
    fn leaves(
        tx: &mut Tx<'_>,
        source: &str,
        target: &str,
        hue: f32,
        sat: f32,
    ) -> Result<bool, AromError> {
        let src = format!("{BLOCK}/{source}");
        if !tx.exists(&src) {
            return Ok(false);
        }
        let img: RgbaImage = (*tx.image(&src)?).clone();
        let out = force_hue_saturation(img, hue, 6.0, sat, 0.22, 0.88);
        let dst = format!("{BLOCK}/{target}");
        tx.put_image(&dst, &out)?;
        let meta = format!("{src}.mcmeta");
        if tx.exists(&meta) {
            let bytes = tx.read(&meta)?.unwrap_or_default();
            tx.put(&format!("{dst}.mcmeta"), bytes)?;
        }
        Ok(true)
    }

    /// 旧 `generate_redwood_cherry_bamboo_planks`：1.19 mangrove / 1.20 cherry+bamboo。
    pub mod redwood_cherry_bamboo {
        use super::*;

        /// (源, 目标, 色相, 明度, 饱和) —— 逐条照抄旧实现的 10 次调用
        const JOBS: [(&str, &str, f32, f32, f32); 10] = [
            ("oak_planks.png", "mangrove_planks.png", -59.0, -15.0, 0.0),
            ("oak_planks.png", "cherry_planks.png", -45.0, 45.0, -18.0),
            ("oak_planks.png", "bamboo_planks.png", 25.0, 20.0, 0.0),
            ("oak_log.png", "mangrove_log.png", -59.0, -15.0, 0.0),
            ("oak_log_top.png", "mangrove_log_top.png", -59.0, -15.0, 0.0),
            ("oak_log.png", "cherry_log.png", -45.0, 45.0, -18.0),
            ("oak_log_top.png", "cherry_log_top.png", -45.0, 45.0, -18.0),
            ("oak_log.png", "bamboo_block.png", 25.0, 20.0, 0.0),
            ("oak_log_top.png", "bamboo_block_top.png", 25.0, 20.0, 0.0),
            ("oak_planks.png", "bamboo_mosaic.png", 25.0, 15.0, 0.0),
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_redwood_cherry_bamboo_planks", Tier::Architect)
                .reads(ScopeSet::prefix(BLOCK))
                .writes(ScopeSet::prefix(BLOCK))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for (source, target, hue, brightness, saturation) in JOBS {
                if recolor_block(tx, source, target, hue, brightness, saturation)? {
                    outcome.changed += 1;
                }
            }
            Ok(outcome)
        }
    }

    /// 旧 `generate_pale_planks`：1.21.4 苍白橡木（3 个文件）。
    pub mod pale {
        use super::*;

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_pale_planks", Tier::Architect)
                .reads(ScopeSet::prefix(BLOCK))
                .writes(ScopeSet::prefix(BLOCK))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for (source, target) in [
                ("oak_planks.png", "pale_oak_planks.png"),
                ("oak_log.png", "pale_oak_log.png"),
                ("oak_log_top.png", "pale_oak_log_top.png"),
            ] {
                if recolor_block(tx, source, target, 0.0, 30.0, -100.0)? {
                    outcome.changed += 1;
                }
            }
            Ok(outcome)
        }
    }

    /// 旧 `generate_poplar_planks`：26.3 杨树全套（原木/木板/家具/树叶）。
    pub mod poplar {
        use super::*;

        /// 木板与家具用的 jungle 参数 / 树皮用的 oak 参数（顺序即语义）
        const JUNGLE: (f32, f32, f32) = (-1.0, -4.0, -35.0);
        const OAK: (f32, f32, f32) = (-7.0, -5.0, -36.0);

        /// (jungle 源, oak 回退源, 目标) —— 相对包根，逐条照抄
        const PREFER_JUNGLE_THEN_OAK: [(&str, &str, &str); 9] = [
            (
                "assets/minecraft/textures/item/jungle_sign.png",
                "assets/minecraft/textures/item/oak_sign.png",
                "assets/minecraft/textures/item/poplar_sign.png",
            ),
            (
                "assets/minecraft/textures/item/jungle_hanging_sign.png",
                "assets/minecraft/textures/item/oak_hanging_sign.png",
                "assets/minecraft/textures/item/poplar_hanging_sign.png",
            ),
            (
                "assets/minecraft/textures/item/jungle_door.png",
                "assets/minecraft/textures/item/oak_door.png",
                "assets/minecraft/textures/item/poplar_door.png",
            ),
            (
                "assets/minecraft/textures/item/jungle_boat.png",
                "assets/minecraft/textures/item/oak_boat.png",
                "assets/minecraft/textures/item/poplar_boat.png",
            ),
            (
                "assets/minecraft/textures/item/jungle_chest_boat.png",
                "assets/minecraft/textures/item/oak_chest_boat.png",
                "assets/minecraft/textures/item/poplar_chest_boat.png",
            ),
            (
                "assets/minecraft/textures/block/jungle_sign.png",
                "assets/minecraft/textures/block/oak_sign.png",
                "assets/minecraft/textures/block/poplar_sign.png",
            ),
            (
                "assets/minecraft/textures/block/jungle_hanging_sign.png",
                "assets/minecraft/textures/block/oak_hanging_sign.png",
                "assets/minecraft/textures/block/poplar_hanging_sign.png",
            ),
            (
                "assets/minecraft/textures/entity/boat/jungle.png",
                "assets/minecraft/textures/entity/boat/oak.png",
                "assets/minecraft/textures/entity/boat/poplar.png",
            ),
            (
                "assets/minecraft/textures/entity/chest_boat/jungle.png",
                "assets/minecraft/textures/entity/chest_boat/oak.png",
                "assets/minecraft/textures/entity/chest_boat/poplar.png",
            ),
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_poplar_planks", Tier::Architect)
                .reads(ScopeSet::prefix("assets/minecraft/textures"))
                .writes(ScopeSet::prefix("assets/minecraft/textures"))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();

            // ── 原木 / 木板 ──
            for (source, target, hue, brightness, saturation) in [
                ("jungle_planks.png", "poplar_planks.png", -1.0, -4.0, -35.0),
                ("jungle_log_top.png", "poplar_log_top.png", -3.0, -5.0, -33.0),
                ("oak_log.png", "poplar_log.png", -7.0, -12.0, -8.0),
                (
                    "stripped_oak_log.png",
                    "stripped_poplar_log.png",
                    -7.0,
                    -5.0,
                    -36.0,
                ),
                (
                    "stripped_oak_log_top.png",
                    "stripped_poplar_log_top.png",
                    -10.0,
                    -3.0,
                    -37.0,
                ),
            ] {
                if recolor_block(tx, source, target, hue, brightness, saturation)? {
                    outcome.changed += 1;
                }
            }

            // ── 家具：优先 jungle，缺失回退 oak（回退用 oak 参数）──
            for (src, dst) in [
                ("jungle_door_top.png", "poplar_door_top.png"),
                ("jungle_door_bottom.png", "poplar_door_bottom.png"),
                ("jungle_trapdoor.png", "poplar_trapdoor.png"),
                ("jungle_shelf.png", "poplar_shelf.png"),
                ("jungle_sapling.png", "poplar_sapling.png"),
            ] {
                if tx.exists(&format!("{BLOCK}/{src}")) {
                    let (h, b, s) = JUNGLE;
                    if recolor_block(tx, src, dst, h, b, s)? {
                        outcome.changed += 1;
                    }
                } else {
                    let oak = src.replace("jungle_", "oak_");
                    let (h, b, s) = OAK;
                    if recolor_block(tx, &oak, dst, h, b, s)? {
                        outcome.changed += 1;
                    }
                }
            }

            // ── 物品图标 / 实体船：优先 jungle，缺失再 oak ──
            for (jungle, oak, target) in PREFER_JUNGLE_THEN_OAK {
                let (h, b, s) = JUNGLE;
                if tx.exists(jungle) {
                    if recolor_rel(tx, jungle, target, h, b, s)? {
                        outcome.changed += 1;
                    }
                } else {
                    let (h, b, s) = OAK;
                    if recolor_rel(tx, oak, target, h, b, s)? {
                        outcome.changed += 1;
                    }
                }
            }

            // ── 树叶：钉色相（原版参考 红 ~8° / 橙 ~28° / 黄 ~45°）──
            for (target, hue, sat) in [
                ("red_poplar_leaves.png", 8.0, 0.72),
                ("orange_poplar_leaves.png", 28.0, 0.80),
                ("yellow_poplar_leaves.png", 45.0, 0.78),
            ] {
                if leaves(tx, "oak_leaves.png", target, hue, sat)? {
                    outcome.changed += 1;
                }
            }

            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "generate_redwood_cherry_bamboo_planks" => Some((
                redwood_cherry_bamboo::decl(),
                redwood_cherry_bamboo::run,
            )),
            "generate_pale_planks" => Some((pale::decl(), pale::run)),
            "generate_poplar_planks" => Some((poplar::decl(), poplar::run)),
            _ => None,
        }
    }
}

/// 前向 Architect 批次（续）：`generate_tricky_trials_breeze`（1.21 旋风系贴图）。
///
/// 逐条照抄 `converters/textures/breeze.rs` 的规则表与**三条容易漏的语义**：
/// 1. **`recolor_skip_existing`**：源缺失 → 跳过；**目标已存在 → 跳过**（不覆盖玩家/原版自定义）；
///    只从源**拷贝**再染色，附属 `{src}.png.mcmeta` 存在才一并拷贝；
/// 2. **候选链是「第一个存在者胜」**：刷怪蛋、风充能图标、重核、flow 模板、不祥之瓶各有一条
///    候选列表，命中即 `break`；
/// 3. **不祥试炼钥匙的源是条件选择**：`trial_key.png` 存在就用它，否则用 `gold_ingot.png`。
pub mod arch_gen_breeze {
    use super::*;
    use crate::converters::color::hue::{adjust_hue_brightness, force_hue_saturation};

    const ITEM: &str = "assets/minecraft/textures/item/";
    const BLOCK: &str = "assets/minecraft/textures/block/";
    const MOB: &str = "assets/minecraft/textures/mob_effect/";
    const ENTITY: &str = "assets/minecraft/textures/entity/";

    /// 旧实现的两种染色（相对色相 / 钉色相）。
    enum Tint {
        Shift { h: f32, b: f32, s: f32 },
        Force {
            hue: f32,
            sat: f32,
            v_min: f32,
            v_max: f32,
        },
    }

    /// 旧 `recolor_skip_existing`：源缺失或目标已存在即跳过。
    fn recolor_skip_existing(
        tx: &mut Tx<'_>,
        src: &str,
        dst: &str,
        tint: &Tint,
    ) -> Result<bool, AromError> {
        if !tx.exists(src) || tx.exists(dst) {
            return Ok(false);
        }
        let img: RgbaImage = (*tx.image(src)?).clone();
        let out = match tint {
            Tint::Shift { h, b, s } => adjust_hue_brightness(img, *h, *b, *s),
            Tint::Force {
                hue,
                sat,
                v_min,
                v_max,
            } => force_hue_saturation(img, *hue, 6.0, *sat, *v_min, *v_max),
        };
        tx.put_image(dst, &out)?;
        let meta = format!("{src}.mcmeta");
        if tx.exists(&meta) {
            let bytes = tx.read(&meta)?.unwrap_or_default();
            tx.put(&format!("{dst}.mcmeta"), bytes)?;
        }
        Ok(true)
    }

    /// 候选链的「第一个存在者」。
    fn first_existing<S: AsRef<str>>(tx: &Tx<'_>, candidates: &[S]) -> Option<String> {
        candidates
            .iter()
            .find(|src| tx.exists(src.as_ref()))
            .map(|src| src.as_ref().to_string())
    }

    /// 铜灯泡的色调分组（旧实现用 `contains` 判定，顺序即优先级：oxidized → weathered → exposed → 默认）
    fn bulb_tint(dst: &str, lit: bool) -> Tint {
        let four = |oxidized, weathered, exposed, plain| -> (f32, f32, f32, f32) {
            if dst.contains("oxidized") {
                oxidized
            } else if dst.contains("weathered") {
                weathered
            } else if dst.contains("exposed") {
                exposed
            } else {
                plain
            }
        };
        let (hue, sat, v_min, v_max) = if lit {
            four(
                (145.0, 0.2, 0.45, 0.95),
                (120.0, 0.25, 0.5, 0.95),
                (35.0, 0.3, 0.55, 0.98),
                (18.0, 0.38, 0.55, 0.98),
            )
        } else {
            four(
                (145.0, 0.22, 0.25, 0.55),
                (120.0, 0.28, 0.3, 0.6),
                (35.0, 0.32, 0.35, 0.65),
                (18.0, 0.42, 0.35, 0.7),
            )
        };
        Tint::Force {
            hue,
            sat,
            v_min,
            v_max,
        }
    }

    pub fn decl() -> TaskDecl {
        TaskDecl::new("generate_tricky_trials_breeze", Tier::Architect)
            .reads(ScopeSet::prefix("assets/minecraft/textures"))
            .writes(ScopeSet::prefix("assets/minecraft/textures"))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();
        let emit = |ok: bool, outcome: &mut Outcome| {
            if ok {
                outcome.changed += 1;
            }
        };

        // ── 第 1 批：好做 ──
        emit(
            recolor_skip_existing(
                tx,
                &format!("{ITEM}blaze_rod.png"),
                &format!("{ITEM}breeze_rod.png"),
                &Tint::Shift {
                    h: 185.0,
                    b: -22.0,
                    s: -18.0,
                },
            )?,
            &mut outcome,
        );
        emit(
            recolor_skip_existing(
                tx,
                &format!("{ITEM}snowball.png"),
                &format!("{ITEM}wind_charge.png"),
                &Tint::Force {
                    hue: 222.0,
                    sat: 0.14,
                    v_min: 0.55,
                    v_max: 0.95,
                },
            )?,
            &mut outcome,
        );
        emit(
            recolor_skip_existing(
                tx,
                &format!("{ENTITY}snowball.png"),
                &format!("{ENTITY}projectiles/wind_charge.png"),
                &Tint::Force {
                    hue: 222.0,
                    sat: 0.14,
                    v_min: 0.5,
                    v_max: 0.95,
                },
            )?,
            &mut outcome,
        );
        emit(
            recolor_skip_existing(
                tx,
                &format!("{ITEM}gold_ingot.png"),
                &format!("{ITEM}trial_key.png"),
                &Tint::Force {
                    hue: 22.0,
                    sat: 0.38,
                    v_min: 0.28,
                    v_max: 0.62,
                },
            )?,
            &mut outcome,
        );

        // 不祥试炼钥匙：源是**条件选择**（trial_key 优先，否则 gold_ingot）
        let trial_key = format!("{ITEM}trial_key.png");
        let ominous_src = if tx.exists(&trial_key) {
            trial_key
        } else {
            format!("{ITEM}gold_ingot.png")
        };
        emit(
            recolor_skip_existing(
                tx,
                &ominous_src,
                &format!("{ITEM}ominous_trial_key.png"),
                &Tint::Force {
                    hue: 160.0,
                    sat: 0.16,
                    v_min: 0.18,
                    v_max: 0.45,
                },
            )?,
            &mut outcome,
        );

        // 旋风刷怪蛋：任意已有刷怪蛋 → 蓝灰（第一个存在者胜）
        let eggs = ["chicken", "spider", "cow", "creeper"].map(|n| format!("{ITEM}{n}_spawn_egg.png"));
        if let Some(src) = first_existing(tx, &eggs) {
            emit(
                recolor_skip_existing(
                    tx,
                    &src,
                    &format!("{ITEM}breeze_spawn_egg.png"),
                    &Tint::Force {
                        hue: 230.0,
                        sat: 0.28,
                        v_min: 0.35,
                        v_max: 0.75,
                    },
                )?,
                &mut outcome,
            );
        }

        // 风充能状态图标
        let effects = ["speed", "jump_boost", "absorption"]
            .map(|n| format!("{MOB}{n}.png"));
        if let Some(src) = first_existing(tx, &effects) {
            emit(
                recolor_skip_existing(
                    tx,
                    &src,
                    &format!("{MOB}wind_charged.png"),
                    &Tint::Force {
                        hue: 220.0,
                        sat: 0.28,
                        v_min: 0.45,
                        v_max: 0.9,
                    },
                )?,
                &mut outcome,
            );
        }

        // 铜灯泡族：熄灭（12 个目标）
        let lamp_off = [
            format!("{BLOCK}redstone_lamp.png"),
            format!("{BLOCK}redstone_lamp_off.png"),
        ];
        let copper_base = [
            format!("{BLOCK}copper_block.png"),
            format!("{BLOCK}cut_copper.png"),
        ];
        let off_src = first_existing(tx, &lamp_off).or_else(|| first_existing(tx, &copper_base));
        if let Some(src) = off_src {
            for dst in [
                "copper_bulb.png",
                "copper_bulb_powered.png",
                "exposed_copper_bulb.png",
                "exposed_copper_bulb_powered.png",
                "weathered_copper_bulb.png",
                "weathered_copper_bulb_powered.png",
                "oxidized_copper_bulb.png",
                "oxidized_copper_bulb_powered.png",
                "waxed_copper_bulb.png",
                "waxed_exposed_copper_bulb.png",
                "waxed_weathered_copper_bulb.png",
                "waxed_oxidized_copper_bulb.png",
            ] {
                let tint = bulb_tint(dst, false);
                emit(
                    recolor_skip_existing(tx, &src, &format!("{BLOCK}{dst}"), &tint)?,
                    &mut outcome,
                );
            }
        }

        // 铜灯泡族：点亮（12 个目标）
        let lamp_on = [
            format!("{BLOCK}redstone_lamp_on.png"),
            format!("{BLOCK}redstone_lamp.png"),
        ];
        if let Some(src) = first_existing(tx, &lamp_on) {
            for dst in [
                "copper_bulb_lit.png",
                "copper_bulb_lit_powered.png",
                "exposed_copper_bulb_lit.png",
                "exposed_copper_bulb_lit_powered.png",
                "weathered_copper_bulb_lit.png",
                "weathered_copper_bulb_lit_powered.png",
                "oxidized_copper_bulb_lit.png",
                "oxidized_copper_bulb_lit_powered.png",
                "waxed_copper_bulb_lit.png",
                "waxed_exposed_copper_bulb_lit.png",
                "waxed_weathered_copper_bulb_lit.png",
                "waxed_oxidized_copper_bulb_lit.png",
            ] {
                let tint = bulb_tint(dst, true);
                emit(
                    recolor_skip_existing(tx, &src, &format!("{BLOCK}{dst}"), &tint)?,
                    &mut outcome,
                );
            }
        }

        // ── 第 2 批：中等 ──
        let heavy = [
            format!("{BLOCK}iron_block.png"),
            format!("{BLOCK}deepslate.png"),
            format!("{BLOCK}polished_deepslate.png"),
        ];
        if let Some(src) = first_existing(tx, &heavy) {
            emit(
                recolor_skip_existing(
                    tx,
                    &src,
                    &format!("{BLOCK}heavy_core.png"),
                    &Tint::Force {
                        hue: 220.0,
                        sat: 0.1,
                        v_min: 0.22,
                        v_max: 0.48,
                    },
                )?,
                &mut outcome,
            );
        }

        // 旋风实体：烈焰人两张 → breeze 两张
        for (src, dst) in [
            (format!("{ENTITY}blaze.png"), format!("{ENTITY}breeze/breeze.png")),
            (
                format!("{ENTITY}blaze_blaze.png"),
                format!("{ENTITY}breeze/breeze_eyes.png"),
            ),
        ] {
            emit(
                recolor_skip_existing(
                    tx,
                    &src,
                    &dst,
                    &Tint::Force {
                        hue: 235.0,
                        sat: 0.26,
                        v_min: 0.35,
                        v_max: 0.85,
                    },
                )?,
                &mut outcome,
            );
        }

        // flow 盔甲纹饰模板
        let trims = [
            "armor_trim_smithing_template",
            "silence_armor_trim_smithing_template",
            "bolt_armor_trim_smithing_template",
            "dune_armor_trim_smithing_template",
        ]
        .map(|n| format!("{ITEM}{n}.png"));
        if let Some(src) = first_existing(tx, &trims) {
            emit(
                recolor_skip_existing(
                    tx,
                    &src,
                    &format!("{ITEM}flow_armor_trim_smithing_template.png"),
                    &Tint::Force {
                        hue: 225.0,
                        sat: 0.3,
                        v_min: 0.35,
                        v_max: 0.8,
                    },
                )?,
                &mut outcome,
            );
        }

        // 不祥之瓶
        let bottles = ["potion", "splash_potion", "awkward_potion"]
            .map(|n| format!("{ITEM}{n}.png"));
        if let Some(src) = first_existing(tx, &bottles) {
            emit(
                recolor_skip_existing(
                    tx,
                    &src,
                    &format!("{ITEM}ominous_bottle.png"),
                    &Tint::Force {
                        hue: 280.0,
                        sat: 0.22,
                        v_min: 0.2,
                        v_max: 0.5,
                    },
                )?,
                &mut outcome,
            );
        }

        Ok(outcome)
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "generate_tricky_trials_breeze" => Some((decl(), run)),
            _ => None,
        }
    }
}

/// 前向 Architect 批次（收尾）：依赖外部 `UImage` 覆盖图的最后三个任务。
///
/// 三个任务的旧实现**都是「取不到覆盖图就整任务跳过」**，而这正是本轮的关键：
/// - `generate_crossbow`：**先要求 `UImage/crossbow/` 解析成功**（失败即整任务返回），
///   且只在 `UImage/crossbow/crossbow_{bow 的宽度}.png` **存在**时才写 `crossbow_standby`；
///   拉弓组只由 `bow_pulling_0` 的宽度决定基准图；
/// - `generate_tipped_arrow_images`：源在**1.9 路径** `textures/items/arrow.png`；
///   缺 `UImage/tipped_arrow_head/tipped_arrow_head_{size}.png` 即整任务跳过；
///   裁头用 `zip` —— 即**任一图短了就在那里停**；
/// - `generate_snow_bucket`：源 `item/milk_bucket.png`；覆盖图**可缺**（缺了就只留拷贝）。
pub mod arch_gen3 {
    use super::*;
    use image::imageops;

    /// 与 `arch_gen2` 同一个解析函数（含「找不到就在用户文档建默认目录」的副作用）。
    fn uimage_dir() -> Option<std::path::PathBuf> {
        match crate::converters::get_uimage_path() {
            Ok(p) => Some(p),
            Err(e) => {
                crate::log_info!("UImage path not available: {}", e);
                None
            }
        }
    }

    fn overlay_pair(base: &RgbaImage, overlay: &RgbaImage) -> RgbaImage {
        let mut combined = base.clone();
        imageops::overlay(&mut combined, overlay, 0, 0);
        combined
    }

    /// 旧 `converters/textures/crossbow.rs`。
    pub mod crossbow {
        use super::*;

        const ITEM: &str = "assets/minecraft/textures/item";

        /// 旧实现的 (尺寸, 文件名) 映射；**顺序即语义**（按 bow 的宽度找基准图）
        const SIZE_TO_NAME: [(u32, &str); 5] = [
            (16, "crossbow_16.png"),
            (32, "crossbow_32.png"),
            (64, "crossbow_64.png"),
            (128, "crossbow_128.png"),
            (256, "crossbow_256.png"),
        ];

        /// 旧实现的拉弓配对表——注意 `bow_pulling_2` 出现两次，对应四个输出。
        const BOW_PULLING: [&str; 4] = [
            "bow_pulling_0.png",
            "bow_pulling_1.png",
            "bow_pulling_2.png",
            "bow_pulling_2.png",
        ];
        const CROSSBOW_OUT: [&str; 4] = [
            "crossbow_pulling_0.png",
            "crossbow_pulling_1.png",
            "crossbow_pulling_2.png",
            "crossbow_arrow.png",
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_crossbow", Tier::Architect)
                .reads(ScopeSet::prefix(ITEM))
                .writes(ScopeSet::prefix(ITEM))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            // 旧实现：**先解析 UImage**，失败即整任务返回（连 bow 分支都不看）
            let Some(dir) = uimage_dir() else {
                crate::log_info!("UImage path not available, skip crossbow generation");
                return Ok(Outcome::default());
            };
            let crossbow_dir = dir.join("crossbow");
            let base_of = |width: u32| -> Option<std::path::PathBuf> {
                SIZE_TO_NAME
                    .iter()
                    .find(|(size, _)| *size == width)
                    .map(|(_, name)| crossbow_dir.join(name))
            };

            let mut outcome = Outcome::default();

            let bow = format!("{ITEM}/bow.png");
            if tx.exists(&bow) {
                let bow_img: RgbaImage = (*tx.image(&bow)?).clone();
                if let Some(base_path) = base_of(bow_img.width()) {
                    if base_path.exists() {
                        let base_img = image::open(&base_path)
                            .map_err(|e| {
                                AromError::io(format!(
                                    "failed to open {}: {}",
                                    base_path.display(),
                                    e
                                ))
                            })?
                            .to_rgba8();
                        let standby = overlay_pair(&base_img, &bow_img);
                        tx.put_image(&format!("{ITEM}/crossbow_standby.png"), &standby)?;
                        outcome.changed += 1;
                    }
                }
            }

            let pulling0 = format!("{ITEM}/bow_pulling_0.png");
            if tx.exists(&pulling0) {
                let sample: RgbaImage = (*tx.image(&pulling0)?).clone();
                if let Some(base_path) = base_of(sample.width()) {
                    if base_path.exists() {
                        let base_img = image::open(&base_path)
                            .map_err(|e| {
                                AromError::io(format!(
                                    "failed to open {}: {}",
                                    base_path.display(),
                                    e
                                ))
                            })?
                            .to_rgba8();
                        for (bow_file, crossbow_file) in
                            BOW_PULLING.iter().zip(CROSSBOW_OUT.iter())
                        {
                            let bow_path = format!("{ITEM}/{bow_file}");
                            if !tx.exists(&bow_path) {
                                continue;
                            }
                            let bow_img: RgbaImage = (*tx.image(&bow_path)?).clone();
                            let output_path = format!("{ITEM}/{crossbow_file}");
                            tx.put_image(&output_path, &overlay_pair(&base_img, &bow_img))?;
                            outcome.changed += 1;

                            if *crossbow_file == "crossbow_arrow.png" {
                                // 旧实现：先把上一步写出的 crossbow_arrow **拷成**
                                // crossbow_firework，再看 firework 覆盖图是否存在
                                let firework_path = format!("{ITEM}/crossbow_firework.png");
                                let arrow_img: RgbaImage = (*tx.image(&output_path)?).clone();
                                let mut firework_img = arrow_img.clone();
                                tx.put_image(&firework_path, &firework_img)?;
                                let overlay_path = crossbow_dir.join(format!(
                                    "crossbow_firework_{}.png",
                                    sample.width()
                                ));
                                if overlay_path.exists() {
                                    let mut overlay_img = image::open(&overlay_path)
                                        .map_err(|e| {
                                            AromError::io(format!(
                                                "failed to open {}: {}",
                                                overlay_path.display(),
                                                e
                                            ))
                                        })?
                                        .to_rgba8();
                                    if overlay_img.dimensions() != firework_img.dimensions() {
                                        overlay_img = imageops::resize(
                                            &overlay_img,
                                            firework_img.width(),
                                            firework_img.height(),
                                            imageops::FilterType::Triangle,
                                        );
                                    }
                                    imageops::overlay(&mut firework_img, &overlay_img, 0, 0);
                                    tx.put_image(&firework_path, &firework_img)?;
                                }
                            }
                        }
                    }
                }
            }

            Ok(outcome)
        }
    }

    /// 旧 `converters/textures/tipped_arrows.rs`。
    pub mod tipped_arrows {
        use super::*;

        /// 源是**1.9 路径**（旧实现写死 `textures/items`），照抄
        const ITEMS_LEGACY: &str = "assets/minecraft/textures/items";

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_tipped_arrow_images", Tier::Architect)
                .reads(ScopeSet::prefix(ITEMS_LEGACY))
                .writes(ScopeSet::prefix(ITEMS_LEGACY))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let arrow = format!("{ITEMS_LEGACY}/arrow.png");
            if !tx.exists(&arrow) {
                crate::log_info!("arrow.png not found, skip tipped arrow generation");
                return Ok(Outcome::default());
            }
            let base: RgbaImage = (*tx.image(&arrow)?).clone();
            let size = base.width();

            let Some(dir) = uimage_dir() else {
                crate::log_info!("UImage path not available, skip tipped arrow generation");
                return Ok(Outcome::default());
            };
            let head_path = dir
                .join("tipped_arrow_head")
                .join(format!("tipped_arrow_head_{}.png", size));
            if !head_path.exists() {
                crate::log_info!("tipped arrow head not found: {}", head_path.display());
                return Ok(Outcome::default());
            }
            let head = image::open(&head_path)
                .map_err(|e| {
                    AromError::io(format!("failed to open {}: {}", head_path.display(), e))
                })?
                .to_rgba8();

            // 旧实现：按**较短者**停止（`zip` 语义），把头部不透明处的底图 alpha 清掉
            let mut base_out = base.clone();
            let limit = (
                base_out.width().min(head.width()),
                base_out.height().min(head.height()),
            );
            for y in 0..limit.1 {
                for x in 0..limit.0 {
                    if head.get_pixel(x, y)[3] > 0 {
                        base_out.get_pixel_mut(x, y)[3] = 0;
                    }
                }
            }
            tx.put_image(&format!("{ITEMS_LEGACY}/tipped_arrow_base.png"), &base_out)?;

            // 头部贴图是**拷贝**（旧实现 `fs::copy`），等价为原字节写入
            let head_bytes = std::fs::read(&head_path)
                .map_err(|e| AromError::io(format!("read {}: {e}", head_path.display())))?;
            tx.put(&format!("{ITEMS_LEGACY}/tipped_arrow_head.png"), head_bytes)?;

            Ok(Outcome {
                changed: 2,
                notes: vec!["tipped_arrow_base.png + tipped_arrow_head.png".into()],
                ..Outcome::default()
            })
        }
    }

    /// 旧 `converters/textures/snow_bucket.rs`。
    pub mod snow_bucket {
        use super::*;

        const ITEMS: &str = "assets/minecraft/textures/item";

        pub fn decl() -> TaskDecl {
            TaskDecl::new("generate_snow_bucket", Tier::Architect)
                .reads(ScopeSet::prefix(ITEMS))
                .writes(ScopeSet::prefix(ITEMS))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let milk = format!("{ITEMS}/milk_bucket.png");
            if !tx.exists(&milk) {
                return Ok(Outcome::default());
            }
            let base: RgbaImage = (*tx.image(&milk)?).clone();
            let (width, height) = base.dimensions();
            if width != height || width == 0 {
                return Ok(Outcome::default());
            }

            let powder = format!("{ITEMS}/powder_snow_bucket.png");
            // 旧实现先 fs::copy，再（可选）叠加覆盖图
            let mut bucket = base.clone();
            tx.put_image(&powder, &bucket)?;

            match uimage_dir() {
                Some(dir) => {
                    let overlay_path = dir
                        .join("powder_snow_bucket")
                        .join(format!("powder_snow_bucket_{}.png", width));
                    if overlay_path.exists() {
                        let mut overlay = image::open(&overlay_path)
                            .map_err(|e| {
                                AromError::io(format!(
                                    "failed to open {}: {}",
                                    overlay_path.display(),
                                    e
                                ))
                            })?
                            .to_rgba8();
                        if overlay.dimensions() != bucket.dimensions() {
                            overlay = imageops::resize(
                                &overlay,
                                bucket.width(),
                                bucket.height(),
                                imageops::FilterType::Triangle,
                            );
                        }
                        imageops::overlay(&mut bucket, &overlay, 0, 0);
                        tx.put_image(&powder, &bucket)?;
                    }
                }
                None => crate::log_info!("UImage path not available, skip snow bucket overlay"),
            }

            Ok(Outcome {
                changed: 1,
                notes: vec!["milk_bucket.png -> powder_snow_bucket.png".into()],
                ..Outcome::default()
            })
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "generate_crossbow" => Some((crossbow::decl(), crossbow::run)),
            "generate_tipped_arrow_images" => {
                Some((tipped_arrows::decl(), tipped_arrows::run))
            }
            "generate_snow_bucket" => Some((snow_bucket::decl(), snow_bucket::run)),
            _ => None,
        }
    }
}

/// **Surgeon 早期组**（计划里排在最前面的几个 Surgeon 旧任务）。
///
/// 为什么这组能**逐个**边迁边验（而不像 §9.51 说的「必须整阶段」）：它们的槽位在旧批次**之前**，
/// 所以「放进前阶段」与生产顺序一致——这正是 §9.53 之后把早期原生任务放进
/// `EARLY_NATIVES` 的那条路的自然延伸。
///
/// 本批两个任务的语义要点：
/// - `fix_slider`：由 `gui/widgets.png` **裁两条**贴到一张同尺寸的**全透明**新图——
///   注意目标画布是 `ImageBuffer::new`（全 0，含 alpha），不是原图副本；复制逐像素且**越界即跳过**；
/// - `fix_clock_compass`：把 `items/{clock,compass}.png` 纵向**均分抽帧**成 `{prefix}_{NN}.png`
///   （`num_splits > retain_num` 时按 `floor(i*step)` 取帧并夹到 `num_splits-1`），
///   然后**删掉原图与其 `.mcmeta`**。
pub mod surgeon_early {
    use super::*;
    use crate::converters::scale_factor::determine_scale_factor;

    const ITEMS_LEGACY: &str = "assets/minecraft/textures/items";

    /// 旧 `copy_and_paste_region`：逐像素覆盖，**越界跳过**（`get_*_checked`）。
    fn copy_and_paste(src: &RgbaImage, dest: &mut RgbaImage, src_box: (u32, u32, u32, u32), dest_pt: (u32, u32)) {
        let (sx, sy, ex, ey) = src_box;
        let (dx, dy) = dest_pt;
        for y in 0..(ey.saturating_sub(sy)) {
            for x in 0..(ex.saturating_sub(sx)) {
                if let Some(p) = src.get_pixel_checked(sx + x, sy + y) {
                    if let Some(d) = dest.get_pixel_mut_checked(dx + x, dy + y) {
                        *d = *p;
                    }
                }
            }
        }
    }

    /// 旧 `converters/ui/slider.rs`。
    pub mod slider {
        use super::*;

        const WIDGETS: &str = "assets/minecraft/textures/gui/widgets.png";
        const SLIDER: &str = "assets/minecraft/textures/gui/slider.png";

        pub fn decl() -> TaskDecl {
            TaskDecl::new("fix_slider", Tier::Surgeon)
                .reads(ScopeSet::exact(WIDGETS))
                .writes(ScopeSet::exact(SLIDER))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(WIDGETS) {
                crate::log_info!("widgets.png not found, skip slider");
                return Ok(Outcome::default());
            }
            let img: RgbaImage = (*tx.image(WIDGETS)?).clone();
            let (width, height) = img.dimensions();
            let (s, _exact) = determine_scale_factor(width, height);

            // 旧实现：目标是**全透明**的同尺寸新图
            let mut slider_img = RgbaImage::new(width, height);
            let sc = |x: u32, y: u32| (x * s, y * s);
            let (x1, y1) = sc(0, 46);
            let (x2, y2) = sc(200, 66);
            copy_and_paste(&img, &mut slider_img, (x1, y1, x2, y2), (0, 0));
            let (x1, y1) = sc(0, 46);
            let (x2, y2) = sc(200, 106);
            let (dx, dy) = sc(0, 20);
            copy_and_paste(&img, &mut slider_img, (x1, y1, x2, y2), (dx, dy));

            // 旧实现**不创建目录**（`widgets.png` 存在即意味着 `gui/` 已在包里），
            // 因此这里也不能写目录条目——多写一条会让「声明范围」契约当场报错（实测）。
            tx.put_image(SLIDER, &slider_img)?;
            Ok(Outcome {
                changed: 1,
                notes: vec![format!("widgets.png -> slider.png (scale {s})")],
                ..Outcome::default()
            })
        }
    }

    /// 旧 `converters/ui/clock_compass.rs`。
    pub mod clock_compass {
        use super::*;

        pub fn decl() -> TaskDecl {
            TaskDecl::new("fix_clock_compass", Tier::Surgeon)
                .reads(ScopeSet::prefix(ITEMS_LEGACY))
                .writes(ScopeSet::prefix(ITEMS_LEGACY))
                .exclusive(true)
        }

        /// 旧 `split_image`：纵向均分抽帧 → 写 `{prefix}_{NN}.png` → 删原图与 `.mcmeta`。
        fn split(
            tx: &mut Tx<'_>,
            image_rel: &str,
            prefix: &str,
            retain_num: u32,
        ) -> Result<usize, AromError> {
            let img: RgbaImage = (*tx.image(image_rel)?).clone();
            let (img_width, img_height) = img.dimensions();
            if img_width == 0 {
                return Err(AromError::io(format!("invalid image width 0 for {image_rel}")));
            }
            let num_splits = img_height / img_width;
            let split_height = img_height / num_splits.max(1);

            let indices: Vec<u32> = if num_splits > retain_num {
                let step = num_splits as f64 / retain_num as f64;
                (0..retain_num)
                    .map(|i| ((i as f64) * step) as u32)
                    .map(|idx| idx.min(num_splits - 1))
                    .collect()
            } else {
                (0..num_splits).collect()
            };

            let mut written = 0usize;
            for (j, &i) in indices.iter().enumerate() {
                let y = i * split_height;
                let cropped = image::imageops::crop_imm(&img, 0, y, img_width, split_height).to_image();
                tx.put_image(&format!("{ITEMS_LEGACY}/{prefix}_{j:02}.png"), &cropped)?;
                written += 1;
            }

            // 旧实现随后**删原图**（含 `.mcmeta` 附属）
            tx.remove(image_rel)?;
            let mcmeta = format!("{image_rel}.mcmeta");
            if tx.exists(&mcmeta) {
                tx.remove(&mcmeta)?;
            }
            Ok(written)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            let mut outcome = Outcome::default();
            for (name, prefix, retain) in [
                ("clock.png", "clock", 64u32),
                ("compass.png", "compass", 32u32),
            ] {
                let rel = format!("{ITEMS_LEGACY}/{name}");
                if !tx.exists(&rel) {
                    crate::log_info!("{name} not found, skip");
                    continue;
                }
                outcome.changed += split(tx, &rel, prefix, retain)?;
            }
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "fix_slider" => Some((slider::decl(), slider::run)),
            "fix_clock_compass" => Some((clock_compass::decl(), clock_compass::run)),
            _ => None,
        }
    }
}

/// **Surgeon 早期组（续）**：`overlay_icons` 与 `fix_brewing_stand_ui`。
///
/// 两个任务都是「整体重写一张 GUI 图」的形态，但**覆盖图的叠加方式不同**，必须分别照抄：
/// - `overlay_icons`：用**覆盖图自己的 alpha 当蒙版**逐像素混合（等价 PIL `paste(overlay, (0,0), overlay)`），
///   且**无论覆盖图是否存在都会重写 `icons.png`**（存在性判定只决定是否混合）；
/// - `fix_brewing_stand_ui`：由 `shulker_box.png` 派生（填 `cover_box` + 把 18×18 区域贴到 5 个位置），
///   覆盖图走 `imageops::overlay`（**源 alpha 混合，非蒙版语义**）。
///
/// 两者当前的**放置**都留在后阶段（见 §9.59/§9.60）：`fix_brewing_stand_ui` 依赖
/// `generate_shulker_box_ui` 的产物，而后者在旧计划里**还没有**（阶段 2–3 在阶段 1–2 之后）——
/// 也就是说它在生产管线里目前也是**空操作**，必须保持这个行为。
pub mod surgeon_early2 {
    use super::*;
    use image::imageops;

    /// `UImage` 目录（与其它试点同一个解析函数）。
    fn uimage_dir() -> Option<std::path::PathBuf> {
        match crate::converters::get_uimage_path() {
            Ok(p) => Some(p),
            Err(e) => {
                crate::log_info!("UImage path not available: {}", e);
                None
            }
        }
    }

    /// 旧 `converters/ui/overlay_icons.rs`。
    pub mod overlay_icons {
        use super::*;

        const ICONS: &str = "assets/minecraft/textures/gui/icons.png";

        /// 旧 `alpha_paste`：以覆盖图 alpha 为蒙版做**线性混合**（不是 `imageops::overlay`）。
        fn alpha_paste(base: &mut RgbaImage, overlay: &RgbaImage, dest_x: u32, dest_y: u32) {
            let (base_w, base_h) = base.dimensions();
            let (overlay_w, overlay_h) = overlay.dimensions();
            for y in 0..overlay_h {
                let target_y = dest_y + y;
                if target_y >= base_h {
                    continue;
                }
                for x in 0..overlay_w {
                    let target_x = dest_x + x;
                    if target_x >= base_w {
                        continue;
                    }
                    let src_pixel = overlay.get_pixel(x, y);
                    let alpha = src_pixel[3] as f64 / 255.0;
                    if alpha > 0.0 {
                        let dst_pixel = base.get_pixel(target_x, target_y);
                        let blended = [
                            ((1.0 - alpha) * dst_pixel[0] as f64 + alpha * src_pixel[0] as f64) as u8,
                            ((1.0 - alpha) * dst_pixel[1] as f64 + alpha * src_pixel[1] as f64) as u8,
                            ((1.0 - alpha) * dst_pixel[2] as f64 + alpha * src_pixel[2] as f64) as u8,
                            dst_pixel[3].max(src_pixel[3]),
                        ];
                        base.put_pixel(target_x, target_y, image::Rgba(blended));
                    }
                }
            }
        }

        pub fn decl() -> TaskDecl {
            TaskDecl::new("overlay_icons", Tier::Surgeon)
                .reads(ScopeSet::exact(ICONS))
                .writes(ScopeSet::exact(ICONS))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(ICONS) {
                crate::log_info!("icons.png not found, skip");
                return Ok(Outcome::default());
            }
            let mut base: RgbaImage = (*tx.image(ICONS)?).clone();
            let (width, height) = base.dimensions();
            if width != height {
                crate::log_info!("icons.png is not square, skip");
                return Ok(Outcome::default());
            }
            let overlay_filename = match width {
                256 => "icons_256.png",
                512 => "icons_512.png",
                1024 => "icons_1024.png",
                2048 => "icons_2048.png",
                _ => {
                    crate::log_info!("unsupported icons.png size, skip");
                    return Ok(Outcome::default());
                }
            };

            let mut changed = 0usize;
            match uimage_dir() {
                Some(dir) => {
                    let overlay_path = dir.join("icons").join(overlay_filename);
                    if overlay_path.exists() {
                        let overlay = image::open(&overlay_path)
                            .map_err(|e| {
                                AromError::io(format!(
                                    "failed to open {}: {}",
                                    overlay_path.display(),
                                    e
                                ))
                            })?
                            .to_rgba8();
                        alpha_paste(&mut base, &overlay, 0, 0);
                        changed = 1;
                    }
                }
                None => crate::log_info!("UImage path not available, skip overlay_icons"),
            }

            // 旧实现**无论是否叠加都会重写** icons.png（即使没装覆盖图）——照抄这个行为
            tx.put_image(ICONS, &base)?;
            Ok(Outcome {
                changed,
                notes: vec![format!("icons.png rewritten (overlay applied: {changed})")],
                ..Outcome::default()
            })
        }
    }

    /// 旧 `converters/ui/brewing_stand.rs`。
    pub mod brewing_stand {
        use super::*;

        const CONTAINER: &str = "assets/minecraft/textures/gui/container";
        const SHULKER: &str = "assets/minecraft/textures/gui/container/shulker_box.png";
        const BREWING: &str = "assets/minecraft/textures/gui/container/brewing_stand.png";

        pub fn decl() -> TaskDecl {
            TaskDecl::new("fix_brewing_stand_ui", Tier::Surgeon)
                .reads(ScopeSet::prefix(CONTAINER))
                .writes(ScopeSet::prefix(CONTAINER))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(SHULKER) {
                crate::log_info!("shulker_box.png not found, skip brewing stand generation");
                return Ok(Outcome::default());
            }
            let mut img: RgbaImage = (*tx.image(SHULKER)?).clone();
            let (width, height) = img.dimensions();
            if width != height {
                crate::log_info!("shulker_box.png is not square, skip");
                return Ok(Outcome::default());
            }
            let s = match width {
                256 => 1,
                512 => 2,
                1024 => 4,
                2048 => 8,
                _ => {
                    crate::log_info!("unsupported shulker_box.png size, skip");
                    return Ok(Outcome::default());
                }
            };

            let fill_color = *img.get_pixel(5 * s, 4 * s);
            for y in (16 * s)..(72 * s) {
                for x in (6 * s)..(170 * s) {
                    img.put_pixel(x, y, fill_color);
                }
            }

            let region = imageops::crop_imm(&img, 7 * s, 83 * s, 18 * s, 18 * s).to_image();
            for (px, py) in [
                (16 * s, 16 * s),
                (78 * s, 16 * s),
                (55 * s, 50 * s),
                (78 * s, 57 * s),
                (101 * s, 50 * s),
            ] {
                // 旧实现用 `crate::image_utils::paste_region`（原始覆盖，非 alpha 混合），
                // 且**失败只记录不中止**；这里按同样的「越界即跳过」语义实现。
                for y in 0..region.height() {
                    for x in 0..region.width() {
                        let (dx, dy) = (px + x, py + y);
                        if dx < img.width() && dy < img.height() {
                            let px_val = *region.get_pixel(x, y);
                            img.put_pixel(dx, dy, px_val);
                        }
                    }
                }
            }

            match uimage_dir() {
                Some(dir) => {
                    let overlay_path = dir
                        .join("brewing_stand")
                        .join(format!("brewing_stand_{}.png", width));
                    if overlay_path.exists() {
                        let overlay = image::open(&overlay_path)
                            .map_err(|e| {
                                AromError::io(format!(
                                    "failed to open overlay {}: {}",
                                    overlay_path.display(),
                                    e
                                ))
                            })?
                            .to_rgba8();
                        imageops::overlay(&mut img, &overlay, 0, 0);
                    }
                }
                None => crate::log_info!("UImage path not available, skip brewing_stand overlay"),
            }

            tx.put_image(BREWING, &img)?;
            Ok(Outcome {
                changed: 1,
                notes: vec!["shulker_box.png -> brewing_stand.png".into()],
                ..Outcome::default()
            })
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "overlay_icons" => Some((overlay_icons::decl(), overlay_icons::run)),
            "fix_brewing_stand_ui" => Some((brewing_stand::decl(), brewing_stand::run)),
            _ => None,
        }
    }
}

/// **Surgeon 组（续）**：`fix2_horse_ui` 与 `fix_sign_entities`。
///
/// 两个任务都留在**后阶段**（实测通过）：它们的读写都在自己独占的目录里
/// （`gui/sprites/container/{horse,slot}` 与 `entity/signs`），不被更早/更晚的旧任务触碰。
///
/// 照抄的重点：
/// - `fix_sign_entities`：11 个木种变体由 `entity/sign.png` 各自 `adjust_hue_brightness` 而来，
///   最后**把原图改名成 `signs/spruce.png`**（云杉就是原图本身；目标已存在时改为直接删源）——
///   漏掉这一步会少一个变体、多一个 `sign.png`；
/// - `fix2_horse_ui`：三个槽位 sprite 的**改名拷贝**（`*_slot.png` → 去掉 `_slot`），源保留。
pub mod surgeon_mid {
    use super::*;
    use crate::converters::color::hue::adjust_hue_brightness;

    /// 旧 `converters/ui/horse_v2.rs`。
    pub mod horse_v2 {
        use super::*;

        const HORSE: &str = "assets/minecraft/textures/gui/sprites/container/horse";
        const SLOT: &str = "assets/minecraft/textures/gui/sprites/container/slot";

        /// (源, 目标) —— 逐条照抄
        const MAPPINGS: [(&str, &str); 3] = [
            ("armor_slot.png", "horse_armor.png"),
            ("llama_armor_slot.png", "llama_armor.png"),
            ("saddle_slot.png", "saddle.png"),
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("fix2_horse_ui", Tier::Surgeon)
                .reads(ScopeSet::prefix(HORSE))
                .writes(ScopeSet::prefix(SLOT))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            // 旧实现：源目录不存在即整任务跳过
            if !tx.has_prefix(HORSE)? {
                crate::log_info!("horse sprites dir not found, skip fix2_horse_ui");
                return Ok(Outcome::default());
            }
            let mut outcome = Outcome::default();
            for (src_name, dest_name) in MAPPINGS {
                let src = format!("{HORSE}/{src_name}");
                if !tx.exists(&src) {
                    continue;
                }
                let bytes = tx.read(&src)?.unwrap_or_default();
                tx.put(&format!("{SLOT}/{dest_name}"), bytes)?;
                outcome.changed += 1;
            }
            Ok(outcome)
        }
    }

    /// 旧 `converters/ui/sign_entities.rs`。
    pub mod sign_entities {
        use super::*;

        const ENTITY: &str = "assets/minecraft/textures/entity";
        const SIGN: &str = "assets/minecraft/textures/entity/sign.png";
        const SIGNS: &str = "assets/minecraft/textures/entity/signs";

        /// (目标名, 色相, 明度, 饱和) —— 逐条照抄（含 `pale_oak` 的 -100 饱和）
        const VARIANTS: [(&str, f32, f32, f32); 11] = [
            ("oak.png", 0.0, 15.0, 0.0),
            ("birch.png", 0.0, 40.0, 0.0),
            ("acacia.png", -23.0, 10.0, 0.0),
            ("dark_oak.png", 0.0, -15.0, 0.0),
            ("jungle.png", -10.0, 4.6, 0.0),
            ("crimson.png", -59.0, -30.0, 0.0),
            ("warped.png", 130.0, -33.0, 0.0),
            ("mangrove.png", -59.0, -10.0, 0.0),
            ("pale_oak.png", 0.0, 30.0, -100.0),
            ("bamboo.png", 25.0, 20.0, 0.0),
            ("cherry.png", -45.0, 30.0, -18.0),
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("fix_sign_entities", Tier::Surgeon)
                .reads(ScopeSet::prefix(ENTITY))
                .writes(ScopeSet::prefix(ENTITY))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(SIGN) {
                crate::log_info!("sign.png not found in entity/, skip fix_sign_entities");
                return Ok(Outcome::default());
            }
            let base: RgbaImage = (*tx.image(SIGN)?).clone();

            let mut outcome = Outcome::default();
            for (filename, hue, bright, sat) in VARIANTS {
                let adjusted = adjust_hue_brightness(base.clone(), hue, bright, sat);
                tx.put_image(&format!("{SIGNS}/{filename}"), &adjusted)?;
                outcome.changed += 1;
            }

            // 旧实现：原图**改名**为 spruce（云杉 = 原图本身）；目标已存在时改为直接删源
            let spruce = format!("{SIGNS}/spruce.png");
            let bytes = tx.read(SIGN)?.unwrap_or_default();
            if tx.exists(&spruce) {
                tx.remove(SIGN)?;
            } else {
                tx.put(&spruce, bytes)?;
                tx.remove(SIGN)?;
            }
            outcome.notes.push("sign.png -> signs/spruce.png".into());
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "fix2_horse_ui" => Some((horse_v2::decl(), horse_v2::run)),
            "fix_sign_entities" => Some((sign_entities::decl(), sign_entities::run)),
            _ => None,
        }
    }
}

/// **Surgeon 组（续）**：`fix_horse_ui` 与 `fix_sign`。
///
/// 两者都是「原位重写一张图」+「派生一组变体」，照抄要点：
/// - `fix_horse_ui`：`gui/container/horse.png` **原地**做四步（裁剪 18×18 → 贴到 (18,220)；
///   用 (7,16) 的颜色填回原区域；再把 (36,202) 那块拷到 (36,220)；最后可选叠加
///   `UImage/horse/horse_{width}.png`）。尺寸不在 {256,512,1024,2048} 即整任务跳过；
/// - `fix_sign`：先删已存在的 `spruce_sign.png`、把 `oak_sign.png` **改名**成它，再以它为底
///   生成 11 个木种变体（**其中又包含一个 `oak_sign.png`**——净效果是「橡木 = 原图按 +15 明度再染一次」）。
///
/// **与 `generate_poplar_planks` 的交互（值得单独记）**：poplar 会**写 `item/oak_sign.png`**，
/// 而 `fix_sign` 又**读它**。两者在计划里的顺序是 `fix_sign`（阶段 3–4）**早于** poplar（阶段 88–97），
/// 而当前 `generate_poplar_planks` 在 `EARLY_NATIVES` 里（前阶段）——若 `fix_sign` 留在旧批次里，
/// 它就会读到**尚未被 poplar 改写**的 `oak_sign.png`，与生产顺序相反。
/// 因此本批把 `fix_sign` 也一并原生化并放**后阶段**（= 旧批次之后），使其与生产的相对顺序一致。
pub mod surgeon_mid2 {
    use super::*;
    use image::imageops;

    /// `UImage` 目录（与其它试点同一个解析函数）。
    fn uimage_dir() -> Option<std::path::PathBuf> {
        match crate::converters::get_uimage_path() {
            Ok(p) => Some(p),
            Err(e) => {
                crate::log_info!("UImage path not available: {}", e);
                None
            }
        }
    }

    /// 旧 `converters/ui/horse.rs`。
    pub mod horse {
        use super::*;

        const HORSE: &str = "assets/minecraft/textures/gui/container/horse.png";

        /// 旧 `scale_factor`：**宽高都必须等于**标准尺寸之一，否则整任务跳过。
        fn scale_factor(width: u32, height: u32) -> Option<u32> {
            match (width, height) {
                (256, 256) => Some(1),
                (512, 512) => Some(2),
                (1024, 1024) => Some(4),
                (2048, 2048) => Some(8),
                _ => None,
            }
        }

        /// 旧实现用 `imageops::overlay` 贴回（**alpha 混合**，不是原始覆盖）。
        fn overlay_at(img: &mut RgbaImage, region: &RgbaImage, x: u32, y: u32) {
            imageops::overlay(img, region, x as i64, y as i64);
        }

        pub fn decl() -> TaskDecl {
            TaskDecl::new("fix_horse_ui", Tier::Surgeon)
                .reads(ScopeSet::exact(HORSE))
                .writes(ScopeSet::exact(HORSE))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(HORSE) {
                crate::log_info!("horse.png not found, skip");
                return Ok(Outcome::default());
            }
            let mut img: RgbaImage = (*tx.image(HORSE)?).clone();
            let (width, height) = img.dimensions();
            let Some(s) = scale_factor(width, height) else {
                crate::log_info!("unsupported horse.png size, skip");
                return Ok(Outcome::default());
            };

            // 步骤 1：把 (7,17)-(25,35) 贴到 (18,220)
            let region =
                imageops::crop_imm(&img, 7 * s, 17 * s, 18 * s, 18 * s).to_image();
            overlay_at(&mut img, &region, 18 * s, 220 * s);

            // 步骤 2：用 (7,16) 的颜色填回刚搬走的区域
            let fill_color = *img.get_pixel(7 * s, 16 * s);
            for y in (17 * s)..(35 * s) {
                for x in (7 * s)..(25 * s) {
                    img.put_pixel(x, y, fill_color);
                }
            }

            // 步骤 3：把 (36,202) 那块 18×18 拷到 (36,220)
            let copy = imageops::crop_imm(&img, 36 * s, 202 * s, 18 * s, 18 * s).to_image();
            overlay_at(&mut img, &copy, 36 * s, 220 * s);

            // 步骤 4：可选覆盖图
            if let Some(dir) = uimage_dir() {
                let overlay_path = dir.join("horse").join(format!("horse_{}.png", width));
                if overlay_path.exists() {
                    let overlay_img = image::open(&overlay_path)
                        .map_err(|e| {
                            AromError::io(format!(
                                "failed to open overlay {}: {}",
                                overlay_path.display(),
                                e
                            ))
                        })?
                        .to_rgba8();
                    imageops::overlay(&mut img, &overlay_img, 0, 0);
                }
            }

            // 旧实现**无论覆盖图是否存在都会重写** horse.png
            tx.put_image(HORSE, &img)?;
            Ok(Outcome {
                changed: 1,
                notes: vec![format!("horse.png fixed in place (scale {s})")],
                ..Outcome::default()
            })
        }
    }

    /// 旧 `converters/ui/sign.rs`。
    pub mod sign {
        use super::*;
        use crate::converters::color::hue::adjust_hue_brightness;

        const ITEM: &str = "assets/minecraft/textures/item";
        const OAK: &str = "assets/minecraft/textures/item/oak_sign.png";
        const SPRUCE: &str = "assets/minecraft/textures/item/spruce_sign.png";

        /// (文件名, 色相, 明度, 饱和) —— 逐条照抄（注意里面**又有一个 `oak_sign.png`**）
        const VARIANTS: [(&str, f32, f32, f32); 11] = [
            ("oak_sign.png", 0.0, 15.0, 0.0),
            ("birch_sign.png", 0.0, 40.0, 0.0),
            ("acacia_sign.png", -23.0, 10.0, 0.0),
            ("dark_oak_sign.png", 0.0, -15.0, 0.0),
            ("jungle_sign.png", -10.0, 4.6, 0.0),
            ("crimson_sign.png", -59.0, -30.0, 0.0),
            ("warped_sign.png", 130.0, -33.0, 0.0),
            ("mangrove_sign.png", -59.0, -10.0, 0.0),
            ("pale_oak_sign.png", 0.0, 30.0, -100.0),
            ("bamboo_sign.png", 25.0, 20.0, 0.0),
            ("cherry_sign.png", -45.0, 30.0, -18.0),
        ];

        pub fn decl() -> TaskDecl {
            TaskDecl::new("fix_sign", Tier::Surgeon)
                .reads(ScopeSet::prefix(ITEM))
                .writes(ScopeSet::prefix(ITEM))
                .exclusive(true)
        }

        pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
            if !tx.exists(OAK) {
                crate::log_info!("未找到 oak_sign.png，跳过告示牌处理");
                return Ok(Outcome::default());
            }
            // 旧实现：先删已有的 spruce，再把 oak **改名**过去
            if tx.exists(SPRUCE) {
                tx.remove(SPRUCE)?;
            }
            let base_bytes = tx.read(OAK)?.unwrap_or_default();
            tx.put(SPRUCE, base_bytes)?;
            tx.remove(OAK)?;
            let base: RgbaImage = (*tx.image(SPRUCE)?).clone();

            let mut outcome = Outcome::default();
            for (filename, hue, bright, sat) in VARIANTS {
                let adjusted = adjust_hue_brightness(base.clone(), hue, bright, sat);
                tx.put_image(&format!("{ITEM}/{filename}"), &adjusted)?;
                outcome.changed += 1;
            }
            Ok(outcome)
        }
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "fix_horse_ui" => Some((horse::decl(), horse::run)),
            "fix_sign" => Some((sign::decl(), sign::run)),
            _ => None,
        }
    }
}

/// **Surgeon 组（续）**：`fix_particles` —— 把 `particle/particles.png` 切成具名小图。
///
/// 逐条照抄 `converters/textures/particles.rs` 的两个要点：
/// 1. **尺寸守卫**：`w != h || w % 16 != 0` → 整任务跳过（不切、也不删源）；
/// 2. **映射表**：16×16 网格里只有部分格子有名字（`generic_0..7`、`splash_0..3`、
///    `bubble`、`fishing_hook`（写到 **`entity/`** 而不是 `particle/`）、`flame`、`lava`、
///    `note/critical_hit/enchanted_hit`、`heart/angry/glint`、`drip_hang/fall/land`、
///    `effect_0..7`、`spell_0..7`、`spark_0..7`），**其余格子丢弃**；
/// 3. 最后**删掉原 `particles.png`**。
pub mod surgeon_mid3 {
    use super::*;

    const PARTICLE: &str = "assets/minecraft/textures/particle";
    const PARTICLES: &str = "assets/minecraft/textures/particle/particles.png";

    /// 旧实现的 (row, col) → 输出路径映射；返回 `None` 表示该格丢弃。
    fn tile_target(row: u32, col: u32) -> Option<String> {
        let particle = |name: &str| Some(format!("{PARTICLE}/{name}"));
        match (row, col) {
            (0, c) if c < 8 => particle(&format!("generic_{c}.png")),
            (1, c) if (3..=6).contains(&c) => particle(&format!("splash_{}.png", c - 3)),
            (2, 0) => particle("bubble.png"),
            // 注意：这一格写到 entity/ 目录（旧实现如此）
            (2, 1) => Some("assets/minecraft/textures/entity/fishing_hook.png".to_string()),
            (3, 0) => particle("flame.png"),
            (3, 1) => particle("lava.png"),
            (4, c) if c < 3 => particle(["note.png", "critical_hit.png", "enchanted_hit.png"][c as usize]),
            (5, c) if c < 3 => particle(["heart.png", "angry.png", "glint.png"][c as usize]),
            (7, c) if c < 3 => particle(["drip_hang.png", "drip_fall.png", "drip_land.png"][c as usize]),
            (8, c) if c < 8 => particle(&format!("effect_{c}.png")),
            (9, c) if c < 8 => particle(&format!("spell_{c}.png")),
            (10, c) if c < 8 => particle(&format!("spark_{c}.png")),
            _ => None,
        }
    }

    pub fn decl() -> TaskDecl {
        TaskDecl::new("fix_particles", Tier::Surgeon)
            .reads(ScopeSet::prefix("assets/minecraft/textures"))
            .writes(ScopeSet::prefix("assets/minecraft/textures"))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        if !tx.exists(PARTICLES) {
            crate::log_info!("particles.png not found, skip fix_particles");
            return Ok(Outcome::default());
        }
        let img: RgbaImage = (*tx.image(PARTICLES)?).clone();
        let (w, h) = img.dimensions();
        if w != h || w % 16 != 0 {
            crate::log_info!("particles.png is not square /16, skip split");
            return Ok(Outcome::default());
        }
        let split = w / 16;

        let mut outcome = Outcome::default();
        for row in 0u32..16 {
            for col in 0u32..16 {
                let Some(target) = tile_target(row, col) else {
                    continue;
                };
                let tile = image::imageops::crop_imm(&img, col * split, row * split, split, split)
                    .to_image();
                tx.put_image(&target, &tile)?;
                outcome.changed += 1;
            }
        }

        // 旧实现最后删掉原图
        tx.remove(PARTICLES)?;
        outcome.notes.push("particles.png split and removed".into());
        Ok(outcome)
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "fix_particles" => Some((decl(), run)),
            _ => None,
        }
    }
}

/// **Surgeon 组（续）**：`fix_tabs` —— 原位搬移 `gui/container/creative_inventory/tabs.png` 的区域。
///
/// 逐条照抄 `converters/ui/tabs.rs` 的四步（缩放因子走共享的 `determine_scale_factor`，
/// **不是**「必须等于标准尺寸」那套）：
/// 1. 把 (168,0)-(196,128) **右移 14**；
/// 2. 六组区域各自**左移**固定像素（(15,0)-(41,128) 左移 2、(43,…) 左移 4、(71,…) 6、
///    (99,…) 8、(127,…) 10、(155,0)-(168,128) 12），左移用 `saturating_sub`（**不会为负**）；
/// 3. 把 (0,0)-(26,128) **拷到** (156,0)；
/// 4. **无论哪一步都没写**，最后都会把 `tabs.png` 重写一次（旧实现如此）。
///
/// 搬移的语义是「先整体裁剪出源区域、再**逐像素覆盖**贴到目标」——读取全部来自裁剪副本，
/// 因此源区与目标区重叠时不会自我污染，且**越界写入跳过**、**越界读取留透明**。
pub mod surgeon_mid4 {
    use super::*;
    use crate::converters::scale_factor::determine_scale_factor;

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
}

/// **Surgeon 组（续）**：`fix_armor_models` —— 把旧路径的盔甲模型**改名搬走**到新路径。
///
/// 逐条照抄 `converters/textures/armor.rs`：
/// `models/armor/{chainmail,diamond,iron,gold,leather,leather_overlay,netherite,copper}_layer_{1,2}.png`
/// → `entity/equipment/humanoid/`（layer_1）与 `entity/equipment/humanoid_leggings/`（layer_2），
/// 目标名去掉 `_layer_N` 后缀；**源被移走**。`models/armor/` 不存在即整任务跳过。
///
/// **它正是 §9.53 的另一半**：两个 `generate_*_armor_models` 之所以必须放前阶段，
/// 就是因为本任务会把它们的**源**搬走。本任务自己不改写被下游消费的图，因此**放后阶段**即可。
pub mod surgeon_late {
    use super::*;

    const ARMOR_SRC: &str = "assets/minecraft/textures/models/armor";
    const HUMANOID: &str = "assets/minecraft/textures/entity/equipment/humanoid";
    const LEGGINGS: &str = "assets/minecraft/textures/entity/equipment/humanoid_leggings";

    /// (源文件名, 目标名) —— 逐条照抄（layer_1 组）
    const LAYER1: [(&str, &str); 8] = [
        ("chainmail_layer_1.png", "chainmail.png"),
        ("diamond_layer_1.png", "diamond.png"),
        ("iron_layer_1.png", "iron.png"),
        ("gold_layer_1.png", "gold.png"),
        ("leather_layer_1.png", "leather.png"),
        ("leather_layer_1_overlay.png", "leather_overlay.png"),
        ("netherite_layer_1.png", "netherite.png"),
        ("copper_layer_1.png", "copper.png"),
    ];

    /// (源文件名, 目标名) —— layer_2 组
    const LAYER2: [(&str, &str); 8] = [
        ("chainmail_layer_2.png", "chainmail.png"),
        ("diamond_layer_2.png", "diamond.png"),
        ("iron_layer_2.png", "iron.png"),
        ("gold_layer_2.png", "gold.png"),
        ("leather_layer_2.png", "leather.png"),
        ("leather_layer_2_overlay.png", "leather_overlay.png"),
        ("netherite_layer_2.png", "netherite.png"),
        ("copper_layer_2.png", "copper.png"),
    ];

    pub fn decl() -> TaskDecl {
        // 注意两点（都踩过）：
        // ① **删除也是写入**（层里是 Tombstone），源目录必须一并声明，否则 `strict_scopes` 报错；
        // ② `TaskDecl::writes()` 是**覆盖**不是累加——多个范围要用 `ScopeSet::with_prefix` 组合。
        TaskDecl::new("fix_armor_models", Tier::Surgeon)
            .reads(ScopeSet::prefix(ARMOR_SRC))
            .writes(
                ScopeSet::prefix(ARMOR_SRC)
                    .with_prefix("assets/minecraft/textures/entity/equipment"),
            )
            .exclusive(true)
    }

    /// 旧实现是 `fs::rename`（**覆盖**目标），这里等价为「读字节 → 写目标 → 删源」。
    fn move_one(
        tx: &mut Tx<'_>,
        src_name: &str,
        dest_dir: &str,
        dest_name: &str,
        outcome: &mut Outcome,
    ) -> Result<(), AromError> {
        let src = format!("{ARMOR_SRC}/{src_name}");
        if !tx.exists(&src) {
            return Ok(());
        }
        let bytes = tx.read(&src)?.unwrap_or_default();
        tx.put(&format!("{dest_dir}/{dest_name}"), bytes)?;
        tx.remove(&src)?;
        outcome.changed += 1;
        Ok(())
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        if !tx.has_prefix(ARMOR_SRC)? {
            crate::log_info!("armor models dir not found, skip");
            return Ok(Outcome::default());
        }
        let mut outcome = Outcome::default();
        for (src, dest) in LAYER1 {
            move_one(tx, src, HUMANOID, dest, &mut outcome)?;
        }
        for (src, dest) in LAYER2 {
            move_one(tx, src, LEGGINGS, dest, &mut outcome)?;
        }
        Ok(outcome)
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "fix_armor_models" => Some((decl(), run)),
            _ => None,
        }
    }
}

/// **Surgeon 组（续）**：`fix_ui_sub_hand` 与 `fix_ui_creative` —— 两个「原位改写 GUI 图」的任务。
///
/// 与 `fix_horse_ui`/`fix_slider` 同族，**计划槽位都在最前面**（阶段 1–2），因此放前阶段与生产一致。
/// 两者共用同一套区域原语，逐条照抄：
/// - **搬移/拷贝都是「先裁剪出源区域、再逐像素覆盖贴到目标」**：读取全部来自裁剪副本，
///   源区与目标区重叠也不会自我污染；越界写入跳过、越界读取留透明；
/// - `fix_ui_sub_hand`：`gui/widgets.png` 的 (1,23)-(23,45) 各拷到 (24,23) 与 (60,23)；
/// - `fix_ui_creative`：`…/creative_inventory/tab_inventory.png` 三步——(6,0)-(84,53) 拷到 (51,0)；
///   左区 (6,0)-(53,53) 用 **(164,27)** 的颜色填充（**注意取色发生在第一步拷贝之后**）；
///   再把 (53,5) 起 18×18 拷到 (34,19)（**同理，源取自被第一步改过的图**）。
pub mod surgeon_ui {
    use super::*;
    use crate::converters::scale_factor::determine_scale_factor;

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
}

/// **Surgeon 组（续）**：`fix_machinery_ui` —— 五个「机械方块 GUI」子步骤。
///
/// 逐条照抄 `converters/ui/machinery.rs`。四个子步骤共用 `process_ui_from_shulker`：
/// 读 **`container/shulker_box.png`**（尺寸必须是 256/512/1024/2048 之一，否则跳过），
/// 填 `cover_box` (6,16)-(170,72)（色取自 **(5,4)**），把 18×18 区域（取自 **(7,83)**）**逐像素覆盖**
/// 贴到各自位置表，再可选叠加 `UImage/{subdir}/{prefix}_{width}.png`，最后可选**贴 anvil 区域**
/// （(176,0) 起 28×21；尺寸不符时先把整张 anvil 按 `Nearest` 缩放到 (width,width)）。
///
/// **两种叠加语义不同，必须分开**：UImage 覆盖图用 `imageops::overlay`（按源 alpha 混合），
/// 而区域搬移/贴上用**原始覆盖**（等价 `Image.paste`，alpha 也照抄）——`copy_from` 在越界时会
/// 直接报错，所以这里手写「越界即跳过」的循环，与旧实现一致。
///
/// `process_villager2_machinery` 单独一支：读 `villager.png` → 生成**双宽**新图 → … →
/// **先把原图备份成 `villager_backup.png`（已存在则跳过备份）**，再把新图写回 `villager.png`。
/// 那个备份是反向任务 `reverse_fix_machinery_ui` 还原的依据，**不能漏**。
pub mod surgeon_machinery {
    use super::*;
    use image::imageops;

    const CONTAINER: &str = "assets/minecraft/textures/gui/container";

    /// `UImage` 目录（与其它试点同一个解析函数）。
    fn uimage_dir() -> Option<std::path::PathBuf> {
        match crate::converters::get_uimage_path() {
            Ok(p) => Some(p),
            Err(e) => {
                crate::log_info!("UImage path not available: {}", e);
                None
            }
        }
    }

    /// 旧的 `paste_region`：**原始覆盖**、越界跳过（不用 `copy_from`，它越界会报错）。
    fn paste(img: &mut RgbaImage, region: &RgbaImage, dx: u32, dy: u32) {
        for y in 0..region.height() {
            for x in 0..region.width() {
                if let Some(px) = region.get_pixel_checked(x, y) {
                    if let Some(dst) = img.get_pixel_mut_checked(dx + x, dy + y) {
                        *dst = *px;
                    }
                }
            }
        }
    }

    /// 旧的 `process_ui_from_shulker`。
    #[allow(clippy::too_many_arguments)]
    fn ui_from_shulker(
        tx: &mut Tx<'_>,
        out_name: &str,
        paste_positions: &[(u32, u32)],
        overlay_subdir: &str,
        overlay_prefix: &str,
        paste_anvil: bool,
    ) -> Result<bool, AromError> {
        let shulker = format!("{CONTAINER}/shulker_box.png");
        if !tx.exists(&shulker) {
            crate::log_info!("shulker_box.png not found, skip {out_name}");
            return Ok(false);
        }
        let mut img: RgbaImage = (*tx.image(&shulker)?).clone();
        let (width, height) = img.dimensions();
        let s = match (width, height) {
            (256, 256) => 1,
            (512, 512) => 2,
            (1024, 1024) => 4,
            (2048, 2048) => 8,
            _ => {
                crate::log_info!("unsupported shulker_box size {width}x{height}, skip {out_name}");
                return Ok(false);
            }
        };

        // 填 cover_box（色取自 (5,4)）
        let fill = *img.get_pixel(5 * s, 4 * s);
        for y in (16 * s)..(72 * s) {
            for x in (6 * s)..(170 * s) {
                img.put_pixel(x, y, fill);
            }
        }

        // 18×18 区域贴到各位置（原始覆盖）
        let region = imageops::crop_imm(&img, 7 * s, 83 * s, 18 * s, 18 * s).to_image();
        for (px, py) in paste_positions {
            paste(&mut img, &region, px * s, py * s);
        }

        // 可选 UImage 覆盖图（**alpha 混合**，与上面的原始覆盖不同）
        if let Some(dir) = uimage_dir() {
            let overlay_path = dir
                .join(overlay_subdir)
                .join(format!("{overlay_prefix}_{width}.png"));
            if overlay_path.exists() {
                let overlay = image::open(&overlay_path)
                    .map_err(|e| {
                        AromError::io(format!("failed to open overlay: {e}"))
                    })?
                    .to_rgba8();
                imageops::overlay(&mut img, &overlay, 0, 0);
            }
        }

        // 可选贴 anvil 区域
        if paste_anvil {
            let anvil = format!("{CONTAINER}/anvil.png");
            if tx.exists(&anvil) {
                let anvil_img: RgbaImage = (*tx.image(&anvil)?).clone();
                let resized = if anvil_img.dimensions() != (width, width) {
                    imageops::resize(&anvil_img, width, width, imageops::FilterType::Nearest)
                } else {
                    anvil_img
                };
                let crop = imageops::crop_imm(&resized, 176 * s, 0, 28 * s, 21 * s).to_image();
                imageops::overlay(&mut img, &crop, (176 * s) as i64, 0);
            }
        }

        tx.put_image(&format!("{CONTAINER}/{out_name}"), &img)?;
        Ok(true)
    }

    /// 旧的 `process_villager2_machinery`。
    fn villager2_machinery(tx: &mut Tx<'_>) -> Result<bool, AromError> {
        let villager = format!("{CONTAINER}/villager.png");
        if !tx.exists(&villager) {
            crate::log_info!("villager.png not found, skip villager2 machinery");
            return Ok(false);
        }
        let img: RgbaImage = (*tx.image(&villager)?).clone();
        let (width, height) = img.dimensions();
        if width != height {
            crate::log_info!("villager.png is not square, skip villager2 machinery");
            return Ok(false);
        }
        let s = match width {
            256 => 1,
            512 => 2,
            1024 => 4,
            2048 => 8,
            _ => {
                crate::log_info!("unsupported villager.png size, skip");
                return Ok(false);
            }
        };

        let new_w = width * 2;
        let new_h = height;
        let mut out = RgbaImage::new(new_w, new_h);

        // 贴 (0,0)-(240,166) 到 (100s, 0)（alpha 混合，旧实现用 overlay）
        let cropped = imageops::crop_imm(&img, 0, 0, 240 * s, 166 * s).to_image();
        imageops::overlay(&mut out, &cropped, (100 * s) as i64, 0);

        // 可选 UImage villager2 覆盖图
        if let Some(dir) = uimage_dir() {
            let overlay_path = dir
                .join("villager2")
                .join(format!("villager2_{}.png", 256 * s));
            if overlay_path.exists() {
                let overlay = image::open(&overlay_path)
                    .map_err(|e| AromError::io(format!("failed to open overlay: {e}")))?
                    .to_rgba8();
                imageops::overlay(&mut out, &overlay, 0, 0);
            }
        }

        // 填 (186,24)-(208,39)，色取自 (185,17)
        let c1 = *out.get_pixel(185 * s, 17 * s);
        for y in (24 * s)..(39 * s) {
            for x in (186 * s)..(208 * s) {
                out.put_pixel(x, y, c1);
            }
        }

        // 把 (133,48)-(242,76) 上移 16s（区域裁剪自**当前**图，再覆盖贴回）
        let mv = imageops::crop_imm(&out, 133 * s, 48 * s, 109 * s, 28 * s).to_image();
        imageops::overlay(&mut out, &mv, (133 * s) as i64, (32 * s) as i64);

        // 填 (133,60)-(242,76)，色取自 (132,60)
        let c2 = *out.get_pixel(132 * s, 60 * s);
        for y in (60 * s)..(76 * s) {
            for x in (133 * s)..(242 * s) {
                out.put_pixel(x, y, c2);
            }
        }

        // (0,166)-(110,198) 置透明
        for y in (166 * s)..(198 * s) {
            for x in 0..(110 * s) {
                out.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
            }
        }

        // 可选贴 anvil（缩放到**新图**尺寸）
        let anvil = format!("{CONTAINER}/anvil.png");
        if tx.exists(&anvil) {
            let anvil_img: RgbaImage = (*tx.image(&anvil)?).clone();
            let resized = if anvil_img.dimensions() != (new_w, new_h) {
                imageops::resize(&anvil_img, new_w, new_h, imageops::FilterType::Nearest)
            } else {
                anvil_img
            };
            let crop = imageops::crop_imm(&resized, 176 * s, 0, 28 * s, 21 * s).to_image();
            imageops::overlay(&mut out, &crop, (176 * s) as i64, 0);
        }

        // **先把原图备份**（已存在则跳过备份），再写回 villager.png
        let backup = format!("{CONTAINER}/villager_backup.png");
        if !tx.exists(&backup) {
            let bytes = tx.read(&villager)?.unwrap_or_default();
            tx.put(&backup, bytes)?;
        }
        tx.put_image(&villager, &out)?;
        Ok(true)
    }

    pub fn decl() -> TaskDecl {
        TaskDecl::new("fix_machinery_ui", Tier::Surgeon)
            .reads(ScopeSet::prefix(CONTAINER))
            .writes(ScopeSet::prefix(CONTAINER))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        if !tx.has_prefix(CONTAINER)? {
            return Ok(Outcome::default());
        }
        let mut outcome = Outcome::default();
        // 顺序照抄旧实现
        for (out_name, positions, subdir, prefix, anvil) in [
            (
                "grindstone.png",
                &[(48u32, 18u32), (128, 33), (48, 39)][..],
                "grindstone",
                "grindstone",
                true,
            ),
            (
                "cartography_table.png",
                &[(14u32, 51u32), (144, 38), (14, 14)][..],
                "cartography_table",
                "cartography_table",
                false,
            ),
            (
                "stonecutter.png",
                &[(19u32, 32u32), (142, 32)][..],
                "stonecutter",
                "stonecutter",
                false,
            ),
            (
                "loom.png",
                &[(12u32, 25u32), (32, 25), (22, 44), (142, 56)][..],
                "loom",
                "loom",
                false,
            ),
        ] {
            if ui_from_shulker(tx, out_name, positions, subdir, prefix, anvil)? {
                outcome.changed += 1;
            }
        }
        if villager2_machinery(tx)? {
            outcome.changed += 1;
            outcome.notes.push("villager.png -> double width + backup".into());
        }
        Ok(outcome)
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "fix_machinery_ui" => Some((decl(), run)),
            _ => None,
        }
    }
}

/// **Surgeon 收尾项：`adapt_java_shaders`（第一步：纯文本改写）**。
///
/// 背景（§9.77/§9.78）：真实包里没有 `shaders/`，该任务在第一个判断就返回——
/// **真实包闸门对它给的是假绿灯**，因此迁移必须靠夹具正题（第 12 个忽略用例已钉住三个分支的基线）。
///
/// 本模块是**分步移植的第一步**：只做**纯文本**翻译，不碰文件系统的删改与表格逻辑。
/// 函数与旧实现一一对应，符号名保持一致便于对照：
///
/// | 本模块 | 旧实现 `converters/shaders/java.rs` |
/// |---|---|
/// | [`convert_moj_import_to_include`] | `fn convert_moj_import_to_include` |
/// | [`rewrite_import_path`] | `fn rewrite_import_path` |
/// | [`namespace_moj_imports`] | `fn namespace_moj_imports` |
/// | [`needs_globals_import`] / [`has_globals_import`] / [`inject_globals_import`] | 同名 |
///
/// **尚未移植**（下一步）：表驱动的 `prune_and_rename_core`（4 张表 + 2 个 allowlist）、
/// `ensure_core_json`、`rewrite_json_matrix_types`、`strip_json_uniforms_for_ubo`、
/// `adapt_post_paths`、`walk_dir` 的遍历骨架。**因此本模块暂不派发**（生产路径不变）。
///
/// **§9.85 起已派发**：生产入口是 [`shader_adapt::run_from_pack`]（驱动派发表已登记），
/// 因此移植期用来压住死代码警告的 `#[cfg(test)]` **已移除**。
pub mod shader_adapt {
    use super::*;

    /// 旧 `rewrite_import_path`：`<a/b.glsl>` → `<a:b.glsl>`；`"x.glsl"` 保持引号形式。
    pub fn rewrite_import_path(rest: &str) -> Option<String> {
        let quoted = if rest.starts_with('<') && rest.ends_with('>') {
            false
        } else if rest.starts_with('"') && rest.ends_with('"') {
            true
        } else {
            return None;
        };
        let inner = rest[1..rest.len() - 1].trim().trim_start_matches('/');
        if quoted {
            return Some(format!("\"{}\"", inner));
        }
        if let Some((ns, path)) = inner.split_once(':') {
            let path = path.strip_prefix("include/").unwrap_or(path);
            return Some(format!("<{}:{}>", ns, path));
        }
        let path = inner.strip_prefix("include/").unwrap_or(inner);
        Some(format!("<{}>", path))
    }

    /// 旧 `convert_moj_import_to_include`（26.3+：`#moj_import` → `#include`）。
    ///
    /// 注意旧实现用 `lines()` 重组、**每行都补 `\n`**，因此会顺手把 CRLF 归一为 LF
    /// 并给末行补上换行——移植时照抄这一点（否则产物字节会不同）。
    pub fn convert_moj_import_to_include(src: &str) -> (String, usize) {
        let mut n = 0usize;
        let mut out = String::with_capacity(src.len());
        for line in src.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("#moj_import") {
                let rest = rest.trim();
                let converted = rewrite_import_path(rest);
                if let Some(new_rest) = converted {
                    out.push_str(&format!("#include {}\n", new_rest));
                    n += 1;
                    continue;
                }
                out.push_str(&format!("#include {}\n", rest));
                n += 1;
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
        (out, n)
    }

    /// 旧 `namespace_moj_imports`（1.21.4+：给无命名空间的 import 补 `minecraft:`）。
    pub fn namespace_moj_imports(src: &str) -> (String, usize) {
        let mut n = 0usize;
        let mut out = String::with_capacity(src.len());
        for line in src.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("#moj_import") {
                if let Some(rest) = trimmed.strip_prefix("#moj_import") {
                    let rest = rest.trim();
                    if (rest.starts_with('<') && rest.ends_with('>') && !rest.contains(':'))
                        || (rest.starts_with('"') && rest.ends_with('"') && !rest.contains(':'))
                    {
                        let inner: &str = &rest[1..rest.len() - 1];
                        let inner = inner.trim_start_matches('/');
                        if inner.starts_with("include/") || inner.contains(':') {
                            out.push_str(line);
                            out.push('\n');
                            continue;
                        }
                        out.push_str(&format!("#moj_import <minecraft:{}>\n", inner));
                        n += 1;
                        continue;
                    }
                }
            }
            out.push_str(line);
            out.push('\n');
        }
        (out, n)
    }

    /// 旧 `needs_globals_import`：源码是否用到 `ScreenSize` / `GameTime`。
    pub fn needs_globals_import(src: &str) -> bool {
        src.contains("ScreenSize") || src.contains("GameTime")
    }

    /// 旧 `has_globals_import`：**逐行**判断是否存在 import/include 了 globals 的行。
    ///
    /// 注意与 `needs_globals_import` 不同：这里**不是**子串搜索——`globals.glsl` 出现在注释里
    /// 不算数。§9.82 的逐函数对照正是靠这条把第一版的子串写法抓了出来。
    pub fn has_globals_import(src: &str) -> bool {
        src.lines().any(|l| {
            let t = l.trim();
            (t.starts_with("#moj_import") || t.starts_with("#include")) && t.contains("globals.glsl")
        })
    }

    /// 旧 `inject_globals_import`：在**第一个非空、非注释**行之前插入 import；
    /// 若通篇都是空行/注释，则追加到末尾。`use_include_directive` 决定用 `#include` 还是 `#moj_import`。
    pub fn inject_globals_import(src: &str, use_include_directive: bool) -> String {
        let import = if use_include_directive {
            "#include <minecraft:globals.glsl>\n"
        } else {
            "#moj_import <minecraft:include/globals.glsl>\n"
        };
        let mut out = String::with_capacity(src.len() + import.len() + 8);
        let mut injected = false;
        for line in src.lines() {
            let t = line.trim();
            if !injected
                && !t.is_empty()
                && !t.starts_with("//")
                && !t.starts_with("/*")
                && !t.starts_with('*')
            {
                out.push_str(import);
                injected = true;
            }
            out.push_str(line);
            out.push('\n');
        }
        if !injected {
            out.push_str(import);
        }
        out
    }

    /// 旧 `count_args_likely_three`：启发式判断 `fn_name(...)` 是否像**三参**调用
    /// （按括号深度数顶层逗号，遇到第一个 ≥2 个逗号的调用即返回真）。
    pub fn count_args_likely_three(src: &str, fn_name: &str) -> bool {
        let pat = format!("{}(", fn_name);
        let mut idx = 0usize;
        while let Some(pos) = src[idx..].find(&pat) {
            let start = idx + pos + pat.len();
            let bytes = src.as_bytes();
            let mut depth = 1usize;
            let mut commas = 0usize;
            let mut i = start;
            while i < bytes.len() && depth > 0 {
                match bytes[i] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    b',' if depth == 1 => commas += 1,
                    _ => {}
                }
                i += 1;
            }
            if commas >= 2 {
                return true;
            }
            idx = start;
        }
        false
    }

    /// 旧 `walk_dir` 里的 fog 标记：`target >= 32` 且像三参调用且未标记时，**在文件开头**插入一行注释。
    pub fn fog_note_if_needed(src: &str, target: u32) -> (String, bool) {
        if target >= FMT_FOG_DISTANCE && src.contains("fog_distance(") {
            if count_args_likely_three(src, "fog_distance") && !src.contains("2PYR: fog_distance") {
                return (
                    format!(
                        "// 2PYR: fog_distance() 1.20.5+ 签名变更，请对照 vanilla fog.glsl\n{}",
                        src
                    ),
                    true,
                );
            }
        }
        (src.to_string(), false)
    }

    const SHADERS: &str = "assets/minecraft/shaders";

    pub fn decl() -> TaskDecl {
        TaskDecl::new("adapt_java_shaders", Tier::Surgeon)
            .reads(ScopeSet::prefix(SHADERS))
            .writes(ScopeSet::prefix(SHADERS))
            .exclusive(true)
    }

    /// 与旧实现同一套里程碑判定。
    pub fn is_modern_shader_api(target: u32) -> bool {
        target >= FMT_MODERN_JSON
    }

    /// 递归改写：**跳过 `include/`**（被 import 引用，写坏会导致整包重载失败）。
    ///
    /// 逐条照抄 `walk_dir` 的三段判定与**顺序**：导入指令 → globals 注入 → fog 标记。
    fn walk_dir(
        tx: &mut Tx<'_>,
        dir: &str,
        target: u32,
        changed: &mut usize,
    ) -> Result<(), AromError> {
        for entry in tx.list(dir)? {
            let path = entry.path.clone();
            if entry.is_dir {
                let folder = path.rsplit('/').next().unwrap_or("");
                if folder.eq_ignore_ascii_case("include") {
                    continue;
                }
                walk_dir(tx, &path, target, changed)?;
                continue;
            }
            let lower = path.to_ascii_lowercase();
            if !(lower.ends_with(".vsh") || lower.ends_with(".fsh") || lower.ends_with(".glsl")) {
                continue;
            }
            let is_core = dir.rsplit('/').next() == Some("core");
            let Some(bytes) = tx.read(&path)? else { continue };
            let Ok(raw) = String::from_utf8(bytes) else {
                continue;
            };
            let mut out = raw.clone();
            let mut file_changed = false;

            if is_core && target >= FMT_IMPORT_NS {
                if target >= FMT_INCLUDE_DIRECTIVE {
                    let (next, n) = convert_moj_import_to_include(&out);
                    if n > 0 {
                        out = next;
                        file_changed = true;
                    }
                } else {
                    let (next, n) = namespace_moj_imports(&out);
                    if n > 0 {
                        out = next;
                        file_changed = true;
                    }
                }
            }

            if is_core
                && target >= FMT_GLOBALS_INCLUDE
                && (lower.ends_with(".vsh") || lower.ends_with(".fsh"))
                && needs_globals_import(&out)
                && !has_globals_import(&out)
            {
                out = inject_globals_import(&out, target >= FMT_INCLUDE_DIRECTIVE);
                file_changed = true;
            }

            let (next, fog_changed) = fog_note_if_needed(&out, target);
            if fog_changed {
                out = next;
                file_changed = true;
            }

            if file_changed {
                tx.put(&path, out.into_bytes())?;
                *changed += 1;
            }
        }
        Ok(())
    }

    /// 旧 `prune_and_rename_core` 的**改名**步骤：`json/vsh/fsh` 成组；目标已存在则删源。
    fn rename_core_group(
        tx: &mut Tx<'_>,
        old: &str,
        new: &str,
        changed: &mut usize,
    ) -> Result<(), AromError> {
        for ext in ["json", "vsh", "fsh"] {
            let src = format!("{SHADERS}/core/{old}.{ext}");
            if !tx.exists(&src) {
                continue;
            }
            let dst = format!("{SHADERS}/core/{new}.{ext}");
            if tx.exists(&dst) {
                tx.remove(&src)?;
            } else {
                let bytes = tx.read(&src)?.unwrap_or_default();
                tx.put(&dst, bytes)?;
                tx.remove(&src)?;
            }
            *changed += 1;
        }
        Ok(())
    }

    /// 旧 `ensure_include_trailing_newline`：include 下的 `glsl/vsh/fsh` 必须以**空行**结尾
    /// （至少两个换行）；空文件跳过。
    fn ensure_include_trailing_newline(tx: &mut Tx<'_>, changed: &mut usize) -> Result<(), AromError> {
        let dir = format!("{SHADERS}/include");
        if !tx.has_prefix(&dir)? {
            return Ok(());
        }
        for entry in tx.list(&dir)? {
            if entry.is_dir {
                continue;
            }
            let ext = entry.path.rsplit('.').next().unwrap_or("");
            if !matches!(ext, "glsl" | "vsh" | "fsh") {
                continue;
            }
            let Some(bytes) = tx.read(&entry.path)? else {
                continue;
            };
            let Ok(mut raw) = String::from_utf8(bytes) else {
                continue;
            };
            if raw.is_empty() {
                continue;
            }
            if raw.ends_with('\n') {
                if !raw.ends_with("\n\n") {
                    raw.push('\n');
                    tx.put(&entry.path, raw.into_bytes())?;
                    *changed += 1;
                }
            } else {
                raw.push_str("\n\n");
                tx.put(&entry.path, raw.into_bytes())?;
                *changed += 1;
            }
        }
        Ok(())
    }

    /// **派发入口**：目标 `pack_format` 由任务**自行从包里的 `pack.mcmeta` 读出**。
    ///
    /// 为什么这样做（§9.85）：旧实现从 `ctx.get_data("target_pack_format")` 取，而原生任务只拿得到
    /// `&mut Tx`。两条可选路径：
    /// ① 改驱动，把 `MixedRunOptions::target_version` 传进执行体——但那要为**一个任务**改动
    ///    `native_for` 的返回类型与约 45 个 return 点（§9.84 两次脚本尝试都编译不过，已回退）；
    /// ②**让任务自己读包**——它是自描述的，且与生产**同源**：
    ///    生产里 `context.set_data("target_pack_format", target_version)`（`invoke_conversion.rs:158`），
    ///    而驱动的收尾步骤把同一个 `target_version` 写进 `pack.mcmeta`（`write_pack_format`），
    ///    所以从包里读出的值与传进来的那个是**同一个数**。
    ///
    /// 取不到 `pack.mcmeta` 或解析失败时按旧实现的缺省值 **88** 处理。
    pub fn run_from_pack(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let target = tx
            .view()
            .mcmeta()
            .ok()
            .and_then(|m| m.effective_format())
            .unwrap_or(88);
        run(tx, target)
    }

    /// 总体编排（**与原实现逐项对照通过**，见 §9.83 的夹具快照对照）。
    ///
    /// 覆盖旧实现 `adapt_java_shaders_at` 的**全部步骤**：
    /// 0) **post 路径**（现代删 `shaders/post` 与 `post_effect`；旧目标删各 namespace 的 `post_effect`）；
    ///    旧着色器 API（`target < 7`）→ 删 `post_effect` + **递归删 JSON** 后返回；
    /// 1) core **改名** + **删除**（移除名单 / 白名单外）+ include 结尾空行；
    /// 2) core JSON 的**补齐**（`<63`）/ **mat 升级**（`≥7`）/ **剥 uniforms**（`≥63`）；
    /// 3) 源码遍历（导入指令 / globals 注入 / fog 标记，跳过 `include/`）。
    ///
    /// **目标 pack_format 由任务自行从包里读出**（见 [`run_from_pack`]），无需驱动传参。
    pub fn run(tx: &mut Tx<'_>, target_pack_format: u32) -> Result<Outcome, AromError> {
        if !tx.has_prefix(SHADERS)? {
            return Ok(Outcome::default());
        }
        let mut changed = 0usize;

        // 0) post 路径（现代删 shaders/post 与 post_effect；旧目标删各 namespace 的 post_effect）
        adapt_post_paths(tx, target_pack_format, &mut changed)?;

        // 旧着色器 API：删 post_effect + 递归删 JSON，然后返回
        if !is_modern_shader_api(target_pack_format) {
            if tx.has_prefix(&format!("{SHADERS}/post_effect"))? {
                tx.remove(&format!("{SHADERS}/post_effect"))?;
                changed += 1;
            }
            strip_json_in(tx, SHADERS, &mut changed)?;
            return Ok(Outcome {
                changed,
                notes: vec![format!("shader adapt (legacy target {target_pack_format})")],
                ..Outcome::default()
            });
        }

        // 1) core：改名 + 删除（两者都只动 core/）
        prune_and_rename_core(tx, target_pack_format, &mut changed)?;
        ensure_include_trailing_newline(tx, &mut changed)?;

        // 2) core JSON：补齐（<63）/ mat 升级（≥7）/ 剥 uniforms（≥63）
        if tx.has_prefix(&format!("{SHADERS}/core"))? {
            if target_pack_format < FMT_GLOBALS_INCLUDE {
                ensure_core_json(tx, &mut changed)?;
            }
            if target_pack_format >= FMT_MODERN_JSON {
                rewrite_json_matrix_types(tx, &mut changed)?;
            }
            if target_pack_format >= FMT_GLOBALS_INCLUDE {
                strip_json_uniforms_for_ubo(tx, &mut changed)?;
            }
        }

        // 3) 源码遍历（跳过 include/）
        walk_dir(tx, SHADERS, target_pack_format, &mut changed)?;

        Ok(Outcome {
            changed,
            notes: vec![format!("shader adapt target={target_pack_format}")],
            ..Outcome::default()
        })
    }

    /// 旧 `adapt_post_paths`。
    fn adapt_post_paths(tx: &mut Tx<'_>, target: u32, changed: &mut usize) -> Result<(), AromError> {
        if target >= FMT_GLOBALS_INCLUDE {
            // 现代：删掉旧位置（这些 JSON 在新版本不会被正确加载，且可能干扰）
            for dir in [format!("{SHADERS}/post"), format!("{SHADERS}/post_effect")] {
                if tx.has_prefix(&dir)? {
                    tx.remove(&dir)?;
                    *changed += 1;
                }
            }
            // 扫描各 namespace：若只有旧式 shaders/post 源而无 post_effect，**不**自动伪造 JSON
        } else {
            // 旧目标：删各 namespace 下的现代路径 post_effect，避免旧版本无法解析
            for entry in tx.list("assets")? {
                if !entry.is_dir {
                    continue;
                }
                let pe = format!("{}/post_effect", entry.path);
                if tx.has_prefix(&pe)? {
                    tx.remove(&pe)?;
                    *changed += 1;
                }
            }
            let pe = format!("{SHADERS}/post_effect");
            if tx.has_prefix(&pe)? {
                tx.remove(&pe)?;
                *changed += 1;
            }
        }
        Ok(())
    }

    /// 旧 `prune_and_rename_core`：改名组（仅 modern）+ 移除名单 + 白名单外清理。
    fn prune_and_rename_core(
        tx: &mut Tx<'_>,
        target: u32,
        changed: &mut usize,
    ) -> Result<(), AromError> {
        let core = format!("{SHADERS}/core");
        if !tx.has_prefix(&core)? {
            return Ok(());
        }
        let modern = target >= FMT_GLOBALS_INCLUDE;
        let allow: Vec<&str> = if modern {
            modern_core_allowlist(target)
        } else {
            legacy_core_allowlist()
        };
        let removed = core_removed_stems(target);

        // 1) 旧名 → 新名（json/vsh/fsh 成组；仅 modern 目标）
        if modern {
            for (old, new) in core_rename_table(target) {
                if old == new {
                    continue;
                }
                rename_core_group(tx, old, new, changed)?;
            }
        }

        // 2) 明确移除名单（旧目标不删 legacy 白名单里仍存在的名字）
        for stem in &removed {
            if !modern && allow.contains(stem) {
                continue;
            }
            for ext in ["json", "vsh", "fsh"] {
                let p = format!("{SHADERS}/core/{stem}.{ext}");
                if tx.exists(&p) {
                    tx.remove(&p)?;
                    *changed += 1;
                }
            }
        }

        // 3) 白名单外的核心程序文件删除（.glsl 与共享 vsh 保留）
        for entry in tx.list(&core)? {
            if entry.is_dir {
                continue;
            }
            let name = entry
                .path
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if name.ends_with(".glsl") {
                continue;
            }
            let stem = if let Some(s) = name.strip_suffix(".vsh") {
                s.to_string()
            } else if let Some(s) = name.strip_suffix(".fsh") {
                s.to_string()
            } else if let Some(s) = name.strip_suffix(".json") {
                s.to_string()
            } else {
                continue;
            };
            if allow.contains(&stem.as_str()) || SHARED_VERTEX_STEMS.contains(&stem.as_str()) {
                continue;
            }
            tx.remove(&entry.path)?;
            *changed += 1;
        }
        Ok(())
    }

    /// 旧 `ensure_core_json`：成对 `vsh+fsh` 且缺 JSON 时补最小定义（共享顶点程序与
    /// `position_color` 除外）。
    fn ensure_core_json(tx: &mut Tx<'_>, changed: &mut usize) -> Result<(), AromError> {
        let core = format!("{SHADERS}/core");
        for entry in tx.list(&core)? {
            if entry.is_dir {
                continue;
            }
            let name = entry.path.rsplit('/').next().unwrap_or("").to_string();
            if !(name.ends_with(".vsh") || name.ends_with(".fsh")) {
                continue;
            }
            let stem = name
                .trim_end_matches(".vsh")
                .trim_end_matches(".fsh")
                .to_string();
            if SHARED_VERTEX_STEMS.contains(&stem.as_str()) || stem == "position_color" {
                continue;
            }
            if !tx.exists(&format!("{core}/{stem}.vsh")) || !tx.exists(&format!("{core}/{stem}.fsh")) {
                continue;
            }
            let json = format!("{core}/{stem}.json");
            if tx.exists(&json) {
                continue;
            }
            tx.put(&json, minimal_core_json(&stem).into_bytes())?;
            *changed += 1;
        }
        Ok(())
    }

    /// 旧 `rewrite_json_matrix_types`：core 下每个 `.json` 做 mat2/mat3 → mat4。
    fn rewrite_json_matrix_types(tx: &mut Tx<'_>, changed: &mut usize) -> Result<(), AromError> {
        let core = format!("{SHADERS}/core");
        for entry in tx.list(&core)? {
            if entry.is_dir || !entry.path.ends_with(".json") {
                continue;
            }
            let Some(bytes) = tx.read(&entry.path)? else {
                continue;
            };
            let Ok(raw) = String::from_utf8(bytes) else {
                continue;
            };
            let (out, did) = rewrite_json_matrix_types_text(&raw);
            if did && out != raw {
                tx.put(&entry.path, out.into_bytes())?;
                *changed += 1;
            }
        }
        Ok(())
    }

    /// 旧 `strip_json_uniforms_for_ubo`：含 `"uniforms"` 的 core JSON 去掉该键。
    fn strip_json_uniforms_for_ubo(tx: &mut Tx<'_>, changed: &mut usize) -> Result<(), AromError> {
        let core = format!("{SHADERS}/core");
        for entry in tx.list(&core)? {
            if entry.is_dir || !entry.path.ends_with(".json") {
                continue;
            }
            let Some(bytes) = tx.read(&entry.path)? else {
                continue;
            };
            let Ok(raw) = String::from_utf8(bytes) else {
                continue;
            };
            if !json_has_uniforms(&raw) {
                continue;
            }
            let stripped = remove_json_key(&raw, "uniforms");
            if stripped != raw {
                tx.put(&entry.path, stripped.into_bytes())?;
                *changed += 1;
            }
        }
        Ok(())
    }

    /// 旧 `strip_json_in`：**递归**删掉目录下所有 `.json`。
    fn strip_json_in(tx: &mut Tx<'_>, dir: &str, changed: &mut usize) -> Result<(), AromError> {
        if !tx.has_prefix(dir)? {
            return Ok(());
        }
        for entry in tx.list(dir)? {
            if entry.is_dir {
                strip_json_in(tx, &entry.path, changed)?;
            } else if entry.path.ends_with(".json") {
                tx.remove(&entry.path)?;
                *changed += 1;
            }
        }
        Ok(())
    }

    // ───────────────────────── 表格（移植自 `converters/shaders/java.rs`）─────────────────────────
    //
    // 这些表是**纯数据**，但逐字错误会静默改变删/改名行为，所以移植时一并抄全，
    // 并用「表对照」测试（§9.80）在多个 target 上与旧实现比对（排序后逐项相等）。

    /// pack_format 里程碑（与旧实现同名同值）。
    pub const FMT_MODERN_JSON: u32 = 7;
    pub const FMT_FOG_DISTANCE: u32 = 32;
    pub const FMT_IMPORT_NS: u32 = 46;
    pub const FMT_GLOBALS_INCLUDE: u32 = 63;
    pub const FMT_ENTITY_BLOCK: u32 = 84;
    pub const FMT_INCLUDE_DIRECTIVE: u32 = 97;

    /// 共享顶点程序（不得按 stem 补 JSON；可能无同名 json）。
    pub const SHARED_VERTEX_STEMS: &[&str] = &["screenquad", "animate_sprite"];

    /// 旧 `modern_core_allowlist`：目标版本仍存在的核心程序 stem。
    pub fn modern_core_allowlist(target: u32) -> Vec<&'static str> {
        let mut set: Vec<&'static str> = vec![
            "animate_sprite_blit",
            "animate_sprite_interpolate",
            "blit_depth",
            "blit_screen",
            "block",
            "debug_point",
            "entity",
            "glint",
            "gui",
            "item",
            "lightmap",
            "oit_composite",
            "panorama",
            "particle",
            "position",
            "position_color",
            "position_tex",
            "position_tex_color",
            "rendertype_beacon_beam",
            "rendertype_crumbling",
            "rendertype_end_portal",
            "rendertype_entity_shadow",
            "rendertype_leash",
            "rendertype_lightning",
            "rendertype_lines",
            "rendertype_outline",
            "rendertype_text",
            "rendertype_text_intensity",
            "rendertype_text_see_through",
            "rendertype_text_intensity_see_through",
            "rendertype_water_mask",
            "sky",
            "stars",
            "terrain",
            "text",
        ];
        set.extend_from_slice(SHARED_VERTEX_STEMS);
        if target >= FMT_ENTITY_BLOCK {
            set.push("block");
        }
        if target >= FMT_INCLUDE_DIRECTIVE {
            set.push("clouds");
            set.push("world_border");
        } else {
            set.push("rendertype_clouds");
            set.push("rendertype_world_border");
            set.push("rendertype_text_background");
            set.push("rendertype_text_background_see_through");
        }
        set.sort_unstable();
        set
    }

    /// 旧 `legacy_core_allowlist`：旧版目标仍存在的核心名。
    pub fn legacy_core_allowlist() -> Vec<&'static str> {
        let mut set: Vec<&'static str> = vec![
            "blit_screen",
            "position",
            "position_color",
            "position_tex",
            "position_tex_color",
            "position_color_tex",
            "position_texture",
            "particle",
            "rendertype_armor_entity_glint",
            "rendertype_armor_entity_glint_direct",
            "rendertype_beacon_beam",
            "rendertype_block",
            "rendertype_breeze_spikes",
            "rendertype_breeze_wind",
            "rendertype_clouds",
            "rendertype_crumbling",
            "rendertype_cutout",
            "rendertype_cutout_mipped",
            "rendertype_cutout_mipped_aliased",
            "rendertype_end_portal",
            "rendertype_end_gateway",
            "rendertype_entity",
            "rendertype_entity_alpha",
            "rendertype_entity_cutout",
            "rendertype_entity_cutout_no_cull",
            "rendertype_entity_cutout_no_cull_z_offset",
            "rendertype_entity_decal",
            "rendertype_entity_glint",
            "rendertype_entity_glint_direct",
            "rendertype_entity_no_outline",
            "rendertype_entity_shadow",
            "rendertype_entity_smooth_cutout",
            "rendertype_entity_solid",
            "rendertype_entity_translucent",
            "rendertype_entity_translucent_cull",
            "rendertype_entity_translucent_emissive",
            "rendertype_entity_translucent_no_outline",
            "rendertype_energy_swirl",
            "rendertype_glint",
            "rendertype_glint_direct",
            "rendertype_glint_translucent",
            "rendertype_gui",
            "rendertype_gui_ghost_recipe_overlay",
            "rendertype_gui_overlay",
            "rendertype_gui_text_highlight",
            "rendertype_item",
            "rendertype_item_entity_translucent_cull",
            "rendertype_leash",
            "rendertype_lightning",
            "rendertype_lines",
            "rendertype_outline",
            "rendertype_solid",
            "rendertype_text",
            "rendertype_text_background",
            "rendertype_text_background_see_through",
            "rendertype_text_intensity",
            "rendertype_text_intensity_see_through",
            "rendertype_text_see_through",
            "rendertype_translucent",
            "rendertype_translucent_moving_block",
            "rendertype_translucent_no_crumbling",
            "rendertype_tripwire",
            "rendertype_water_mask",
            "rendertype_world_border",
            "rendertype_phantom",
            "rendertype_dragon_explosion_alpha",
            "rendertype_chain",
            "screenquad",
            "animate_sprite",
            "lightmap",
            "gui",
            "sky",
            "stars",
            "terrain",
            "entity",
            "text",
            "item",
            "glint",
            "clouds",
            "world_border",
            "block",
            "panorama",
            "oit_composite",
            "animate_sprite_blit",
            "animate_sprite_interpolate",
            "blit_depth",
            "debug_point",
        ];
        set.sort_unstable();
        set
    }

    /// 旧 `core_rename_table`：旧名 → 新名（仅 ≥63 之后应用合并改名）。
    pub fn core_rename_table(target: u32) -> Vec<(&'static str, &'static str)> {
        if target < FMT_GLOBALS_INCLUDE {
            return Vec::new();
        }
        let mut map: Vec<(&'static str, &'static str)> = vec![
            ("rendertype_solid", "terrain"),
            ("rendertype_cutout", "terrain"),
            ("rendertype_cutout_mipped", "terrain"),
            ("rendertype_cutout_mipped_aliased", "terrain"),
            ("rendertype_translucent", "terrain"),
            ("rendertype_translucent_no_crumbling", "terrain"),
            ("rendertype_tripwire", "terrain"),
            ("rendertype_entity_solid", "entity"),
            ("rendertype_entity_cutout", "entity"),
            ("rendertype_entity_cutout_no_cull", "entity"),
            ("rendertype_entity_cutout_no_cull_z_offset", "entity"),
            ("rendertype_entity_translucent", "entity"),
            ("rendertype_entity_translucent_cull", "entity"),
            ("rendertype_entity_translucent_emissive", "entity"),
            ("rendertype_entity_translucent_no_outline", "entity"),
            ("rendertype_entity_no_outline", "entity"),
            ("rendertype_entity_smooth_cutout", "entity"),
            ("rendertype_energy_swirl", "entity"),
            ("rendertype_breeze_spikes", "entity"),
            ("rendertype_breeze_wind", "entity"),
            ("rendertype_chain", "entity"),
            ("rendertype_armor_entity_glint", "glint"),
            ("rendertype_armor_entity_glint_direct", "glint"),
            ("rendertype_entity_glint", "glint"),
            ("rendertype_entity_glint_direct", "glint"),
            ("rendertype_glint", "glint"),
            ("rendertype_glint_direct", "glint"),
            ("rendertype_glint_translucent", "glint"),
            ("position_texture", "position_tex"),
            ("position_color_tex", "position_tex_color"),
            ("position_color_tex_lightmap", "position_tex_color"),
            ("rendertype_gui", "gui"),
            ("rendertype_gui_overlay", "position_tex_color"),
            ("rendertype_gui_text_highlight", "gui"),
            ("rendertype_gui_ghost_recipe_overlay", "gui"),
            ("rendertype_text", "text"),
            ("rendertype_text_intensity", "text"),
            ("rendertype_text_see_through", "text"),
            ("rendertype_text_intensity_see_through", "text"),
            ("text_see_through", "text"),
        ];
        if target >= FMT_ENTITY_BLOCK {
            map.push(("rendertype_entity_alpha", "entity"));
            map.push(("rendertype_entity_decal", "entity"));
            map.push(("rendertype_item_entity_translucent_cull", "entity"));
            map.push(("rendertype_translucent_moving_block", "block"));
        }
        if target >= FMT_INCLUDE_DIRECTIVE {
            map.push(("rendertype_clouds", "clouds"));
            map.push(("rendertype_world_border", "world_border"));
        }
        map
    }

    /// 旧 `core_removed_stems`：目标版本下应直接删除的 stem。
    pub fn core_removed_stems(target: u32) -> Vec<&'static str> {
        let mut gone = vec![
            "position_color_normal",
            "position_tex_lightmap_color",
            "position_color_lightmap",
            "rendertype_end_gateway",
            "rendertype_dragon_explosion_alpha",
            "rendertype_phantom",
            "rendertype_water_mask_offset",
            "position_tex_lightmap",
            "position_color_tex_lightmap_color",
        ];
        if target >= FMT_INCLUDE_DIRECTIVE {
            gone.push("text_background");
            gone.push("text_background_see_through");
            gone.push("rendertype_text_background");
            gone.push("rendertype_text_background_see_through");
        }
        gone
    }

    /// 旧 `rewrite_json_matrix_types` 的核心文本变换：`"type": "mat2"|"mat3"` → `"type": "mat4"`。
    ///
    /// 逐条照抄：只匹配**带空格的**字面量形式（§9.78 实测到过——写成无空格的 `"mat3"` 不会命中），
    /// 返回 `(新文本, 是否改变)`。
    pub fn rewrite_json_matrix_types_text(raw: &str) -> (String, bool) {
        let mut out = raw.to_string();
        let mut changed = false;
        for from in ["\"type\": \"mat2\"", "\"type\": \"mat3\""] {
            if out.contains(from) {
                out = out.replace(from, "\"type\": \"mat4\"");
                changed = true;
            }
        }
        (out, changed)
    }

    /// 旧 `ensure_core_json` 的 JSON 体（成对 vsh+fsh 且缺 JSON 时补的最小定义）。
    pub fn minimal_core_json(stem: &str) -> String {
        format!(
            "{{\n  \"vertex\": \"{}\",\n  \"fragment\": \"{}\"\n}}\n",
            stem, stem
        )
    }

    /// 旧 `strip_json_uniforms_for_ubo` 的判定：该 JSON 是否含 `"uniforms"`。
    pub fn json_has_uniforms(raw: &str) -> bool {
        raw.contains("\"uniforms\"")
    }

    /// 旧 `remove_json_key`：从 JSON 对象文本中删除顶层 `"key": …`（粗粒度，够用即可）。
    pub fn remove_json_key(src: &str, key: &str) -> String {
        let pattern = format!("\"{}\"", key);
        let Some(start) = src.find(&pattern) else {
            return src.to_string();
        };
        let after_key = &src[start + pattern.len()..];
        let Some(colon_rel) = after_key.find(':') else {
            return src.to_string();
        };
        let value_start = start + pattern.len() + colon_rel + 1;
        let bytes = src.as_bytes();
        let mut i = value_start;
        while i < bytes.len() && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            return src.to_string();
        }
        let value_end = match bytes[i] {
            b'{' | b'[' => {
                let mut depth = 0usize;
                let mut j = i;
                while j < bytes.len() {
                    match bytes[j] {
                        b'{' | b'[' => depth += 1,
                        b'}' | b']' => {
                            depth -= 1;
                            if depth == 0 {
                                j += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                j
            }
            b'"' => {
                let mut j = i + 1;
                while j < bytes.len() {
                    if bytes[j] == b'"' && bytes[j - 1] != b'\\' {
                        j += 1;
                        break;
                    }
                    j += 1;
                }
                j
            }
            _ => {
                let mut j = i;
                while j < bytes.len() && bytes[j] != b',' && bytes[j] != b'}' && bytes[j] != b'\n' {
                    j += 1;
                }
                j
            }
        };
        let mut end = value_end;
        while end < bytes.len() && (bytes[end] as char).is_whitespace() {
            end += 1;
        }
        if end < bytes.len() && bytes[end] == b',' {
            end += 1;
        } else {
            let mut k = start;
            while k > 0 && (bytes[k - 1] as char).is_whitespace() {
                k -= 1;
            }
            if k > 0 && bytes[k - 1] == b',' {
                return format!("{}{}", &src[..k - 1], &src[end..]);
            }
        }
        format!("{}{}", &src[..start], &src[end..])
    }
}
/// **Surgeon 组（续）**：`fix_ui_survival` —— 生存背包界面的四步修复。
///
/// 逐条照抄 `converters/ui/survival.rs`：
/// 1. **抽状态图标**：把 `gui/container/inventory.png` 里 y=198 起的三行 18×18 图标
///    （8+8+3 共 19 个）裁出来写成 `mob_effect/*.png`——**这正是 `generate_tricky_trials_breeze`
///    的源**（§9.55 记过这条顺序依赖），所以本任务必须留在**前阶段之前/以内**的正确位置；
/// 2. **移动区域**：把 (86,24)-(162,62) 移到 (+10,-8)。旧实现的搬移有个**特意保留的怪癖**：
///    先用**目标位置**的颜色填掉源区，而目标位置此时尚未被粘贴——照抄，不"顺手修正"；
/// 3. **填充两处背景**（取色点 (90,10)）与**拷贝一块**（(152,26)-(172,46) → (75,60)，**原始覆盖**）；
/// 4. **可选叠加** `UImage/inventory/inventory_{width}.png`（尺寸不符时按 `Lanczos3` 缩放整张）；
///    再抽出两张 1.21 药水背景 sprite 写到 `gui/sprites/container/inventory/`。
///
/// **不复刻 `HurrayContext` 的纹理缓存**（`cache_texture`）——那是旧执行器的内存优化，
/// 与产物无关；原生实现从 `Tx` 读、写回 `Tx`。
pub mod surgeon_survival {
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
            if let Ok(dir) = crate::converters::get_uimage_path() {
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
}
/// **Surgeon 阶段**：`fix_smithing2_villager2_ui` —— 铁砧/村民 GUI 的第二步重排。
///
/// 逐条照抄 `converters/ui/smithing_villager.rs`（364 行，两个子过程）。
/// 与反向任务 `reverse_fix_smithing2_villager2_ui`（已迁移，见 `reverse_defer::smithing_villager`）配对。
///
/// **`s` 的语义**（两个子过程各自判定，互不共享）：
/// - `process_smithing2` 用 **`(width, height)`** 匹配 `(256,256)→1 / (512,512)→2 / (1024,1024)→4 / (2048,2048)→8`，
///   非方形或其它尺寸**跳过**；
/// - `process_villager2` 先要求 **`width == height`**，再用 **`width`** 匹配 `256/512/1024/2048`。
///
/// **刻意照抄的两处**：
/// 1. `get_pixel` 一律用 `*img.get_pixel(..)`（越界会 panic），不做"顺手加保护"；
/// 2. 写回用 `DynamicImage::write_to(.., Png)`——与旧 `img.save(path)` 同一编码路径（§9.87 由闸门实测把关）。
pub mod surgeon_smithing2 {
    use super::*;
    use image::imageops;

    const GUI: &str = "assets/minecraft/textures/gui/container";
    const ANVIL: &str = "assets/minecraft/textures/gui/container/anvil.png";
    const SMITHING: &str = "assets/minecraft/textures/gui/container/smithing.png";
    const VILLAGER: &str = "assets/minecraft/textures/gui/container/villager.png";
    const BACKUP: &str = "assets/minecraft/textures/gui/container/villager_backup.png";

    /// `UImage` 目录（与其它试点同一个解析函数）。
    fn uimage_dir() -> Option<std::path::PathBuf> {
        match crate::converters::get_uimage_path() {
            Ok(p) => Some(p),
            Err(e) => {
                crate::log_info!("UImage path not available: {}", e);
                None
            }
        }
    }

    fn read_rgba(tx: &Tx<'_>, path: &str) -> Result<Option<RgbaImage>, AromError> {
        let Some(bytes) = tx.read(path)? else {
            return Ok(None);
        };
        match image::load_from_memory(&bytes) {
            Ok(img) => Ok(Some(img.to_rgba8())),
            Err(e) => Err(AromError::Io(format!("failed to open {path}: {e}"))),
        }
    }

    fn png_bytes(img: &RgbaImage) -> Result<Vec<u8>, AromError> {
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img.clone())
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .map_err(|e| AromError::Io(format!("png encode failed: {e}")))?;
        Ok(buf)
    }

    /// 旧 `paste_region` 在**同尺寸**区域上的等价物：整块覆盖（含 alpha）。
    fn paste(img: &mut RgbaImage, region: &RgbaImage, dx: u32, dy: u32) {
        let (rw, rh) = region.dimensions();
        for y in 0..rh {
            for x in 0..rw {
                let px = dx + x;
                let py = dy + y;
                if px < img.width() && py < img.height() {
                    img.put_pixel(px, py, *region.get_pixel(x, y));
                }
            }
        }
    }

    fn fill_rect(img: &mut RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32, color: image::Rgba<u8>) {
        for y in y0..y1 {
            for x in x0..x1 {
                img.put_pixel(x, y, color);
            }
        }
    }

    /// 旧 `process_smithing2`。
    fn process_smithing2(tx: &mut Tx<'_>, outcome: &mut Outcome) -> Result<(), AromError> {
        if !tx.exists(ANVIL) {
            crate::log_info!("anvil.png not found, skip smithing2");
            return Ok(());
        }
        let Some(mut img) = read_rgba(tx, ANVIL)? else {
            return Ok(());
        };
        let (width, height) = img.dimensions();
        let s = match (width, height) {
            (256, 256) => 1,
            (512, 512) => 2,
            (1024, 1024) => 4,
            (2048, 2048) => 8,
            _ => {
                crate::log_info!(
                    "unsupported anvil.png size: {}x{}, skip smithing2",
                    width,
                    height
                );
                return Ok(());
            }
        };

        // 用 (5,4) 的颜色填 (5,5)-(171,72)
        let fill_color = *img.get_pixel(5 * s, 4 * s);
        fill_rect(&mut img, 5 * s, 5 * s, 171 * s, 72 * s, fill_color);

        // 把 (7,83) 起的 18x18 区域贴到 4 个位置
        let region = imageops::crop_imm(&img, 7 * s, 83 * s, 18 * s, 18 * s).to_image();
        for &(px, py) in &[
            (7 * s, 47 * s),
            (25 * s, 47 * s),
            (43 * s, 47 * s),
            (97 * s, 47 * s),
        ] {
            paste(&mut img, &region, px, py);
        }

        // 外部叠加（与其它试点一致：UImage 在真实文件系统上）
        if let Some(uimage) = uimage_dir() {
            let overlay_path = uimage.join("smithing2").join(format!("smithing2_{}.png", width));
            if overlay_path.exists() {
                if let Ok(overlay_img) = image::open(&overlay_path).map(|i| i.to_rgba8()) {
                    imageops::overlay(&mut img, &overlay_img, 0, 0);
                    crate::log_info!("overlayed smithing2_{}.png", width);
                }
            }
        }

        tx.put(SMITHING, png_bytes(&img)?)?;
        outcome.changed += 1;
        outcome.notes.push("smithing2: saved smithing.png".into());
        Ok(())
    }

    /// 旧 `process_villager2`。
    fn process_villager2(tx: &mut Tx<'_>, outcome: &mut Outcome) -> Result<(), AromError> {
        if !tx.exists(VILLAGER) {
            crate::log_info!("villager.png not found, skip villager2");
            return Ok(());
        }
        let Some(img) = read_rgba(tx, VILLAGER)? else {
            return Ok(());
        };
        let (width, height) = img.dimensions();
        if width != height {
            crate::log_info!(
                "villager.png is not square ({}x{}), skip villager2",
                width,
                height
            );
            return Ok(());
        }
        let s = match width {
            256 => 1,
            512 => 2,
            1024 => 4,
            2048 => 8,
            _ => {
                crate::log_info!("unsupported villager.png size: {}, skip villager2", width);
                return Ok(());
            }
        };

        let scaled = |c: u32| c * s;
        let new_w = width * 2;
        let new_h = height;
        let mut v2 = RgbaImage::new(new_w, new_h);

        // 把 (0,0)-(240,166) 贴到 (100*s, 0)
        let cropped = imageops::crop_imm(&img, 0, 0, scaled(240), scaled(166)).to_image();
        imageops::overlay(&mut v2, &cropped, scaled(100) as i64, 0);

        // 外部叠加 villager2/villager2_{256*s}.png
        if let Some(uimage) = uimage_dir() {
            let overlay_path = uimage
                .join("villager2")
                .join(format!("villager2_{}.png", 256 * s));
            if overlay_path.exists() {
                if let Ok(overlay_img) = image::open(&overlay_path).map(|i| i.to_rgba8()) {
                    imageops::overlay(&mut v2, &overlay_img, 0, 0);
                    crate::log_info!("overlayed villager2_{}.png", 256 * s);
                }
            }
        }

        // 用 (185,17) 的颜色填 (186,24)-(208,39)
        let color1 = *v2.get_pixel(scaled(185), scaled(17));
        fill_rect(&mut v2, scaled(186), scaled(24), scaled(208), scaled(39), color1);

        // 把 (133,48)-(242,76) 上移 16*s
        let move_w = scaled(242) - scaled(133);
        let move_h = scaled(76) - scaled(48);
        let moved = imageops::crop_imm(&v2, scaled(133), scaled(48), move_w, move_h).to_image();
        let dst_y = scaled(48) - scaled(16);
        imageops::overlay(&mut v2, &moved, scaled(133) as i64, dst_y as i64);

        // 用 (132,60) 的颜色填 (133,60)-(242,76)
        let color2 = *v2.get_pixel(scaled(132), scaled(60));
        fill_rect(&mut v2, scaled(133), scaled(60), scaled(242), scaled(76), color2);

        // (0,166)-(110,198) 置为全透明
        fill_rect(&mut v2, 0, scaled(166), scaled(110), scaled(198), image::Rgba([0, 0, 0, 0]));

        // 把 anvil 的 (176,0)-(204,21) 贴到 villager2 的同位置（尺寸不符时按 Nearest 缩放到新尺寸）
        if tx.exists(ANVIL) {
            if let Some(anvil_img) = read_rgba(tx, ANVIL)? {
                let anvil_resized = if anvil_img.dimensions() != (new_w, new_h) {
                    imageops::resize(&anvil_img, new_w, new_h, imageops::FilterType::Nearest)
                } else {
                    anvil_img
                };
                let anvil_crop = imageops::crop_imm(
                    &anvil_resized,
                    scaled(176),
                    0,
                    scaled(204) - scaled(176),
                    scaled(21),
                )
                .to_image();
                imageops::overlay(&mut v2, &anvil_crop, scaled(176) as i64, 0);
            }
        }

        // 备份原图（仅当备份不存在）
        if !tx.exists(BACKUP) {
            let bytes = tx.read(VILLAGER)?.unwrap_or_default();
            tx.put(BACKUP, bytes)?;
            outcome.changed += 1;
            outcome.notes.push("backed up villager.png".into());
        }

        // 覆盖写回 villager.png
        tx.put(VILLAGER, png_bytes(&v2)?)?;
        outcome.changed += 1;
        outcome.notes.push("villager2: saved villager.png".into());
        Ok(())
    }

    pub fn decl() -> TaskDecl {
        TaskDecl::new("fix_smithing2_villager2_ui", Tier::Surgeon)
            .reads(ScopeSet::prefix(GUI))
            .writes(ScopeSet::prefix(GUI))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        if !tx.has_prefix(GUI)? {
            return Ok(Outcome::default());
        }
        let mut outcome = Outcome::default();
        process_smithing2(tx, &mut outcome)?;
        process_villager2(tx, &mut outcome)?;
        crate::log_info!("fix_smithing2_villager2_ui completed");
        Ok(outcome)
    }
}
/// **M2 收尾：`cut_gui` 的 workdir 形态入口**（§9.99）。
///
/// `cut_gui` 是**唯一一个「位置敏感 + 必须直接读盘」**的转换任务：它在计划里的位置是
/// `(15,18)`（夹在旧批次中间），而 `GuiSurgeon` 按设计**直接读写工作目录**。
///
/// `Tx` 形态（`PilotFn = fn(&mut Tx)`）给不了它工作目录——`Tx::origin()` 是**标签**不是路径，
/// 公开方法里也没有任何文件系统路径。因此本模块提供**另一个形态**：由驱动在
/// **旧批次循环的同一位置**调用它，workdir 由驱动传入（驱动本来就有）。
///
/// **边界（如实说明）**：这里**没有**把 `GuiSurgeon` 的 1100+ 行本地化到 `Tx`，
/// 只是把「调用点」从旧适配层搬到了原生模块。代价是它仍直接读写磁盘；
/// 收益是 `cut_gui` 的**计划槽位先原生化**（计入原生任务），且**接口不变**——
/// 将来把本函数内部换成逐函数移植（§9.96 的替换表）即可，无需再改驱动。
///
/// **分辨率**：旧 `cut_gui` 调 `detect_resolution` 后**忽略其返回值**（`GuiSurgeon` 不读它），
/// 因此探测结果对产物无影响；此处照样探测，保持与旧实现同样的日志与副作用。
pub mod surgeon_cut_gui {
    use super::*;
    use std::path::Path;

    /// `decl()` 目前**没有调用者**——`cut_gui` 由调度器在旧批次内执行（闭包体已是本模块的
    /// `run_in_workdir`），不经过 A-ROM 派发表，故声明暂时用不上；保留它是为了将来真正
    /// 本地化到 `Tx` 形态时可直接接入（那时它就有调用者了）。
    #[allow(dead_code)]
    pub fn decl() -> TaskDecl {
        // 阶段与活注册表一致：`invoke_conversion.rs` 把 `cut_gui` 登记为
        // `TaskType::Hybrid` / `TaskTier::Surgeon`。
        // 范围覆盖 gui 子树（读 `container/*.png`、写 `sprites/**`）。
        TaskDecl::new("cut_gui", Tier::Surgeon)
            .reads(ScopeSet::prefix("assets/minecraft/textures/gui"))
            .writes(ScopeSet::prefix("assets/minecraft/textures/gui"))
            .exclusive(true)
    }

    /// **workdir 形态入口**（§9.113）：内部走 **`Tx`**，即 `gui_surgeon_tx::run`。
    ///
    /// **它做什么**：把 workdir 的 `gui/` 子树读成一份内存包（`MemSource`），
    /// 在其上跑纯 `Tx` 形态的 `gui_surgeon_tx::run`，再把产出的层**写回 workdir**，
    /// 最后把 `Outcome.deferred_removals` 登记到**调用方的 `ctx`** 上——
    /// 这样收尾的 `execute_cleanup()` 仍在**同一时机**删除它们（与旧实现一致）。
    ///
    /// **为什么只读 `gui/` 子树而不是整包**：`GuiSurgeon` 的读写**全部**在
    /// `assets/minecraft/textures/gui/` 之下（源图与 `sprites/**` 产物），
    /// 清理清单的 20 个文件也都在其内。只读这一棵子树既够用，又避免把整个包（几千个文件）
    /// 读进内存。这是**有界且已知**的输入面，不是"碰巧够用"。
    ///
    /// **为什么需要这层桥**：`cut_gui` 的计划槽位在**旧批次内部**（`(15,18)`），
    /// 而 `Tx` 形态只在批次之外可用（§9.100 实测：挪出批次就少 3 个 sprite）。
    /// 因此这里的做法是**位置不变、实现换成 `Tx`**——那层"建内存包 → 应用回 workdir"
    /// 的往返，正是调用方本来就有的 workdir 形态所要求的，不是新增的架构。
    pub fn run_in_workdir(ctx: &crate::hurray::context::HurrayContext, workdir: &Path) -> Result<(), String> {
        crate::log_info!("2-Pyramid: starting cut_gui (native Tx pipeline)...");

        // ① 读 workdir 的 gui 子树 → 内存包
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        collect_gui_files(workdir, GUI_PREFIX, &mut files)?;
        let source = crate::arom::MemSource::new(files).map_err(|e| e.to_string())?;
        let mut pack = crate::arom::Pack::from_source(Box::new(source), None)
            .map_err(|e| e.to_string())?;

        // ② 跑纯 `Tx` 形态，取出层
        let (outcome, layer) = {
            let mut tx = pack.tx("cut_gui");
            let o = super::gui_surgeon_tx::run(&mut tx).map_err(|e| e.to_string())?;
            (o, tx.into_layer())
        };

        // ③ 层写回 workdir（`sprites/**` 产物 + `gui/title/minecraft.png` 的就地回写）
        write_layer_to_dir(workdir, &pack, &layer)?;
        pack.commit(layer);

        // ④ 延迟删除登记到**调用方的 ctx**（收尾 `execute_cleanup()` 同一时机生效）
        for rel in &outcome.deferred_removals {
            ctx.defer_remove_file(&workdir.join(rel));
        }

        crate::log_info!(
            "cut_gui: wrote {} entries, deferred {} removals",
            outcome.changed,
            outcome.deferred_removals.len()
        );
        Ok(())
    }

    /// `GuiSurgeon` 的读写**全部**落在这个前缀之下（源图 + `sprites/**` 产物 + 清理清单）。
    const GUI_PREFIX: &str = "assets/minecraft/textures/gui";

    /// 递归收集 `workdir/<rel_prefix>/` 下的所有文件，路径为**包内相对**。
    fn collect_gui_files(
        workdir: &Path,
        rel_prefix: &str,
        out: &mut Vec<(String, Vec<u8>)>,
    ) -> Result<(), String> {
        let dir = workdir.join(rel_prefix);
        let Ok(rd) = std::fs::read_dir(&dir) else {
            return Ok(()); // 目录不存在 = 没有可读输入（GuiSurgeon 会各自跳过）
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            let rel = format!("{rel_prefix}/{name}");
            if p.is_dir() {
                collect_gui_files(workdir, &rel, out)?;
            } else if let Ok(bytes) = std::fs::read(&p) {
                out.push((rel, bytes));
            }
        }
        Ok(())
    }

    /// 把层里的写入落到 `workdir` 上（`GuiSurgeon` 不产生改名规则，故只处理写入）。
    fn write_layer_to_dir(workdir: &Path, pack: &crate::arom::Pack, layer: &crate::arom::Layer) -> Result<(), String> {
        for (path, slot) in layer.writes() {
            let full = workdir.join(path);
            match slot {
                crate::arom::Slot::Tombstone => {
                    if full.is_dir() {
                        std::fs::remove_dir_all(&full).map_err(|e| e.to_string())?;
                    } else if full.exists() {
                        std::fs::remove_file(&full).map_err(|e| e.to_string())?;
                    }
                }
                crate::arom::Slot::Present(crate::arom::Body::Dir) => {
                    std::fs::create_dir_all(&full).map_err(|e| e.to_string())?;
                }
                crate::arom::Slot::Present(body) => {
                    if let Some(parent) = full.parent() {
                        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    }
                    let bytes = pack.read_body(body).map_err(|e| e.to_string())?;
                    std::fs::write(&full, &bytes).map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }
}
/// **`GuiSurgeon` 的 `Tx` 本地化（阶段 1：`SPRITE_MAP` 主循环）**（§9.105）。
///
/// 背景（§9.96/§9.104）：`GuiSurgeon` 是 M3 依赖链的**链首**——它是唯一「必须直接读盘」的转换器，
/// 只要它还在读磁盘，`temp_dir` / 旧闭包路径 / `Foray Rom` 三项都无法动。
/// 本模块按 §9.96 的替换表把它的**主循环**（产出 Late 那 62 项 sprite 的那一段）
/// 从「`&Path` + `TexturePool`」改写成「`Tx` + `PackView`」，**不动其余 7 个 `process_*`**。
///
/// **机械替换**（与 §9.96 的表一致）：
/// | 旧 | 新 |
/// |---|---|
/// | `base_path.join(x)` | `x`（`x` 本就是包内相对路径） |
/// | `pool.load_texture(&p)`（失败静默跳过） | `tx.image(x).ok()`（`PackView::image` 缺失即 `Err`，`.ok()` 等价） |
/// | `pool.store_texture(&p, img)` | `tx.put_image(x, &img)` |
/// | `pool.commit_all()` | 不需要（Tx 写层） |
///
/// **本阶段刻意不做**：7 个 `process_*`（`slider`/`icons`/`widgets`/`tabs`/`resource_packs`/
/// `server_selection`/`title`）与 20 个 atlas 文件的**延迟删除**。因此本模块**暂不派发**——
/// 它只用于夹具对照，证明主循环的原生实现与旧实现逐像素一致。
pub mod gui_surgeon_tx {
    use super::*;
    use image::imageops;

    /// 一条 sprite 的切图定义（与 `converters/ui/gui_surgeon.rs::GuiSpriteDef` 字段一致）。
    struct SpriteDef {
        source_name: &'static str,
        target_path: &'static str,
        rect: (u32, u32, u32, u32),
        base_width: u32,
    }

    /// 与旧 `SPRITE_MAP` **逐条一致**（§9.97 记过：一个字节错就会改变产物）。
    /// 顺序即语义（同名 target 会被后者覆盖），故照抄原顺序。
    const SPRITE_MAP: &[SpriteDef] = &[
        SpriteDef { source_name: "anvil", target_path: "container/anvil/error", rect: (176, 0, 204, 21), base_width: 256 },
        SpriteDef { source_name: "anvil", target_path: "container/anvil/text_field", rect: (0, 166, 110, 182), base_width: 256 },
        SpriteDef { source_name: "anvil", target_path: "container/anvil/text_field_disabled", rect: (0, 182, 110, 198), base_width: 256 },
        SpriteDef { source_name: "beacon", target_path: "container/beacon/button", rect: (0, 219, 22, 241), base_width: 256 },
        SpriteDef { source_name: "beacon", target_path: "container/beacon/button_selected", rect: (22, 219, 44, 241), base_width: 256 },
        SpriteDef { source_name: "beacon", target_path: "container/beacon/button_disabled", rect: (44, 219, 66, 241), base_width: 256 },
        SpriteDef { source_name: "beacon", target_path: "container/beacon/button_highlighted", rect: (66, 219, 88, 241), base_width: 256 },
        SpriteDef { source_name: "beacon", target_path: "container/beacon/confirm", rect: (90, 220, 108, 238), base_width: 256 },
        SpriteDef { source_name: "beacon", target_path: "container/beacon/cancel", rect: (112, 220, 130, 238), base_width: 256 },
        SpriteDef { source_name: "furnace", target_path: "container/furnace/lit_progress", rect: (176, 0, 190, 14), base_width: 256 },
        SpriteDef { source_name: "furnace", target_path: "container/furnace/burn_progress", rect: (176, 14, 200, 31), base_width: 256 },
        SpriteDef { source_name: "blast_furnace", target_path: "container/blast_furnace/lit_progress", rect: (176, 0, 190, 14), base_width: 256 },
        SpriteDef { source_name: "blast_furnace", target_path: "container/blast_furnace/burn_progress", rect: (176, 14, 200, 31), base_width: 256 },
        SpriteDef { source_name: "smoker", target_path: "container/smoker/lit_progress", rect: (176, 0, 190, 14), base_width: 256 },
        SpriteDef { source_name: "smoker", target_path: "container/smoker/burn_progress", rect: (176, 14, 200, 31), base_width: 256 },
        SpriteDef { source_name: "brewing_stand", target_path: "container/brewing_stand/brew_progress", rect: (176, 0, 185, 28), base_width: 256 },
        SpriteDef { source_name: "brewing_stand", target_path: "container/brewing_stand/bubbles", rect: (185, 14, 197, 29), base_width: 256 },
        SpriteDef { source_name: "brewing_stand", target_path: "container/brewing_stand/fuel_length", rect: (176, 29, 194, 33), base_width: 256 },
        SpriteDef { source_name: "inventory", target_path: "container/inventory/effect_background_large", rect: (0, 166, 120, 198), base_width: 256 },
        SpriteDef { source_name: "inventory", target_path: "container/inventory/effect_background_small", rect: (0, 198, 32, 230), base_width: 256 },
        SpriteDef { source_name: "horse", target_path: "container/horse/armor_slot", rect: (0, 220, 18, 238), base_width: 256 },
        SpriteDef { source_name: "horse", target_path: "container/horse/saddle_slot", rect: (18, 220, 36, 238), base_width: 256 },
        SpriteDef { source_name: "horse", target_path: "container/horse/llama_armor_slot", rect: (36, 220, 54, 238), base_width: 256 },
        SpriteDef { source_name: "horse", target_path: "container/horse/chest_slots", rect: (0, 166, 90, 220), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/enchantment_slot", rect: (0, 166, 108, 185), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/enchantment_slot_disabled", rect: (0, 185, 108, 204), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/enchantment_slot_highlighted", rect: (0, 204, 108, 223), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_1", rect: (0, 223, 16, 239), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_2", rect: (16, 223, 32, 239), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_3", rect: (32, 223, 48, 239), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_1_disabled", rect: (0, 239, 16, 255), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_2_disabled", rect: (16, 239, 32, 255), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_3_disabled", rect: (32, 239, 48, 255), base_width: 256 },
        SpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/recipe", rect: (0, 166, 16, 184), base_width: 256 },
        SpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/recipe_selected", rect: (0, 184, 16, 202), base_width: 256 },
        SpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/recipe_highlighted", rect: (0, 202, 16, 220), base_width: 256 },
        SpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/scroller", rect: (176, 0, 188, 15), base_width: 256 },
        SpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/scroller_disabled", rect: (188, 0, 200, 15), base_width: 256 },
        SpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/input_slot", rect: (176, 0, 192, 16), base_width: 256 },
        SpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/output_slot", rect: (192, 0, 208, 16), base_width: 256 },
        SpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/result_slot", rect: (192, 0, 208, 16), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/banner_slot", rect: (176, 0, 192, 16), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/dye_slot", rect: (192, 0, 208, 16), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/pattern_slot", rect: (208, 0, 224, 16), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/pattern", rect: (0, 166, 14, 180), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/pattern_selceted", rect: (0, 180, 14, 194), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/pattern_selected", rect: (0, 180, 14, 194), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/pattern_highlighted", rect: (0, 194, 14, 208), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/scroller", rect: (232, 0, 244, 15), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/scroller_disabled", rect: (244, 0, 256, 15), base_width: 256 },
        SpriteDef { source_name: "smithing", target_path: "container/smithing/error", rect: (176, 0, 204, 21), base_width: 256 },
        SpriteDef { source_name: "smithing", target_path: "container/smithing/template_slot", rect: (16, 0, 32, 16), base_width: 256 },
        SpriteDef { source_name: "smithing", target_path: "container/smithing/base_slot", rect: (32, 0, 48, 16), base_width: 256 },
        SpriteDef { source_name: "smithing", target_path: "container/smithing/addition_slot", rect: (16, 16, 32, 32), base_width: 256 },
        SpriteDef { source_name: "smithing", target_path: "container/smithing/result_slot", rect: (32, 16, 48, 32), base_width: 256 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/discount_strikethrough", rect: (0, 176, 9, 178), base_width: 512 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/experience_bar_result", rect: (0, 181, 102, 186), base_width: 512 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/experience_bar_background", rect: (0, 186, 102, 191), base_width: 512 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/experience_bar_current", rect: (0, 191, 102, 196), base_width: 512 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/trade_arrow", rect: (15, 171, 25, 180), base_width: 512 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/out_of_stuck", rect: (25, 171, 35, 180), base_width: 512 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/out_of_stock", rect: (25, 171, 35, 180), base_width: 512 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/scroller", rect: (0, 199, 6, 226), base_width: 512 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/scroller_disabled", rect: (6, 199, 12, 226), base_width: 512 },
        SpriteDef { source_name: "cartography_table", target_path: "container/cartography_table/duplicated_map", rect: (176, 132, 226, 198), base_width: 256 },
        SpriteDef { source_name: "cartography_table", target_path: "container/cartography_table/scaled_map", rect: (176, 66, 242, 132), base_width: 256 },
        SpriteDef { source_name: "cartography_table", target_path: "container/cartography_table/map", rect: (176, 0, 242, 66), base_width: 256 },
        SpriteDef { source_name: "cartography_table", target_path: "container/cartography_table/locked", rect: (52, 214, 62, 228), base_width: 256 },
        SpriteDef { source_name: "cartography_table", target_path: "container/cartography_table/error", rect: (226, 132, 254, 153), base_width: 256 },
        SpriteDef { source_name: "grindstone", target_path: "container/grindstone/error", rect: (176, 0, 204, 21), base_width: 256 },
        SpriteDef { source_name: "grindstone", target_path: "container/grindstone/input_slot", rect: (30, 53, 48, 71), base_width: 256 },
        SpriteDef { source_name: "grindstone", target_path: "container/grindstone/additional_slot", rect: (66, 53, 84, 71), base_width: 256 },
        SpriteDef { source_name: "grindstone", target_path: "container/grindstone/output_slot", rect: (66, 53, 84, 71), base_width: 256 },
        SpriteDef { source_name: "grindstone", target_path: "container/grindstone/result_slot", rect: (66, 53, 84, 71), base_width: 256 },
    ];

    const CONTAINER: &str = "assets/minecraft/textures/gui/container";
    const SPRITES: &str = "assets/minecraft/textures/gui/sprites";

    /// 旧 `scale_from_image_base`。
    fn scale_from_image_base(img: &RgbaImage, base_width: u32) -> f32 {
        let width = img.width().max(1);
        let base = base_width.max(1);
        width as f32 / base as f32
    }

    /// **裁剪守卫**（§9.122）：`crop_imm` 越界时返回**空图**，而 PNG 编码器拒绝 0 宽/0 高。
    /// 旧实现里这个错误发生在并行 `commit_all` 内部并被吞掉；`Tx` 形态若直接 `?`
    /// 会把**整次转换**打断。返回 `None` 表示"这次裁剪无意义，跳过"，与旧实现的净效果一致。
    fn crop_checked(
        img: &RgbaImage,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        what: &str,
    ) -> Option<RgbaImage> {
        if w == 0 || h == 0 || x >= img.width() || y >= img.height() {
            crate::log_warn!(
                "{}: 裁剪越界（源 {}x{}，rect {x},{y},{w},{h}）——跳过",
                what, img.width(), img.height()
            );
            return None;
        }
        Some(imageops::crop_imm(img, x, y, w, h).to_image())
    }
    /// 旧 `scale_coordinate`。
    fn scale_coordinate(scale: f32, coord: u32) -> u32 {
        (coord as f32 * scale).round() as u32
    }

    /// 旧 `scale_rect`（注意第 3/4 个返回值是**宽高**，不是右/下坐标）。
    fn scale_rect(scale: f32, x1: u32, y1: u32, x2: u32, y2: u32) -> (u32, u32, u32, u32) {
        (
            scale_coordinate(scale, x1),
            scale_coordinate(scale, y1),
            scale_coordinate(scale, x2 - x1),
            scale_coordinate(scale, y2 - y1),
        )
    }

    /// **主循环**（旧 `execute_transformation` 的第一个 `for def in SPRITE_MAP` 段）。
    ///
    /// 语义与旧实现逐条对应：源图缺失**静默跳过**（`tx.image(..).ok()`）；
    /// 缩放按 `def.base_width`；裁剪后写 `sprites/<target_path>.png`。
    pub fn cut_sprite_map(tx: &mut Tx<'_>) -> Result<usize, AromError> {
        let mut written = 0usize;
        for def in SPRITE_MAP {
            let src = format!("{CONTAINER}/{}.png", def.source_name);
            let Ok(img) = tx.image(&src) else {
                continue; // 与旧实现一致：源图缺失 → 跳过（旧侧是 load_texture 返回 Err）
            };
            let (x1, y1, x2, y2) = def.rect;
            let scale = scale_from_image_base(&img, def.base_width);
            let (rx1, ry1, rw, rh) = scale_rect(scale, x1, y1, x2, y2);
            // `tx.image` 返回 `Arc<RgbaImage>`；`crop_imm` 要 `&RgbaImage`，故解引用（不克隆整图）。
            let sprite = imageops::crop_imm(&*img, rx1, ry1, rw, rh).to_image();
            let target = format!("{SPRITES}/{}.png", def.target_path);
            tx.put_image(&target, &sprite)?;
            written += 1;
        }
        Ok(written)
    }

    /// 旧 `SplitMode`（`save_slices` 的切分方式）。
    #[derive(Clone, Copy)]
    enum SplitMode {
        None,
        Horizontal,
        Vertical,
    }

    /// 旧 `GuiSurgeon::sprite_dir`。
    fn sprite_dir(subdir: &str) -> String {
        format!("{SPRITES}/{subdir}")
    }

    /// 旧 `save_slices`：裁一块 → 按 `split` 切成 `names.len()` 片 → 逐片写 `sprites/<target_dir>/<name>`。
    ///
    /// 与旧实现的对应关系：`base_path` 参数**删除**（`x` 直接是包内相对路径）；
    /// `pool` 换成 `tx`；旧签名里的 `_res` 本来就**未被使用**（旧代码就带下划线），故不保留。
    /// `names` 里的元素**自带 `.png` 后缀**（与旧表一致）。
    fn save_slices(
        tx: &mut Tx<'_>,
        img: &RgbaImage,
        crop: (u32, u32, u32, u32),
        split: SplitMode,
        slice_size: (u32, u32),
        names: &[&str],
        target_dir: &str,
    ) -> Result<usize, AromError> {
        let scale = scale_from_image_base(img, 256);
        let (x1, y1, x2, y2) = crop;
        let (rx, ry, rw, rh) = scale_rect(scale, x1, y1, x2, y2);
        // **零尺寸守卫**（§9.122）：`crop_imm` 在越界时返回**空图**，而 PNG 编码器
        // **拒绝 0 宽/0 高**。旧实现里这个错误发生在并行的 `commit_all` 内部并被吞掉
        // （转换照常完成，只是少一个 sprite）；`Tx` 形态若直接 `?` 会把**整次转换**打断。
        // 因此这里显式跳过并告警——与旧实现的净效果一致，且不再静默。
        if rw == 0 || rh == 0 {
            crate::log_warn!(
                "save_slices: 裁剪越界（源 {}x{}，scale {:.3}，rect {rx},{ry},{rw},{rh}）——跳过",
                img.width(), img.height(), scale
            );
            return Ok(0);
        }
        let cropped = imageops::crop_imm(img, rx, ry, rw, rh).to_image();

        let slice_w = scale_coordinate(scale, slice_size.0);
        let slice_h = scale_coordinate(scale, slice_size.1);

        let mut slices: Vec<RgbaImage> = Vec::new();
        match split {
            SplitMode::None => slices.push(cropped),
            SplitMode::Horizontal => {
                for i in 0..names.len() {
                    let sx = i as u32 * slice_w;
                    slices.push(imageops::crop_imm(&cropped, sx, 0, slice_w, slice_h).to_image());
                }
            }
            SplitMode::Vertical => {
                for i in 0..names.len() {
                    let sy = i as u32 * slice_h;
                    slices.push(imageops::crop_imm(&cropped, 0, sy, slice_w, slice_h).to_image());
                }
            }
        }

        let root = sprite_dir(target_dir);
        let mut written = 0usize;
        for (idx, name) in names.iter().enumerate() {
            if let Some(slice) = slices.get(idx) {
                tx.put_image(&format!("{root}/{name}"), slice)?;
                written += 1;
            }
        }
        Ok(written)
    }

    /// 旧 `process_resource_packs`。源图缺失 → 静默跳过（旧侧先 `exists()` 再 `load_texture`）。
    pub fn process_resource_packs(tx: &mut Tx<'_>) -> Result<usize, AromError> {
        let src = "assets/minecraft/textures/gui/resource_packs.png";
        let Ok(img) = tx.image(src) else {
            crate::log_info!("resource_packs.png not found, skip");
            return Ok(0);
        };
        let mut n = 0usize;
        n += save_slices(
            tx, &*img, (0, 0, 128, 32), SplitMode::Horizontal, (32, 32),
            &["select.png", "unselect.png", "move_down.png", "move_up.png"],
            "transferable_list",
        )?;
        n += save_slices(
            tx, &*img, (0, 32, 128, 64), SplitMode::Horizontal, (32, 32),
            &["select_highlighted.png", "unselect_highlighted.png",
              "move_down_highlighted.png", "move_up_highlighted.png"],
            "transferable_list",
        )?;
        Ok(n)
    }

    /// 旧 `process_server_selection`。源图缺失 → 静默跳过。
    pub fn process_server_selection(tx: &mut Tx<'_>) -> Result<usize, AromError> {
        let src = "assets/minecraft/textures/gui/server_selection.png";
        let Ok(img) = tx.image(src) else {
            crate::log_info!("server_selection.png not found, skip");
            return Ok(0);
        };
        let mut n = 0usize;
        n += save_slices(
            tx, &*img, (0, 0, 128, 32), SplitMode::Horizontal, (32, 32),
            &["join.png", "emm.png", "move_down.png", "move_up.png"],
            "server_list",
        )?;
        n += save_slices(
            tx, &*img, (0, 32, 128, 64), SplitMode::Horizontal, (32, 32),
            &["join_highlighted.png", "emmm.png",
              "move_down_highlighted.png", "move_up_highlighted.png"],
            "server_list",
        )?;
        Ok(n)
    }
    /// 旧 `process_slider`：由 `gui/slider.png`（vanilla 基准 200 宽）产出 3 个 widget sprite。
    ///
    /// 注意 `scale_from_image`（基准 **256**）与 slider 自身的 200 宽基准**不是一回事**——
    /// 与旧实现一致，照抄。
    pub fn process_slider(tx: &mut Tx<'_>) -> Result<usize, AromError> {
        let src = "assets/minecraft/textures/gui/slider.png";
        let Ok(img) = tx.image(src) else {
            crate::log_info!("slider.png not found, skip");
            return Ok(0);
        };
        let scale = scale_from_image_base(&img, 256);
        let mut n = 0usize;

        // 滑块主体
        let (x1, y1, w, h) = scale_rect(scale, 0, 0, 200, 20);
        let Some(slider) = crop_checked(&img, x1, y1, w, h, "slider") else { return Ok(0) };
        tx.put_image("assets/minecraft/textures/gui/sprites/widget/slider.png", &slider)?;
        n += 1;

        // 手柄 = 左片(0,40)-(4,60) 与 右片(196,40)-(200,60) 横向拼接
        let (lx, ly, lw, lh) = scale_rect(scale, 0, 40, 4, 60);
        let (rx, ry, rw, rh) = scale_rect(scale, 196, 40, 200, 60);
        let (Some(left), Some(right)) = (crop_checked(&img, lx, ly, lw, lh, "slider_handle_l"), crop_checked(&img, rx, ry, rw, rh, "slider_handle_r")) else {
            return Ok(n);
        };
        let mut handle = RgbaImage::new(left.width() + right.width(), left.height());
        for y in 0..left.height() {
            for x in 0..left.width() {
                handle.put_pixel(x, y, *left.get_pixel(x, y));
            }
            for x in 0..right.width() {
                handle.put_pixel(x + left.width(), y, *right.get_pixel(x, y));
            }
        }
        tx.put_image(
            "assets/minecraft/textures/gui/sprites/widget/slider_handle.png",
            &handle,
        )?;
        n += 1;

        // 高亮手柄 = 同样的拼法，取 y=60..80 那一行
        let (hx, hy, hw, hh) = scale_rect(scale, 0, 60, 4, 80);
        let (hrx, hry, hrw, hrh) = scale_rect(scale, 196, 60, 200, 80);
        let (Some(hl), Some(hr)) = (crop_checked(&img, hx, hy, hw, hh, "slider_hl_l"), crop_checked(&img, hrx, hry, hrw, hrh, "slider_hl_r")) else {
            return Ok(n);
        };
        let mut h_handle = RgbaImage::new(hl.width() + hr.width(), hl.height());
        for y in 0..hl.height() {
            for x in 0..hl.width() {
                h_handle.put_pixel(x, y, *hl.get_pixel(x, y));
            }
            for x in 0..hr.width() {
                h_handle.put_pixel(x + hl.width(), y, *hr.get_pixel(x, y));
            }
        }
        tx.put_image(
            "assets/minecraft/textures/gui/sprites/widget/slider_handle_highlighted.png",
            &h_handle,
        )?;
        n += 1;

        Ok(n)
    }

    /// 旧 `process_title`：由 `gui/title/minecraft.png` 产出 `sprites/title/{realms,minecraft}.png`，
    /// **并且把拼接结果回写到源文件 `gui/title/minecraft.png` 本身**。
    ///
    /// 那处"回写源文件"是旧实现的**刻意行为**（1.21 的 title 用法变了），照抄——
    /// 在 `Tx` 形态下它表现为"同一路径既读又写"，层会以最后一次写为准。
    pub fn process_title(tx: &mut Tx<'_>) -> Result<usize, AromError> {
        let src = "assets/minecraft/textures/gui/title/minecraft.png";
        let Ok(img) = tx.image(src) else {
            crate::log_info!("title/minecraft.png not found, skip");
            return Ok(0);
        };
        let scale = scale_from_image_base(&img, 256);
        let mut n = 0usize;

        // realms：直接裁一块
        let (rx, ry, rw, rh) = scale_rect(scale, 0, 94, 200, 194);
        let Some(realms) = crop_checked(&img, rx, ry, rw, rh, "title/realms") else { return Ok(0) };
        tx.put_image("assets/minecraft/textures/gui/sprites/title/realms.png", &realms)?;
        n += 1;

        // 两片横向拼接，再补一段透明底
        let (x1, y1, w1, h1) = scale_rect(scale, 0, 0, 155, 44);
        let Some(part1) = crop_checked(&img, x1, y1, w1, h1, "title/part1") else { return Ok(n) };
        let (x2, y2, w2, h2) = scale_rect(scale, 0, 45, 119, 89);
        let Some(part2) = crop_checked(&img, x2, y2, w2, h2, "title/part2") else { return Ok(n) };

        let concat_w = part1.width() + part2.width();
        let concat_h = part1.height().max(part2.height());
        let mut concatenated = RgbaImage::new(concat_w, concat_h);
        imageops::overlay(&mut concatenated, &part1, 0, 0);
        imageops::overlay(&mut concatenated, &part2, part1.width() as i64, 0);

        let tw = scale_coordinate(scale, 274);
        let th = scale_coordinate(scale, 25);
        let final_w = tw.max(concatenated.width());
        let final_h = concatenated.height() + th;
        let mut final_img = RgbaImage::new(final_w, final_h);
        imageops::overlay(&mut final_img, &concatenated, 0, 0);

        tx.put_image("assets/minecraft/textures/gui/sprites/title/minecraft.png", &final_img)?;
        n += 1;
        // 旧实现：把同一张图**回写源文件**
        tx.put_image(src, &final_img)?;
        n += 1;

        Ok(n)
    }
    /// 旧 `equalize_nine_slice_frame`：把 sprite 的上下左右边框统一为 `border` 像素，
    /// 避免 1.21 九宫格拆开时某侧变薄/缺边导致 UI 错位。
    ///
    /// 逐条照抄：①中心原样；②上下边条用**最外 1 行**沿边方向拉伸；③左右边条用**最外 1 列**拉伸；
    /// ④四角用原图角像素块（保留立体倒角）。`border` 被 `clamp(1, min(w,h)/2)`。
    fn equalize_nine_slice_frame(img: &RgbaImage, border: u32) -> RgbaImage {
        let (w, h) = img.dimensions();
        let b = border.clamp(1, w.min(h) / 2);
        if w < 2 || h < 2 {
            return img.clone();
        }
        let mut out = RgbaImage::new(w, h);

        // 1) 中心
        for y in b..(h - b) {
            for x in b..(w - b) {
                out.put_pixel(x, y, *img.get_pixel(x, y));
            }
        }
        // 2) 上下边条
        for x in 0..w {
            let top_src = *img.get_pixel(x.min(w - 1), 0);
            let bot_src = *img.get_pixel(x.min(w - 1), h - 1);
            for t in 0..b {
                out.put_pixel(x, t, top_src);
                out.put_pixel(x, h - 1 - t, bot_src);
            }
        }
        // 3) 左右边条
        for y in 0..h {
            let left_src = *img.get_pixel(0, y.min(h - 1));
            let right_src = *img.get_pixel(w - 1, y.min(h - 1));
            for t in 0..b {
                out.put_pixel(t, y, left_src);
                out.put_pixel(w - 1 - t, y, right_src);
            }
        }
        // 4) 四角
        for ty in 0..b {
            for tx in 0..b {
                let src_tx = tx.min(b - 1);
                let src_ty = ty.min(b - 1);
                out.put_pixel(tx, ty, *img.get_pixel(src_tx, src_ty));
                out.put_pixel(w - 1 - tx, ty, *img.get_pixel(w - 1 - src_tx, src_ty));
                out.put_pixel(tx, h - 1 - ty, *img.get_pixel(src_tx, h - 1 - src_ty));
                out.put_pixel(w - 1 - tx, h - 1 - ty, *img.get_pixel(w - 1 - src_tx, h - 1 - src_ty));
            }
        }
        out
    }

    /// 旧 `process_widgets`：由 `gui/widgets.png` 产出 HUD / icon / widget 三组 sprite。
    ///
    /// **照抄未改的两处**：
    /// 1. `language.png` 被**写两次**（旧代码里是两段完全相同的 `save_slices`）—— 幂等，但照抄；
    /// 2. 按钮族用 `equalize_nine_slice_frame` 统一边厚，`border = (2*scale).round().max(2)`。
    pub fn process_widgets(tx: &mut Tx<'_>) -> Result<usize, AromError> {
        let src = "assets/minecraft/textures/gui/widgets.png";
        let Ok(img) = tx.image(src) else {
            crate::log_info!("widgets.png not found, skip");
            return Ok(0);
        };
        let mut n = 0usize;

        n += save_slices(tx, &*img, (0, 0, 182, 22), SplitMode::None, (182, 22), &["hotbar.png"], "hud")?;
        n += save_slices(tx, &*img, (0, 22, 24, 45), SplitMode::None, (24, 23), &["hotbar_selection.png"], "hud")?;
        n += save_slices(tx, &*img, (24, 22, 53, 46), SplitMode::None, (29, 24), &["hotbar_offhand_left.png"], "hud")?;
        n += save_slices(tx, &*img, (53, 22, 82, 46), SplitMode::None, (29, 24), &["hotbar_offhand_right.png"], "hud")?;

        // 按钮族：切出后统一 `2*scale` 边厚
        {
            let bscale = scale_from_image_base(&img, 256);
            let border = ((2.0 * bscale).round() as u32).max(2);
            for (crop_y, name) in [
                (46u32, "button_disabled.png"),
                (66, "button.png"),
                (86, "button_highlighted.png"),
            ] {
                let (x1, y1, w, h) = scale_rect(bscale, 0, crop_y, 200, crop_y + 20);
                let Some(raw) = crop_checked(&img, x1, y1, w, h, "widgets/button") else { continue };
                let fixed = equalize_nine_slice_frame(&raw, border);
                tx.put_image(&format!("{SPRITES}/widget/{name}"), &fixed)?;
                n += 1;
            }
        }

        // language.png：旧实现写两次（两段相同调用），照抄
        n += save_slices(tx, &*img, (3, 109, 18, 124), SplitMode::None, (15, 15), &["language.png"], "icon")?;
        n += save_slices(tx, &*img, (3, 109, 18, 124), SplitMode::None, (15, 15), &["language.png"], "icon")?;

        // locked / unlocked 按钮族：竖直切成 3 片，各做边厚统一
        {
            let bscale = scale_from_image_base(&img, 256);
            let border = ((2.0 * bscale).round() as u32).max(2);
            for (x1, x2, names) in [
                (0u32, 20u32, ["locked_button.png", "locked_button_highlighted.png", "locked_button_disabled.png"]),
                (20, 40, ["unlocked_button.png", "unlocked_button_highlighted.png", "unlocked_button_disabled.png"]),
            ] {
                let (rx1, ry1, rw, rh) = scale_rect(bscale, x1, 146, x2, 206);
                let Some(strip) = crop_checked(&img, rx1, ry1, rw, rh, "widgets/strip") else { continue };
                let slice_h = scale_coordinate(bscale, 20);
                let slice_w = scale_coordinate(bscale, 20);
                for (i, name) in names.iter().enumerate() {
                    let sy = i as u32 * slice_h;
                    let Some(raw) = crop_checked(&strip, 0, sy, slice_w, slice_h, "widgets/locked") else { continue };
                    let fixed = equalize_nine_slice_frame(&raw, border);
                    tx.put_image(&format!("{SPRITES}/widget/{name}"), &fixed)?;
                    n += 1;
                }
            }
        }

        Ok(n)
    }
    /// 旧 `process_tabs`：由 `gui/container/creative_inventory/tabs.png` 产出 4 行 × 7 个 tab sprite。
    ///
    /// **照抄未改的三处语义**：
    /// 1. **只接受 256/512/1024/2048 的精确方形**，其它尺寸整体跳过（`(w.max(h), w == h)` 匹配）；
    /// 2. **始终裁 168 宽（6 个 tab），第 7 个复制第 6 个**——旧注释明确写了不去探测 168–196，
    ///    因为 1.8 图集该区域可能有残留像素会导致误裁；
    /// 3. 每片的 `slice_height == ch`（即"切片高"与"行高"同值），故竖直方向上正好切满。
    pub fn process_tabs(tx: &mut Tx<'_>) -> Result<usize, AromError> {
        let src = "assets/minecraft/textures/gui/container/creative_inventory/tabs.png";
        let Ok(img) = tx.image(src) else {
            crate::log_info!("tabs.png not found, skip");
            return Ok(0);
        };

        let (w, h) = img.dimensions();
        let scale = match (w.max(h), w == h) {
            (256, true) => 1u32,
            (512, true) => 2,
            (1024, true) => 4,
            (2048, true) => 8,
            _ => {
                crate::log_info!("unsupported tabs.png size {}x{}, skip", w, h);
                return Ok(0);
            }
        };

        const OUT: &str = "assets/minecraft/textures/gui/sprites/container/creative_inventory";
        let mut n = 0usize;

        // 一行 = 裁 (0, cy*scale) 起 168*scale × ch*scale，再横切 6 片，最后复制第 6 片为第 7 片。
        let mut store_tabs =
            |tx: &mut Tx<'_>, cy: u32, ch: u32, names: [&str; 7]| -> Result<usize, AromError> {
                let cropped = imageops::crop_imm(
                    &*img,
                    0,
                    cy * scale,
                    168 * scale,
                    ch * scale,
                )
                .to_image();
                let slice_w = 28 * scale;
                let slice_h = ch * scale; // 旧的 slice_h 参数与 ch 同值，故此处等价

                let mut last: Option<RgbaImage> = None;
                let mut k = 0usize;
                for i in 0..6u32 {
                    let tab = imageops::crop_imm(&cropped, i * slice_w, 0, slice_w, slice_h).to_image();
                    tx.put_image(&format!("{OUT}/{}", names[i as usize]), &tab)?;
                    last = Some(tab);
                    k += 1;
                }
                if let Some(tab7) = last {
                    tx.put_image(&format!("{OUT}/{}", names[6]), &tab7)?;
                    k += 1;
                }
                Ok(k)
            };

        // y 区间与旧实现逐条一致
        n += store_tabs(tx, 2, 30, [
            "tab_top_unselected_1.png", "tab_top_unselected_2.png", "tab_top_unselected_3.png",
            "tab_top_unselected_4.png", "tab_top_unselected_5.png", "tab_top_unselected_6.png",
            "tab_top_unselected_7.png",
        ])?;
        n += store_tabs(tx, 32, 32, [
            "tab_top_selected_1.png", "tab_top_selected_2.png", "tab_top_selected_3.png",
            "tab_top_selected_4.png", "tab_top_selected_5.png", "tab_top_selected_6.png",
            "tab_top_selected_7.png",
        ])?;
        n += store_tabs(tx, 64, 30, [
            "tab_bottom_unselected_1.png", "tab_bottom_unselected_2.png", "tab_bottom_unselected_3.png",
            "tab_bottom_unselected_4.png", "tab_bottom_unselected_5.png", "tab_bottom_unselected_6.png",
            "tab_bottom_unselected_7.png",
        ])?;
        n += store_tabs(tx, 96, 32, [
            "tab_bottom_selected_1.png", "tab_bottom_selected_2.png", "tab_bottom_selected_3.png",
            "tab_bottom_selected_4.png", "tab_bottom_selected_5.png", "tab_bottom_selected_6.png",
            "tab_bottom_selected_7.png",
        ])?;

        Ok(n)
    }
    /// 旧 `process_icons`：由 `gui/icons.png` 产出 HUD / icon / server_list 三组 sprite。
    ///
    /// **本函数在旧实现里是纯声明式的**——17 次 `save_slices` 调用，没有任何直接的裁剪/写入，
    /// 因此移植是"逐条誊抄参数"，风险集中在**抄错数字或漏抄一条**。
    /// 夹具对照会逐文件比对，且数量断言能抓出"整段漏抄"。
    ///
    /// 注意 `names` 里有一批 `wtf*.png`：那是旧实现里**刻意保留**的槽位名（对应 1.20 图集中
    /// 1.21 已不用的小格），照抄——改名会改变产物。
    pub fn process_icons(tx: &mut Tx<'_>) -> Result<usize, AromError> {
        let src = "assets/minecraft/textures/gui/icons.png";
        let Ok(img) = tx.image(src) else {
            crate::log_info!("icons.png not found, skip");
            return Ok(0);
        };
        let img = &*img;
        let mut n = 0usize;

        n += save_slices(tx, img, (0, 0, 15, 15), SplitMode::None, (15, 15),
            &["crosshair.png"], "hud")?;

        n += save_slices(tx, img, (16, 0, 196, 9), SplitMode::Horizontal, (9, 9),
            &["container.png", "container_blinking.png", "wtf.png", "wtf2.png",
              "full.png", "half.png", "full_blinking.png", "half_blinking.png",
              "poisoned_full.png", "poisoned_half.png", "poisoned_full_blinking.png",
              "poisoned_half_blinking.png", "withered_full.png", "withered_half.png",
              "withered_full_blinking.png", "withered_half_blinking.png",
              "absorbing_full.png", "absorbing_half.png", "frozen_full.png",
              "frozen_half.png"], "hud/heart")?;

        n += save_slices(tx, img, (16, 9, 124, 18), SplitMode::Horizontal, (9, 9),
            &["armor_empty.png", "armor_half.png", "armor_full.png", "wtf3.png"], "hud")?;

        n += save_slices(tx, img, (52, 9, 124, 18), SplitMode::Horizontal, (9, 9),
            &["vehicle_container.png", "wtf4.png", "wtf5.png", "wtf6.png",
              "vehicle_full.png", "vehicle_half.png", "wtf7.png", "wtf8.png"], "hud/heart")?;

        n += save_slices(tx, img, (16, 18, 52, 27), SplitMode::Horizontal, (9, 9),
            &["air.png", "air_bursting.png", "wtf9.png", "wtf10.png"], "hud")?;

        n += save_slices(tx, img, (16, 27, 142, 36), SplitMode::Horizontal, (9, 9),
            &["food_empty.png", "wtf11.png", "wtf123.png", "wtf13.png", "food_full.png",
              "food_half.png", "wtf14.png", "wtf15.png", "food_full_hunger.png",
              "food_half_hunger.png", "wtf16.png", "wtf17.png", "wtf18.png",
              "food_empty_hunger.png"], "hud")?;

        n += save_slices(tx, img, (16, 45, 196, 54), SplitMode::Horizontal, (9, 9),
            &["container_hardcore.png", "container_hardcore_blinking.png", "wtf19.png",
              "wtf20.png", "hardcore_full.png", "hardcore_half.png",
              "hardcore_full_blinking.png", "hardcore_half_blinking.png",
              "poisoned_hardcore_full.png", "poisoned_hardcore_half.png",
              "poisoned_hardcore_full_blinking.png", "poisoned_hardcore_half_blinking.png",
              "withered_hardcore_full.png", "withered_hardcore_half.png",
              "withered_hardcore_full_blinking.png", "withered_hardcore_half_blinking.png",
              "absorbing_hardcore_full.png", "absorbing_hardcore_half.png",
              "frozen_hardcore_full.png", "frozen_hardcore_half.png"], "hud/heart")?;

        n += save_slices(tx, img, (0, 15, 10, 63), SplitMode::Vertical, (10, 8),
            &["ping_5.png", "ping_4.png", "ping_3.png", "ping_2.png", "ping_1.png",
              "ping_unknown.png"], "icon")?;

        n += save_slices(tx, img, (0, 64, 182, 94), SplitMode::Vertical, (182, 5),
            &["experience_bar_background.png", "experience_bar_progress.png",
              "jump_bar_cooldown.png", "wtf21.png", "jump_bar_background.png",
              "jump_bar_progress.png"], "hud")?;

        n += save_slices(tx, img, (0, 94, 18, 112), SplitMode::None, (18, 18),
            &["hotbar_attack_indicator_background.png"], "hud")?;
        n += save_slices(tx, img, (18, 94, 36, 112), SplitMode::None, (18, 18),
            &["hotbar_attack_indicator_progress.png"], "hud")?;
        n += save_slices(tx, img, (36, 94, 52, 98), SplitMode::None, (16, 4),
            &["crosshair_attack_indicator_background.png"], "hud")?;
        n += save_slices(tx, img, (52, 94, 68, 98), SplitMode::None, (16, 4),
            &["crosshair_attack_indicator_progress.png"], "hud")?;
        n += save_slices(tx, img, (68, 94, 84, 110), SplitMode::None, (16, 16),
            &["crosshair_attack_indicator_full.png"], "hud")?;

        n += save_slices(tx, img, (0, 176, 10, 224), SplitMode::Vertical, (10, 8),
            &["ping_5.png", "ping_4.png", "ping_3.png", "ping_2.png", "ping_1.png",
              "unreachable.png"], "server_list")?;
        n += save_slices(tx, img, (10, 176, 20, 216), SplitMode::Vertical, (10, 8),
            &["pinging_5.png", "pinging_4.png", "pinging_3.png", "pinging_2.png",
              "pinging_1.png"], "server_list")?;

        Ok(n)
    }
    /// **`GuiSurgeon` 的完整 `Tx` 形态入口**（§9.111）。
    ///
    /// 顺序与旧 `execute_transformation` **完全一致**：主循环 → 7 个 `process_*` → 登记延迟删除。
    /// （旧实现在 `commit_all()` 之后才登记删除，此处等价：`Tx` 形态下删除**必须延迟**，
    /// 早删会让更晚的任务看不到文件——§9.23 的坑。）
    ///
    /// **清理清单逐条照抄，尤其两点**：
    /// 1. **不含 `container/inventory.png`**——1.21 客户端仍需要它渲染生存/创造背包背景，
    ///    删掉会让背包 GUI 消失（旧注释专门写了这段，§9.96 也记过）；
    /// 2. 用 `tx.has_prefix` 判断"此刻是否存在"（对应旧实现的 `path.exists()`）。
    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();

        outcome.changed += cut_sprite_map(tx)?;
        outcome.changed += process_slider(tx)?;
        outcome.changed += process_icons(tx)?;
        outcome.changed += process_widgets(tx)?;
        outcome.changed += process_tabs(tx)?;
        outcome.changed += process_resource_packs(tx)?;
        outcome.changed += process_server_selection(tx)?;
        outcome.changed += process_title(tx)?;

        // 旧 atlas 文件在 1.21+ 已不再使用；**延迟**到收尾时机统一删除。
        for path in cleanup_list().iter().copied() {
            if tx.has_prefix(path)? {
                outcome.deferred_removals.push(path.to_string());
                outcome.notes.push(format!("defer removal of {path}"));
            }
        }

        Ok(outcome)
    }

    /// 清理清单（20 项，供测试断言"清单没被漏抄/没混入 inventory.png"）。
    pub fn cleanup_list() -> &'static [&'static str] {
        const CLEANUP: [&str; 20] = [
            "assets/minecraft/textures/gui/container/anvil.png",
            "assets/minecraft/textures/gui/container/beacon.png",
            "assets/minecraft/textures/gui/container/furnace.png",
            "assets/minecraft/textures/gui/container/blast_furnace.png",
            "assets/minecraft/textures/gui/container/smoker.png",
            "assets/minecraft/textures/gui/container/brewing_stand.png",
            "assets/minecraft/textures/gui/container/horse.png",
            "assets/minecraft/textures/gui/container/enchanting_table.png",
            "assets/minecraft/textures/gui/container/stonecutter.png",
            "assets/minecraft/textures/gui/container/loom.png",
            "assets/minecraft/textures/gui/container/smithing.png",
            "assets/minecraft/textures/gui/container/villager2.png",
            "assets/minecraft/textures/gui/container/cartography_table.png",
            "assets/minecraft/textures/gui/container/grindstone.png",
            "assets/minecraft/textures/gui/icons.png",
            "assets/minecraft/textures/gui/widgets.png",
            "assets/minecraft/textures/gui/slider.png",
            "assets/minecraft/textures/gui/container/creative_inventory/tabs.png",
            "assets/minecraft/textures/gui/resource_packs.png",
            "assets/minecraft/textures/gui/server_selection.png",
        ];
        &CLEANUP
    }
    /// `SPRITE_MAP` 的条目数（供测试断言"表没被漏抄"）。
    pub fn sprite_map_len() -> usize {
        SPRITE_MAP.len()
    }
}