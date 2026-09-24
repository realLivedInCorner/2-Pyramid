//! Foray — 资源包结构分析与轻量编辑工作台（平行于 Hurray）。
//!
//! 模块：
//! - `zip_safe`：Zip Slip / Bomb / Symlink 安全门禁
//! - `mcmeta`：pack.mcmeta 解析
//! - `rom`：Resource-pack Object Model（Foray 侧独立树）
//! - `probe`：文件 / 权限 / 加密 / 解析 / 恶意探针
//! - `paint`：涂抹 / 吸管 / HSV
//! - `export`：另存 / 原地覆盖
//! - `ai`：OpenAI 兼容分析（档位 + 提示词 JSON）

pub mod ai;
pub mod export;
pub mod mcmeta;
pub mod paint;
pub mod probe;
pub mod rom;
pub mod zip_safe;

pub use mcmeta::PackMeta;
pub use probe::ProbeReport;
pub use rom::{FileKind, ParseStatus, Rom, RomFile};
