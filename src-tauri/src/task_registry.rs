//! 任务注册表的**纯元数据**（M3 收口，§9.125）。
//!
//! **这是「谁跑什么、在哪个阶段跑」的单一来源**：生产驱动 `native_run` 用
//! [`tier_of`] 决定每个任务的放置侧（早/晚阶段，见 `native_placements`），
//! 旧闭包注册表（`invoke_conversion`）**已于 M3 删除**——它历史上按同一张表、同一顺序注册，
//! 因此两份元数据不可能漂移；现在只剩这一份，漂移风险本身消失了。
//!
//! **顺序即语义**：同阶段内的执行顺序、`plan()` 选出的名字集合都依赖它。
//! **本文件不依赖 `converters`**（旧转换器树），因此默认构建不含旧代码。
//!
//! 生成：`pwsh tools/gen-task-registry.ps1`（从 `invoke_conversion.rs` 机械提取）。

use crate::arom::Tier;
use crate::arom::engine::scheduler::TaskType;

/// 一个任务的名字、并发类型与阶段。
///
/// 不派生 `Copy`（`TaskType` 只有 `Clone`）；元数据是 `&'static`，不需要按值复制。
#[derive(Debug, Clone)]
pub struct TaskMeta {
    pub name: &'static str,
    pub task_type: TaskType,
    pub tier: Tier,
}

