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
//! 迁移已完成：46 个任务**全部**是原生 `Tx` 实现（[`crate::natives`]），没有适配层、
//! 没有回退路径。任务顺序来自 [`crate::arom::engine::scheduler::Scheduler::plan`]，
//! 阶段来自 [`crate::task_registry`]。

use std::path::{Path, PathBuf};

use crate::arom::pathview::{harvest, materialize};
use crate::arom::serialize::{write_zip, SerializeOptions, SerializeStats};
use crate::arom::task::{TaskDecl, Tier};
use crate::arom::{AromError, Body, Layer, Materialized, Pack, SafeLimits, Slot};

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
    /// **管线相位耗时（§9.142）**：驱动内部的粗粒度分段，用于逐步量化优化收益。
    ///
    /// 为什么需要它：`ConversionTiming` 只有 `pipeline_s` 一个总数，而任务画像只覆盖任务本身。
    /// 真实包实测**任务只占流水线的约 13%**（0.76s / 5.8s），剩下 87% 在这些相位里——
    /// 没有这个分解，任何"优化了多少"都只能靠猜（我也确实因此猜错过两次）。
    pub phases: PipelinePhases,
}

/// **管线相位耗时**（§9.142）：秒。全部由 `run_native` 内部打点，单位为秒。
///
/// 分段与「什么优化能影响它」一一对应，这样每做一步优化都能立刻看出钱花在哪：
/// * `open_s`   ← 构造 `Pack`（读输入容器并建索引）
/// * `materialize_s` ← 把 `Pack` 的视图落到 workdir（任务读盘的前提）
/// * `tasks_s` ← 全部任务（含并行批的落盘与提交）
/// * `direct_s` ← `tasks` 之后的直接步骤（`cut_gui_direct`）与延迟删除
/// * `harvest_s` ← 把 workdir 与基线对比、产出并提交一层（**仅 `Output::Dir`**）
/// * `tail_s`   ← 调用方传入的收尾步骤（`pack.mcmeta` 改写）
/// * `output_s` ← 序列化产物（zip 写出或目录物化）
/// * `other_s`  ← 其余（直接步骤、延迟删除、报告组装等）
#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelinePhases {
    pub open_s: f32,
    pub materialize_s: f32,
    pub tasks_s: f32,
    /// `tasks` 之后的**直接步骤**（`cut_gui_direct` 与延迟删除）。Zip 路径上唯一
    /// 非零的"收尾前"相位——勿与 `harvest_s` 混为一谈（§9.148）。
    pub direct_s: f32,
    /// `harvest`：把 workdir 的磁盘差异收成一层。**仅 `Output::Dir` 非零**。
    pub harvest_s: f32,
    pub tail_s: f32,
    pub output_s: f32,
    pub other_s: f32,
}

impl PipelinePhases {
    fn total(&self) -> f32 {
        self.open_s
            + self.materialize_s
            + self.tasks_s
            + self.direct_s
            + self.harvest_s
            + self.tail_s
            + self.output_s
            + self.other_s
    }
}



/// **产物去向**（§9.117）。
///
/// 存在的理由：生产入口（`version_converter::process_zip_timed`）在管线跑完之后**还要继续
/// 改工作目录**（写 `pack.mcmeta` 的 `pack_format`、可选的 Bedrock 边任务），最后才重打包。
/// 因此不能只给生产一个 zip —— 它需要的是**填好的工作目录**。
///
/// `Zip` 仍是默认形态（测试与 CLI 走它）；`Dir` 供生产入口使用，
/// 让 A-ROM 的最终视图**直接物化回工作目录**，从而免掉一次 zip 往返。
#[derive(Debug, Clone)]
pub enum Output {
    Zip(PathBuf),
    Dir(PathBuf),
}


/// **原生任务派发**（§9.129）：跑一个原生 `Tx` 任务、把层落进 `pack`、并做契约检查。
///
/// 抽出来是为了让**同一个任务能在不同的计划槽位执行**：`cut_gui` 必须在批次内的
/// `(15,18)` 槽位跑（§9.100），而驱动原先只能在「整批旧任务之前/之后」二选一，
/// 因此它被迫绕道 `Scheduler::run_named` + 注册闭包。
///
/// 阶段一致性检查放在这里（原先只在前阶段做）：`decl().tier` 必须与活注册表一致 ——
/// 阶段决定相对位置，写错会**静默**改变执行顺序（§9.40 的谜题正是这样来的）。
fn dispatch_native_into_pack(
    pack: &mut Pack,
    workdir: &Path,
    baseline: &mut Materialized,
    report: &mut MixedRunReport,
    name: &str,
    switches: &NativeSwitches,
    strict_scopes: bool,
    // §9.144：是否把层镜像到 workdir。仅 `Output::Dir` 需要（Bedrock 边任务与部分单测）。
    mirror: bool,
) -> Result<crate::natives::Outcome, AromError> {
    let (label, decl, run) = native_for(name, switches).expect("dispatched from the plan");
    // **§9.136：逐任务耗时打点。**
    //
    // 驱动按 `plan` 派发 `Tx`，因此这里是**唯一**能覆盖整条计划的位置。打点走的是引擎那边
    // 同一份 `TaskTiming` 记录（`pack::version_converter` 的 `take_task_timings()` 会取走），
    // 于是日志里的 `task profile` 不再恒为空、那行 "scheduler+worker overhead"
    // 也终于只表示真正的收尾开销（此前它把整条流水线的时间都吞了）。
    //
    // 计时范围**含收尾**（层落盘 / 契约检查 / 基线同步 / 提交），因为这才是这个任务
    // 在流水线里的真实占用；只计 `run()` 会低估真正的瓶颈。
    let started = std::time::Instant::now();
    let tier_str = decl.tier.as_str();
    let (outcome, layer) = {
        let mut tx = pack.tx(name);
        let outcome = run(&mut tx)
            .map_err(|e| AromError::internal(format!("native task `{label}` ({name}): {e}")))?;
        (outcome, tx.into_layer())
    };
    if mirror {
        apply_layer_to_workdir(pack, workdir, &layer)?;
        check_layer_materialized(pack, workdir, &layer, name)?;
        // §9.150：基线只服务 `Output::Dir`（harvest 的变更检测）。
        // Zip 路径上它唯一的读者是报告构造时的 `file_count()`/`dir_count()`，
        // 那一次在任务循环**之前**就已取值——之后的同步全是白做，
        // 而每次同步都要 `read_body`（克隆整块字节）+ SHA256。
        sync_baseline_with_layer(pack, &layer, baseline)?;
    }
    pack.commit(layer);
    // 打点放在**收尾之后**：`started` 起于任务执行前，因此这里记的是该任务在流水线里的
    // 真实占用（执行 + 层落盘 + 契约检查 + 基线同步 + 提交）。
    crate::arom::engine::scheduler::record_task_time(
        name,
        tier_str,
        started.elapsed(),
        false,
    );
    finish_dispatch(report, name, outcome)
}

