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
use crate::arom::task::TaskDecl;
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
    /// ui 模块：已迁移的 `process_chest_folder`。
    pub ui: bool,
    /// reverse 模块：已迁移的 `reverse_process_chest_folder`。
    pub reverse: bool,
}

impl NativeSwitches {
    /// 全关（等价于旧管线）。
    pub fn none() -> Self {
        Self::default()
    }

    /// 全部已迁移任务。
    pub fn all() -> Self {
        Self {
            textures: true,
            ui: true,
            reverse: true,
        }
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
    /// 原生任务写出声明范围即报错（D12 契约）。关掉则只记录在
    /// [`MixedRunReport::undeclared`] 里，便于诊断。
    pub strict_scopes: bool,
    /// 实验模式：旧任务逐个执行（每个之后收层）。用于验证「同阶段内交错是否等价」。
    pub legacy_one_by_one: bool,
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
            strict_scopes: true,
            legacy_one_by_one: false,
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
    /// 原生任务登记的**延迟删除**路径（在清理点统一应用）。
    pub deferred_removals: Vec<String>,
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
        let (label, decl, run) = native_for(name, &opts.native).expect("checked above");
        let (outcome, layer) = {
            let mut tx = pack.tx(name);
            let outcome = run(&mut tx)
                .map_err(|e| AromError::internal(format!("native task `{label}` ({name}): {e}")))?;
            (outcome, tx.into_layer())
        };
        // 契约检查：原生任务只能写它声明过的路径
        let violations = scope_violations(&decl, &layer);
        if !violations.is_empty() {
            if opts.strict_scopes {
                return Err(AromError::internal(format!(
                    "native task `{label}` ({name}) wrote outside its declared scope: {:?} (declared writes: {})",
                    violations,
                    decl.writes.describe()
                )));
            }
            report.undeclared.extend(violations);
        }
        apply_layer_to_workdir(&pack, workdir, &layer)?;
        sync_baseline_with_layer(&pack, &layer, &mut baseline)?;
        pack.commit(layer);
        report.native_tasks += 1;
        report.deferred_removals.extend(outcome.deferred_removals.iter().cloned());
        report.native_names.push(name.clone());
    }

    // ② 旧任务：**一次性**批量执行（与 `execute_version_conversion` 完全同构）
    let legacy_names: Vec<String> = plan
        .iter()
        .filter(|name| !native_names.contains(name))
        .cloned()
        .collect();
    report.legacy_tasks = legacy_names.len();

    if opts.legacy_one_by_one {
        // 实验模式：逐个任务执行并在每个之后收层——用来回答「同阶段内交错是否等价」。
        // 生产默认不走这条（§9.10 曾实测它与一次性批量不等价，本模式的结论见 §9.18）。
        for name in &legacy_names {
            scheduler
                .run_named(std::slice::from_ref(name), &ctx, &mut pool)
                .map_err(|e| AromError::internal(format!("legacy task `{name}`: {e}")))?;
            let one = harvest(&pack, workdir, &baseline, None)?;
            report.harvested_changes += one.changed();
            report.added += one.added.len();
            report.modified += one.modified.len();
            report.removed += one.removed.len();
            sync_baseline_with_layer(&pack, &one.layer, &mut baseline)?;
            pack.commit(one.layer);
        }
    } else {
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
    }

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

