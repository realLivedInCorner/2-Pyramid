// Converter modules.
//
// §9.125（M3 收口）：本模块现在**只装生产需要的部分**——
//
// | 子模块 | 性质 |
// |---|---|
// | `pack_analysis` | 只读结构分析（Tauri 命令 / CLI `--analyze` 在用） |
// | `pack_diff` | 产物对比（CLI `--pack-diff`、闸门在用） |
// | `version_converter` | **生产转换入口**（`process_zip_timed`） |
// | `zip` | 解压 / 重打包 / 工作目录清理 |
//
// 旧转换器树（`audio` / `blockstate_adapter` / `main_converter` / `reverse` /
// `shaders` / `textures` / `ui`）由 `legacy-oracle` feature 门控，**默认不编译**；
// Bedrock 结构转换是生产功能，已移到 `crate::bedrock_convert`；
// UImage 路径解析与缩放因子是两边共用的工具，已移到 `crate::image_utils` / `crate::scale_factor`。

pub mod pack_analysis;
pub mod pack_diff;
pub mod version_converter;
pub mod zip;

// ── 旧转换器树：仅 `legacy-oracle`（`legacy` / `off` 对照配置与基线重生成）──
#[cfg(feature = "legacy-oracle")]
pub mod audio;
#[cfg(feature = "legacy-oracle")]
pub mod blockstate_adapter;
#[cfg(feature = "legacy-oracle")]
pub mod main_converter;
#[cfg(feature = "legacy-oracle")]
pub mod reverse;
#[cfg(feature = "legacy-oracle")]
pub mod shaders;
#[cfg(feature = "legacy-oracle")]
pub mod textures;
#[cfg(feature = "legacy-oracle")]
pub mod ui;

// ── 兼容转发（§9.125）：两个**两边共用**的工具已移到 crate 根（它们不是旧转换器的一部分，
/// 见 `image_utils` / `scale_factor` 的模块注释）。旧转换器树里仍写 `crate::converters::X`，
/// 这里保留同名转发，使**被忽略的旧源码不必改动**——
/// 少改一个文件就少一次「用局部证据推断全局」的机会（§9.123 的教训）。
#[cfg(feature = "legacy-oracle")]
pub use crate::image_utils::get_uimage_path;
/// 旧代码里写的是 `crate::converters::scale_factor::determine_scale_factor`（模块路径），
/// 因此这里再挂一个同文件模块别名（与 crate 根的 `scale_factor` 指向同一份源码）。
#[cfg(feature = "legacy-oracle")]
#[path = "scale_factor.rs"]
pub mod scale_factor;