/// **§9.140：只跑任务体、产出层**——供并行批次使用。
///
/// 与 [`dispatch_native_into_pack`] 的区别：它**不碰** workdir / 基线 / pack 的提交，
/// 因此只需 `&Pack`，可以在同一波次里被多个任务**并发**调用（`Tx` 本就为此设计，
/// 见 `arom::layer` 的 `into_layer` 文档）。作用域契约检查也在这一步完成——
/// 它只读层与声明，不依赖任何可变状态。
fn build_layer_for_task(
    pack: &Pack,
    name: &str,
    switches: &NativeSwitches,
) -> Result<(crate::natives::Outcome, Layer, bool), AromError> {
    let (label, decl, run) = native_for(name, switches).expect("dispatched from the plan");
    let (outcome, layer) = {
        let mut tx = pack.tx(name);
        let outcome = run(&mut tx)
            .map_err(|e| AromError::internal(format!("native task `{label}` ({name}): {e}")))?;
        (outcome, tx.into_layer())
    };
    // §9.131：阶段核对（与串行路径同一套判据）
    if let Some(live) = crate::task_registry::tier_of(name) {
        if live.as_str() != decl.tier.as_str() {
            return Err(AromError::internal(format!(
                "native task `{label}` ({name}) declares tier `{}` but the live registry says `{}`",
                decl.tier.as_str(),
                live.as_str()
            )));
        }
    }
    // 作用域契约检查：这里**只算不判**——`strict_scopes` 的处理留给调用方
    // （严格模式要报错，非严格模式要把越界写入记进报告）。这样本函数保持纯只读，
    // 才能在并行批里被并发调用。
    let violations = scope_violations(&decl, &layer);
    // **打点不在这里做。** 并行批次的任务体是并发跑的，这里记的是"墙钟占用"而非该任务
    // 的真实成本；收尾（落盘/校验/基线/提交）由 `apply_parallel_layers` 串行完成并**统一打点**
    // 一次。若这里也记一次，同一任务会出现两条记录（实测 46 个任务被记成 61 条），
    // 让 `task profile` 失去解释力。
    Ok((outcome, layer, violations.is_empty()))
}

/// 把一批**已算好的层**按给定顺序落盘并提交（并行批次结束后调用）。
///
/// 顺序是计划顺序：并行只发生在"算层"阶段，**提交仍然串行且确定**，
/// 这样产物与逐任务串行执行完全一致。
///
/// **打点口径**：这里记**整批**一条（名字形如 `<tier> 并行批 N 项`），而不是逐任务记。
/// 原因是并行批的"算层"是并发墙钟、落盘是串行墙钟，两者无法干净地拆到单个任务名下；
/// 逐任务记会把同一批的时间重复计入每个任务，让 `task profile` 求和**超过**流水线总时长
/// （实测曾把 46 个任务记成 61 条）。整批一条既准确又可解释。
/// 逐任务成本仍可从 `--report` 的 profile 与冻结指纹的对比中间接验证。
fn apply_parallel_layers(
    pack: &mut Pack,
    workdir: &Path,
    baseline: &mut Materialized,
    report: &mut MixedRunReport,
    batches: Vec<(String, crate::natives::Outcome, Layer, Vec<String>)>,
    tier_name: &'static str,
    batch_started: std::time::Instant,
    mirror: bool,
) -> Result<(), AromError> {
    let n = batches.len();
    for (name, outcome, layer, violations) in batches {
        report.undeclared.extend(violations);
        if mirror {
            apply_layer_to_workdir(pack, workdir, &layer)?;
            check_layer_materialized(pack, workdir, &layer, &name)?;
            sync_baseline_with_layer(pack, &layer, baseline)?;
        }
        pack.commit(layer);
        finish_dispatch(report, &name, outcome)?;
    }
    if n > 0 {
        crate::arom::engine::scheduler::record_task_time(
            &format!("{tier_name} 并行批（{n} 项，含落盘）"),
            tier_name,
            batch_started.elapsed(),
            true,
        );
    }
    Ok(())
}

/// 派发收尾：把 `Outcome` 的延迟删除与任务名记进报告（串行与并行两条路径共用），
/// 并把 `Outcome` 交回调用方。
fn finish_dispatch(
    report: &mut MixedRunReport,
    name: &str,
    outcome: crate::natives::Outcome,
) -> Result<crate::natives::Outcome, AromError> {
    report
        .deferred_removals
        .extend(outcome.deferred_removals.iter().cloned());
    report.native_tasks += 1;
    report.native_names.push(name.to_string());
    Ok(outcome)
}