    // 原生任务登记的**延迟删除**：与旧实现的 `defer_remove_*` **同一时机**（清理点）统一应用。
    // 早删会让更晚的任务看不到文件（实测：反向改名搬不动已被删的源，§9.23）。
    if !report.deferred_removals.is_empty() {
        let deferred = std::mem::take(&mut report.deferred_removals);
        let layer = {
            let mut tx = pack.tx("deferred-removals");
            for path in &deferred {
                tx.remove(path)?;
            }
            tx.into_layer()
        };
        apply_layer_to_workdir(&pack, workdir, &layer)?;
        sync_baseline_with_layer(&pack, &layer, &mut baseline)?;
        pack.commit(layer);
        report.deferred_removals = deferred;
    }

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

/// 已迁移任务的派发表：**任务名 → (标签, 声明, 原生实现)**。开关关闭即返回 `None`（走旧路径）。
fn native_for(
    name: &str,
    switches: &NativeSwitches,
) -> Option<(&'static str, TaskDecl, crate::pilots::PilotFn)> {
    if switches.reverse {
        if name == "reverse_process_chest_folder" {
            return Some((
                "chest_reverse",
                crate::pilots::chest_reverse::decl(),
                crate::pilots::chest_reverse::run,
            ));
        }
        if name == "reverse_rename_blocks_items" {
            return Some((
                "rename_blocks_reverse",
                crate::pilots::rename_blocks_reverse::decl(),
                crate::pilots::rename_blocks_reverse::run,
            ));
        }
        if let Some((decl, run)) = crate::pilots::reverse_defer::lookup(name) {
            return Some(("reverse_defer", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::reverse_trivial::lookup(name) {
            return Some(("reverse_trivial", decl, run));
        }
    }
    if switches.ui && name == "process_chest_folder" {
        return Some((
            "chest",
            crate::pilots::chest::decl(),
            crate::pilots::chest::run,
        ));
    }
    if !switches.textures {
        return None;
    }
    match name {
        // 只登记**活注册表里存在**的任务名；`convert_old_texture_paths` 与
        // `convert_animated_textures` 不出现在任何版本映射段里（见细则 §9.12），
        // 因此不派发。
        "delete_font_folder" => Some((
            "drop_font",
            crate::pilots::drop_font::decl(),
            crate::pilots::drop_font::run,
        )),
        "delete_blockstates_models" => Some((
            "drop_blockstates",
            crate::pilots::drop_blockstates::decl(),
            crate::pilots::drop_blockstates::run,
        )),
        "delete_horse_folder" => Some((
            "drop_horse",
            crate::pilots::drop_horse::decl(),
            crate::pilots::drop_horse::run,
        )),
        "delete_shaders_folder" => Some((
            "drop_shaders",
            crate::pilots::drop_shaders::decl(),
            crate::pilots::drop_shaders::run,
        )),
        "delete_enchanted_item_glint" => Some((
            "drop_glint",
            crate::pilots::drop_glint::decl(),
            crate::pilots::drop_glint::run,
        )),
        // `rename_blocks_items` 的试点已实现，但真实包上仍与旧实现有差异（见细则 §9.13），
        // 因此**暂不派发**：它留在 `pilots::all()` 里由夹具双轨覆盖。
        "rename_blocks_items" => Some((
            "rename_blocks",
            crate::pilots::rename_blocks::decl(),
            crate::pilots::rename_blocks::run,
        )),
        "rename_mcpatcher_to_optifine" => Some((
            "mcpatcher_optifine",
            crate::pilots::mcpatcher_optifine::decl(),
            crate::pilots::mcpatcher_optifine::run,
        )),
        _ => None,
    }
}

/// 原生任务的写入是否落在它**声明**的范围内（D12 契约的落地检查）。
///
/// 旧任务经适配层时范围是「整包」（未迁移者默认串行），而原生任务必须精确声明；
/// 越界即契约违约，因此驱动默认直接报错（`MixedRunOptions::strict_scopes`）。
pub fn scope_violations(decl: &TaskDecl, layer: &Layer) -> Vec<String> {
    layer
        .writes()
        .keys()
        .filter(|path| decl.check_write(path).is_err())
        .cloned()
        .collect()
}

/// 把一层写入落到 workdir，使目录镜像与 pack 保持一致。
pub(crate) fn apply_layer_to_workdir(
    pack: &Pack,
    workdir: &Path,
    layer: &Layer,
) -> Result<(), AromError> {
    // 顺序与 `entries()` 一致：先套用本层改名规则，再套用本层写入
    apply_renames_to_workdir(workdir, layer)?;
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


/// 把一层的**改名规则**落到 workdir（Move = 磁盘移动，Copy = 复制）。
///
/// 逐条规则处理：`from` 是文件时按单条移动（旧实现里文件级改名很常见，早先这里漏了分支、
/// 直接 `read_dir` 会 panic）；是目录时先深后浅移动，最后清掉空的源目录。
fn apply_renames_to_workdir(workdir: &Path, layer: &Layer) -> Result<(), AromError> {
    use crate::arom::RenameMode;

    for rule in layer.renames() {
        let from = workdir.join(&rule.from);
        if !from.exists() {
            continue;
        }
        if from.is_file() {
            move_or_copy(&from, &workdir.join(&rule.to), rule.mode)?;
            continue;
        }

        let mut items: Vec<std::path::PathBuf> = Vec::new();
        collect_paths(&from, &mut items)?;
        items.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
        for path in items {
            let rest = path
                .strip_prefix(&from)
                .map_err(|e| AromError::io(format!("strip {}: {e}", path.display())))?;
            move_or_copy(&path, &workdir.join(&rule.to).join(rest), rule.mode)?;
        }
        if rule.mode == RenameMode::Move && from.exists() {
            std::fs::remove_dir_all(&from)
                .map_err(|e| AromError::io(format!("rmdir {}: {e}", from.display())))?;
        }
    }
    Ok(())
}

fn move_or_copy(
    from: &Path,
    to: &Path,
    mode: crate::arom::RenameMode,
) -> Result<(), AromError> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AromError::io(format!("mkdir {}: {e}", parent.display())))?;
    }
    if mode == crate::arom::RenameMode::Move {
        if to.exists() {
            remove_path(to)?;
        }
        std::fs::rename(from, to)
            .map_err(|e| AromError::io(format!("rename {}: {e}", from.display())))?;
    } else {
        std::fs::copy(from, to)
            .map_err(|e| AromError::io(format!("copy {}: {e}", from.display())))?;
    }
    Ok(())
}

fn collect_paths(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<(), AromError> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| AromError::io(format!("read {}: {e}", dir.display())))?;
    for entry in entries {
        let entry = entry.map_err(|e| AromError::io(format!("dir entry: {e}")))?;
        let path = entry.path();
        if path.is_dir() {
            collect_paths(&path, out)?;
        }
        out.push(path);
    }
    Ok(())
}

fn remove_path(path: &Path) -> Result<(), AromError> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
    .map_err(|e| AromError::io(format!("rm {}: {e}", path.display())))
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

