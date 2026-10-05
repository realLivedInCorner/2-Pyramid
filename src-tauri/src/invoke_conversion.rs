//! 任务**注册**：把元数据表登记进调度器，并接上必须经注册表执行的实现。
//!
//! §9.121（M3 ②-c）：`invoke_conversion` 与其 `_ex` 变体已删除——生产入口改走
//! `native_run`（§9.118）。
//!
//! §9.128（M3 收官）：**88 个旧闭包体与整个 `legacy-oracle` 兼容层已送走**。
//! 现在这里只做两件事：
//!
//! 1. **注册元数据**（总是执行）：按 [`crate::task_registry::REGISTRY`] 的顺序把
//!    `(名字, 并发类型, 阶段)` 登记进调度器。生产驱动 `native_run` 用
//!    [`Scheduler::task_tier`] 决定每个任务的**放置侧**，因此这一段是生产必需的；
//! 2. **接上 `cut_gui` 的默认实现**：它必须留在旧批次的 `(15,18)` 槽位（§9.100），
//!    所以驱动把它当「适配层任务」交给注册表闭包跑，实现是原生
//!    `natives::surgeon_cut_gui`（见 [`install_native_defaults`]）。
//!
//! 旧实现（`converters/` 下的 1.6 万行）作为**历史参考**留在 `archive/legacy-converters/`，
//! 不参与构建；`tools/legacy-oracle/` 下的取回脚本也已随之退场。

use std::path::Path;

use crate::hurray::scheduler::Scheduler;

/// 注册调度器所需的全部任务。
///
/// **不再需要任何旧转换器代码**：88 个任务里，89 项（含 `cut_gui`）由驱动直接派发到
/// `natives::*`；`cut_gui` 例外，走本文件的注册闭包（原因见 [`install_native_defaults`]）。
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
    install_native_defaults(scheduler);

    let _ = (
        target_path,
        target_version,
        source_version,
        run_gui_surgeon,
        fix_alpha_layers,
        adapt_shaders,
    );
}

/// **`cut_gui` 的默认实现**。
///
/// 它是唯一「名字在元数据表里、但实现必须经注册表闭包执行」的任务：
/// §9.100/§9.102 定下它**必须留在旧批次的 `(15,18)` 槽位**，因此驱动把它当
/// 「适配层任务」交给 `run_named` 路径（`native_run.rs`）。
///
/// 若给它装空实现，产物会**少 3 个 sprite**
/// （`gui/sprites/container/slot/{horse_armor,llama_armor,saddle}.png`）——
/// 交接文档记录过同样的 3 项缺失，§9.125 也实测复现（4015 条目 / `0xc07855d73488424d`）。
/// 批次后的直连步骤**不能**替代它：§9.90/§9.91 实测「两者各自必需」。
///
/// 这里接的是**原生实现** `natives::surgeon_cut_gui::run_in_workdir`（§9.101 起闭包体就是它）。
fn install_native_defaults(scheduler: &mut Scheduler) {
    use crate::hurray::context::HurrayContext;

    let meta = crate::task_registry::lookup("cut_gui")
        .expect("`cut_gui` 必须在元数据表里（本函数依赖它的阶段）");
    let task_type = meta.task_type.clone();
    scheduler.register_task("cut_gui", task_type, meta.tier, |ctx: &HurrayContext| {
        // §9.128：`run_in_workdir` 现在把延迟删除**经 `Outcome` 返回**（与其余 43 个原生任务一致）。
        // 但注册闭包的签名固定是 `Result<(), String>`，拿不到驱动的 `report`；而这一路
        // （批次内 slot `(15,18)`）仍然只能经 `HurrayContext` 的清理清单交付 ——
        // 驱动末尾的 `ctx.execute_cleanup()` 与它自己的 `report.deferred_removals`
        // **本来就相邻**，因此时机等价（§9.128 已核实）。
        //
        // 彻底去掉这个 ctx 依赖，要做的是「让驱动能在批次槽位内部派发 `Tx` 任务」
        // ——那是下一步（届时本闭包整体消失）。
        let outcome = crate::natives::surgeon_cut_gui::run_in_workdir(std::path::Path::new(
            ctx.temp_dir(),
        ))
        .map_err(|e| e.to_string())?;
        for rel in &outcome.deferred_removals {
            ctx.defer_remove_file(&std::path::Path::new(ctx.temp_dir()).join(rel));
        }
        Ok(())
    });
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
