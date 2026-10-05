//! **执行引擎**（原 `crate::hurray`，§9.134 搬入 A-ROM）。
//!
//! 它现在只做三件事，全部与"旧转换器实现"无关：
//!
//! | 文件 | 职责 |
//! |---|---|
//! | [`conversion_maps`] | 版本对 → 该跑哪些任务（**纯数据**，顺序即语义） |
//! | [`scheduler`] | **按阶段分桶**执行已注册任务（`plan` 取顺序、`execute_version_conversion` 执行） |
//! | [`error`] | `EngineError` / `EngineResult` |
//!
//! ## 为什么搬进来（§9.134 的"再见了 hurray"）
//!
//! `hurray` 原本是**旧引擎**：任务闭包签名是 `Fn(&HurrayContext)`，自带纹理池、分辨率探测、
//! 88 个旧转换器的注册表。A-ROM 接手后这些逐项退场：
//!
//! | 退场物 | 提交 |
//! |---|---|
//! | 88 个旧闭包（曾计划由 `legacy-oracle` feature 门控，**该 feature 从未实现**） | §9.128 |
//! | `cut_gui` 注册闭包、批次路径 | §9.129 |
//! | `HurrayContext`、`TexturePool` | §9.130 |
//! | 重复的阶段枚举 `TaskTier`、`Scheduler::task_tier` | §9.131 |
//! | `Scheduler::run_named`、`registered_task_names` | §9.132 |
//! | 只写不读的 `tasks` 字段、`clear()` | §9.133 |
//!
//! 于是剩下的**不是"旧引擎"，只是一个按阶段执行的执行器**——`arom` 也早已不依赖它
//! （依赖方向反过来：`hurray` 依赖 `arom::Tier`）。因此搬进来，`hurray` 这个名字随之消失。
//!
//! ## 它仍在生产路径上的理由
//!
//! 驱动 `native_run` 自己按 `plan` 顺序逐任务派发 `Tx`；本引擎负责的是
//! **Bedrock 边任务**（j2b/b2j，`pack::version_converter::run_bedrock_edge_task`），
//! 以及**计划本身**（`plan`）——后者是驱动取任务顺序的唯一来源。

pub mod conversion_maps;
pub mod error;
pub mod scheduler;
