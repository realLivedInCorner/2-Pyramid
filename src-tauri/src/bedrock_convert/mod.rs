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

use crate::hurray::scheduler::{Scheduler, TaskTier, TaskType};

pub use b2j::convert_bedrock_to_java;
pub use j2b::convert_java_to_bedrock;

/// 解压后的目录是否为 Bedrock 资源包。
pub fn is_bedrock_resource_pack(root: &Path) -> bool {
    metadata::is_bedrock_resource_pack(root)
}

/// 注册 j2b / b2j 到调度器（Exclusive + Surgeon）。
/// 由 invoke_conversion 调用一次；具体转换逻辑在 j2b/b2j 模块内。
pub fn register_tasks(scheduler: &mut Scheduler) {
    scheduler.register_task(
        "bedrock_java_to_bedrock",
        TaskType::Exclusive,
        TaskTier::Surgeon,
        |ctx| {
            let temp = Path::new(ctx.temp_dir());
            // §9.93（M3）：包名改为**只读构造期字段**，不再经 `shared_data`。
            let pack_name = ctx.pack_name();
            convert_java_to_bedrock(temp, pack_name).map_err(|e| e)
        },
    );
    scheduler.register_task(
        "bedrock_bedrock_to_java",
        TaskType::Exclusive,
        TaskTier::Surgeon,
        |ctx| {
            let temp = Path::new(ctx.temp_dir());
            convert_bedrock_to_java(temp).map(|_| ()).map_err(|e| e)
        },
    );
}
