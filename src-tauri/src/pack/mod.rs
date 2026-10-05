// 资源包 I/O 与分析工具（原 `converters/`）。
//
// §9.128（M3 收官）：旧转换器树已**整体删除**（曾计划的 `legacy-oracle` 门控 feature 从未实现），
// 本模块只保留生产真正必需的四块，并把文件名改成直白的名字：
//
// | 文件 | 模块 | 职责 | 谁在用 |
// |---|---|---|---|
// | `io.rs` | `pack::io` | 解压 / 重打包 / 工作目录清理 | 转换入口、overlay 命令、CLI |
// | `analysis.rs` | `pack::analysis` | 只读结构分析（分层 / 区间 / 折叠目录 / 多根） | Tauri `--analyze`、CLI |
// | `diff.rs` | `pack::diff` | 产物对比（容器级 + 内容级闸门） | CLI `--pack-diff`、`--ignored` 闸门 |
// | `version_converter.rs` | `pack::version_converter` | **生产转换入口**（`process_zip_timed`） | GUI 命令、CLI |
//
// 旧实现（106 文件 / 16,580 行：`audio` / `blockstate_adapter` / `main_converter` /
// `reverse` / `shaders` / `textures` / `ui` / `color` / `bedrock`）作为**历史参考**
// 保留在 `archive/legacy-converters/`，**不参与构建**。
//
// Bedrock 结构转换是生产功能，已移到 `crate::bedrock_convert`；
// 缩放因子移到 crate 根 `crate::scale_factor`。

pub mod analysis;
pub mod diff;
pub mod io;
pub mod version_converter;
