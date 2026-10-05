//! 转换启动提示（历史名 `invoke_conversion` 保留，避免大面积改调用点与注释）。
//!
//! ## 这个模块现在**不做任何事**
//!
//! 它曾经是任务注册表：把 88 个任务的**元数据**登记进调度器，并提供那些任务的
//! **闭包实现**。M3 收官后两件事都已不复存在：
//!
//! | 曾经 | 现在 |
//! |---|---|
//! | 88 个闭包实现 | 全部删除，改为 `natives/` 下的原生 `TaskDecl` |
//! | 元数据在这里登记 | 移到 [`crate::task_registry::REGISTRY`]（纯数据表） |
//! | `cut_gui` 靠注册闭包执行 | 并入 `native_run::native_for` 派发表（§9.129） |
//! | Bedrock 边任务在此注册 | 走 `pack::version_converter::run_bedrock_edge_task`，从不经本驱动 | 
//!
//! 计划（哪些任务、什么顺序、什么阶段）由 `ConversionMaps` 与 `task_registry::REGISTRY`
//! 决定，驱动用 `Scheduler::plan` 取顺序、`Scheduler::task_tier` 取阶段——**都不需要注册实现**。
//!
//! ## 关于 `legacy-oracle`
//!
//! 本模块与其它几处注释曾描述一个 `legacy-oracle` cargo feature（"旧闭包由该 feature 门控"）。
//! **那个 feature 从未实现**（`Cargo.toml` 的 `[features]` 里只有 `store`），旧闭包是被直接
//! 删除的。相关注释已按事实更正——请勿再引入"门控/对照能力仍在"的表述。
//!
//! 代价是**冻结基线不能再重新生成**：它的**冻结值**仍由 `tools/arom-baseline.txt` 与
//! `native_run` 的指纹用例守住，但没有旧实现可供重算。要改基线必须有意为之。

use std::path::Path;

use crate::arom::engine::scheduler::Scheduler;

/// 历史入口：曾经在这里注册全部任务。
///
/// **§9.130 起不再注册任何任务**——调度器需要的名字与阶段来自
/// [`crate::task_registry::REGISTRY`]，实现由 `native_run::native_for` 提供。
///
/// 保留函数是**为了不改调用点**（`native_run` 的放置规则用例与 `pack::diff` 闸门都调它）。
/// 参数的唯一用途是让启动日志能说明"这份计划是针对哪个包、哪对版本"。
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
