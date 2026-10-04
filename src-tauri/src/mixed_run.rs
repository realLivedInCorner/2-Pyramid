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
    /// **逐步读数**（诊断）：每一步之后记录 workdir 的指纹，写入
    /// [`MixedRunReport::step_trace`]。默认关闭（对产物无影响）。
    pub step_trace: bool,
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
            step_trace: false,
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
    /// 声明阶段与活注册表不一致的原生任务（见 §9.40：阶段决定相对位置）。
    pub tier_mismatches: Vec<String>,
    /// **逐步读数**（诊断，`step_trace` 打开时才填充）：`(标签, 文件数, 总字节, 名字指纹)`。
    /// 用来回答「分叉是哪一步先发生的」（§9.73）。
    pub step_trace: Vec<(String, usize, u64, u64)>,
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
    // **逐步读数**（诊断，§9.73）：局部累积，末尾一次性写入报告（避免借用冲突）
    let mut trace: Vec<(String, usize, u64, u64)> = Vec::new();
    let trace_step = |label: &str, trace: &mut Vec<(String, usize, u64, u64)>| {
        if opts.step_trace {
            let (files, bytes, hash) = digest_workdir(workdir);
            trace.push((label.to_string(), files, bytes, hash));
        }
    };

    // ① 原生前阶段：只放「按语义就该最先跑」的任务（当前批次都是 Eraser 阶段的删除/改名）。
    //
    // 为什么不做「逐任务交错」：实测证明按名字逐个跑旧闭包与生产**不等价**
    // （真实包上 105 个文件差异——`TexturePool` 的提交时机、延迟清理与阶段内并行分组
    // 相互耦合）。一次性批量执行旧任务则与生产逐字一致，因此把风险关在这一侧。
    //
    // **放置规则（§9.52 再次修正）**：旧批次不可拆分（§9.42），因此每个原生任务只能在
    // 「整批旧任务之前」或「之后」二选一。选哪一侧**由依赖关系决定**，而不是由阶段名比较
    // 决定（§9.49 的阶段比较在 `generate_smithing_ui` 上给了错答案：它的阶段 Architect
    // 高于旧批次里的 Eraser，于是被放到旧批次之后，结果**晚于**计划中消费它输出的
    // Surgeon 旧任务，顺序被反转）。判据：
    //
    // > 与某原生任务**有重叠**的旧任务，若在计划里排在它**之后**，该原生任务必须放前阶段
    // > （否则那一步会读到原生改动后的内容）；若排在它**之前**，则必须放后阶段
    // > （它自己必须读到旧任务的改动）。两侧都要求时属**歧义**，放后阶段并记录待裁决。
    let native_placements = native_placements(&plan, &opts.native, &scheduler);
    let early_natives: Vec<String> = plan
        .iter()
        .filter(|name| matches!(native_placements.get(*name), Some(Side::Early)))
        .cloned()
        .collect();
    for name in &early_natives {
        let (label, decl, run) = native_for(name, &opts.native).expect("checked above");
        let (outcome, layer) = {
            let mut tx = pack.tx(name);
            let outcome = run(&mut tx)
                .map_err(|e| AromError::internal(format!("native task `{label}` ({name}): {e}")))?;
            (outcome, tx.into_layer())
        };
        // **阶段一致性检查**：`decl().tier` 必须与活注册表登记的阶段一致。
        // 阶段不是装饰：驱动目前把所有原生任务放在同一个「前阶段」，只有 Eraser 级任务的
        // 位置才与生产一致；写错阶段会静默改变执行顺序（§9.40 的谜题正是这样来的）。
        if let Some(live) = scheduler.task_tier(name) {
            let live_str = match live {
                crate::hurray::scheduler::TaskTier::Eraser => "eraser",
                crate::hurray::scheduler::TaskTier::Architect => "architect",
                crate::hurray::scheduler::TaskTier::Surgeon => "surgeon",
                crate::hurray::scheduler::TaskTier::Closure => "closure",
            };
            if live_str != decl.tier.as_str() {
                return Err(AromError::internal(format!(
                    "native task `{label}` ({name}) declares tier `{}` but the live registry says `{live_str}`",
                    decl.tier.as_str()
                )));
            }
        }
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
        check_layer_materialized(&pack, workdir, &layer, name)?;
        sync_baseline_with_layer(&pack, &layer, &mut baseline)?;
        pack.commit(layer);
        report.native_tasks += 1;
        report.deferred_removals.extend(outcome.deferred_removals.iter().cloned());
        report.native_names.push(name.clone());
        trace_step(&format!("pre:{name}"), &mut trace);
    }

    // ② 旧任务：**一次性**批量执行（与 `execute_version_conversion` 完全同构）
    let native_names: Vec<String> = plan
        .iter()
        .filter(|name| native_for(name, &opts.native).is_some())
        .cloned()
        .collect();
    let legacy_names: Vec<String> = plan
        .iter()
        .filter(|name| !native_names.contains(name))
        .cloned()
        .collect();
    report.legacy_tasks = legacy_names.len();

    trace_step("before-legacy", &mut trace);
    if opts.legacy_one_by_one || opts.step_trace {
        // `legacy_one_by_one`：实验模式，逐个任务执行并在每个之后收层（§9.18 的结论即出自它）。
        // `step_trace`：**诊断模式**也走这条，只为拿到「旧批次里是**哪一步**先偏离」的读数——
        // 它不改变产物（收层粒度比生产细，但产物由最终 harvest 决定）。
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
            trace_step(&format!("legacy:{name}"), &mut trace);
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
    trace_step("after-legacy", &mut trace);

    // **后阶段**：放依赖关系要求「晚于旧批次」的原生任务，位置在旧批次之后、GuiSurgeon 之前。
    // 依据（§9.42 实测）：旧批次不能被拆分（它只有一次提交 + 一次清理）；而「必须早于全部
    // 旧任务」的（见前阶段的判据）走前阶段，两段各自都保持「原生在旧批次的一侧」。
    for name in &native_names {
        if matches!(native_placements.get(name), Some(Side::Early)) {
            continue;
        }
        let (label, decl, run) = native_for(name, &opts.native).expect("dispatched above");
        let (outcome, layer) = {
            let mut tx = pack.tx(name);
            let outcome = run(&mut tx)
                .map_err(|e| AromError::internal(format!("native task `{label}` ({name}): {e}")))?;
            (outcome, tx.into_layer())
        };
        let violations = scope_violations(&decl, &layer);
        if !violations.is_empty() && opts.strict_scopes {
            return Err(AromError::internal(format!(
                "native task `{label}` ({name}) wrote outside its declared scope: {violations:?}"
            )));
        }
        // **必须**把层落到 workdir：GuiSurgeon / cut_gui 是「注册表之外的直接步骤」，
        // 它们与旧批次一样**直接读写磁盘**。只把层提交进 Pack 会让这一步读到旧内容，
        // 产物随之分叉（实测：`generate_smithing_ui` 的 4 个 sprite 就是这样丢的——
        // 原生与旧 Architect 产物逐像素相同，是 Surgeon 的 `process_smithing2` 没生效，§9.52）。
        apply_layer_to_workdir(&pack, workdir, &layer)?;
        check_layer_materialized(&pack, workdir, &layer, name)?;
        sync_baseline_with_layer(&pack, &layer, &mut baseline)?;
        pack.commit(layer);
        report.deferred_removals.extend(outcome.deferred_removals);
        // 后阶段的原生任务同样计入（否则报告与断言都会少算）
        report.native_tasks += 1;
        report.native_names.push(name.clone());
        trace_step(&format!("post:{name}"), &mut trace);
    }
    {
        let harvested = harvest(&pack, workdir, &baseline, None)?;
        report.harvested_changes += harvested.changed();
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
    trace_step("after-direct-steps", &mut trace);

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

    report.step_trace = trace;
    report.stats = {
        let view = pack.view();
        write_zip(&pack, &view, output, &opts.serialize)?
    };
    Ok(report)
}

/// **必须放在旧批次之前**的原生任务（显式名单，不许靠推断）。
///
/// 为什么需要名单而不是一条判据：旧批次不可拆分（§9.42），原生任务只能在「整批之前/之后」
/// 二选一；而哪一侧正确取决于**它在计划里的槽位与后续旧任务的关系**，这一点无法只从阶段名
/// 推出——同一阶段里两侧都有实例（实测，§9.52/§9.53）：
///
/// | 任务 | 正确侧 | 证据 |
/// |---|---|---|
/// | `generate_boat` | **后** | 提前则真实包立刻分叉：旧批次里的 `generate_boat` 随后在**已被改名**的 `boat.png` 上重跑，`acacia/birch/dark_oak/jungle_boat.png` 全部消失 |
/// | `generate_potion_lingering` | **后** | 同一次实测（`lingering_potion.png` OnlyInB） |
/// | `rename_blocks_items` | **后** | 它对旧批次改过的 426 个路径做**后置**改名，提前会改变旧任务读到的文件名 |
/// | `generate_smithing_ui` | **前** | 计划顺序是 `generate_smithing_ui` → Surgeon 的 `fix_smithing2_villager2_ui`，后者会**重新派生并覆盖** `container/smithing.png`；放后阶段等于被它覆盖回去，`cut_gui` 切出的 4 个 sprite 随之分叉 |
/// | `generate_{copper,netherite}_armor_models` | **前** | 旧任务 `fix_armor_models`（Surgeon）会把 `models/armor/{copper,netherite}_layer_*.png` **改名搬走**到 `entity/equipment/humanoid(_leggings)/`；放后阶段时源已被搬走 → 整任务跳过 → 新路径下 4 个文件消失（OnlyInA） |
/// | `generate_poplar_planks` | **前** | 它的「优先 jungle、缺失回退 oak」判据依赖**当时**磁盘上还有哪些源；放后阶段时更早的删除类旧任务已经把 jungle 源搬走 → 单个 `item/poplar_sign.png` 走了 oak 回退链 → 132/256 像素不同（实测） |
/// | `generate_tricky_trials_breeze` | **前** | 它的状态图标源 `mob_effect/{speed,jump_boost,absorption}.png` **是旧任务 `fix_ui_survival` 造出来的**；放后阶段时这三个源"凭空出现" → 多生成 `mob_effect/wind_charged.png`（OnlyInB，实测） |
/// | ~~`fix_clock_compass`~~ | **后**（实测后从名单撤回） | 旧实现读 `textures/items/{clock,compass}.png`，但计划里 `rename_blocks_items`（阶段 3–4）**已经把它们改名到 `item/`**——所以它在生产管线里是**空操作**。提前到前阶段会让"源又存在了"，于是真的拆出 `clock_00..63.png`（实测 100 项差异）→ **必须留在后面**才能复刻这个空操作 |
/// | `fix_horse_ui` | **前** | 它**原位改写** `gui/container/horse.png`，而该图随后被 GUI 切片链消费成 `gui/sprites/container/slot/*`；放后阶段会让 `fix2_horse_ui` 用**旧图切出的槽位**覆盖正确产物（实测 `llama_armor.png` 88/324、`saddle.png` 118/324 像素不同） |
/// | `fix_ui_sub_hand` / `fix_ui_creative` | **前** | 同为「原位改写 GUI 图」，且计划槽位在阶段 1–2（最前）；放前阶段与生产顺序一致（`fix_slider` 读 `widgets.png`，两者区域不重叠但顺序仍应正确） |
///
/// 阶段判据（严格早于旧批次最小阶段 → 前阶段）继续兜底；本名单只用来**额外**授权提前。
const EARLY_NATIVES: [&str; 10] = [
    "generate_smithing_ui",
    "generate_copper_armor_models",
    "generate_netherite_armor_models",
    "generate_poplar_planks",
    "generate_tricky_trials_breeze",
    "generate_shulker_box_ui",
    "fix_slider",
    "fix_horse_ui",
    "fix_ui_sub_hand",
    "fix_ui_creative",
];

/// 原生任务相对**整批旧任务**的落点：之前还是之后。///
/// 旧批次不可拆分（§9.42），因此每个原生任务只有这两个位置可选；选哪一侧由
/// **依赖相对次序**决定（详见 `native_placements`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Early,
    Late,
}