    /// 声明范围检查必须真的能抓住越界写入（否则它就是空转的）。
    #[test]
    fn scope_violations_flags_out_of_scope_writes() {
        use crate::arom::task::{ScopeSet, Tier};

        let tmp = tempfile::tempdir().expect("tempdir");
        let input = tmp.path().join("fixture.zip");
        fixture(&input);
        let mut pack =
            Pack::open_zip(&input, &SafeLimits::preserving_current(), None).expect("open");

        let layer = {
            let mut tx = pack.tx("probe");
            tx.put("assets/minecraft/textures/bar/x.png", b"x".to_vec())
                .expect("put");
            tx.into_layer()
        };

        let narrow = TaskDecl::new("narrow", Tier::Eraser)
            .writes(ScopeSet::prefix("assets/minecraft/textures/foo"));
        assert_eq!(
            scope_violations(&narrow, &layer),
            vec!["assets/minecraft/textures/bar/x.png".to_string()],
            "越界写入必须被报出"
        );

        let wide = TaskDecl::new("wide", Tier::Eraser)
            .writes(ScopeSet::prefix("assets/minecraft/textures/bar"));
        assert!(
            scope_violations(&wide, &layer).is_empty(),
            "范围内的写入不报"
        );
    }

    #[test]
    /// 层里的**改名规则**必须镜像到 workdir：目录级与文件级各一条
    /// （后者早先会因对文件调用 `read_dir` 而 panic）。
    #[test]
    fn layer_renames_are_mirrored_into_the_work_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let input = tmp.path().join("fixture.zip");
        fixture(&input);
        let mut pack =
            Pack::open_zip(&input, &SafeLimits::preserving_current(), None).expect("open");
        let work = tmp.path().join("work");
        materialize(&pack.view(), &work).expect("materialize");

        let layer = {
            let mut tx = pack.tx("mirror-probe");
            tx.rename_dir(
                "assets/minecraft/textures/item",
                "assets/minecraft/textures/moved",
            )
            .expect("dir rename");
            tx.rename_dir(
                "assets/minecraft/lang/zh_cn.json",
                "assets/minecraft/lang/zh.json",
            )
            .expect("file rename");
            tx.put("assets/minecraft/new.txt", b"x".to_vec())
                .expect("put");
            tx.into_layer()
        };
        apply_layer_to_workdir(&pack, &work, &layer).expect("mirror");

