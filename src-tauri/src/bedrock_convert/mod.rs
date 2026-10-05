//! 基岩 ↔ Java 资源包结构转换。
//!
//! 子模块：
//! - `mapping`   路径别名与语言键映射
//! - `textures`  贴图目录重组、床/实体、flipbook
//! - `ui`        快捷栏 / HUD / 容器 UI 补全
//! - `potions`   药水物品栏变体铺开
//! - `metadata`  manifest / pack.mcmeta / lang / sounds
//! - `fsutil`    目录合并等文件工具
//! - `j2b` / `b2j` 两个方向的编排入口
//!
//! 调度：通过 [`register_tasks`] 挂到 Scheduler（Exclusive + Surgeon），
//! 版本边 (84,1000) / (1000,84) 触发，不在 invoke_conversion 里写转换逻辑。
//!
//! **§9.125（M3 收口）**：本模块原在 `converters/bedrock/`。它是**生产功能**
//! （`process_zip_timed` 的 Bedrock 目标、Bedrock 源预检都要用它），因此随
//! 「旧转换器树按 `legacy-oracle` 门控」一起移出 `converters/`——否则
//! `crate::converters` 无法整体按 feature 门控。两个任务的阶段登记在
//! `crate::task_registry::AUXILIARY`。

pub mod b2j;
pub mod fsutil;
pub mod j2b;
pub mod mapping;
pub mod metadata;
pub mod potions;
pub mod shaders;
pub mod skybox;
pub mod textures;
pub mod ui;

use std::path::Path;

use crate::arom::Tier;
use crate::arom::engine::scheduler::{Scheduler, TaskType};

pub use b2j::convert_bedrock_to_java;
pub use j2b::convert_java_to_bedrock;

/// 解压后的目录是否为 Bedrock 资源包。
pub fn is_bedrock_resource_pack(root: &Path) -> bool {
    metadata::is_bedrock_resource_pack(root)
}

/// 注册 j2b / b2j 到调度器（Exclusive + Surgeon）。
///
/// **§9.130：`workdir` 与 `pack_name` 改为注册期捕获**，不再经 `HurrayContext`。
///
/// 原先闭包从 `ctx.temp_dir()` / `ctx.pack_name()` 取值。但这两个值在**注册时就已经确定**
/// （注册发生在转换开始前，`workdir` 与包名都不会再变），而 `ctx` 存在的唯一理由就是
/// "任务之间共享可变状态"——§9.93 已经把 `shared_data` 删掉、§9.128 又把 `cut_gui` 的
/// 清理登记改成回传 `Outcome`。于是这里成了**最后一个**读 `ctx` 的地方。
///
/// 改成捕获之后，`Scheduler` 执行任务时**不再需要 context**，
/// `HurrayContext` 与 `TexturePool` 随之可以整体退场。
pub fn register_tasks(scheduler: &mut Scheduler, workdir: &Path, pack_name: &str) {
    let workdir = workdir.to_path_buf();
    let pack_name = pack_name.to_string();

    let temp = workdir.clone();
    let name_for_j2b = pack_name.clone();
    scheduler.register_task(
        "bedrock_java_to_bedrock",
        TaskType::Exclusive,
        Tier::Surgeon,
        move || {
            // §9.93（M3）：包名是**只读构造期值**，此处由注册期捕获。
            convert_java_to_bedrock(&temp, &name_for_j2b).map_err(|e| e)
        },
    );
    scheduler.register_task(
        "bedrock_bedrock_to_java",
        TaskType::Exclusive,
        Tier::Surgeon,
        move || convert_bedrock_to_java(&workdir).map(|_| ()).map_err(|e| e),
    );
}
