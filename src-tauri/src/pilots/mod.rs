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
    /// 原生任务同样只登记，由驱动在清理点应用（见 `mixed_run`）。
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
        use crate::converters::ui::process_chest_folder as legacy;

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
                crate::converters::reverse::chest_folder::swap_and_mirror(&mut img, a, b)
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
                crate::converters::ui::process_chest_folder::mirror_region(&mut img, b);
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
        ($m:ident, $task:literal) => {
            pub mod $m {
                use super::*;
                pub fn decl() -> TaskDecl {
                    TaskDecl::new($task, Tier::Eraser).exclusive(true)
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

    noop_pilot!(cut_gui, "reverse_cut_gui");
    noop_pilot!(horse, "reverse_fix_horse_ui");
    noop_pilot!(overlay_icons, "reverse_overlay_icons");
    noop_pilot!(sub_hand, "reverse_fix_ui_sub_hand");

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
}
