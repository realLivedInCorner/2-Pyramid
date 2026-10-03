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
//! **当前进度：Step 1（L0）**。L0 之外尚未落地，调用点也尚未替换——本模块此刻
//! 不被任何既有管线使用，因此可以整体删除而不影响现状。

pub mod error;
pub mod limits;
pub mod source;

pub use error::AromError;
pub use limits::SafeLimits;
pub use source::{MemSource, Source, SourceEntry, SourceMeta, ZipSource};
