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

    pub const TEXTURES: &str = "assets/minecraft/textures";
    const ITEMS: &str = "assets/minecraft/textures/items";
    const ITEM: &str = "assets/minecraft/textures/item";
    const BLOCKS: &str = "assets/minecraft/textures/blocks";
    const BLOCK: &str = "assets/minecraft/textures/block";

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
    fn merge_or_rename_dir(tx: &mut Tx<'_>, from: &str, to: &str) -> Result<bool, AromError> {
        if !tx.has_prefix(from)? {
            return Ok(false);
        }
        if !tx.has_prefix(to)? {
            tx.rename_dir(from, to)?;
            return Ok(true);
        }
        // 两侧都存在：逐文件移动（源覆盖目标），最后删掉源目录。
        //
        // 注意这里**不能**用「逐文件改名规则 + 源目录 tombstone」：tombstone 覆盖整棵子树，
        // 会把已经改名出去的子文件一起隐藏（真实包上 `blocks/` 与 `block/` 并存时，
        // 整个 `block/` 会凭空消失）。改为「读出内容写到目标 + 整段删除源目录」，
        // 语义与旧实现逐条对应：同名源覆盖目标、目标独有者保留、源目录最终消失。
        let files: Vec<String> = tx
            .list(from)?
            .into_iter()
            .filter(|res| !res.is_dir)
            .map(|res| res.path)
            .collect();
        for path in files {
            let target = format!("{to}/{}", child_of(from, &path));
            let bytes = tx.read(&path)?.unwrap_or_default();
            if tx.exists(&target) {
                tx.remove(&target)?;
            }
            tx.put(&target, bytes)?;
        }
        tx.remove(from)?;
        Ok(true)
    }

    /// 旧 `rename_with_mcmeta`：png 改名 + 顺带搬同名 `.png.mcmeta`。
    fn rename_with_mcmeta(
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
        if tx.exists(&new_path) {
            tx.remove(&new_path)?;
        }
        tx.rename_dir(&old_path, &new_path)?;

        let old_meta = format!("{old_path}.mcmeta");
        if tx.exists(&old_meta) {
            let new_meta = format!("{new_path}.mcmeta");
            if tx.exists(&new_meta) {
                tx.remove(&new_meta)?;
            }
            tx.rename_dir(&old_meta, &new_meta)?;
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
            if tx.exists(&new_path) {
                tx.remove(&new_path)?;
            }
            tx.rename_dir(&old_path, &new_path)?;

            let old_meta = format!("{old_path}.mcmeta");
            if tx.exists(&old_meta) {
                let new_meta = format!("{new_path}.mcmeta");
                if tx.exists(&new_meta) {
                    tx.remove(&new_meta)?;
                }
                tx.rename_dir(&old_meta, &new_meta)?;
            }
        }
        for (old, new) in pairs {
            let old_meta = format!("{dir}/{old}.mcmeta");
            let new_meta = format!("{dir}/{new}.mcmeta");
            if tx.exists(&old_meta) && !tx.exists(&new_meta) {
                tx.rename_dir(&old_meta, &new_meta)?;
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
    const REAL_PACK_SKIP: [&str; 1] = ["rename_blocks"];

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