/// 为每个**已派发**的原生任务决定落点（前阶段 / 后阶段）。
///
/// 判据（保留 §9.49 的阶段窗口，并补上显式名单）：
///
/// 判据（**不依赖任何"最小阶段"基准**，因此不会随迁移进度漂移）：
///
/// - 阶段 **== Eraser** → 前阶段。删除/改名类在生产顺序里就排最前；把它们挪到后面会让
///   早阶段生成的新格式产物**不再被删除**（§9.76 实测：产物多 57 个文件，读数第一处偏离
///   就是 `pre:delete_blockstates_models` 从首位消失）；
/// - 或者列在 [`EARLY_NATIVES`] 里（**实测证据**见该常量）→ 前阶段；
/// - 其余 → 后阶段。
///
/// **历史**：这条判据曾经写作「阶段**严格早于**旧批次里最小的阶段」。当时旧批次的基准是
/// `Architect`，Eraser 级恰好满足，于是"碰巧"正确；但该基准**会随迁移进度漂移**——
/// 真实包上派发 `generate_shulker_box_ui` 之后基准变成 `Surgeon`，`generate_boat` /
/// `generate_potion_lingering` / `generate_tipped_arrow_images` / `generate_furnace`…
/// 全部被挪到前阶段，立刻复现 §9.53 的 8 项 `OnlyInB`（§9.74 的逐步读数里直接可见）。
/// 改成「整批计划的最小阶段」又让 Eraser 级**不再满足** `stage < min`（因为 `min` 就是
/// `Eraser`），于是出现上面那条 57 个文件的偏差（§9.76）。两次实验合起来给出现在的形式。
///
/// 注意「同级或更晚一律提前」这类更"整齐"的判据**已被实测否决**：`generate_boat` 与
/// `rename_blocks_items` 一旦提前，真实包立刻分叉（§9.53 记录了 8 项 OnlyInB 的产物）。
fn native_placements(
    plan: &[String],
    switches: &NativeSwitches,
    scheduler: &crate::hurray::scheduler::Scheduler,
) -> std::collections::HashMap<String, Side> {
    // **Eraser 级原生任务必须最先跑**——它们是删除/改名类，生产顺序里就排在最前（阶段 1–12 之前）。
    //
    // 这一条**必须写成「阶段 == Eraser」而不是任何"最小阶段"比较**：§9.76 用逐步读数实测到，
    // 一旦把 Eraser 级原生任务挪到后阶段，早阶段生成的新格式产物（`poplar_*`、铜灯泡族、
    // `breeze_rod`…）就**不再被删除**，最终产物多出 57 个文件（`pre:` 读数的第一处偏离即
    // `pre:delete_blockstates_models` 从首位消失）。
    //
    // 之前写成「阶段**严格早于**旧批次里最小的阶段」时，Eraser 级恰好满足（旧批次的基准是
    // `Architect`），于是"碰巧"正确；而这个基准**会随迁移进度漂移**（§9.73/§9.75）。
    // 现在把它写成**不依赖任何基准**的显式判据：Eraser → 前阶段。
    let mut out = std::collections::HashMap::new();
    for name in plan {
        if native_for(name, switches).is_none() {
            continue;
        }
        let is_eraser = matches!(
            scheduler.task_tier(name),
            Some(crate::hurray::scheduler::TaskTier::Eraser)
        );
        let side = if is_eraser || EARLY_NATIVES.contains(&name.as_str()) {
            Side::Early
        } else {
            Side::Late
        };
        out.insert(name.clone(), side);
    }
    out
}