/// 88 个任务的元数据，**顺序与旧注册表逐字一致**（勿排序、勿插入）。
pub const REGISTRY: &[TaskMeta] = &[
    TaskMeta { name: "rename_blocks_items", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "convert_animated_textures", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "delete_blockstates_models", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "delete_horse_folder", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "delete_enchanted_item_glint", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "delete_shaders_folder", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "delete_font_folder", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "process_chest_folder", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "rename_mcpatcher_to_optifine", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_rename_blocks_items", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_process_chest_folder", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "generate_tipped_arrow_images", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_boat", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_potion_lingering", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_shulker_box_ui", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_furnace", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_fish_bucket", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_crossbow", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_netherite_block", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_netherite_ingot", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_netherite_tools", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_netherite_armor_models", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_copper_ingot", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_copper_block", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_copper_tools", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_copper_armor_models", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_snow_bucket", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_smithing_ui", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_redwood_cherry_bamboo_planks", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_pale_planks", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_poplar_planks", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "generate_tricky_trials_breeze", task_type: TaskType::Parallel, tier: Tier::Architect },
    // **这一次追加的是新能力，不是"补登记"**（与表里其它 88 项的来历不同）：
    // 1.21.4 物品模型定义此前在本仓库**完全不存在**（`item_definition` 全库 0 处命中），
    // 因此它在旧闭包注册表里也从来没有过——这不是"漏抄一项"，而是"新增一项功能"。
    // 放在 Architect 段的**末尾**：它的读写对象（`models/item` / `items`）与前面每一项都不相交，
    // 顺序对它没有语义影响，放末尾就不会扰动既有 88 项的相对顺序。
    TaskMeta { name: "generate_item_definitions", task_type: TaskType::Parallel, tier: Tier::Architect },
    TaskMeta { name: "fix_clock_compass", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "fix_brewing_stand_ui", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "fix_particles", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "fix_sign", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "fix_sign_entities", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "fix_ui_creative", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "fix_ui_sub_hand", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "fix_ui_survival", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "fix_armor_models", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "fix_horse_ui", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "fix2_horse_ui", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "fix_machinery_ui", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "fix_tabs", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "fix_slider", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "fix_smithing2_villager2_ui", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "overlay_icons", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "cut_gui", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "reverse_fix_armor_models", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "reverse_fix_brewing_stand_ui", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "reverse_fix_clock_compass", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "reverse_fix_particles", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "reverse_fix_ui_creative", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "reverse_fix_ui_survival", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "reverse_generate_boat", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_potion_lingering", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_shulker_box_ui", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_furnace", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_netherite_block", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_netherite_ingot", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_netherite_tools", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_netherite_armor_models", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_copper_ingot", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_copper_block", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_copper_tools", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_copper_armor_models", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_smithing_ui", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_crossbow", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_fish_bucket", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_snow_bucket", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_tipped_arrow_images", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_redwood_cherry_bamboo_planks", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_pale_planks", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_poplar_planks", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_generate_tricky_trials_breeze", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_rename_mcpatcher_to_optifine", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_fix_sign", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_fix_sign_entities", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_fix_slider", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    /*
     * 这一项**没有实现，也不在任何版本边的计划里**（`conversion_maps.rs` 里
     * 没有一条边引用它），因此永远不会被 `plan()` 选中。
     *
     * 保留而不删除：本表的不变量是「与旧注册表逐字一致」，删一项就破坏它，
     * 而那个不变量正是用来防「静默换侧」的。标注在这里，免得下一个人
     * 以为它是个能跑的任务。
     */
    TaskMeta { name: "reverse_fix_tabs", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "reverse_fix_horse_ui", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "reverse_fix2_horse_ui", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_fix_machinery_ui", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_fix_ui_sub_hand", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "reverse_fix_smithing2_villager2_ui", task_type: TaskType::Exclusive, tier: Tier::Eraser },
    TaskMeta { name: "reverse_overlay_icons", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    TaskMeta { name: "reverse_cut_gui", task_type: TaskType::Hybrid, tier: Tier::Surgeon },
    // **同一次新增的另一半**：OptiFine CIT（`optifine/cit/**/*.properties`）→ 1.21.4+ 物品定义。
    // 阶段是 **Surgeon** 而不是 Architect：它读的 `optifine/cit/**` 由 Eraser 级的
    // `rename_mcpatcher_to_optifine`（槽位 8→9）改名产生，放 Architect 会在改名之前跑，
    // 于是永远看不到文件（空跑）。挂在 `(46,55)`：默认定义已由上一项在 `(42,46)` 写好。
    // 类型标 `Exclusive` 与它的 `decl()`（`exclusive(true)`）一致——它要读别的任务
    // 可能刚写过的 `assets/minecraft/items/`，声明独占换确定性。
    TaskMeta { name: "generate_cit_item_definitions", task_type: TaskType::Exclusive, tier: Tier::Surgeon },
];

/// **不在** [`REGISTRY`] 里、但会出现在计划中的任务的阶段。
///
/// 它们由别处注册（`bedrock_convert::register_tasks`；旧闭包注册表已删除，
/// `converters::{shaders::java, textures::alpha_layers}`），因此不在上面那张表里；
/// 但 `native_placements` 仍需要它们的阶段来决定放置侧（§9.52/§9.76）。
/// **注意**：这张表只提供**查询兜底**，不改变任何注册条件（尤其不改变 `adapt_shaders` 开关）。
pub const AUXILIARY: &[TaskMeta] = &[
    TaskMeta { name: "adapt_java_shaders", task_type: TaskType::Exclusive, tier: Tier::Surgeon },
    TaskMeta { name: "fix_alpha_layers_in_textures", task_type: TaskType::Exclusive, tier: Tier::Surgeon },
    TaskMeta { name: "bedrock_java_to_bedrock", task_type: TaskType::Exclusive, tier: Tier::Surgeon },
    TaskMeta { name: "bedrock_bedrock_to_java", task_type: TaskType::Exclusive, tier: Tier::Surgeon },
];

/// 名字 → 元数据（表很小，线性查找足够，且**不引入顺序以外的假设**）。
pub fn lookup(name: &str) -> Option<&'static TaskMeta> {
    REGISTRY.iter().find(|m| m.name == name)
}

/// 某个任务名登记的阶段：先查 [`REGISTRY`]，再查 [`AUXILIARY`]。
///
/// `None` = 两张表都没登记。`Scheduler::task_tier` 在活注册表查不到时回落到这里。
pub fn tier_of(name: &str) -> Option<Tier> {
    lookup(name)
        .or_else(|| AUXILIARY.iter().find(|m| m.name == name))
        .map(|m| m.tier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// **表形状守卫**（§9.125）：90 项、无重名，且三个阶段/三种并发类型都有实例。
    ///
    /// 为什么钉住这个数：这张表是「驱动取阶段的数据源」，**少一项就会让某个原生任务
    /// 在 `native_placements` 里查不到阶段**，从而静默换到另一侧 ⇒ 产物分叉（§9.124 的风险）。
    /// 谁要动这个数，必须同时解释清楚上面那句话。
    ///
    /// **88 → 90 的理由（2026-10，1.21.4+ 物品模型定义）**：新增**两个**任务，
    /// 它们是同一块能力的两条输入路径，缺一不可：
    ///
    /// * `generate_item_definitions`（Architect / Parallel）—— 把旧包
    ///   `models/item/*.json` 的 `overrides` 升成 `assets/minecraft/items/*.json`；
    /// * `generate_cit_item_definitions`（Surgeon / Exclusive）—— 把包里的 **OptiFine CIT**
    ///   （`optifine/cit/**/*.properties`）升成同一批物品定义里的 `select` 分支。
    ///
    /// 这两条**不是**从旧闭包注册表里漏抄的项——1.21.4 物品定义在本仓库此前完全不存在
    /// （`item_definition` / `custom_model_data` 全库 0 处命中），是**新能力**；
    /// 它们仍必须进这张表，因为 `native_run` 只从这里取放置侧
    /// （少了它，`native_for` 的派发永远不会被调用）。
    #[test]
    fn registry_shape_is_pinned() {
        assert_eq!(REGISTRY.len(), 90, "元数据表项数必须与旧注册表一致");
        let names: BTreeSet<&str> = REGISTRY.iter().map(|m| m.name).collect();
        assert_eq!(names.len(), 90, "出现重名：表里每个任务名必须唯一");

        // 三种阶段都必须有实例，否则「阶段」这个维度事实上没被覆盖
        for tier in [Tier::Eraser, Tier::Architect, Tier::Surgeon] {
            assert!(
                REGISTRY.iter().any(|m| m.tier == tier),
                "没有 {tier:?} 级的任务，阶段维度失去覆盖"
            );
        }
        for tt in [TaskType::Parallel, TaskType::Exclusive, TaskType::Hybrid] {
            assert!(
                REGISTRY.iter().any(|m| m.task_type == tt),
                "没有 {tt:?} 类型的任务，并发类型维度失去覆盖"
            );
        }

        // `lookup` 与 `tier_of` 必须对表里的每一项都给出一致答案
        for m in REGISTRY {
            assert_eq!(lookup(m.name).map(|x| x.name), Some(m.name));
            assert_eq!(tier_of(m.name), Some(m.tier));
        }
        // 辅助表（Bedrock 两项 + 两个开关任务）也必须答得出来
        for m in AUXILIARY {
            assert_eq!(tier_of(m.name), Some(m.tier), "辅助表项 {} 查不到", m.name);
        }
        assert_eq!(tier_of("no_such_task_at_all"), None);
    }

    /// **驱动真正读的那条路**（§9.125 / §9.131）：`native_run` 用 `tier_of` 决定每个任务的
    /// 放置侧，**没有任何任务实现被注册**（默认构建就是这样：旧闭包已随 §9.128 送走）
    /// 也必须给出表里的阶段。
    ///
    /// 这是「元数据是生产必需」这句话的可执行版本：若哪天有人让阶段改成从"已注册的实现"
    /// 推导，本用例会红——而不是等到真实包产物分叉。
    ///
    /// §9.131 之前本用例走的是 `Scheduler::task_tier`（它在活注册表查不到时回落到本表）；
    /// `TaskTier` 合并进 `Tier` 之后，驱动直接读本表，因此改成直接验证 `tier_of`——
    /// **被测对象就是生产路径本身**，而不再是一条同义反复的回落链。
    #[test]
    fn tier_lookup_works_without_any_registered_implementation() {
        for m in REGISTRY.iter().chain(AUXILIARY.iter()) {
            assert_eq!(
                tier_of(m.name),
                Some(m.tier),
                "查不到 `{}` 的阶段——元数据表不完整",
                m.name
            );
        }
    }
}
