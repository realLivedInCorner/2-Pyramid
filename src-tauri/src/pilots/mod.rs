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
        let sources: [(&str, &str); 8] = [
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
            ] {
                let (_, _, run) = crate::mixed_run::native_for_probe(name)
                    .unwrap_or_else(|| panic!("{name} 未在派发表里"));
                let mut tx = pack.tx(name);
                run(&mut tx).expect("native run");
                pack.commit(tx.into_layer());
            }
            let view = pack.view();
            crate::arom::pathview::materialize(&view, &native_dir).expect("materialize");
        }

        // ⑤ 跑旧函数并逐个输出对比
        let cases: [(&str, fn(&std::path::Path) -> Result<(), String>, Vec<String>); 4] = [
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