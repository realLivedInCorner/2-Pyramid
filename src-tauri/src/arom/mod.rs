//! A-ROM —— Astray ResourcePacks Object Model 的内核。
//!
//! 本模块是**新设计**、不是 Foray `rom` 的推广：Foray 的树是为「单包 · 单线程 ·
//! 读多写少 · UI 展示」设计的，其形态（全量字节常驻、急切 SHA-256、嵌套目录树、
//! 读完即关闭容器句柄）与转换引擎的需求正面冲突。设计依据见
//! `docs/compose/spec/astray.md`（总纲）与 `astray-arom-model.md`（接口细则）。
//!
//! 分层（自下而上）：
//!
//! ```text
//! L4 Facade     引擎（&PackView + Tx） · Foray 编辑投影 · CLI 只读
//! L3 Structure  多根 · overlays 分层 · 版本折叠目录 · pack_format(min/max) · 目标版本维度
//! L2 Views      按需类型化视图：mcmeta() · image(path) · model() · lang() · manifest()
//! L1 Store      arena<EntryId> + 路径驻留 + path→id 索引 + ContentRef + Layer/Tx
//! L0 Sources    ZipSource（句柄常开，可取原始压缩字节） · BlobSource · DirSource
//! ```
//!
//! **当前进度：M1（Step 7）进行中**——L0–L3 已在 M0 落地（Step 0–6，见 `astray-arom-model.md` §9），
//! 现补任务契约与冲突感知调度；调用点仍未替换，因此本模块可以整体删除而不影响现状。

pub mod error;
pub mod layer;
pub mod limits;
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
pub use serialize::{
    method_for_path, write_zip, write_zip_atomic, OrderPolicy, SerializeOptions, SerializeStats,
};
pub use source::{MemSource, Source, SourceEntry, SourceMeta, ZipSource};
pub use store::{BaseEntry, BasePack, EntryId, PathId, PathInterner};
pub use structure::{
    format_write_rule, FoldedDir, OverlayLayer, PackRoot, PackStructure,
};
pub use task::{conflict_reason, plan, Plan, ScopeSet, TaskDecl, Tier, Wave};
pub use view::{PackMeta, ViewCache};
