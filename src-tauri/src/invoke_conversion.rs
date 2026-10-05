//! 任务**注册**：把元数据表登记进调度器。
//!
//! §9.121（M3 ②-c）：`invoke_conversion` 与其 `_ex` 变体已删除——生产入口改走
//! `native_run`（§9.118）。
//!
//! §9.128（M3 收官）：**88 个旧闭包体与整个 `legacy-oracle` 兼容层已送走**。
//!
//! §9.129：**`cut_gui` 的注册闭包也已删除**——它是最后一个"必须经注册表执行"的任务，
//! 原因是它的计划槽位在旧批次**内部**（`(15,18)`，§9.100），而驱动当时只能在
//! 「整批旧任务之前/之后」二选一。现在驱动改按 `plan` 顺序**逐任务**派发 `Tx`，
//! `cut_gui` 因此并入 `native_run::native_for` 派发表，跑在自己的精确槽位上。
//!
//! 于是本模块现在**只做一件事**：把元数据表登记进调度器。
//! 计划编排（哪些任务、什么顺序）由 [`crate::task_registry::REGISTRY`] 决定，
//! 驱动用 `Scheduler::plan` 取顺序、用 `Scheduler::task_tier` 取阶段。

use std::path::Path;

use crate::arom::engine::scheduler::Scheduler;

/// 注册调度器所需的全部任务。
///
/// **§9.130：本函数现在不再注册任何任务。**
///
/// 它历史上注册两样东西：
/// 1. **Bedrock 边任务**（j2b/b2j）——但它们只在 `target_version == 1000` 时进计划，
///    而那条路走的是 `pack::version_converter::run_bedrock_edge_task`（自建 scheduler 并直接执行），
///    **从不经过本驱动的计划**。原先这里注册一份是白做工；
/// 2. **`cut_gui` 的注册闭包**——§9.129 已删除（驱动改按 `plan` 顺序派发 `Tx`）。
///
/// 保留函数本身是因为调用点（`native_run`）与 `pack::diff` 的闸门都用它来确定计划；
/// 计划顺序由 [`crate::task_registry::REGISTRY`] 与 `ConversionMaps` 决定，不需要注册闭包。
///
/// 参数保留旧签名是为了不改调用点；它们如今只用于日志与元数据段，因此显式忽略。
#[allow(clippy::too_many_arguments)]
pub fn register_tasks(
    scheduler: &mut Scheduler,
    target_path: &Path,
    target_version: u32,
    source_version: u32,
    run_gui_surgeon: bool,
    fix_alpha_layers: bool,
    adapt_shaders: bool,
) {
    use crate::log_debug;

    // 计划（哪些任务、什么顺序）由 `ConversionMaps` 决定，与"注册了什么实现"无关；
    // 每个名字的实现由驱动 `native_run::native_for` 提供。
    log_debug!(
        "task plan source ready (registered implementations: driver-native); input={:?} {} -> {}",
        target_path.file_name().unwrap_or_default(),
        source_version,
        target_version
    );
    let _ = (scheduler, target_path, run_gui_surgeon, fix_alpha_layers, adapt_shaders);
}
