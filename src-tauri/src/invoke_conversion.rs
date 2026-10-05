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

use crate::hurray::scheduler::Scheduler;

/// 注册调度器所需的全部任务。
///
/// **不再需要任何旧转换器代码，也不再有注册闭包**：计划里的每个名字都由驱动直接派发到
/// `natives::*`（见 `native_run::native_for`）。
///
/// 参数保留旧签名（`target_path` 等）是为了不改调用点；它们如今只服务元数据段，
/// 而元数据段不需要这些值——因此显式忽略，避免"看起来有用"的误导。
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
    init_meta_tasks(scheduler);

    let _ = (
        target_path,
        target_version,
        source_version,
        run_gui_surgeon,
        fix_alpha_layers,
        adapt_shaders,
    );
}

/// 与开关无关、也**不属于** [`crate::task_registry::REGISTRY`] 的注册项。
///
/// Bedrock 边任务（j2b/b2j）是**生产功能**：注册项与实现都在 `bedrock_convert`
/// （其阶段登记在 `task_registry::AUXILIARY`）。
fn init_meta_tasks(scheduler: &mut Scheduler) {
    use crate::log_debug;

    crate::bedrock_convert::register_tasks(scheduler);

    log_debug!("all mapping table tasks registered");
}
