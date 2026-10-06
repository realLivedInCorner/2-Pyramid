//! A-ROM —— Astray ResourcePacks Object Model 的内核。
//!
//! **这是生产的唯一转换内核。** 旧引擎（`hurray`）与旧转换器树（`converters`）已整体删除；
//! 46 个转换任务全部实现于 [`crate::natives`]，其声明与阶段来自
//! [`crate::task_registry::REGISTRY`]。**没有"待迁移"的部分**——`Tx` 在本模块之外被引用
//! 749 次、`Pack` 425 次，删掉它就不再有任何转换能力。
//!
//! 本模块是**新设计**、不是 Foray `rom` 的推广：Foray 的树是为「单包 · 单线程 ·
//! 读多写少 · UI 展示」设计的，其形态（全量字节常驻、急切 SHA-256、嵌套目录树、
//! 读完即关闭容器句柄）与转换引擎的需求正面冲突。设计依据见
//! `docs/compose/spec/astray-summary.md`。
//!
//! 分层（自下而上）：
//!
//! ```text
//! L4 Facade     驱动（&PackView + Tx） · Foray 编辑投影 · CLI 只读
//! L3 Structure  多根 · overlays 分层 · 版本折叠目录 · pack_format(min/max) · 目标版本维度
//! L2 Views      按需类型化视图：mcmeta() · image(path) · model() · lang() · manifest()
//! L1 Store      arena<EntryId> + 路径驻留 + path→id 索引 + ContentRef + Layer/Tx
//! L0 Sources    ZipSource（句柄常开，可取原始压缩字节） · BlobSource · DirSource
//! ```
//!
//! `engine/` 是**计划与分桶执行**（版本对 → 该跑哪些任务、按阶段分桶），不持有转换实现。
//! 驱动 [`crate::native_run`] 自己按 `plan` 顺序逐任务派发 `Tx`；真正的执行者是
//! [`crate::natives`]。

/// **计划与分桶执行**：版本对 → 该跑哪些任务（`conversion_maps`）、按阶段分桶执行
/// （`scheduler`）、错误类型。
///
/// 它**不持有任何转换实现**——实现在 [`crate::natives`]，元数据在
/// [`crate::task_registry`]。本模块是从旧 `crate::hurray` 里**保留下来的那部分**
/// （旧引擎的 `HurrayContext` / `TexturePool` / `TaskTier` / `TaskFn(&ctx)` 均已删除），
/// 现在只做计划与执行，是 A-ROM 的一部分。
pub mod engine;
pub mod error;
pub mod layer;
pub mod limits;
pub mod pathview;
pub mod serialize;
pub mod source;
pub mod store;
pub mod structure;
pub mod task;
pub mod view;

pub use error::AromError;
pub use layer::{
    BlobId, BlobStore, Body, ConflictPolicy, Layer, Pack, PackView, PrefixRule, RenameMode,
    Resolved, Slot, Tx,
};
pub use limits::SafeLimits;
pub use pathview::{
    decl_any, decl_prefix, decl_scopes, harvest, materialize, run_legacy, Harvest, Materialized,
};
pub use serialize::{
    method_for_path, write_zip, write_zip_atomic, OrderPolicy, SerializeOptions, SerializeStats,
};
pub use source::{DirSource, MemSource, Source, SourceEntry, SourceMeta, ZipSource};
pub use store::{BaseEntry, BasePack, EntryId, PathId, PathInterner};
pub use structure::{
    format_write_rule, FoldedDir, OverlayLayer, PackRoot, PackStructure,
};
pub use task::{conflict_reason, plan, Plan, ScopeSet, TaskDecl, Tier, Wave};
pub use view::{PackMeta, ViewCache};
