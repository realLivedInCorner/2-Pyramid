//! 基岩 ↔ Java 资源包结构转换。
//!
//! 子模块：
//! - `mapping`   路径别名与语言键映射
//! - `textures`  贴图目录重组与 flipbook
//! - `metadata`  manifest / pack.mcmeta / lang / sounds
//! - `fsutil`    目录合并等文件工具
//! - `j2b` / `b2j` 两个方向的编排入口
//!
//! 调度：通过 [`register_tasks`] 挂到 Scheduler（Exclusive + Surgeon），
//! 版本边 (84,1000) / (1000,84) 触发，不在 invoke_conversion 里写转换逻辑。

pub mod b2j;
pub mod fsutil;
pub mod j2b;
pub mod mapping;
pub mod metadata;
pub mod textures;

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
            let pack_name = ctx
                .get_data("pack_name")
                .unwrap_or_else(|| "resource_pack".to_string());
            convert_java_to_bedrock(temp, &pack_name).map_err(|e| e)
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