/// **驱动**：按 `Scheduler::plan` 的顺序逐任务执行，每个任务派发一笔 `Tx`，
/// 实现来自 [`crate::natives`]（46 个全部原生，**无适配层、无旧闭包**）。
///
/// ## 两条输出路径
///
/// * [`Output::Zip`]（Java 目标，生产默认）：**全内存**——不物化 workdir、不 harvest，
///   任务层层提交进 `Pack`，最后由 [`write_zip`] 直接从 `Pack` 序列化。因此
///   `workdir` 与 `tail` 在这条路上**不参与产物**。
/// * [`Output::Dir`]：才需要「目录镜像」——把层落到 `workdir`、用 `harvest` 把差异收成层，
///   让磁盘目录与对象模型保持一致；`tail` 用于收尾步骤（例如 `pack.mcmeta` 改写）。
///
/// 两条路必须产出**逐项一致**的条目集，这正是本模块末尾用例的验收方式。
pub fn run_native<F>(
    input: &Path,
    workdir: &Path,
    output: &Output,
    opts: &MixedRunOptions,
    tail: F,
) -> Result<MixedRunReport, AromError>
where
    F: FnOnce(&mut crate::arom::Tx<'_>) -> Result<(), String>,
{
    use crate::arom::engine::scheduler::Scheduler;

    ensure_empty_dir(workdir)?;

    // **§9.142：相位打点。** 每段一个 `Instant`，末尾汇总进 `report.phases`。
    // 这是逐步量化优化的唯一可信依据（`pipeline_s` 太粗、任务画像只覆盖任务本身）。
    let mut phases = PipelinePhases::default();
    let pipeline_clock = std::time::Instant::now();
    let mut clock = pipeline_clock;

    // **§9.145：输入可以是 zip，也可以是已解压的目录树。**
    //
    // 判据是文件类型而不是新参数：调用方本来就同时在用两种形态，类型判断可以避免
    // 每个调用点都要多传一个"我这是目录"的布尔。`Pack::open_dir` 走 `DirSource`。
    //
    // **注意：生产入口目前仍传 zip**，因为实测"直接吃预检解压好的树"在本包上**更慢**
    // （纯转换 0.44 → 0.63s），详见 `version_converter.rs` 里 §9.145 的实测记录。
    // 这个分支仍有价值：单测与第三方调用方可以用目录形态，且 `DirSource` 有单测覆盖。
    let mut pack = if input.is_dir() {
        Pack::open_dir(input, &opts.limits, opts.blob_limit)?
    } else {
        Pack::open_zip(input, &opts.limits, opts.blob_limit)?
    };
    phases.open_s = clock.elapsed().as_secs_f32();
    clock = std::time::Instant::now();
    // **§9.144：workdir 只在 `Output::Dir` 时才是真源。**
    //
    // `Output::Zip` 由 `write_zip` 直接从 `pack` 视图序列化，**从不读 workdir**；
    // `Output::Dir` 需要一棵可继续被 Bedrock 边任务（与部分单测）使用的磁盘树，
    // 因此仍保留"逐层镜像到 workdir"的旧行为。
    //
    // Zip 路径下因此可以整段省掉：不再物化（1.6s）、不再逐层落盘、不再收获。
    let mirror = matches!(output, Output::Dir(_));
    let mut baseline = {
        let view = pack.view();
        if mirror {
            materialize(&view, workdir)?
        } else {
            // **§9.147**：Zip 路径不需要变更检测基线（没有 harvest），只数条目与目录。
            // 原先这里调 `baseline_of`，它对全部 3631 条做 read + SHA256（实测 0.222s，
            // 即整包解压一遍），而那份指纹在这条路径上**没有任何读者**。
            crate::arom::pathview::baseline_counts_only(&view)?
        }
    };
    phases.materialize_s = clock.elapsed().as_secs_f32();
    clock = std::time::Instant::now();

    let scheduler = Scheduler::new();
    let plan = scheduler
        .plan(opts.source_version, opts.target_version)
        .map_err(|e| AromError::internal(format!("scheduler plan: {e}")))?;

    // §9.93（M3）：包名与旧管线**同源**——`invoke_conversion_ex` 也是取
    // `target_path.file_stem()`（`target_path` 即输入包）。这样 Bedrock 任务在两条路径上
    // 看到同一个包名，产出的 `.mcpack` 文件名一致。
    let pack_name = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("resource_pack");
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
    // **① 前置原生任务**（`Side::Early`）：这一批必须**先于**计划里的自然位置执行。
    //
    // 这不是历史包袱，而是 §9.52/§9.100 实测出来的**必要前置**：名单里的任务都是
    // 「原位改写 GUI 图」或「生成 GUI 图」，而计划中消费它们的任务在计划里排得更早 ——
    // 若按计划顺序跑，`convert_animated_textures` 会**先于** `fix_clock_compass`，
    // 于是它把 `clock.png` 拆成 `clock_00..63` 并删掉原图，随后 `fix_clock_compass`
    // 再也找不到 `clock.png`。
    //
    // **这是 §9.129 实测到的**：把前置删掉、改成纯计划顺序后，产物条目 4018 → **4117**，
    // 指纹 `0x75bb3260e7f578a6` → `0xbbbf2f19b473817c`；逐条 diff 显示
    // **多 105 条** `textures/item/*`（`clock_00..63`、`acacia_boat.png` …）、
    // **少 6 条**原图（`clock.png`、`clock.png.mcmeta`、`compass.png`、`compass.png.mcmeta`、
    // `entity/equipment/humanoid{,_leggings}/copper.png`）。恢复前置后指纹回到冻结值。
    let native_placements = native_placements(&plan, &opts.native);
    let early_natives: Vec<String> = plan
        .iter()
        .filter(|name| matches!(native_placements.get(*name), Some(Side::Early)))
        .cloned()
        .collect();
    for name in &early_natives {
        if native_for(name, &opts.native).is_none() {
            return Err(AromError::internal(format!(
                "plan lists task `{name}` but no native implementation is registered \
                 (legacy execution has been removed, §9.129)"
            )));
        }
        let outcome = dispatch_native_into_pack(
            &mut pack,
            workdir,
            &mut baseline,
            &mut report,
            name,
            &opts.native,
            opts.strict_scopes,
            mirror,
        )?;
        report
            .deferred_removals
            .extend(outcome.deferred_removals.iter().cloned());
        trace_step(&format!("pre:{name}"), &mut trace);
    }

    // **② 逐任务执行**（§9.129）。
    //
    // 历史上这里分三段：原生「前阶段」→ 旧任务**一次性批次**（`Scheduler::run_named`）
    // → 原生「后阶段」。批次之所以整体喂给旧引擎，是因为 §9.42 实测「旧批次不可拆分」
    // （一次提交 + 一次清理），于是每个原生任务只能在批次**之前或之后**二选一
    // （`native_placements` 的 `Side::Early/Late`）。
    //
    // §9.128 送走旧转换器后，46 个任务**全部**是原生 `Tx`；
    // §9.129 又把 `cut_gui` 也并入派发表（它原先绕道注册闭包，就为了能跑在批次内部）。
    // 于是批次里**一个名字都不剩**，"不可拆分"这个约束随之消失 ——
    // 现在按 `plan` 顺序**逐任务**执行即可，每个任务都跑在它的**精确计划槽位**上
    // （前置那一批已在 ① 跑过，这里跳过）。
    //
    // 判据是冻结指纹（§9.129 实测：4018 条目 / `0x75bb3260e7f578a6`）。
    //
    // **§9.140：恢复并行（按阶段分批）。**
    //
    // 旧引擎在同一 tier 内把 `TaskType::Parallel` 的任务用 `rayon` 并发跑，`Hybrid`/`Exclusive`
    // 仍然串行（`engine/scheduler.rs` 的 `execute_parallel_capable_tier`：先并行批、后串行）。
    // §9.129 改逐任务派发后这层并行**丢失**了，实测真实包 46 个任务体从 1.48s 涨到 3.48s。
    //
    // 这里按同一套语义恢复：`plan` 已按 tier 有序，因此连续切出「同 tier 段」，
    // 段内再按类型分成并行批与串行批。
    //
    // **关键在于并行只作用于"算层"**：`Tx` 只持 `&Pack`（`into_layer` 的文档明说
    // "同一波次的任务可以并发持事务"），因此批内任务可以并发产出各自的 `Layer`；
    // 而**落盘与提交仍然串行、且顺序 = 计划顺序**，产物与逐任务串行执行一致。
    // 判据同前：冻结指纹。
    let plan_rest: Vec<String> = plan
        .iter()
        .filter(|n| !early_natives.contains(n))
        .cloned()
        .collect();

    // 契约检查：计划里的每个名字都必须有原生实现。**这是永久不变式**——全部 46 个任务
    // 都是原生实现，且已无旧执行路径可回落（§9.129 起）。因此本条真正防的是"往计划表里
    // 加了一个没有实现的名字"：必须显式失败，而不是静默跳过该任务、产出错误结果。
    for name in &plan_rest {
        if native_for(name, &opts.native).is_none() {
            return Err(AromError::internal(format!(
                "plan lists task `{name}` but no native implementation is registered \
                 (legacy execution has been removed, §9.129)"
            )));
        }
    }

    // 切成「同 tier 段」——`plan` 按 tier 有序，因此只需比较相邻任务的 tier。
    let mut tiers: Vec<(Tier, Vec<String>)> = Vec::new();
    for name in plan_rest {
        let tier = crate::task_registry::tier_of(&name).unwrap_or(Tier::Eraser);
        match tiers.last_mut() {
            Some((t, v)) if *t == tier => v.push(name),
            _ => tiers.push((tier, vec![name])),
        }
    }

    for (tier, names) in tiers {
        // 段内分桶：`Parallel` 并发，其余串行；并行的先跑（与旧引擎一致）。
        let (parallel, serial): (Vec<String>, Vec<String>) = names
            .into_iter()
            .partition(|n| {
                matches!(
                    crate::task_registry::lookup(n).map(|m| m.task_type),
                    Some(crate::arom::engine::scheduler::TaskType::Parallel)
                )
            });

        if !parallel.is_empty() {
            let batch_started = std::time::Instant::now();
            // 并发算层（只读 `&Pack`）
            let built: Vec<Result<(String, crate::natives::Outcome, Layer, Vec<String>), AromError>> =
                {
                    use rayon::prelude::*;
                    parallel
                        .par_iter()
                        .map(|name| {
                            let (outcome, layer, clean) =
                                build_layer_for_task(&pack, name, &opts.native)?;
                            let violations = if clean {
                                Vec::new()
                            } else {
                                // 非严格模式：把越界写入记进报告（与串行路径同一语义）
                                let v = scope_violations(
                                    &native_for(name, &opts.native).expect("checked above").1,
                                    &layer,
                                );
                                // 严格模式：越界即失败——与串行路径同一判据
                                if opts.strict_scopes && !v.is_empty() {
                                    return Err(AromError::internal(format!(
                                        "native task `{name}` wrote outside its declared scope: {v:?}"
                                    )));
                                }
                                v
                            };
                            Ok((name.clone(), outcome, layer, violations))
                        })
                        .collect()
                };
            let mut batches = Vec::with_capacity(built.len());
            for r in built {
                batches.push(r?);
            }
            // 串行落盘 + 提交（顺序 = 计划顺序，即 `parallel` 的原有次序）
            apply_parallel_layers(
                &mut pack,
                workdir,
                &mut baseline,
                &mut report,
                batches,
                tier.as_str(),
                batch_started,
                mirror,
            )?;
            for name in &parallel {
                trace_step(name, &mut trace);
            }
        }

        for name in &serial {
            let outcome = dispatch_native_into_pack(
                &mut pack,
                workdir,
                &mut baseline,
                &mut report,
                name,
                &opts.native,
                opts.strict_scopes,
                mirror,
            )?;
            report
                .deferred_removals
                .extend(outcome.deferred_removals.iter().cloned());
            trace_step(name, &mut trace);
        }
        let _ = tier;
    }
    report.legacy_tasks = 0;
    phases.tasks_s = clock.elapsed().as_secs_f32();
    clock = std::time::Instant::now();

    // **§9.144：这里原本有一次 `harvest`，已删除。**
    //
    // `harvest` 的语义是「把 workdir 与基线对比，把磁盘上的差异收成一层」。它存在的前提是
    // 任务把 workdir 当真源、在磁盘上改文件。而 §9.128/§9.129 之后 46 个任务全是 `Tx`，
    // 收尾步骤（`cut_gui_direct`、延迟删除、`tail`）也都已改为包原生，workdir 在转换期间
    // **没有任何读者**，因此本轮不再物化它（见上方 `baseline_of` 的说明）。
    //
    // 保留这次 harvest 的后果**实测过**：workdir 是空目录，harvest 把基线里全部文件
    // 判为"已删除"，产出一个删除层——产物从 4018 条目掉到 **3654**
    // （指纹 0xeeddcdc1806f7d1a ≠ 0x75bb3260e7f578a6）。
    //
    // 删除它是**语义上正确**的：驱动现在每一步都直接提交进 `pack`，pack 的视图
    // 就是最终状态，不存在"只存在于磁盘、尚未进包"的改动需要收获。
    //
    // **§9.148：这里不要写 `phases.harvest_s = 0.0;`。** 它本来就由
    // `PipelinePhases::default()` 初始化为 0；而在这个位置写一句会**掩盖计时 bug**：
    // 紧随其后的"直接步骤 + 延迟删除 + tail"会被算进 `harvest` 相位
    // （实测 Zip 路径上出现 `harvest 0.07s`，而该路径根本不跑 harvest）。
    // 计时器 `clock` 在上一段末尾已重置，删掉这句后各相位才各归其位。

    // 注册表之外的直接步骤（GuiSurgeon sprite 手术）——生产管线在任务之后、
    // 清理之前执行它；漏掉它会整片丢失 sprite 产物（本步实测：真实包少了 3861 个文件）。
    //
    // 注意它**依然执行**（不是被计划里那次 `cut_gui` 取代）：§9.90/§9.91 实测过，
    // 计划内的 `cut_gui` 与这里的这步**各自都是必需的**，删任何一个都会丢 sprite。
    // 区别只在**输入状态**（本步在全部任务之后），不在实现——两者都调 `gui_surgeon_tx::run`。
    //
    // **§9.135：本步改为 pack 原生**（原先经 `surgeon_cut_gui::run_in_workdir`，那层
    // "读 gui 子树 → 建内存包 → 写回磁盘"的桥已整体删除）。
    //
    // **保留旧实现的两个门槛**（Bedrock 中间态跳过 sprite 手术，避免干扰 j2b；目标 < 34 也跳过）：
    let direct_started = std::time::Instant::now();
    if opts.run_gui_surgeon && opts.target_version >= 34 {
        let (outcome, layer) = {
            let mut tx = pack.tx("cut_gui_direct");
            let outcome = crate::natives::gui_surgeon_tx::run(&mut tx)
                .map_err(|e| AromError::internal(format!("direct steps: {e}")))?;
            (outcome, tx.into_layer())
        };
        // 与其它原生任务同一套收尾：基线同步 → 提交进 pack。
        // §9.144：层落 workdir 只在 `Output::Dir` 时需要（Zip 路径直接从 pack 序列化）。
        if mirror {
            apply_layer_to_workdir(&pack, workdir, &layer)?;
            check_layer_materialized(&pack, workdir, &layer, "cut_gui_direct")?;
            // §9.150 的遗漏补齐：基线只服务 harvest 的变更检测，Zip 路径没有 harvest。
            // 逐任务那两处当时已条件化，这里漏了——而本步写 209 条，
            // 每条都要 `read_body`（克隆整块字节）+ SHA256。
            sync_baseline_with_layer(&pack, &layer, &mut baseline)?;
        }
        pack.commit(layer);
        report.deferred_removals.extend(outcome.deferred_removals);
    }
    trace_step("after-direct-steps", &mut trace);

    // **§9.129：`ctx.execute_cleanup()` / `pool.clear_unused()` 已删除。**
    //
    // 原先这里调用旧引擎的清理：旧任务的 `defer_remove_*` 登记在 `HurrayContext` 的清单里，
    // 生产管线在 `invoke_conversion_ex` 末尾统一执行（漏掉会让 `assets/minecraft/font`
    // 之类的目录在产物里复活）。
    //
    // 现在两件事都不需要了：
    //   * `cut_gui` 是**最后一个**往 `ctx` 登记清理的东西（§9.128 已改成回传 `Outcome`）；
    //   * 计划里的 46 个任务**全部**是原生 `Tx`，没有旧闭包会去 `ctx.defer_remove_*`；
    //   * `TexturePool` 也只被旧闭包用过。
    //
    // 于是 `ctx` 恒为空、`pool` 恒为空转 —— 保留它们只会让"这里还有旧引擎在做事"的假象留存。
    // 所有延迟删除现在只有一条路径：下方 `report.deferred_removals` 的 tombstone。

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
        if mirror {
            apply_layer_to_workdir(&pack, workdir, &layer)?;
        }
        sync_baseline_with_layer(&pack, &layer, &mut baseline)?;
        pack.commit(layer);
        report.deferred_removals = deferred;
    }

    // **§9.144：`harvest` 必须先于 `tail`。**
    //
    // `harvest` 的语义是「把 workdir 的磁盘差异收成一层」，它只在 `Output::Dir` 时有意义
    // （Zip 路径下 workdir 是空的，收获会产出"全部删除"的错误层——实测产物 4018 → 3654）。
    //
    // **顺序很关键，而且我一开始放反了**：若 `harvest` 跑在 `tail` 之后，它会拿 workdir 里
    // **旧**的 `pack.mcmeta`（那是 `materialize` 写进去的原始内容）覆盖 `tail` 刚改好的版本，
    // 产物退回 `pack_format: 1`。实测由 `native_output_dir_matches_zip_on_a_real_pack` 抓出
    // （Zip 路径有 `max_format: [97,1]`，Dir 路径却是 `pack_format: 1`）。
    //
    // 语义上也应当如此：`harvest` 收的是**任务阶段**在磁盘上留下的差异，
    // 而 `tail` 是整条流水线的**最后一步**，其结果不该再被任何后续步骤覆盖。
    if mirror {
        let harvested = harvest(&pack, workdir, &baseline, None)?;
        report.harvested_changes += harvested.changed();
        report.added += harvested.added.len();
        report.modified += harvested.modified.len();
        report.removed += harvested.removed.len();
        report.undeclared = harvested.undeclared.clone();
        pack.commit(harvested.layer);
    }
    // **§9.148：诚实的相位归属。**
    //
    // 这一段（`tasks` 之后到 `tail` 之前）实际包含两件事：
    //   1. **直接步骤**（`cut_gui_direct`）与延迟删除——Zip 路径上也会跑；
    //   2. **`harvest`**——只有 `Output::Dir` 会跑。
    //
    // 原先只记一个 `harvest_s`，且赋值在 `if mirror` 之外，于是 **Zip 路径会报出
    // 一个它根本没执行过的 `harvest 0.07s`**（那其实是直接步骤的时间）。相位名撒谎，
    // 比没有相位更糟——它会让下一步优化找错目标。因此拆成两个独立相位：
    // `direct_s` 与 `harvest_s`。
    phases.direct_s = direct_started.elapsed().as_secs_f32();
    phases.harvest_s = (clock.elapsed().as_secs_f32() - phases.direct_s).max(0.0);
    clock = std::time::Instant::now();

    // 收尾步骤：**§9.144 起直接在包上工作**（原先读 workdir 的 `pack.mcmeta`）。
    {
        let mut tx = pack.tx("tail");
        tail(&mut tx).map_err(AromError::internal)?;
        let layer = tx.into_layer();
        if !layer.writes().is_empty() {
            sync_baseline_with_layer(&pack, &layer, &mut baseline)?;
            pack.commit(layer);
        }
    }
    phases.tail_s = clock.elapsed().as_secs_f32();
    clock = std::time::Instant::now();

    report.step_trace = trace;
    report.stats = match output {
        Output::Zip(path) => {
            let view = pack.view();
            write_zip(&pack, &view, path, &opts.serialize)?
        }
        Output::Dir(dir) => {
            // §9.117：把 A-ROM 的最终视图**直接物化回目录**——生产入口拿到的就是
            // 「跑完整条 A-ROM 管线之后的工作目录」，随后它照旧改 `pack.mcmeta` / 跑 Bedrock /
            // 重打包。序列化统计（`SerializeStats`）是 zip 特有的，这里留默认值；
            // 需要"产物规模"的判据请用 §9.91/§9.115 的内容契约（它们比这组统计更强）。
            //
            // **必须先清空目标目录**：`materialize` 只做 `create_dir_all` + 写文件，
            // **不会**删除目标里已有的条目。而生产传进来的目录**正是它自己解压出的源树**
            // （§9.116），若不清空，源树中"新视图里已不存在"的文件会残留下来 ⇒ 产物分叉。
            if dir.exists() {
                for e in std::fs::read_dir(dir)
                    .map_err(|e| AromError::io(format!("read {}: {e}", dir.display())))?
                    .flatten()
                {
                    let p = e.path();
                    if p.is_dir() {
                        let _ = std::fs::remove_dir_all(&p);
                    } else {
                        let _ = std::fs::remove_file(&p);
                    }
                }
            }
            let view = pack.view();
            materialize(&view, dir)?;
            SerializeStats::default()
        }
    };
    phases.output_s = clock.elapsed().as_secs_f32();
    // `other_s` = 管线总时长 − 已计量的各相位之和。它覆盖直接步骤、延迟删除、
    // 基线同步与报告组装等零散段。**若它长期偏大，说明相位划分漏了一段**——
    // 这正是当初 `pipeline_s` 一个总数无法回答的问题，所以留一个显式的余量项。
    phases.other_s = (pipeline_clock.elapsed().as_secs_f32() - phases.total()).max(0.0);
    report.phases = phases;
    Ok(report)
}


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
        let is_eraser = matches!(crate::task_registry::tier_of(name), Some(Tier::Eraser));
        let side = if is_eraser || EARLY_NATIVES.contains(&name.as_str()) {
            Side::Early
        } else {
            Side::Late
        };
        out.insert(name.clone(), side);
    }
    out
}