/// 给 `pilots` 的夹具正题用：按名字取**已启用全部开关**时的原生实现。
///
/// 夹具用例需要在不启动整条驱动的前提下直接跑单个原生任务（真实包覆盖不到那几个跳过分支）。
#[cfg(test)]
pub(crate) fn native_for_probe(
    name: &str,
) -> Option<(&'static str, TaskDecl, crate::pilots::PilotFn)> {
    // `generate_shulker_box_ui` 的实现在闭包里**但不在派发表里**（§9.51：它的正确位置在
    // Architect 旧任务之后、Surgeon 旧任务之前，那个位置要等 Surgeon 原生化后才存在）。
    // 夹具正题可以直接验证它的算法，因此这里额外暴露；生产路径不受影响。
    if name == "generate_shulker_box_ui" {
        return Some((
            "shulker_box_gen",
            crate::pilots::shulker_box_gen::decl(),
            crate::pilots::shulker_box_gen::run,
        ));
    }
    native_for(name, &NativeSwitches::all())
}

/// **逐步读数**（诊断用，§9.73 的建议）：每一步之后对 workdir 取一个廉价的指纹。
///
/// 目的：当真实包分叉时，能回答「**是哪一步先偏离**」，而不是只知道终点不同——
/// §9.73 卡住的原因正是缺这份读数。
///
/// 指纹 = workdir 的（文件数, 总字节, 名字+大小序列的 FNV-1a）。**不读文件内容**，
/// 所以很便宜；对「多一个文件 / 少一个文件 / 大小变了」这类分叉足够敏感。
/// 需要定位内容差异时，再看紧随其后的**按目录**细粒度读数。
fn digest_workdir(workdir: &Path) -> (usize, u64, u64) {
    let mut entries: Vec<(String, u64)> = Vec::new();
    let mut stack = vec![workdir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            match entry.file_type() {
                Ok(t) if t.is_dir() => stack.push(p),
                Ok(t) if t.is_file() => {
                    let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    let rel = p
                        .strip_prefix(workdir)
                        .map(|r| r.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_default();
                    entries.push((rel, len));
                }
                _ => {}
            }
        }
    }
    entries.sort();
    let files = entries.len();
    let bytes: u64 = entries.iter().map(|(_, len)| *len).sum();
    // FNV-1a over "path\0size\0"
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for (path, len) in &entries {
        for b in path.as_bytes().iter().chain(b"\0").chain(len.to_le_bytes().iter()).chain(b"\0") {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x1000_0000_01b3);
        }
    }
    (files, bytes, hash)
}

/// 逐步读数的细粒度一档（**已就绪、暂未接线**）：对**指定前缀**下的每个文件取
/// `名字→(大小, 内容 FNV-1a)`，用来在两份粗粒度读数之间做**逐文件**对比、
/// 直接指出分歧出现在哪些文件上。下一轮定位 §9.73 的分叉时会用到。
#[allow(dead_code)]
fn digest_scope(workdir: &Path, prefix: &str) -> Vec<(String, u64, u64)> {
    let root = workdir.join(prefix);
    let mut out = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            match entry.file_type() {
                Ok(t) if t.is_dir() => stack.push(p),
                Ok(t) if t.is_file() => {
                    let Ok(bytes) = std::fs::read(&p) else {
                        continue;
                    };
                    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
                    for b in &bytes {
                        hash ^= *b as u64;
                        hash = hash.wrapping_mul(0x1000_0000_01b3);
                    }
                    let rel = p
                        .strip_prefix(workdir)
                        .map(|r| r.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_default();
                    out.push((rel, bytes.len() as u64, hash));
                }
                _ => {}
            }
        }
    }
    out.sort();
    out
}

