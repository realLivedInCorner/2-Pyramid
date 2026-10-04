//! M2：混合运行驱动。
//!
//! 目标：让 **A-ROM 接管一次真实转换的读入与写出**，而任务仍按既定节奏迁移。
//! 第一版（本文件）刻意不要求迁移任何任务——全部旧任务经 [`pathview`](crate::arom::pathview)
//! 适配层跑，用来回答一个必须先回答的问题：
//!
//! > 把「解压到临时目录 → 跑任务 → 重新打包」换成
//! > 「建 Pack → 落盘给任务 → 收成层 → A-ROM 序列化」，
//! > **产物是不是一模一样？**
//!
//! 流程：
//!
//! ```text
//! Pack::open(输入)
//!   → materialize(有效条目落盘 + 基线哈希)
//!   → legacy_run(workdir)        // 旧执行器，签名不变：给它一个目录
//!   → harvest(对比基线产出一层) → commit
//!   → write_zip(A-ROM 序列化)
//! ```
//!
//! 与旧管线的唯一差别就是「字节从哪里来、写到哪里去」，因此两者的产物必须逐项一致——
//! 这正是本模块的验收方式（见文件末尾用例）。
//!
//! 迁移期往后，本驱动会逐步把注册表里的任务换成原生实现（`arom::task::plan` 负责排序与并行），
//! 未迁移的继续走适配层；编译期开关按模块控制（已裁决：D6 按模块）。

use std::path::Path;

use crate::arom::pathview::{harvest, materialize};
use crate::arom::serialize::{write_zip, SerializeOptions, SerializeStats};
use crate::arom::{AromError, Body, Layer, Pack, SafeLimits, Slot};

/// 逐模块迁移开关（**编译期**：默认全关，行为与旧管线逐字一致）。
///
/// 打开某模块后，该模块**已迁移**的任务由 A-ROM 原生执行，其余仍在 `workdir` 上跑旧闭包。
/// 每个开关都能单独关回去——这是「每批可独立回退」的落点。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeSwitches {
    /// textures 模块：已迁移的三个活任务（`delete_font_folder` /
    /// `rename_mcpatcher_to_optifine` / `convert_animated_textures`）。
    pub textures: bool,
}

impl NativeSwitches {
    /// 全关（等价于旧管线）。
    pub fn none() -> Self {
        Self::default()
    }

    /// 全部已迁移任务。
    pub fn all() -> Self {
        Self { textures: true }
    }
}

#[derive(Debug, Clone)]
pub struct MixedRunOptions {
    /// 读入限额。已裁决取宽松值（不改变今天能转的包）。
    pub limits: SafeLimits,
    pub serialize: SerializeOptions,
    /// 层内字节预算（`None` = 不限；溢出后端在后续里程碑接入）。
    pub blob_limit: Option<u64>,
    pub source_version: u32,
    pub target_version: u32,
    /// 与 `invoke_conversion_ex` 的三个开关一致；驱动据此建**同一份**注册表。
    pub run_gui_surgeon: bool,
    pub fix_alpha_layers: bool,
    pub adapt_shaders: bool,
    pub native: NativeSwitches,
}

