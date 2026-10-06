//! **计划与分桶执行**。它只做三件事，全部与"转换实现"无关：
//!
//! | 文件 | 职责 |
//! |---|---|
//! | [`conversion_maps`] | 版本对 → 该跑哪些任务（**纯数据**，顺序即语义） |
//! | [`scheduler`] | **按阶段分桶**执行已注册任务（`plan` 取顺序、`execute_version_conversion` 执行） |
//! | [`error`] | `EngineError` / `EngineResult` |
//!
//! ## 谁在生产路径上用它
//!
//! 驱动 [`crate::native_run`] 自己按 `plan` 顺序逐任务派发 `Tx`；本模块负责的是
//! **计划本身**（`plan`，驱动取任务顺序的唯一来源），以及 **Bedrock 边任务**
//! （j2b/b2j，`pack::version_converter::run_bedrock_edge_task`）。
//!
//! **转换实现不在这里**——在 [`crate::natives`]；任务声明在 [`crate::task_registry`]。
//!
//! ## 与旧 `hurray` 的关系（历史，非现状）
//!
//! 本模块是从旧 `crate::hurray` 里**保留下来的那部分**。旧引擎的整套设施已删除：
//! 88 个任务闭包（§9.128）、`cut_gui` 注册闭包（§9.129）、`HurrayContext` 与
//! `TexturePool`（§9.130）、重复的 `TaskTier`（§9.131）、`run_named` 与
//! `registered_task_names`（§9.132）、只写不读的 `tasks` 字段（§9.133）。
//! 剩下的只是一个按阶段执行的执行器，于是并入 A-ROM、`hurray` 这个名字消失（§9.134）。
//!
//! 需要逐条对照的迁移记录见 `CHANGELOG` 的 2.200.0 段；本文档只描述现状。

pub mod conversion_maps;
pub mod error;
pub mod scheduler;