/// 已迁移任务的派发表：**任务名 → (标签, 声明, 原生实现)**。开关关闭即返回 `None`（走旧路径）。
///
/// **这里是「谁已经原生化」的唯一来源**：`native_placements`、`native_names`、前/后阶段三处
/// 都只通过 `native_for` 判断。§9.73 的教训：若另外单开一条分支返回实现（例如只给
/// `native_for_probe` 特判），就会出现「能执行但不算原生」的分裂状态，
/// `EARLY_NATIVES` 也随之对它失效——真实包立刻分叉。
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
        if name == "reverse_rename_mcpatcher_to_optifine" {
            return Some((
                "mcpatcher_optifine_reverse",
                crate::pilots::mcpatcher_optifine_reverse::decl(),
                crate::pilots::mcpatcher_optifine_reverse::run,
            ));
        }
        if name == "reverse_rename_blocks_items" {
            return Some((
                "rename_blocks_reverse",
                crate::pilots::rename_blocks_reverse::decl(),
                crate::pilots::rename_blocks_reverse::run,
            ));
        }
        if name == "reverse_fix_armor_models" {
            return Some((
                "reverse_armor",
                crate::pilots::reverse_armor::decl(),
                crate::pilots::reverse_armor::run,
            ));
        }
        if name == "reverse_fix_ui_survival" {
            return Some((
                "reverse_survival",
                crate::pilots::reverse_survival::decl(),
                crate::pilots::reverse_survival::run,
            ));
        }
        if let Some((decl, run)) = crate::pilots::reverse_compose::lookup(name) {
            return Some(("reverse_compose", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::reverse_pixels::lookup(name) {
            return Some(("reverse_pixels", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::reverse_defer_ui::lookup(name) {
            return Some(("reverse_defer_ui", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::reverse_defer_metal::lookup(name) {
            return Some(("reverse_defer_metal", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::reverse_defer_extra::lookup(name) {
            return Some(("reverse_defer_extra", decl, run));
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
    if switches.textures {
        if name == "generate_potion_lingering" {
            return Some((
                "potion_lingering_gen",
                crate::pilots::potion_lingering_gen::decl(),
                crate::pilots::potion_lingering_gen::run,
            ));
        }
        if let Some((decl, run)) = crate::pilots::arch_gen::lookup(name) {
            return Some(("arch_gen", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::arch_gen2::lookup(name) {
            return Some(("arch_gen2", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::arch_gen_metal::lookup(name) {
            return Some(("arch_gen_metal", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::arch_gen_planks::lookup(name) {
            return Some(("arch_gen_planks", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::arch_gen_breeze::lookup(name) {
            return Some(("arch_gen_breeze", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::arch_gen3::lookup(name) {
            return Some(("arch_gen3", decl, run));
        }
        // Surgeon 早期组：槽位在旧批次之前，因此与前阶段原生任务同侧（见 `EARLY_NATIVES`）。
        if let Some((decl, run)) = crate::pilots::surgeon_early::lookup(name) {
            return Some(("surgeon_early", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::surgeon_early2::lookup(name) {
            return Some(("surgeon_early2", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::surgeon_mid::lookup(name) {
            return Some(("surgeon_mid", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::surgeon_mid2::lookup(name) {
            return Some(("surgeon_mid2", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::surgeon_mid3::lookup(name) {
            return Some(("surgeon_mid3", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::surgeon_mid4::lookup(name) {
            return Some(("surgeon_mid4", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::surgeon_late::lookup(name) {
            return Some(("surgeon_late", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::surgeon_ui::lookup(name) {
            return Some(("surgeon_ui", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::surgeon_machinery::lookup(name) {
            return Some(("surgeon_machinery", decl, run));
        }
        if let Some((decl, run)) = crate::pilots::surgeon_survival::lookup(name) {
            return Some(("surgeon_survival", decl, run));
        }
        if name == "generate_shulker_box_ui" {
            return Some((
                "shulker_box_gen",
                crate::pilots::shulker_box_gen::decl(),
                crate::pilots::shulker_box_gen::run,
            ));
        }
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
        // `adapt_java_shaders`（§9.85）：**目标 pack_format 由任务自己从包里读**
        // （`run_from_pack`），因此不需要改驱动签名——真实包里没有 `shaders/`，
        // 生产路径上它是「源缺失 → 跳过」，与原实现一致。
        "adapt_java_shaders" => Some((
            "shader_adapt",
            crate::pilots::shader_adapt::decl(),
            crate::pilots::shader_adapt::run_from_pack,
        )),
        // `fix_smithing2_villager2_ui`（§9.87）：铁砧/村民 GUI 的第二步重排。
        "fix_smithing2_villager2_ui" => Some((
            "surgeon_smithing2",
            crate::pilots::surgeon_smithing2::decl(),
            crate::pilots::surgeon_smithing2::run,
        )),
        // `convert_animated_textures`（§9.88）：动画 mcmeta 升级。
        // 它此前**不在任何 plan 段**里（注册了却永不执行，见 §9.88），补段后本包会真的跑到它。
        "convert_animated_textures" => Some((
            "animated",
            crate::pilots::animated::decl(),
            crate::pilots::animated::run,
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

/// 契约检查：原生层**写出的文件**必须已经在 `workdir` 里就位。
///
/// 为什么值得一次 stat：驱动让 A-ROM 层与旧实现**共用同一个 workdir**，而
/// **直接步骤**（GuiSurgeon / cut_gui）与旧批次一样是直接读盘的。若某处漏了
/// `apply_layer_to_workdir`，它们会读到**上一阶段**的内容，产物静默分叉——
/// `generate_smithing_ui` 的 4 个 sprite 就是这样丢的（§9.52）。
/// 这里比对磁盘文件大小与层里 blob 的大小：漏写会命中「文件不存在」，
/// 写了旧内容多半命中「大小不符」，两者都直接**自报任务名与路径**。
fn check_layer_materialized(
    pack: &Pack,
    workdir: &Path,
    layer: &Layer,
    name: &str,
) -> Result<(), AromError> {
    for (path, slot) in layer.writes() {
        let Slot::Present(Body::Blob(id)) = slot else {
            continue;
        };
        let want = pack.blobs().get(*id)?.len() as u64;
        let full = workdir.join(path);
        match std::fs::metadata(&full) {
            Ok(meta) if meta.len() == want => {}
            Ok(meta) => {
                return Err(AromError::internal(format!(
                    "native task `{name}`: workdir copy of `{path}` is stale \
                     ({} bytes on disk, {want} bytes in the layer)",
                    meta.len()
                )));
            }
            Err(e) => {
                return Err(AromError::internal(format!(
                    "native task `{name}`: `{path}` is missing from the workdir after \
                     apply_layer_to_workdir ({e})"
                )));
            }
        }
    }
    Ok(())
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
        // 让失败自报身份：路径即配置标签（legacy.zip / v2_on.zip / v2_one_by_one.zip …）
        println!("compare a={} b={}", report.a, report.b);
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

    /// **放置规则的回归测试**：这条规则是本项目最贵的教训沉淀（§9.42/§9.48/§9.50/§9.52），
    /// 却一直没有单测护着——只有真实包闸门兜底（跑一次 40 秒）。这里用**真实计划**把它钉住。
    ///
    /// 断言三件事：
    /// 1. `EARLY_NATIVES` 里的任务确实被判为 `Early`（**名单不是装饰**）；
    /// 2. **Eraser 级的每个已派发原生任务都判为 `Early`**——这是 §9.76 定稿的判据，
    ///    且实测是**载荷**的：删到后阶段会让早阶段生成的产物不再被删除（多 57 个文件）；
    /// 3. 曾被误判的**必须留在 `Late`**：`fix_clock_compass`（§9.59 的 100 项差异）
    ///    与 `generate_boat`（§9.53 的 8 项 OnlyInB）——将来谁"顺手提前"会当场红。
    ///
    /// **历史（§9.73/§9.75/§9.76）**：判据曾经是「阶段严格早于旧批次最小阶段」，而那个基准
    /// 会随迁移进度漂移；改用「整批计划最小阶段」又让 Eraser 级不再满足 `stage < min`。
    /// 两次实验合起来给出现在的显式形式：**Eraser → 前阶段，名单 → 前阶段，其余 → 后阶段**。
    #[test]
    fn native_placement_rule_is_pinned_by_the_real_plan() {
        use crate::hurray::scheduler::Scheduler;

        let tmp = tempfile::tempdir().expect("tempdir");
        let input = tmp.path().join("fixture.zip");
        fixture(&input);

        let mut scheduler = Scheduler::new();
        crate::invoke_conversion::register_legacy_tasks(
            &mut scheduler, &input, 97, 1, true, false, true,
        );
        let plan = scheduler.plan(1, 97).expect("plan");

        // 只在本夹具能覆盖到这些任务时才断言（夹具的任务集合可能随版本映射变化）
        let placements = native_placements(&plan, &NativeSwitches::all(), &scheduler);
        if placements.is_empty() {
            println!("夹具计划里没有已派发的原生任务，跳过");
            return;
        }

        for name in EARLY_NATIVES {
            if let Some(side) = placements.get(name) {
                assert_eq!(
                    *side,
                    Side::Early,
                    "`{name}` 在 EARLY_NATIVES 里，必须判为 Early"
                );
            }
        }
        for name in ["fix_clock_compass", "generate_boat"] {
            if let Some(side) = placements.get(name) {
                assert_eq!(
                    *side,
                    Side::Late,
                    "`{name}` 必须判为 Late（提前会让真实包分叉，见 §9.53/§9.59）"
                );
            }
        }
        // **阶段判据的真实边界**（§9.76 定稿）：判据**不再依赖任何"最小阶段"基准**，
        // 而是直接的「阶段 == Eraser → 前阶段」。所以这里断言：凡是 Eraser 级的已派发原生任务
        // 都必须判为 Early——§9.76 的实测表明这条是**载荷**的：把它们挪到后阶段，
        // 早阶段生成的新格式产物就不再被删除，真实包产出会多 57 个文件。
        let mut saw_eraser = false;
        for name in &plan {
            if native_for(name, &NativeSwitches::all()).is_none() {
                continue;
            }
            let is_eraser = matches!(
                scheduler.task_tier(name),
                Some(crate::hurray::scheduler::TaskTier::Eraser)
            );
            if is_eraser {
                saw_eraser = true;
                assert_eq!(
                    placements.get(name),
                    Some(&Side::Early),
                    "`{name}` 是 Eraser 级，必须判为 Early（§9.76：删除类必须最先跑）"
                );
            }
        }
        assert!(saw_eraser, "计划里应当至少有一个 Eraser 级原生任务");
    }

    /// **逐步读数**（默认忽略，§9.73 的诊断工具）：
    /// `AROM_REAL_PACK=<包> cargo test --lib trace_stepwise -- --ignored --nocapture`
    ///
    /// 它做两件事：
    /// 1. **验证读数本身不改变产物**——打开 `step_trace` 会把旧批次改成「逐任务执行 + 逐任务收层」
    ///    （为了拿到 `legacy:<任务名>` 粒度的读数），所以先断言它与生产口径（一次性批量）**产物一致**；
    /// 2. 打印每一步的 `(文件数, 总字节, 名字指纹)`，用来回答「分叉是**哪一步**先发生的」。
    ///
    /// 对比两份读数时：前缀相同、从某一标签起指纹不同 ⇒ 该标签即**第一处偏离**。
    #[test]
    #[ignore]
    fn trace_stepwise_on_a_real_pack() {
        let Ok(src) = std::env::var("AROM_REAL_PACK") else {
            println!("AROM_REAL_PACK 未设置，跳过");
            return;
        };
        let input = PathBuf::from(&src);
        let source = {
            let pack = Pack::open_zip(&input, &SafeLimits::preserving_current(), None)
                .expect("open for source format");
            pack.view()
                .mcmeta()
                .ok()
                .and_then(|m| m.effective_format())
                .unwrap_or(34)
        };
        let target: u32 = std::env::var("AROM_TARGET")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(97);
        let tmp = tempfile::tempdir().expect("tempdir");

        // 生产口径
        let (prod, _) = mixed_v2_output(
            &input,
            tmp.path(),
            target,
            source,
            NativeSwitches::all(),
            false,
            "prod",
        );
        // 读数口径（step_trace=true → 旧批次逐任务收层）
        let work = tmp.path().join("v2_work_trace");
        let out = tmp.path().join("v2_trace.zip");
        let opts = MixedRunOptions {
            source_version: source,
            target_version: target,
            native: NativeSwitches::all(),
            step_trace: true,
            ..MixedRunOptions::default()
        };
        let report = run_mixed(&input, &work, &out, &opts, |dir| {
            let mcmeta = dir.join("pack.mcmeta");
            if mcmeta.exists() {
                write_pack_format(&mcmeta, target).map_err(|e| e.to_string())?;
            }
            Ok(())
        })
        .expect("traced run");

        println!("=== 逐步读数（label\tfiles\tbytes\thash）===");
        for (label, files, bytes, hash) in &report.step_trace {
            println!("{label}\t{files}\t{bytes}\t{hash:016x}");
        }
        println!("=== 读数结束（共 {} 步）===", report.step_trace.len());

        // 读数不得改变产物
        assert_equivalent(&prod, &out);
    }

    /// **`adapt_java_shaders` 的三分支正题**（默认忽略，§9.77 建议①）。
    ///
    /// 为什么必须单独测：真实包里 `shaders` 相关条目为 **0**，该函数在第一个判断
    /// （`!shaders.is_dir()`）就返回——**真实包闸门对它给的是假绿灯**（§9.77）。
    ///
    /// 本用例造一份含 `shaders/{core,post,post_effect,include}` 的夹具，对三个版本分支
    /// 各跑一次**旧实现** `adapt_java_shaders_at`，把结果落成**逐文件快照**并打印，
    /// 同时钉住每个分支的可观测语义（删了什么、改了什么）。它既是"旧行为"的基线记录，
    /// 也是后续原生移植的**对照标准**（移植后应能对同一夹具产出同样的快照）。
    #[test]
    #[ignore]
    fn adapt_java_shaders_three_branches_on_a_fixture() {
        use std::collections::BTreeMap;

        /// 造一份 1.20.1 风格（format 15）的 shader 包，覆盖各分支关心的结构。
        fn make_pack(root: &std::path::Path) {
            let s = root.join("assets/minecraft/shaders");
            for d in ["core", "post", "post_effect", "include"] {
                std::fs::create_dir_all(s.join(d)).expect("mkdir");
            }
            // core：成对 vsh+fsh（缺 json）、已有 json、旧名（→ 新名）、已移除名、未知名、共享 vsh
            std::fs::write(s.join("core/rendertype_entity.vsh"), "// v\n").unwrap();
            std::fs::write(s.join("core/rendertype_entity.fsh"), "// f\n").unwrap();
            std::fs::write(s.join("core/rendertype_text.json"), "{\"keep\":true}\n").unwrap();
            std::fs::write(s.join("core/rendertype_text.vsh"), "// tv\n").unwrap();
            std::fs::write(
                s.join("core/rendertype_entity_translucent.fsh"),
                "#moj_import <fog.glsl>\nvoid main(){}\n",
            )
            .unwrap();
            std::fs::write(s.join("core/unknown_thing.vsh"), "// unknown\n").unwrap();
            std::fs::write(s.join("core/screenquad.vsh"), "// shared\n").unwrap();
            std::fs::write(s.join("core/something.glsl"), "// glsl kept\n").unwrap();
            // core JSON：mat3（≥7 要升级为 mat4）、uniform 块（≥63 要剥）
            std::fs::write(
                s.join("core/rendertype_entity.json"),
                "{\n \"uniforms\": [{\"name\":\"ModelViewMat\",\"type\": \"mat3\"}],\n \"blend\": {}\n}\n",
            )
            .unwrap();
            std::fs::write(
                s.join("core/screenquad.json"),
                "{\n \"uniforms\": [{\"name\":\"ProjMat\",\"type\": \"mat4\"}],\n \"blend\": {}\n}\n",
            )
            .unwrap();
            // post / post_effect
            std::fs::write(s.join("post/blur.json"), "{\"targets\":[]}\n").unwrap();
            std::fs::write(s.join("post_effect/blur.json"), "{\"targets\":[]}\n").unwrap();
            // include：故意**不带**结尾空行，触发 include_newline_fixed
            std::fs::write(s.join("include/fog.glsl"), "// no trailing newline").unwrap();
        }

        fn snapshot(root: &std::path::Path) -> BTreeMap<String, String> {
            let mut out = BTreeMap::new();
            let mut stack = vec![root.join("assets/minecraft/shaders")];
            while let Some(dir) = stack.pop() {
                let Ok(rd) = std::fs::read_dir(&dir) else {
                    continue;
                };
                for e in rd.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else if let Ok(rel) = p.strip_prefix(root) {
                        let rel = rel.to_string_lossy().replace('\\', "/");
                        let body = std::fs::read_to_string(&p).unwrap_or_default();
                        out.insert(rel, body);
                    }
                }
            }
            out
        }

        for target in [1u32, 34, 97] {
            let tmp = tempfile::tempdir().expect("tempdir");
            make_pack(tmp.path());
            let before = snapshot(tmp.path());
            crate::converters::shaders::java::adapt_java_shaders_at(tmp.path(), target)
                .unwrap_or_else(|e| panic!("legacy adapt failed for {target}: {e}"));
            let after = snapshot(tmp.path());

            let mut removed: Vec<&String> =
                before.keys().filter(|k| !after.contains_key(*k)).collect();
            removed.sort();
            let mut changed: Vec<&String> = after
                .iter()
                .filter(|(k, v)| before.get(*k).map(|b| b != *v).unwrap_or(false))
                .map(|(k, _)| k)
                .collect();
            changed.sort();
            let mut added: Vec<&String> =
                after.keys().filter(|k| !before.contains_key(*k)).collect();
            added.sort();

            println!("=== target={target} ===");
            println!("  删除({}): {removed:?}", removed.len());
            println!("  修改({}): {changed:?}", changed.len());
            println!("  新增({}): {added:?}", added.len());

            // 钉住每个分支的可观测语义（读数来自本次实测，见 §9.78）
            match target {
                1 => {
                    assert!(
                        !after.keys().any(|k| k.contains("post_effect/")),
                        "legacy 目标应删掉 post_effect/：{after:?}"
                    );
                    assert!(
                        !after.keys().any(|k| k.ends_with(".json") && k.contains("core/")),
                        "legacy 目标应 strip 掉 core/ 下的 JSON"
                    );
                    assert!(
                        after.keys().any(|k| k.ends_with("core/rendertype_entity.vsh")),
                        "legacy 目标不该给 core 改名"
                    );
                }
                34 => {
                    // ≥7：JSON 里的 mat3 要升级为 mat4
                    let ent = after
                        .iter()
                        .find(|(k, _)| k.ends_with("core/rendertype_entity.json"))
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default();
                    assert!(
                        !ent.contains("matrix3x3"),
                        "target≥7 应把 JSON 的 matrix3x3 升级为 matrix4x4，实际：{ent:?}"
                    );
                    assert!(
                        after.keys().any(|k| k.ends_with("core/rendertype_entity.vsh")),
                        "target=34 仍用旧名，不该改名"
                    );
                }
                97 => {
                    assert!(
                        !after.keys().any(|k| k.contains("post/")),
                        "现代目标应删掉过时的 shaders/post/"
                    );
                    // 重命名组的**新名**上检查指令改写
                    let ent = after
                        .iter()
                        .find(|(k, _)| k.ends_with("core/entity.fsh"))
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default();
                    assert!(
                        ent.contains("#include") && !ent.contains("#moj_import"),
                        "target≥97 应把 #moj_import 改成 #include，实际：{ent:?}"
                    );
                }
                _ => {}
            }
        }
    }

    /// **`shader_adapt` 的逐函数对照**（默认忽略，§9.79）。
    ///
    /// 移植 `adapt_java_shaders` 时，最便宜也最硬的验证不是"跑一遍看差异"，
    /// 而是**逐函数在同一语料上比对**：本用例把四种真实写法的导入行/入口文件喂给
    /// 「原生移植版」与「旧实现」，逐字节比较输出与计数。
    ///
    /// 语料覆盖：`#moj_import <a/b.glsl>`、带 `include/` 前缀、带命名空间、引号形式、
    /// 无路径（不可解析）、CRLF、无结尾换行、以及 `ScreenSize`/`GameTime`/`globals.glsl` 的判定。
    #[test]
    #[ignore]
    fn shader_adapt_text_ops_match_the_legacy_implementation() {
        use crate::converters::shaders::java::legacy_text_ops as legacy;
        use crate::pilots::shader_adapt as native;

        let corpus: [&str; 12] = [
            "#moj_import <fog.glsl>\nvoid main(){}\n",
            "#moj_import <minecraft:fog.glsl>\n",
            "#moj_import <include/fog.glsl>\n",
            "#moj_import <ns:include/fog.glsl>\n",
            "#moj_import \"fog.glsl\"\n",
            "#moj_import \"/leading/fog.glsl\"\n",
            "#moj_import\n",
            "#moj_import   \n",
            "// no import here\n",
            "#moj_import <fog.glsl>\r\nvoid main(){}\r\n",
            "// no trailing newline at all",
            "vec2 ScreenSize; float GameTime;\n#include <globals.glsl>\n",
        ];

        let mut problems: Vec<String> = Vec::new();
        let mut checked = 0usize;
        for (i, src) in corpus.iter().enumerate() {
            let (ln, lc) = legacy::convert_moj_import_to_include(src);
            let (nn, nc) = native::convert_moj_import_to_include(src);
            if ln != nn || lc != nc {
                problems.push(format!(
                    "corpus[{i}] convert_moj_import_to_include 不同：legacy=({ln:?},{lc}) native=({nn:?},{nc})"
                ));
            }
            let (ln, lc) = legacy::namespace_moj_imports(src);
            let (nn, nc) = native::namespace_moj_imports(src);
            if ln != nn || lc != nc {
                problems.push(format!(
                    "corpus[{i}] namespace_moj_imports 不同：legacy=({ln:?},{lc}) native=({nn:?},{nc})"
                ));
            }
            if legacy::needs_globals_import(src) != native::needs_globals_import(src) {
                problems.push(format!("corpus[{i}] needs_globals_import 不同"));
            }
            if legacy::has_globals_import(src) != native::has_globals_import(src) {
                problems.push(format!("corpus[{i}] has_globals_import 不同"));
            }
            checked += 4;
        }

        // `rewrite_import_path` 单独列举（含非法输入）
        for rest in [
            "<a/b.glsl>",
            "<include/a.glsl>",
            "<ns:a.glsl>",
            "<ns:include/a.glsl>",
            "\"a.glsl\"",
            "\"/a.glsl\"",
            "a.glsl",
            "<>",
            "\"\"",
        ] {
            if legacy::rewrite_import_path(rest) != native::rewrite_import_path(rest) {
                problems.push(format!(
                    "rewrite_import_path({rest:?}) 不同：legacy={:?} native={:?}",
                    legacy::rewrite_import_path(rest),
                    native::rewrite_import_path(rest)
                ));
            }
            checked += 1;
        }

        println!("shader_adapt 逐函数对照：{checked} 项");
        assert!(problems.is_empty(), "逐函数对照差异：{problems:#?}");
    }

    /// **`shader_adapt` 的表格对照**（默认忽略，§9.80）。
    ///
    /// 移植的四张表是**纯数据**，但一个字符之差就会静默改变「删哪些 / 改成什么名」——
    /// 因此这里在**多个 target** 上把「原生表」与「旧表」排序后逐项比对
    /// （包括里程碑边界 7/32/46/63/84/97 的两侧）。
    #[test]
    #[ignore]
    fn shader_adapt_tables_match_the_legacy_implementation() {
        use crate::converters::shaders::java::legacy_text_ops as legacy;
        use crate::pilots::shader_adapt as native;

        let mut problems: Vec<String> = Vec::new();
        let mut checked = 0usize;
        // 覆盖各里程碑的两侧 + 常规值
        for target in [
            1u32, 6, 7, 8, 31, 32, 33, 45, 46, 47, 62, 63, 64, 83, 84, 85, 96, 97, 98, 120,
        ] {
            let a = legacy::modern_core_allowlist(target);
            let b = native::modern_core_allowlist(target);
            // 旧实现用 `HashSet`（天然去重），移植版用 `Vec` 并会重复 push（如 `block`）——
            // 因此这里比较的是**去重后的集合**（同一集合即语义相同；§9.80 实测到过这个差异）
            let a_set: std::collections::BTreeSet<&str> = a.iter().copied().collect();
            let b_set: std::collections::BTreeSet<&str> = b.iter().copied().collect();
            if a_set != b_set {
                let only_legacy: Vec<&&str> = a_set.difference(&b_set).collect();
                let only_native: Vec<&&str> = b_set.difference(&a_set).collect();
                problems.push(format!(
                    "target={target}: modern_core_allowlist 不同（旧独有 {only_legacy:?} / 原生独有 {only_native:?}）"
                ));
            }
            let a = legacy::core_rename_table(target);
            let b = native::core_rename_table(target);
            if a != b {
                problems.push(format!(
                    "target={target}: core_rename_table 不同（旧 {} 项 / 原生 {} 项）",
                    a.len(),
                    b.len()
                ));
            }
            let a = legacy::core_removed_stems(target);
            let b = native::core_removed_stems(target);
            if a != b {
                problems.push(format!("target={target}: core_removed_stems 不同"));
            }
            checked += 3;
        }
        // 与 target 无关的两张表
        if legacy::legacy_core_allowlist() != native::legacy_core_allowlist() {
            problems.push("legacy_core_allowlist 不同".to_string());
        }
        if legacy::shared_vertex_stems() != native::SHARED_VERTEX_STEMS {
            problems.push("SHARED_VERTEX_STEMS 不同".to_string());
        }
        checked += 2;

        println!("shader_adapt 表格对照：{checked} 项");
        assert!(problems.is_empty(), "表格对照差异：{problems:#?}");
    }

    /// **`shader_adapt` 的 JSON 文本对照**（默认忽略，§9.81）。
    ///
    /// 三个纯文本变换（`rewrite_json_matrix_types_text` / `remove_json_key` /
    /// `minimal_core_json`）在各式 JSON 写法上逐例与旧实现比对。
    /// `remove_json_key` 是**粗粒度**实现（按括号深度找值尾、吃掉后随逗号或删前导逗号），
    /// 边界多，因此语料刻意覆盖：值在中间/末尾、后随空白、无逗号、嵌套对象/数组、
    /// 字符串值、布尔/数字值、键不存在、只有键没有冒号。
    #[test]
    #[ignore]
    fn shader_adapt_json_ops_match_the_legacy_implementation() {
        use crate::converters::shaders::java::legacy_text_ops as legacy;
        use crate::pilots::shader_adapt as native;

        let corpus: [&str; 14] = [
            "{\n \"uniforms\": [{\"name\":\"A\",\"type\": \"mat3\"}],\n \"blend\": {}\n}\n",
            "{\n  \"uniforms\": [],\n  \"vertex\": \"v\"\n}\n",
            "{\n  \"vertex\": \"v\",\n  \"uniforms\": [],\n  \"fragment\": \"f\"\n}\n",
            "{\n  \"uniforms\": []\n}\n",
            "{\n  \"uniforms\": {\"a\": 1}\n}\n",
            "{\n  \"uniforms\": \"str\"\n}\n",
            "{\n  \"uniforms\": true\n}\n",
            "{\n  \"uniforms\": 12\n}\n",
            "{\n  \"a\": 1,\n  \"uniforms\": []\n}\n",
            "{\n  \"uniforms\"\n}\n",
            "{\n  \"other\": []\n}\n",
            "{ \"uniforms\": [ { \"nested\": [1,2] } ], \"x\": 1 }\n",
            "{\n \"uniforms\": [{\"name\":\"A\",\"type\":\"mat3\"}]\n}\n",
            "{\n \"uniforms\": [{\"name\":\"A\",\"type\": \"mat2\"}],\n \"defines\": {\"B\":\"1\"}\n}\n",
        ];

        let mut problems: Vec<String> = Vec::new();
        let mut checked = 0usize;
        for (i, src) in corpus.iter().enumerate() {
            let a = legacy::remove_json_key(src, "uniforms");
            let b = native::remove_json_key(src, "uniforms");
            if a != b {
                problems.push(format!("corpus[{i}] remove_json_key 不同：旧={a:?} 原生={b:?}"));
            }
            let a = legacy::rewrite_json_matrix_types_text(src);
            let b = native::rewrite_json_matrix_types_text(src);
            if a != b {
                problems.push(format!("corpus[{i}] mat 改写不同：旧={a:?} 原生={b:?}"));
            }
            checked += 2;
        }
        for stem in ["rendertype_entity", "screenquad", "x"] {
            if legacy::minimal_core_json(stem) != native::minimal_core_json(stem) {
                problems.push(format!("minimal_core_json({stem}) 不同"));
            }
            checked += 1;
        }
        // 键不存在时必须是原样返回
        let src = "{\n \"vertex\": \"v\"\n}\n";
        assert_eq!(native::remove_json_key(src, "uniforms"), src, "无该键应原样返回");

        println!("shader_adapt JSON 对照：{checked} 项");
        assert!(problems.is_empty(), "JSON 对照差异：{problems:#?}");
    }

    /// **`shader_adapt` 的 globals / fog 文本对照**（默认忽略，§9.82）。
    ///
    /// 覆盖移植第四步新增的三个纯函数：
    /// `has_globals_import`（**逐行**判定，不是子串）、`inject_globals_import`（插到第一个
    /// 非空非注释行之前，全注释则追加）、`count_args_likely_three`（按括号深度数顶层逗号）
    /// 与 `fog_note_if_needed`（在文件开头插一行标记）。
    #[test]
    #[ignore]
    fn shader_adapt_globals_and_fog_ops_match_the_legacy() {
        use crate::converters::shaders::java::legacy_text_ops as legacy;
        use crate::pilots::shader_adapt as native;

        let corpus: [&str; 10] = [
            "void main(){}\n",
            "// leading comment\nvoid main(){}\n",
            "/* block */\nvoid main(){}\n",
            " * continuation\nvoid main(){}\n",
            "\n\n// only comments\n",
            "",
            "// globals.glsl mentioned in a comment\nvoid main(){}\n",
            "#moj_import <minecraft:include/globals.glsl>\nvoid main(){}\n",
            "#include <minecraft:globals.glsl>\nvoid main(){}\n",
            "uniform vec2 ScreenSize;\nvoid main(){}\n",
        ];

        let mut problems: Vec<String> = Vec::new();
        let mut checked = 0usize;
        for (i, src) in corpus.iter().enumerate() {
            if legacy::has_globals_import(src) != native::has_globals_import(src) {
                problems.push(format!(
                    "corpus[{i}] has_globals_import 不同：旧={} 原生={}",
                    legacy::has_globals_import(src),
                    native::has_globals_import(src)
                ));
            }
            for use_include in [false, true] {
                let a = legacy::inject_globals_import(src, use_include);
                let b = native::inject_globals_import(src, use_include);
                if a != b {
                    problems.push(format!(
                        "corpus[{i}] inject_globals_import(include={use_include}) 不同：旧={a:?} 原生={b:?}"
                    ));
                }
                checked += 1;
            }
            if native::needs_globals_import(src) != legacy::needs_globals_import(src) {
                problems.push(format!("corpus[{i}] needs_globals_import 不同"));
            }
            checked += 2;
        }

        // fog：三参 / 两参 / 单参 / 已标记 / 嵌套括号
        for src in [
            "float d = fog_distance(Pos, FogStart, FogEnd);\n",
            "float d = fog_distance(Pos, FogStart);\n",
            "float d = fog_distance(a);\n",
            "// 2PYR: fog_distance\nd = fog_distance(a,b,c);\n",
            "d = fog_distance(f(a,b), c, d);\n",
            "d = fog_distance(f(a,b));\n",
        ] {
            if legacy::count_args_likely_three(src, "fog_distance")
                != native::count_args_likely_three(src, "fog_distance")
            {
                problems.push(format!("count_args_likely_three 不同：{src:?}"));
            }
            for target in [31u32, 32, 97] {
                let (a, ca) = {
                    // 旧侧等价物：walk_dir 里那段的直接复刻
                    let hit = target >= 32
                        && src.contains("fog_distance(")
                        && legacy::count_args_likely_three(src, "fog_distance")
                        && !src.contains("2PYR: fog_distance");
                    if hit {
                        (
                            format!(
                                "// 2PYR: fog_distance() 1.20.5+ 签名变更，请对照 vanilla fog.glsl\n{}",
                                src
                            ),
                            true,
                        )
                    } else {
                        (src.to_string(), false)
                    }
                };
                let (b, cb) = native::fog_note_if_needed(src, target);
                if a != b || ca != cb {
                    problems.push(format!("fog_note_if_needed(target={target}) 不同：{src:?}"));
                }
                checked += 1;
            }
            checked += 1;
        }

        println!("shader_adapt globals/fog 对照：{checked} 项");
        assert!(problems.is_empty(), "globals/fog 对照差异：{problems:#?}");
    }

    /// **`shader_adapt` 骨架在夹具上与旧实现对照**（默认忽略，§9.83）。
    ///
    /// 骨架（改名组 / include 结尾空行 / 源码遍历）没有纯函数可逐例比对，
    /// 因此这里在**与 §9.78 同一份夹具**上跑两侧：
    /// - 旧侧：`adapt_java_shaders_at(root, target)`（完整实现，写盘）；
    /// - 原生侧：把夹具塞进 `Pack`，跑 `shader_adapt::run`，再物化回目录。
    ///
    /// **已知差异必须被显式解释**：原生是**分步移植**，尚未做
    /// 「删除/补 JSON/剥 uniforms/post 路径」，所以旧侧会**多删**一些文件。
    /// 因此断言分两部分：
    /// 1. **原生不得碰**它不该碰的文件（只允许出现它已移植的改动）；
    /// 2. 旧侧的删除集合**必须**是「原生未实现的那几类」——用白名单核对，
    ///    这样一旦有人误把未移植的行为也做进去（或反过来漏掉已移植的），用例会红。
    #[test]
    #[ignore]
    fn shader_adapt_skeleton_matches_legacy_on_the_fixture() {
        use std::collections::BTreeMap;

        fn make_pack(root: &std::path::Path) {
            let s = root.join("assets/minecraft/shaders");
            for d in ["core", "post", "post_effect", "include"] {
                std::fs::create_dir_all(s.join(d)).expect("mkdir");
            }
            std::fs::write(s.join("core/rendertype_entity.vsh"), "// v\n").unwrap();
            std::fs::write(s.join("core/rendertype_entity.fsh"), "// f\n").unwrap();
            std::fs::write(s.join("core/rendertype_text.json"), "{\"keep\":true}\n").unwrap();
            std::fs::write(s.join("core/rendertype_text.vsh"), "// tv\n").unwrap();
            std::fs::write(
                s.join("core/rendertype_entity_translucent.fsh"),
                "#moj_import <fog.glsl>\nvoid main(){}\n",
            )
            .unwrap();
            std::fs::write(s.join("core/unknown_thing.vsh"), "// unknown\n").unwrap();
            std::fs::write(s.join("core/screenquad.vsh"), "// shared\n").unwrap();
            std::fs::write(s.join("core/something.glsl"), "// glsl kept\n").unwrap();
            std::fs::write(
                s.join("core/rendertype_entity.json"),
                "{\n \"uniforms\": [{\"name\":\"ModelViewMat\",\"type\": \"mat3\"}],\n \"blend\": {}\n}\n",
            )
            .unwrap();
            std::fs::write(
                s.join("core/screenquad.json"),
                "{\n \"uniforms\": [{\"name\":\"ProjMat\",\"type\": \"mat4\"}],\n \"blend\": {}\n}\n",
            )
            .unwrap();
            std::fs::write(s.join("post/blur.json"), "{\"targets\":[]}\n").unwrap();
            std::fs::write(s.join("post_effect/blur.json"), "{\"targets\":[]}\n").unwrap();
            std::fs::write(s.join("include/fog.glsl"), "// no trailing newline").unwrap();
        }

        fn snapshot(root: &std::path::Path) -> BTreeMap<String, String> {
            let mut out = BTreeMap::new();
            let mut stack = vec![root.join("assets/minecraft/shaders")];
            while let Some(dir) = stack.pop() {
                let Ok(rd) = std::fs::read_dir(&dir) else {
                    continue;
                };
                for e in rd.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else if let Ok(rel) = p.strip_prefix(root) {
                        let rel = rel.to_string_lossy().replace('\\', "/");
                        out.insert(rel, std::fs::read_to_string(&p).unwrap_or_default());
                    }
                }
            }
            out
        }

        let mut problems: Vec<String> = Vec::new();
        let mut checked = 0usize;
        for target in [1u32, 34, 97] {
            // 旧侧
            let tmp_legacy = tempfile::tempdir().expect("tempdir");
            make_pack(tmp_legacy.path());
            let before = snapshot(tmp_legacy.path());
            crate::converters::shaders::java::adapt_java_shaders_at(tmp_legacy.path(), target)
                .expect("legacy");
            let after_legacy = snapshot(tmp_legacy.path());

            // 原生侧：夹具 → Pack → run → 物化
            let tmp_native = tempfile::tempdir().expect("tempdir");
            make_pack(tmp_native.path());
            let zip_path = tmp_native.path().join("shaders_fixture.zip");
            {
                use std::io::Write as _;
                let file = std::fs::File::create(&zip_path).expect("create");
                let mut zip = zip::ZipWriter::new(file);
                let opts = zip::write::FileOptions::default();
                let mut add = |name: String, body: Vec<u8>| {
                    zip.start_file(name, opts).expect("start");
                    zip.write_all(&body).expect("write");
                };
                for (rel, body) in &before {
                    add(rel.clone(), body.clone().into_bytes());
                }
                zip.finish().expect("finish");
            }
            let mut pack = Pack::open_zip(&zip_path, &SafeLimits::preserving_current(), None)
                .expect("open fixture");
            {
                let mut tx = pack.tx("adapt_java_shaders");
                crate::pilots::shader_adapt::run(&mut tx, target).expect("native run");
                pack.commit(tx.into_layer());
            }
            let out_dir = tmp_native.path().join("out");
            std::fs::create_dir_all(&out_dir).expect("mkdir");
            crate::arom::pathview::materialize(&pack.view(), &out_dir).expect("materialize");
            let after_native = snapshot(&out_dir);

            // ① 原生只允许改动它已移植的部分：逐文件比对前先分类
            let mut native_changed: Vec<&String> = after_native
                .iter()
                .filter(|(k, v)| before.get(*k).map(|b| b != *v).unwrap_or(false))
                .map(|(k, _)| k)
                .collect();
            native_changed.sort();
            let mut native_removed: Vec<&String> = before
                .keys()
                .filter(|k| !after_native.contains_key(*k))
                .collect();
            native_removed.sort();
            let mut native_added: Vec<&String> = after_native
                .keys()
                .filter(|k| !before.contains_key(*k))
                .collect();
            native_added.sort();

            let mut legacy_removed: Vec<&String> = before
                .keys()
                .filter(|k| !after_legacy.contains_key(*k))
                .collect();
            legacy_removed.sort();

            println!(
                "target={target}: 原生 改{} 删{} 增{} | 旧 改{} 删{} 增{}",
                native_changed.len(),
                native_removed.len(),
                native_added.len(),
                after_legacy
                    .iter()
                    .filter(|(k, v)| before.get(*k).map(|b| b != *v).unwrap_or(false))
                    .count(),
                legacy_removed.len(),
                after_legacy.keys().filter(|k| !before.contains_key(*k)).count()
            );

            // **完整对照**（移植补齐后）：两侧的快照必须逐项相同——
            // 条目集合（增/删）+ 每个文件的内容。
            let native_keys: std::collections::BTreeSet<&String> = after_native.keys().collect();
            let legacy_keys: std::collections::BTreeSet<&String> = after_legacy.keys().collect();
            if native_keys != legacy_keys {
                let only_native: Vec<&&String> = native_keys.difference(&legacy_keys).collect();
                let only_legacy: Vec<&&String> = legacy_keys.difference(&native_keys).collect();
                problems.push(format!(
                    "target={target}: 条目集合不同（仅原生 {only_native:?} / 仅旧 {only_legacy:?}）"
                ));
            }
            for (k, v) in &after_native {
                if let Some(lv) = after_legacy.get(k) {
                    if lv != v {
                        problems.push(format!("target={target}: {k} 内容不同"));
                    }
                }
            }
            checked += 1;
        }

        // 顺带钉住 §9.78 记录的基线（防止旧实现本身被改动而无人察觉）
        {
            let tmp = tempfile::tempdir().expect("tempdir");
            make_pack(tmp.path());
            let before = snapshot(tmp.path());
            crate::converters::shaders::java::adapt_java_shaders_at(tmp.path(), 97).expect("legacy");
            let after = snapshot(tmp.path());
            let removed = before.keys().filter(|k| !after.contains_key(*k)).count();
            let added = after.keys().filter(|k| !before.contains_key(*k)).count();
            assert_eq!((removed, added), (9, 3), "§9.78 的 target=97 基线变了");
            checked += 1;
        }

        println!("shader_adapt 骨架夹具对照：{checked} 项");
        assert!(problems.is_empty(), "骨架对照问题：{problems:#?}");
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

    /// **真实包上的绝对产物契约**（默认忽略，§9.91）。
    ///
    /// **为什么必须有这个用例**：其余真实包用例都是「legacy vs mixed」的**相对**对照——
    /// 如果某个改动让**两条路径一起**少产出文件，它们仍然彼此相等、全部报绿。
    /// §9.91 实测到过这个盲区：把 `cut_gui` 移出计划后产物从 **4018 文件 / 19294735 字节**
    /// 掉到 **4015 / 19293011**（少 3 个文件、1724 字节），而**全部 18 个忽略用例依然通过**。
    ///
    /// 因此这里把**绝对值**钉住（文件数 / 字节数）。这些数字是「TapL 16x + 目标 97」这一
    /// 固定输入的**已确认基线**；它们变化时应当**先解释清楚为什么**，再更新本用例。
    #[test]
    #[ignore]
    fn real_pack_absolute_output_is_pinned() {
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

        let tmp = tempfile::tempdir().expect("tempdir");
        let (_mixed, report) = mixed_output(&input, tmp.path(), target, source);
        let stats = &report.stats;

        println!(
            "绝对产物：files={} bytes={}（输入 {} → 目标 {}）",
            stats.files, stats.bytes, source, target
        );

        // **已确认基线**（TapL 16x → 97）。改动此处前必须先解释产物为何变化。
        assert_eq!(
            stats.files, 4018,
            "产物文件数偏离已确认基线 4018（实际 {}）——\n\
             这通常是「某个任务不再被执行」或「某个产物不再生成」，\n\
             而相对对照（legacy vs mixed）**不会**发现这类问题（见本用例文档注释）。",
            stats.files
        );
        assert_eq!(
            stats.bytes, 19294735,
            "产物字节数偏离已确认基线 19294735（实际 {}）",
            stats.bytes
        );
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