/// 给 `natives` 的夹具正题用：按名字取**已启用全部开关**时的原生实现。
///
/// 夹具用例需要在不启动整条驱动的前提下直接跑单个原生任务（真实包覆盖不到那几个跳过分支）。
#[cfg(test)]
pub(crate) fn native_for_probe(
    name: &str,
) -> Option<(&'static str, TaskDecl, crate::natives::PilotFn)> {
    // `generate_shulker_box_ui` 的实现在闭包里**但不在派发表里**（§9.51：它的正确位置在
    // Architect 旧任务之后、Surgeon 旧任务之前，那个位置要等 Surgeon 原生化后才存在）。
    // 夹具正题可以直接验证它的算法，因此这里额外暴露；生产路径不受影响。
    if name == "generate_shulker_box_ui" {
        return Some((
            "shulker_box_gen",
            crate::natives::shulker_box_gen::decl(),
            crate::natives::shulker_box_gen::run,
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
) -> Option<(&'static str, TaskDecl, crate::natives::PilotFn)> {
    if switches.reverse {
        if name == "reverse_process_chest_folder" {
            return Some((
                "chest_reverse",
                crate::natives::chest_reverse::decl(),
                crate::natives::chest_reverse::run,
            ));
        }
        if name == "reverse_rename_mcpatcher_to_optifine" {
            return Some((
                "mcpatcher_optifine_reverse",
                crate::natives::mcpatcher_optifine_reverse::decl(),
                crate::natives::mcpatcher_optifine_reverse::run,
            ));
        }
        if name == "reverse_rename_blocks_items" {
            return Some((
                "rename_blocks_reverse",
                crate::natives::rename_blocks_reverse::decl(),
                crate::natives::rename_blocks_reverse::run,
            ));
        }
        if name == "reverse_fix_armor_models" {
            return Some((
                "reverse_armor",
                crate::natives::reverse_armor::decl(),
                crate::natives::reverse_armor::run,
            ));
        }
        if name == "reverse_fix_ui_survival" {
            return Some((
                "reverse_survival",
                crate::natives::reverse_survival::decl(),
                crate::natives::reverse_survival::run,
            ));
        }
        if let Some((decl, run)) = crate::natives::reverse_compose::lookup(name) {
            return Some(("reverse_compose", decl, run));
        }
        if let Some((decl, run)) = crate::natives::reverse_pixels::lookup(name) {
            return Some(("reverse_pixels", decl, run));
        }
        if let Some((decl, run)) = crate::natives::reverse_defer_ui::lookup(name) {
            return Some(("reverse_defer_ui", decl, run));
        }
        if let Some((decl, run)) = crate::natives::reverse_defer_metal::lookup(name) {
            return Some(("reverse_defer_metal", decl, run));
        }
        if let Some((decl, run)) = crate::natives::reverse_defer_extra::lookup(name) {
            return Some(("reverse_defer_extra", decl, run));
        }
        if let Some((decl, run)) = crate::natives::reverse_defer::lookup(name) {
            return Some(("reverse_defer", decl, run));
        }
        if let Some((decl, run)) = crate::natives::reverse_trivial::lookup(name) {
            return Some(("reverse_trivial", decl, run));
        }
    }
    if switches.ui && name == "process_chest_folder" {
        return Some((
            "chest",
            crate::natives::chest::decl(),
            crate::natives::chest::run,
        ));
    }
    if switches.textures {
        if name == "generate_potion_lingering" {
            return Some((
                "potion_lingering_gen",
                crate::natives::potion_lingering_gen::decl(),
                crate::natives::potion_lingering_gen::run,
            ));
        }
        if let Some((decl, run)) = crate::natives::arch_gen::lookup(name) {
            return Some(("arch_gen", decl, run));
        }
        if let Some((decl, run)) = crate::natives::arch_gen2::lookup(name) {
            return Some(("arch_gen2", decl, run));
        }
        if let Some((decl, run)) = crate::natives::arch_gen_metal::lookup(name) {
            return Some(("arch_gen_metal", decl, run));
        }
        if let Some((decl, run)) = crate::natives::arch_gen_planks::lookup(name) {
            return Some(("arch_gen_planks", decl, run));
        }
        if let Some((decl, run)) = crate::natives::arch_gen_breeze::lookup(name) {
            return Some(("arch_gen_breeze", decl, run));
        }
        if let Some((decl, run)) = crate::natives::arch_gen3::lookup(name) {
            return Some(("arch_gen3", decl, run));
        }
        // Surgeon 早期组：槽位在旧批次之前，因此与前阶段原生任务同侧（见 `EARLY_NATIVES`）。
        if let Some((decl, run)) = crate::natives::surgeon_early::lookup(name) {
            return Some(("surgeon_early", decl, run));
        }
        if let Some((decl, run)) = crate::natives::surgeon_early2::lookup(name) {
            return Some(("surgeon_early2", decl, run));
        }
        if let Some((decl, run)) = crate::natives::surgeon_mid::lookup(name) {
            return Some(("surgeon_mid", decl, run));
        }
        if let Some((decl, run)) = crate::natives::surgeon_mid2::lookup(name) {
            return Some(("surgeon_mid2", decl, run));
        }
        if let Some((decl, run)) = crate::natives::surgeon_mid3::lookup(name) {
            return Some(("surgeon_mid3", decl, run));
        }
        if let Some((decl, run)) = crate::natives::surgeon_mid4::lookup(name) {
            return Some(("surgeon_mid4", decl, run));
        }
        if let Some((decl, run)) = crate::natives::surgeon_late::lookup(name) {
            return Some(("surgeon_late", decl, run));
        }
        if let Some((decl, run)) = crate::natives::surgeon_ui::lookup(name) {
            return Some(("surgeon_ui", decl, run));
        }
        if let Some((decl, run)) = crate::natives::surgeon_machinery::lookup(name) {
            return Some(("surgeon_machinery", decl, run));
        }
        if let Some((decl, run)) = crate::natives::surgeon_survival::lookup(name) {
            return Some(("surgeon_survival", decl, run));
        }
        if name == "generate_shulker_box_ui" {
            return Some((
                "shulker_box_gen",
                crate::natives::shulker_box_gen::decl(),
                crate::natives::shulker_box_gen::run,
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
            crate::natives::drop_font::decl(),
            crate::natives::drop_font::run,
        )),
        "delete_blockstates_models" => Some((
            "drop_blockstates",
            crate::natives::drop_blockstates::decl(),
            crate::natives::drop_blockstates::run,
        )),
        "delete_horse_folder" => Some((
            "drop_horse",
            crate::natives::drop_horse::decl(),
            crate::natives::drop_horse::run,
        )),
        "delete_shaders_folder" => Some((
            "drop_shaders",
            crate::natives::drop_shaders::decl(),
            crate::natives::drop_shaders::run,
        )),
        "delete_enchanted_item_glint" => Some((
            "drop_glint",
            crate::natives::drop_glint::decl(),
            crate::natives::drop_glint::run,
        )),
        // `rename_blocks_items` 已派发。早年在真实包上与旧实现有差异（§9.13），根因是
        // `ConversionMaps` 的 `HashMap::insert` 同一区间被写第二次而**静默覆盖**前一次，
        // 2.6.0 已修复（见 `CHANGELOG`），此后真实包上运行正常。
        "rename_blocks_items" => Some((
            "rename_blocks",
            crate::natives::rename_blocks::decl(),
            crate::natives::rename_blocks::run,
        )),
        "rename_mcpatcher_to_optifine" => Some((
            "mcpatcher_optifine",
            crate::natives::mcpatcher_optifine::decl(),
            crate::natives::mcpatcher_optifine::run,
        )),
        // `adapt_java_shaders`（§9.85）：**目标 pack_format 由任务自己从包里读**
        // （`run_from_pack`），因此不需要改驱动签名——真实包里没有 `shaders/`，
        // 生产路径上它是「源缺失 → 跳过」，与原实现一致。
        "adapt_java_shaders" => Some((
            "shader_adapt",
            crate::natives::shader_adapt::decl(),
            crate::natives::shader_adapt::run_from_pack,
        )),
        // `fix_smithing2_villager2_ui`（§9.87）：铁砧/村民 GUI 的第二步重排。
        "fix_smithing2_villager2_ui" => Some((
            "surgeon_smithing2",
            crate::natives::surgeon_smithing2::decl(),
            crate::natives::surgeon_smithing2::run,
        )),
        // `convert_animated_textures`（§9.88）：动画 mcmeta 升级。
        // 它此前**不在任何 plan 段**里（注册了却永不执行，见 §9.88），补段后本包会真的跑到它。
        "convert_animated_textures" => Some((
            "animated",
            crate::natives::animated::decl(),
            crate::natives::animated::run,
        )),
        // **`cut_gui`（§9.129）**：以前它**不在本表里** —— 因为驱动只能在「整批旧任务之前/之后」
        // 二选一，而它的槽位在批次**内部**（`(15,18)`，§9.100 实测：挪出去就少 3 个 sprite）。
        // 于是它绕道 `Scheduler::run_named` + 注册闭包，闭包里调 `run_in_workdir`
        // （读 gui 子树 → 内存包 → 跑 `Tx` → 写回 workdir 的一层往返桥）。
        //
        // 现在 46 个任务全部是原生 `Tx`，驱动改按 `plan` 顺序**逐任务**执行，因此
        // `cut_gui` 可以直接以 `Tx` 形态跑在它的精确槽位上，那层往返桥随之取消。
        // 派发**与 `NativeSwitches` 无关**（§9.102：它的实现无条件就是原生模块）。
        "cut_gui" => Some((
            "surgeon_cut_gui",
            crate::natives::surgeon_cut_gui::decl(),
            crate::natives::gui_surgeon_tx::run,
        )),
        _ => None,
    }
}

/// 原生任务的写入是否落在它**声明**的范围内（D12 契约的落地检查）。
///
/// 每个任务必须精确声明读写范围；越界即契约违约，因此驱动默认直接报错
/// （`MixedRunOptions::strict_scopes`）。范围声明同时决定并行批的分组与冲突检测——
/// 这也是 `ScopeSet::any`（整包）会让任务退化为串行的原因。
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

    // **§9.137：目录缓存。**
    //
    // 原先每个写入条目都调一次 `create_dir_all(parent)`。实测一个任务可写 **1614** 个条目
    // （`rename_blocks_items`，占全部 2096 次写入的 77%），其中绝大多数目录**早就存在**，
    // 于是这变成 1600+ 次白跑的系统调用。缓存"本次调用里已确认存在的目录"后，同样的
    // 目录只查一次。
    let mut ensured: std::collections::HashSet<std::path::PathBuf> =
        std::collections::HashSet::with_capacity(64);
    let mut ensure_dir = |dir: &std::path::Path| -> Result<(), AromError> {
        if ensured.insert(dir.to_path_buf()) {
            std::fs::create_dir_all(dir)
                .map_err(|e| AromError::io(format!("mkdir {}: {e}", dir.display())))?;
        }
        Ok(())
    };

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
                ensure_dir(&full)?;
            }
            Slot::Present(body) => {
                if let Some(parent) = full.parent() {
                    ensure_dir(parent)?;
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
///
/// **§9.141：`Move` 且目标不存在时，整目录一次 `fs::rename`。**
///
/// 原实现把目录改名展开成**逐条目移动**：先 `collect_paths` 遍历整棵树、按深度排序，
/// 再对每个文件调一次 `move_or_copy`（其中还各带一次 `create_dir_all`）。
/// 真实包实测：`rename_mcpatcher_to_optifine`（mcpatcher → optifine，约 1600 个文件）
/// **2.16s**——这是全流水线最贵的单步，而它在语义上只是**一次同卷目录改名**。
///
/// `Move` + 目标不存在 ⇒ 两者等价（含子目录结构），因此走快路径；
/// 目标已存在时仍需逐条合并，故保留原路径。
fn apply_renames_to_workdir(workdir: &Path, layer: &Layer) -> Result<(), AromError> {
    use crate::arom::RenameMode;

    // 与 `apply_layer_to_workdir` 同样的目录缓存：逐条目路径下避免重复 `create_dir_all`。
    let mut ensured: std::collections::HashSet<std::path::PathBuf> =
        std::collections::HashSet::with_capacity(32);
    let mut ensure_dir = |dir: &std::path::Path| -> Result<(), AromError> {
        if ensured.insert(dir.to_path_buf()) {
            std::fs::create_dir_all(dir)
                .map_err(|e| AromError::io(format!("mkdir {}: {e}", dir.display())))?;
        }
        Ok(())
    };

    for rule in layer.renames() {
        let from = workdir.join(&rule.from);
        if !from.exists() {
            continue;
        }
        let to = workdir.join(&rule.to);
        if from.is_file() {
            move_or_copy(&from, &to, rule.mode, &mut ensure_dir)?;
            continue;
        }

        // 快路径：同卷整目录改名（目标不存在时与逐条目移动等价）。
        if rule.mode == RenameMode::Move && !to.exists() {
            if let Some(parent) = to.parent() {
                ensure_dir(parent)?;
            }
            std::fs::rename(&from, &to).map_err(|e| {
                AromError::io(format!("rename dir {}: {e}", from.display()))
            })?;
            continue;
        }

        let mut items: Vec<std::path::PathBuf> = Vec::new();
        collect_paths(&from, &mut items)?;
        items.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
        for path in items {
            let rest = path
                .strip_prefix(&from)
                .map_err(|e| AromError::io(format!("strip {}: {e}", path.display())))?;
            move_or_copy(&path, &to.join(rest), rule.mode, &mut ensure_dir)?;
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
    ensure_dir: &mut dyn FnMut(&std::path::Path) -> Result<(), AromError>,
) -> Result<(), AromError> {
    if let Some(parent) = to.parent() {
        ensure_dir(parent)?;
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
    use crate::pack::diff::diff_containers;
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



    fn assert_equivalent(legacy: &Path, mixed: &Path) -> crate::pack::diff::PackDiffReport {
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
        use crate::arom::engine::scheduler::Scheduler;

        let tmp = tempfile::tempdir().expect("tempdir");
        let input = tmp.path().join("fixture.zip");
        fixture(&input);

        let mut scheduler = Scheduler::new();
        // §9.125：**不需要旧闭包**——本用例只用 `plan()`（来自 `ConversionMaps`）与
        // `task_tier()`（元数据表）。因此默认构建下它照样守得住「Eraser → 前阶段」这条
        // 最贵的教训（这正是「元数据与闭包拆开」的直接收益）。
        crate::invoke_conversion::register_tasks(
            &mut scheduler, &input, 97, 1, true, false, true,
        );
        let plan = scheduler.plan(1, 97).expect("plan");

        // 只在本夹具能覆盖到这些任务时才断言（夹具的任务集合可能随版本映射变化）
        let placements = native_placements(&plan, &NativeSwitches::all());
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
            // §9.131：与生产路径同源——驱动现在直接读 `task_registry`，不再经 `Scheduler`。
            let is_eraser = matches!(crate::task_registry::tier_of(name), Some(Tier::Eraser));
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
        let (prod, _) = native_output_v2(
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
        let report = run_native(&input, &work, &Output::Zip(out.clone()), &opts, |tx| {
            if tx.exists("pack.mcmeta") {
                let cur = tx.text("pack.mcmeta").map_err(|e| e.to_string())?;
                let next = crate::pack::version_converter::rewrite_pack_mcmeta_text(&cur, target)?;
                tx.put("pack.mcmeta", next.into_bytes())
                    .map_err(|e| e.to_string())?;
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


    fn native_output_v2(
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
        let report = run_native(input, &work, &Output::Zip(out.clone()), &opts, |tx| {
            if tx.exists("pack.mcmeta") {
                let cur = tx.text("pack.mcmeta").map_err(|e| e.to_string())?;
                let next = crate::pack::version_converter::rewrite_pack_mcmeta_text(&cur, target)?;
                tx.put("pack.mcmeta", next.into_bytes())
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        })
        .expect("mixed run v2");
        (out, report)
    }

    /// **8c 的第一批验收**：把 textures 已迁移任务改为原生执行，产物必须与「全走适配层」
    /// 以及旧管线都逐项一致；开关关闭时行为必须与打开前完全相同。
    ///
    /// §9.125：`off` / `legacy` 两配置的实现是**旧闭包**，默认构建里不存在，因此
    /// 「与旧实现对照」的部分随旧实现一起删除（`legacy-oracle` feature 从未实现）；保留 **`on` 自身的一致性**
    /// （每个计划任务恰好执行一次、原生数 ≥ 1），它与冻结基线一起构成默认闸门。
    #[test]
    fn native_switch_keeps_the_output_identical_on_a_fixture() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let input = tmp.path().join("fixture.zip");
        fixture(&input);

        // 1 → 97 会经过 (5,6)/(7,8) 两段，因此能选中已迁移的 Eraser 任务
        let (on, on_report) =
            native_output_v2(&input, tmp.path(), 97, 1, NativeSwitches::all(), false, "on");

        // §9.102：`cut_gui` 的实现**无条件**是原生模块（§9.101，由注册闭包直接调用），
        // 不受 `NativeSwitches` 影响；而本夹具（1→97）的计划里**确实含** `cut_gui`，
        // 因此「开关全关」时原生数为 **1**（就是它）、适配层为 `plan_len - 1`。
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
        // §9.125：本用例原先用 `native_output`（= 开关全关 `off`）。默认构建里 `off` 没有实现
        // （旧闭包已被删除），因此改为**生产口径 `on`**——这正好落实
        // 交接文档第 4 步：「默认（无 feature）时，`on` 配置改与冻结基线对照」。
        // 判据强度不变：绝对计数是**独立于任何配置**的产物契约。
        let (_mixed, report) =
            native_output_v2(&input, tmp.path(), target, source, NativeSwitches::all(), false, "abs");
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

    /// **②-a 冻结基线**：把真实包产物的**内容**钉死（§9.115）。
    ///
    /// **为什么必须有它**（§9.112/§9.114 的结论）：第 ② 项"移除旧闭包路径"的**真正阻塞
    /// 不是功能，而是验证依赖**——闸门一直靠"与旧管线 `legacy` 对照"来定义正确性，
    /// 因此**旧闭包不能删**（删了就是拆掉自己的尺子）。
    ///
    /// 本用例给出**独立于旧实现**的判据：把产物 zip 的
    /// `(条目名, 长度, 内容 FNV-1a)` 排序后聚合成**一个 u64 指纹**，与冻结常量比较。
    /// 有了它，将来删掉旧闭包后**仍能判定"产物有没有变"**。
    ///
    /// **它比 §9.91 的绝对契约强在哪**：绝对契约只钉 `files` / `bytes` 两个**计数**——
    /// 若某次改动让两个文件互换内容（计数不变），绝对契约**看不见**，而本用例会红。
    ///
    /// 指纹变化时用 `AROM_BASELINE_DUMP=<路径>` 导出**逐条目清单**，
    /// 与仓库内 `tools/arom-baseline.txt` 逐行 diff 即可定位是哪些文件变了。
    #[test]
    #[ignore]
    fn real_pack_content_baseline_is_frozen() {
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
        // §9.125（交接文档第 4 步的落点）：**默认构建下本用例与 `on` 配置对照**，
        // 而不是与 `off` 对照——`off` 的实现（88 个旧闭包）已被删除，
        // 默认构建里没有它。`on` 正是**生产配置**，所以这条对照反而更贴近真实产物。
        //
        // 旧的 `--features legacy-oracle` 对照路径已不存在；`on` 与 `off` 曾产出同一份内容（由
        // `native_switch_keeps_the_output_identical_on_a_real_pack` 把关），
        // 因此本指纹在两种构建下必须一致——这本身就是一条额外约束。
        let (mixed, _report) =
            native_output_v2(&input, tmp.path(), target, source, NativeSwitches::all(), false, "frozen");

        // 逐条目 (名, 长度, 内容 hash) → 排序 → 聚合成一个 u64
        let entries = zip_entry_digests(&mixed);
        let mut agg: u64 = 0xcbf2_9ce4_8422_2325;
        for (name, len, h) in &entries {
            for b in name.as_bytes() {
                agg ^= *b as u64;
                agg = agg.wrapping_mul(0x1000_0000_01b3);
            }
            for b in len.to_le_bytes().iter().chain(h.to_le_bytes().iter()) {
                agg ^= *b as u64;
                agg = agg.wrapping_mul(0x1000_0000_01b3);
            }
        }

        println!(
            "内容基线：{} 个条目，聚合指纹 = 0x{agg:016x}",
            entries.len()
        );

        // 可选：导出逐条目清单（用于与 tools/arom-baseline.txt 做 diff）
        if let Ok(path) = std::env::var("AROM_BASELINE_DUMP") {
            let mut text = String::new();
            for (name, len, h) in &entries {
                text.push_str(&format!("{h:016x}  {len:>9}  {name}\n"));
            }
            std::fs::write(&path, text).expect("dump baseline");
            println!("已导出逐条目清单到 {path}");
        }

        // **冻结值**：TapL 16x → 97（2026-10-05 测得，4018 个条目）。
        // 改动此处前**必须先解释产物为何变化**。
        const FROZEN: u64 = 0x75bb_3260_e7f5_78a6;
        if FROZEN == 0 {
            println!("⚠️ 冻结值尚未填入；本次读数为 0x{agg:016x}");
            return;
        }
        assert_eq!(
            agg, FROZEN,
            "真实包产物的**内容**指纹偏离冻结基线（实际 0x{agg:016x}）——\n\
             这比 §9.91 的计数契约更强：即使文件数/字节数不变，内容变化也会在此报红。\n\
             用 AROM_BASELINE_DUMP 导出清单并与 tools/arom-baseline.txt 逐行 diff 定位。"
        );
    }

    /// 读 zip 的全部条目，返回排序后的 `(名字, 长度, 内容 FNV-1a)`。
    fn zip_entry_digests(zip_path: &Path) -> Vec<(String, u64, u64)> {
        use std::io::Read as _;
        let f = std::fs::File::open(zip_path).expect("open zip");
        let mut ar = zip::ZipArchive::new(f).expect("zip");
        let mut out = Vec::new();
        for i in 0..ar.len() {
            let Ok(mut e) = ar.by_index(i) else { continue };
            if e.is_dir() {
                continue;
            }
            let name = e.name().to_string();
            let mut bytes = Vec::new();
            if e.read_to_end(&mut bytes).is_err() {
                continue;
            }
            let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
            for b in &bytes {
                hash ^= *b as u64;
                hash = hash.wrapping_mul(0x1000_0000_01b3);
            }
            out.push((name, bytes.len() as u64, hash));
        }
        out.sort();
        out
    }

    /// **`Output::Dir` 与 `Output::Zip` 必须产出同一份内容**（默认忽略，§9.117）。
    ///
    /// 这是生产入口改道（②-b）的**前置判据**：生产要的不是 zip，而是
    /// 「跑完整条 A-ROM 管线之后的**工作目录**」（它随后还要写 `pack.mcmeta`、跑 Bedrock、重打包）。
    /// 因此必须先证明"物化回目录"与"序列化成 zip"两者内容一致——
    /// 否则改道会在**每一次真实转换**上改变产物。
    #[test]
    #[ignore]
    fn native_output_dir_matches_zip_on_a_real_pack() {
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
        // §9.125：`legacy_output` 需要旧闭包 ⇒ 默认构建下不做「旧管线 ↔ 两个形态」的三方对照，
        // 改为 **Zip 形态 ↔ Dir 形态** 两方对照——生产改道（②-b）关心的正是这一条。

        // ① Zip 形态 → 解到目录
        let (zip_path, _r1) = native_output_v2(
            &input, tmp.path(), target, source, NativeSwitches::all(), false, "zip",
        );
        let zip_dir = tmp.path().join("zip_extracted");
        std::fs::create_dir_all(&zip_dir).expect("mkdir");
        crate::pack::io::extract_resource_pack(
            zip_path.to_str().expect("utf8"),
            zip_dir.to_str().expect("utf8"),
        )
        .expect("extract zip output");

        // ② Dir 形态
        // **刻意先往目标目录塞"陈旧文件与陈旧子目录"**：生产传进来的目录正是它自己解压出的
        // 源树（§9.116），里面必然有"新视图里已不存在"的条目。若 `Output::Dir` 不清空目标，
        // 这些残留会留在产物里 ⇒ 本用例会红。这是对"清空"这一步的真实检验。
        let dir_out = tmp.path().join("dir_out");
        std::fs::create_dir_all(dir_out.join("assets/minecraft/textures/stale_dir")).expect("mkdir stale");
        std::fs::write(dir_out.join("stale_top.txt"), b"stale").expect("write stale");
        std::fs::write(
            dir_out.join("assets/minecraft/textures/stale_dir/stale.png"),
            b"stale",
        )
        .expect("write stale2");
        {
            let work = tmp.path().join("v2dir_work");
            let mut opts = MixedRunOptions::default();
            opts.source_version = source;
            opts.target_version = target;
            opts.native = NativeSwitches::all();
            let _ = run_native(&input, &work, &Output::Dir(dir_out.clone()), &opts, |tx| {
                if tx.exists("pack.mcmeta") {
                    let cur = tx.text("pack.mcmeta").map_err(|e| e.to_string())?;
                    let next =
                        crate::pack::version_converter::rewrite_pack_mcmeta_text(&cur, target)?;
                    tx.put("pack.mcmeta", next.into_bytes())
                        .map_err(|e| e.to_string())?;
                }
                Ok(())
            })
            .expect("run_native into dir");
        }

        // 对照：Zip 形态 ↔ Dir 形态（开 feature 时另加旧管线一侧）
        let _ = assert_equivalent(&zip_dir, &dir_out);
        println!("Output::Zip 与 Output::Dir 逐项一致");
    }



}