impl Default for MixedRunOptions {
    fn default() -> Self {
        Self {
            limits: SafeLimits::preserving_current(),
            serialize: SerializeOptions::default(),
            blob_limit: None,
            source_version: 34,
            target_version: 34,
            run_gui_surgeon: true,
            fix_alpha_layers: false,
            adapt_shaders: true,
            native: NativeSwitches::none(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MixedRunReport {
    pub materialized_files: usize,
    pub materialized_dirs: usize,
    /// 旧任务改动到的条目数（层里的写入数）。
    pub harvested_changes: usize,
    pub added: usize,
    pub modified: usize,
    pub removed: usize,
    /// 层写入中落在声明范围之外者（本版传入的声明为 `None`，故为空）。
    pub undeclared: Vec<String>,
    /// 本次执行计划里的任务总数（`Scheduler::plan` 的有序名字数）。
    pub plan_len: usize,
    /// 其中由 A-ROM 原生执行的个数。
    pub native_tasks: usize,
    /// 其中经适配层跑旧闭包的个数。
    pub legacy_tasks: usize,
    pub native_names: Vec<String>,
    pub stats: SerializeStats,
}

/// 用 A-ROM 接管读入与写出，任务由 `legacy_run` 在 `workdir` 上执行。
///
/// `legacy_run` 的签名刻意与旧管线一致：**给它一个目录，它自己跑完所有任务**
/// （例如 `invoke_conversion_ex(...)` + 收尾的 `pack.mcmeta` 改写）。
pub fn run_with_legacy_tasks<F>(
    input: &Path,
    workdir: &Path,
    output: &Path,
    opts: &MixedRunOptions,
    legacy_run: F,
) -> Result<MixedRunReport, AromError>
where
    F: FnOnce(&Path) -> Result<(), String>,
{
    ensure_empty_dir(workdir)?;

    let mut pack = Pack::open_zip(input, &opts.limits, opts.blob_limit)?;

    // 1) 落盘（含空目录——旧执行器在磁盘上看到的目录必须完整）
    let baseline = {
        let view = pack.view();
        materialize(&view, workdir)?
    };

    // 2) 旧任务照常读写目录
    legacy_run(workdir).map_err(AromError::internal)?;

    // 3) 收成一层
    let harvested = harvest(&pack, workdir, &baseline, None)?;

    let report = MixedRunReport {
        materialized_files: baseline.file_count(),
        materialized_dirs: baseline.dir_count(),
        harvested_changes: harvested.changed(),
        added: harvested.added.len(),
        modified: harvested.modified.len(),
        removed: harvested.removed.len(),
        undeclared: harvested.undeclared.clone(),
        ..MixedRunReport::default()
    };
    pack.commit(harvested.layer);

    // 4) A-ROM 序列化
    let stats = {
        let view = pack.view();
        write_zip(&pack, &view, output, &opts.serialize)?
    };

    Ok(MixedRunReport { stats, ..report })
}

/// **驱动 v2**：按 `Scheduler::plan` 的顺序逐任务执行——已迁移的走 A-ROM 原生实现，
/// 未迁移的在 `workdir` 上跑旧闭包并 `harvest` 成层；两者交替时把原生写入同步回 workdir，
/// 让「目录镜像」与「对象模型」始终一致。`tail` 用于收尾步骤（例如 `pack.mcmeta` 改写），
/// 它仍然在 workdir 上跑一次、随后被收获（保持与旧管线同一套逻辑，不重复实现）。
pub fn run_mixed<F>(
    input: &Path,
    workdir: &Path,
    output: &Path,
    opts: &MixedRunOptions,
    tail: F,
) -> Result<MixedRunReport, AromError>
where
    F: FnOnce(&Path) -> Result<(), String>,
{
    use crate::hurray::context::HurrayContext;
    use crate::hurray::scheduler::Scheduler;
    use crate::hurray::texture::TexturePool;

    ensure_empty_dir(workdir)?;

    let mut pack = Pack::open_zip(input, &opts.limits, opts.blob_limit)?;
    let mut baseline = {
        let view = pack.view();
        materialize(&view, workdir)?
    };

    let mut scheduler = Scheduler::new();
    crate::invoke_conversion::register_legacy_tasks(
        &mut scheduler,
        input,
        opts.target_version,
        opts.source_version,
        opts.run_gui_surgeon,
        opts.fix_alpha_layers,
        opts.adapt_shaders,
    );
    let plan = scheduler
        .plan(opts.source_version, opts.target_version)
        .map_err(|e| AromError::internal(format!("scheduler plan: {e}")))?;

    let ctx = HurrayContext::new(workdir.to_str().unwrap_or_default());
    let mut pool = TexturePool::new();
    let mut report = MixedRunReport {
        materialized_files: baseline.file_count(),
        materialized_dirs: baseline.dir_count(),
        plan_len: plan.len(),
        ..MixedRunReport::default()
    };

    // ① 原生前阶段：只放「按语义就该最先跑」的任务（当前批次都是 Eraser 阶段的删除/改名）。
    //
    // 为什么不做「逐任务交错」：实测证明按名字逐个跑旧闭包与生产**不等价**
    // （真实包上 105 个文件差异——`TexturePool` 的提交时机、延迟清理与阶段内并行分组
    // 相互耦合）。一次性批量执行旧任务则与生产逐字一致，因此把风险关在这一侧。
    let native_names: Vec<String> = plan
        .iter()
        .filter(|name| native_for(name, &opts.native).is_some())
        .cloned()
        .collect();
    for name in &native_names {
        let (label, run) = native_for(name, &opts.native).expect("checked above");
        let layer = {
            let mut tx = pack.tx(name);
            run(&mut tx)
                .map_err(|e| AromError::internal(format!("native task `{label}` ({name}): {e}")))?;
            tx.into_layer()
        };
        apply_layer_to_workdir(&pack, workdir, &layer)?;
        sync_baseline_with_layer(&pack, &layer, &mut baseline)?;
        pack.commit(layer);
        report.native_tasks += 1;
        report.native_names.push(name.clone());
    }

    // ② 旧任务：**一次性**批量执行（与 `execute_version_conversion` 完全同构）
    let legacy_names: Vec<String> = plan
        .iter()
        .filter(|name| !native_names.contains(name))
        .cloned()
        .collect();
    report.legacy_tasks = legacy_names.len();
    scheduler
        .run_named(&legacy_names, &ctx, &mut pool)
        .map_err(|e| AromError::internal(format!("legacy tasks: {e}")))?;

    let harvested = harvest(&pack, workdir, &baseline, None)?;
    report.harvested_changes += harvested.changed();
    report.added += harvested.added.len();
    report.modified += harvested.modified.len();
    report.removed += harvested.removed.len();
    sync_baseline_with_layer(&pack, &harvested.layer, &mut baseline)?;
    pack.commit(harvested.layer);

    // 注册表之外的直接步骤（GuiSurgeon sprite 手术）——生产管线在任务之后、
    // 清理之前执行它；漏掉它会整片丢失 sprite 产物（本步实测：真实包少了 3861 个文件）。
    crate::invoke_conversion::run_direct_steps(
        &ctx,
        &mut pool,
        workdir,
        opts.target_version,
        opts.run_gui_surgeon,
    )
    .map_err(|e| AromError::internal(format!("direct steps: {e}")))?;

    // 旧任务的删除是**延迟清理**（`defer_remove_dir` 等），生产管线在
    // `invoke_conversion_ex` 末尾统一执行；驱动必须做同样的事，否则删不掉的目录
    // 会在最终产物里复活（本步实测：`assets/minecraft/font` 曾因此残留）。
    ctx.execute_cleanup()
        .map_err(|e| AromError::internal(format!("execute_cleanup: {e}")))?;
    pool.clear_unused();

    // 收尾步骤：仍在 workdir 上跑一次，然后收获
    tail(workdir).map_err(AromError::internal)?;
    let harvested = harvest(&pack, workdir, &baseline, None)?;
    report.harvested_changes += harvested.changed();
    report.added += harvested.added.len();
    report.modified += harvested.modified.len();
    report.removed += harvested.removed.len();
    report.undeclared = harvested.undeclared.clone();
    pack.commit(harvested.layer);

    report.stats = {
        let view = pack.view();
        write_zip(&pack, &view, output, &opts.serialize)?
    };
    Ok(report)
}

/// 已迁移任务的派发表：**任务名 → 原生实现**。开关关闭即返回 `None`（走旧路径）。
fn native_for(name: &str, switches: &NativeSwitches) -> Option<(&'static str, crate::pilots::PilotFn)> {
    if !switches.textures {
        return None;
    }
    match name {
        // 只登记**活注册表里存在**的任务名；`convert_old_texture_paths` 不在活计划里
        // （它只存在于已删除的死注册路径，见细则 §9.10），因此不派发。
        "delete_font_folder" => Some(("drop_font", crate::pilots::drop_font::run)),
        "delete_blockstates_models" => {
            Some(("drop_blockstates", crate::pilots::drop_blockstates::run))
        }
        "delete_horse_folder" => Some(("drop_horse", crate::pilots::drop_horse::run)),
        "delete_shaders_folder" => Some(("drop_shaders", crate::pilots::drop_shaders::run)),
        "delete_enchanted_item_glint" => Some(("drop_glint", crate::pilots::drop_glint::run)),
        "rename_mcpatcher_to_optifine" => {
            Some(("mcpatcher_optifine", crate::pilots::mcpatcher_optifine::run))
        }
        "convert_animated_textures" => Some(("animated", crate::pilots::animated::run)),
        _ => None,
    }
}

/// 把一层写入落到 workdir，使目录镜像与 pack 保持一致。
fn apply_layer_to_workdir(pack: &Pack, workdir: &Path, layer: &Layer) -> Result<(), AromError> {
    for (path, slot) in layer.writes() {
        let full = workdir.join(path);
        match slot {
            Slot::Tombstone => {
                if full.is_dir() {
                    std::fs::remove_dir_all(&full)
                        .map_err(|e| AromError::io(format!("rmdir {}: {e}", full.display())))?;
                } else if full.exists() {
                    std::fs::remove_file(&full)
                        .map_err(|e| AromError::io(format!("rm {}: {e}", full.display())))?;
                }
            }
            Slot::Present(Body::Dir) => {
                std::fs::create_dir_all(&full)
                    .map_err(|e| AromError::io(format!("mkdir {}: {e}", full.display())))?;
            }
            Slot::Present(body) => {
                if let Some(parent) = full.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| AromError::io(format!("mkdir {}: {e}", parent.display())))?;
                }
                let bytes = pack.read_body(body)?;
                std::fs::write(&full, &bytes)
                    .map_err(|e| AromError::io(format!("write {}: {e}", full.display())))?;
            }
        }
    }
    Ok(())
}

/// 把一层的效果落到基线上（只处理被改动的路径，不做全量重扫）。
fn sync_baseline_with_layer(
    pack: &Pack,
    layer: &Layer,
    baseline: &mut crate::arom::Materialized,
) -> Result<(), AromError> {
    for (path, slot) in layer.writes() {
        match slot {
            Slot::Tombstone => {
                baseline.files.remove(path);
                baseline.dirs.remove(path);
                baseline.files.retain(|p, _| !is_under(p, path));
                baseline.dirs.retain(|p| !is_under(p, path));
            }
            Slot::Present(Body::Dir) => {
                baseline.dirs.insert(path.clone());
            }
            Slot::Present(body) => {
                let bytes = pack.read_body(body)?;
                baseline
                    .files
                    .insert(path.clone(), (bytes.len() as u64, sha256_hex(&bytes)));
                let mut cursor = parent_of(path);
                while let Some(p) = cursor {
                    baseline.dirs.insert(p.to_string());
                    cursor = parent_of(p);
                }
            }
        }
    }
    Ok(())
}

fn is_under(path: &str, prefix: &str) -> bool {
    path.len() > prefix.len() && path.starts_with(prefix) && path.as_bytes()[prefix.len()] == b'/'
}

fn parent_of(path: &str) -> Option<&str> {
    path.rfind('/').map(|i| &path[..i])
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// 工作目录必须是空目录：残留文件会被 `harvest` 误判成「任务新增」。
fn ensure_empty_dir(dir: &Path) -> Result<(), AromError> {
    if dir.exists() {
        let mut entries = std::fs::read_dir(dir)
            .map_err(|e| AromError::io(format!("read {}: {e}", dir.display())))?;
        if entries.next().is_some() {
            return Err(AromError::io(format!(
                "work dir must be empty: {}",
                dir.display()
            )));
        }
    } else {
        std::fs::create_dir_all(dir)
            .map_err(|e| AromError::io(format!("create {}: {e}", dir.display())))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::converters::pack_diff::diff_containers;
    use crate::converters::version_converter::write_pack_format;
    use std::io::Write as _;
    use std::path::PathBuf;

    fn png(size: (u32, u32)) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(size.0, size.1, image::Rgba([120, 60, 30, 255]));
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .expect("encode png");
        buf
    }

    /// 夹具刻意带上几个会被真实任务处理的结构（mcpatcher 目录、font 目录、
    /// item 下的动画 mcmeta、旧贴图路径）。
    fn fixture(path: &Path) {
        let f = std::fs::File::create(path).expect("create fixture");
        let mut zip = zip::ZipWriter::new(f);
        let opts = zip::write::FileOptions::default();
        let mut add = |name: &str, body: Vec<u8>| {
            zip.start_file(name, opts).expect("start");
            zip.write_all(&body).expect("write");
        };
        add(
            "pack.mcmeta",
            br#"{"pack":{"pack_format":34,"description":"fixture"}}"#.to_vec(),
        );
        add("assets/minecraft/mcpatcher/cit/a.properties", b"a=1".to_vec());
        add("assets/minecraft/font/default.json", b"{}".to_vec());
        add("assets/minecraft/terrain.png", png((16, 16)));
        add("assets/minecraft/gui/items.png", png((16, 32)));
        add("assets/minecraft/textures/item/water.png", png((32, 64)));
        add(
            "assets/minecraft/textures/item/water.png.mcmeta",
            br#"{"animation": {}}"#.to_vec(),
        );
        add("assets/minecraft/lang/zh_cn.json", br#"{"a":"b"}"#.to_vec());
        zip.add_directory("assets/empty/", opts).expect("dir");
        zip.finish().expect("finish");
    }

    /// 旧管线的等价物：解压 → 跑任务 → 重打包（与 `process_zip_timed` 同序）。
    fn legacy_output(input: &Path, tmp: &Path, target: u32, source: u32) -> PathBuf {
        let work = tmp.join("legacy_work");
        std::fs::create_dir_all(&work).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            input.to_str().expect("utf8"),
            work.to_str().expect("utf8"),
        )
        .expect("extract");

        crate::invoke_conversion::invoke_conversion_ex(input, &work, target, source, true, false, true)
            .expect("legacy pipeline");
        let mcmeta = work.join("pack.mcmeta");
        if mcmeta.exists() {
            write_pack_format(&mcmeta, target).expect("write_pack_format");
        }

        let out = tmp.join("legacy.zip");
        crate::converters::zip::repack_resource_pack(
            work.to_str().expect("utf8"),
            out.to_str().expect("utf8"),
        )
        .expect("repack");
        out
    }

    fn mixed_output(input: &Path, tmp: &Path, target: u32, source: u32) -> (PathBuf, MixedRunReport) {
        let work = tmp.join("mixed_work");
        let out = tmp.join("mixed.zip");
        let report = run_with_legacy_tasks(
            input,
            &work,
            &out,
            &MixedRunOptions::default(),
            |dir| {
                // `target_path` 与旧管线一致地传输入包（`invoke_conversion_ex` 用它推导包名），
                // 否则两边会因为包名不同而产生差异——那与「谁负责 IO」无关。
                crate::invoke_conversion::invoke_conversion_ex(input, dir, target, source, true, false, true)
                    .map_err(|e| e.to_string())?;
                let mcmeta = dir.join("pack.mcmeta");
                if mcmeta.exists() {
                    write_pack_format(&mcmeta, target).map_err(|e| e.to_string())?;
                }
                Ok(())
            },
        )
        .expect("mixed run");
        (out, report)
    }

    fn assert_equivalent(legacy: &Path, mixed: &Path) -> crate::converters::pack_diff::PackDiffReport {
        let report = diff_containers(legacy, mixed).expect("diff");
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
        assert!(report.passed(true), "{:?}", report.container);
        report
    }

    #[test]
    fn a_rom_owns_io_of_a_real_conversion_on_a_fixture() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let input = tmp.path().join("fixture.zip");
        fixture(&input);

        let legacy = legacy_output(&input, tmp.path(), 97, 34);
        let (mixed, report) = mixed_output(&input, tmp.path(), 97, 34);

        assert_equivalent(&legacy, &mixed);
        assert!(report.materialized_files >= 8, "{report:?}");
        assert!(report.materialized_dirs >= 5, "空目录也要落盘：{report:?}");
        assert!(report.stats.files > 0, "{report:?}");
    }

    #[test]
    fn work_dir_must_be_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let input = tmp.path().join("fixture.zip");
        fixture(&input);
        let work = tmp.path().join("dirty");
        std::fs::create_dir_all(&work).expect("mkdir");
        std::fs::write(work.join("leftover.txt"), b"x").expect("write");

        let err = run_with_legacy_tasks(
            &input,
            &work,
            &tmp.path().join("out.zip"),
            &MixedRunOptions::default(),
            |_| Ok(()),
        )
        .expect_err("must reject dirty work dir");
        assert_eq!(err.kind(), "io");
    }

    fn mixed_v2_output(
        input: &Path,
        tmp: &Path,
        target: u32,
        source: u32,
        native: NativeSwitches,
        tag: &str,
    ) -> (PathBuf, MixedRunReport) {
        let work = tmp.join(format!("v2_work_{tag}"));
        let out = tmp.join(format!("v2_{tag}.zip"));
        let opts = MixedRunOptions {
            source_version: source,
            target_version: target,
            native,
            ..MixedRunOptions::default()
        };
        let report = run_mixed(input, &work, &out, &opts, |dir| {
            let mcmeta = dir.join("pack.mcmeta");
            if mcmeta.exists() {
                write_pack_format(&mcmeta, target).map_err(|e| e.to_string())?;
            }
            Ok(())
        })
        .expect("mixed run v2");
        (out, report)
    }

    /// **8c 的第一批验收**：把 textures 已迁移任务改为原生执行，产物必须与「全走适配层」
    /// 以及旧管线都逐项一致；开关关闭时行为必须与打开前完全相同。
    #[test]
    fn native_switch_keeps_the_output_identical_on_a_fixture() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let input = tmp.path().join("fixture.zip");
        fixture(&input);

        // 1 → 97 会经过 (5,6)/(7,8) 两段，因此能选中已迁移的 Eraser 任务
        let legacy = legacy_output(&input, tmp.path(), 97, 1);
        let (off, off_report) =
            mixed_v2_output(&input, tmp.path(), 97, 1, NativeSwitches::none(), "off");
        let (on, on_report) =
            mixed_v2_output(&input, tmp.path(), 97, 1, NativeSwitches::all(), "on");

        assert_eq!(off_report.native_tasks, 0, "开关关闭时不得有原生任务");
        assert_eq!(off_report.legacy_tasks, off_report.plan_len, "关闭时应全走适配层");
        assert_eq!(
            on_report.native_tasks + on_report.legacy_tasks,
            on_report.plan_len,
            "每个计划任务必须恰好执行一次"
        );
        assert!(
            on_report.native_tasks >= 1,
            "该版本对应当命中已迁移任务，实际原生：{:?}",
            on_report.native_names
        );

        assert_equivalent(&legacy, &off);
        assert_equivalent(&legacy, &on);
        assert_equivalent(&off, &on);
    }