        assert!(
            work.join("assets/minecraft/textures/moved/water.png").exists(),
            "目录规则要落到磁盘"
        );
        assert!(
            !work.join("assets/minecraft/textures/item").exists(),
            "源目录应消失"
        );
        assert!(
            work.join("assets/minecraft/lang/zh.json").exists(),
            "文件级规则要落到磁盘"
        );
        assert!(
            !work.join("assets/minecraft/lang/zh_cn.json").exists(),
            "文件级规则的源应消失"
        );
        assert!(work.join("assets/minecraft/new.txt").exists(), "写入照旧");
    }

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
        legacy_one_by_one: bool,
        tag: &str,
    ) -> (PathBuf, MixedRunReport) {
        let work = tmp.join(format!("v2_work_{tag}"));
        let out = tmp.join(format!("v2_{tag}.zip"));
        let opts = MixedRunOptions {
            source_version: source,
            target_version: target,
            native,
            legacy_one_by_one,
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
            mixed_v2_output(&input, tmp.path(), 97, 1, NativeSwitches::none(), false, "off");
        let (on, on_report) =
            mixed_v2_output(&input, tmp.path(), 97, 1, NativeSwitches::all(), false, "on");

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
            mixed_v2_output(&input, tmp.path(), target, source, NativeSwitches::none(), false, "off");
        let (on, on_report) =
            mixed_v2_output(&input, tmp.path(), target, source, NativeSwitches::all(), false, "on");
        // 实验模式：旧任务逐个执行并在每个之后收层——用来判定「同阶段内交错」是否等价。
        let (one_by_one, obb_report) = mixed_v2_output(
            &input,
            tmp.path(),
            target,
            source,
            NativeSwitches::all(),
            true,
            "one_by_one",
        );

        println!("off        = {off_report:?}");
        println!("on         = {on_report:?}");
        println!("one_by_one = {obb_report:?}");
        assert_eq!(off_report.native_tasks, 0);
        assert_eq!(
            on_report.native_tasks + on_report.legacy_tasks,
            on_report.plan_len
        );

        assert_equivalent(&legacy, &off);
        assert_equivalent(&legacy, &on);
        assert_equivalent(&off, &on);
        // **刻意不断言** one_by_one 与生产的等价性：实测它不等价（§9.18），这里保留运行与
        // 打印，作为「同阶段内交错仍不可用」的可复现证据。
    }

    /// **反向**整包对照（默认忽略）：拿正向产物当反向输入——正向输出正是反向转换的输入形态
    /// （现代命名 `item/`、`block/`），而基准包本身是旧命名，反向任务在它身上大多会跳过。
    ///
    /// `AROM_REAL_PACK=<包> cargo test --lib reverse_whole_pack -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn reverse_whole_pack_matches_the_old_pipeline() {
        let Ok(src) = std::env::var("AROM_REAL_PACK") else {
            println!("AROM_REAL_PACK 未设置，跳过");
            return;
        };
        let input = PathBuf::from(&src);
        assert!(input.is_file(), "不是文件：{}", input.display());
        let forward_target: u32 = std::env::var("AROM_TARGET")
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
                .unwrap_or(1)
        };

        let tmp = tempfile::tempdir().expect("tempdir");
        // 第一步：正向转换（全适配层，等价性已由另一个用例覆盖），产物作为反向输入
        let (forward_out, forward_report) = mixed_v2_output(
            &input,
            tmp.path(),
            forward_target,
            source,
            NativeSwitches::none(),
            false,
            "fwd",
        );
        println!("forward = {forward_report:?}");

        // 第二步：反向 97 → 1，三种配置对照
        let legacy = legacy_output(&forward_out, tmp.path(), source, forward_target);
        let (off, off_report) = mixed_v2_output(
            &forward_out,
            tmp.path(),
            source,
            forward_target,
            NativeSwitches::none(),
            false,
            "rev_off",
        );
        let (on, on_report) = mixed_v2_output(
            &forward_out,
            tmp.path(),
            source,
            forward_target,
            NativeSwitches::all(),
            false,
            "rev_on",
        );
        println!("rev off = {off_report:?}");
        println!("rev on  = {on_report:?}");

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