    /// 真实包上的三种配置对照（默认忽略）：
    /// `AROM_REAL_PACK=<包> [AROM_TARGET=97] cargo test --lib native_switch -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn native_switch_keeps_the_output_identical_on_a_real_pack() {
        let Ok(src) = std::env::var("AROM_REAL_PACK") else {
            println!("AROM_REAL_PACK 未设置，跳过");
            return;
        };
        let input = PathBuf::from(&src);
        assert!(input.is_file(), "不是文件：{}", input.display());
        let target: u32 = std::env::var("AROM_TARGET")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(97);
        let source = {
            let pack = Pack::open_zip(&input, &SafeLimits::preserving_current(), None)
                .expect("open for source format");
            pack.view()
                .mcmeta()
                .ok()
                .and_then(|m| m.effective_format())
                .unwrap_or(34)
        };
        println!("source format = {source}, target = {target}");

        let tmp = tempfile::tempdir().expect("tempdir");
        let legacy = legacy_output(&input, tmp.path(), target, source);
        let (off, off_report) =
            mixed_v2_output(&input, tmp.path(), target, source, NativeSwitches::none(), "off");
        let (on, on_report) =
            mixed_v2_output(&input, tmp.path(), target, source, NativeSwitches::all(), "on");

        println!("off = {off_report:?}");
        println!("on  = {on_report:?}");
        assert_eq!(off_report.native_tasks, 0);
        assert_eq!(
            on_report.native_tasks + on_report.legacy_tasks,
            on_report.plan_len
        );

        assert_equivalent(&legacy, &off);
        assert_equivalent(&legacy, &on);
        assert_equivalent(&off, &on);
    }

    /// 真实包上的等价性（默认忽略）：
    /// `AROM_REAL_PACK=<包> [AROM_TARGET=97] cargo test --lib mixed_run -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn a_rom_owns_io_of_a_real_conversion_on_a_real_pack() {
        let Ok(src) = std::env::var("AROM_REAL_PACK") else {
            println!("AROM_REAL_PACK 未设置，跳过");
            return;
        };
        let input = PathBuf::from(&src);
        assert!(input.is_file(), "不是文件：{}", input.display());
        let target: u32 = std::env::var("AROM_TARGET")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(97);

        // 源格式从包里读（L2 视图）
        let source = {
            let pack = Pack::open_zip(&input, &SafeLimits::preserving_current(), None)
                .expect("open for source format");
            pack.view()
                .mcmeta()
                .ok()
                .and_then(|m| m.effective_format())
                .unwrap_or(34)
        };
        println!("source format = {source}, target = {target}");

        let tmp = tempfile::tempdir().expect("tempdir");
        let legacy = legacy_output(&input, tmp.path(), target, source);
        let (mixed, report) = mixed_output(&input, tmp.path(), target, source);
        let diff = assert_equivalent(&legacy, &mixed);
        println!("mixed run = {report:?}");
        println!("diff = {}", diff.summary());
    }
}
