//! M1 试点：三个低风险转换器的 A-ROM 原生实现，配双轨对照。
//!
//! 目的不是「把这三个任务迁完」，而是**证明新契约能表达真任务**：三个试点刻意覆盖三种机制——
//!
//! | 试点 | 机制 | 旧实现 | A-ROM 表达 |
//! |---|---|---|---|
//! | [`drop_font`] | 整棵子树删除 | `ctx.defer_remove_dir` + 收尾清理 | `tx.remove(prefix)`（层里的 Tombstone，子树语义已由层保证） |
//! | [`old_paths`] | 复制而不动源 | `fs::copy` | `tx.alias(from, to)`（零字节复制） |
//! | [`animated`] | 读 JSON + 按 PNG 尺寸推导 + 写回 | `fs::read_to_string` / `fs::write` | `tx.text()` + `tx.image()` + `tx.put()`（写入进层，可回滚） |
//!
//! **旧实现原样保留**：本模块只是并行新增，双轨对照用例同时跑两边并比对最终产物
//! （旧：解压 → 跑旧函数 → 重打包；新：`Pack` → 跑试点 → 序列化 → 容器级闸门）。
//! 注册切换属 M2，本模块不参与生产路径。

use std::sync::Arc;

use image::RgbaImage;

use crate::arom::{AromError, ScopeSet, TaskDecl, Tier, Tx};

/// 任务执行结果（可观测性最小集）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    pub changed: usize,
    pub skipped: usize,
    pub notes: Vec<String>,
    /// **延迟删除**：旧实现用 `defer_remove_file/dir` 登记、在全局清理点统一执行；
    /// 原生任务同样只登记，由驱动在清理点应用（见 `native_run`）。
    pub deferred_removals: Vec<String>,
}

/// **延迟删除**：只登记路径，由驱动在清理点统一 tombstone——复刻旧实现 `defer_remove_*` 的时机。
///
/// 立即删除会让**更晚**的任务看不到文件（实测：反向改名因源文件已被删而搬不动东西，§9.23）。
fn defer_remove_if_present(tx: &Tx<'_>, path: &str) -> Result<Outcome, AromError> {
    if !tx.has_prefix(path)? {
        return Ok(Outcome::default());
    }
    Ok(Outcome {
        deferred_removals: vec![path.to_string()],
        notes: vec![format!("defer removal of {path}")],
        ..Outcome::default()
    })
}

/// 整棵子树删除（对应旧 `delete_font_folder`）。
#[path = "eraser/drop_font.rs"]
pub mod drop_font;

/// 「存在就整段删除」类任务的共同实现。
///
/// 旧实现一律是 `path.exists()` + `defer_remove_file/dir`；这里用 `has_prefix`，
/// 以覆盖「目录只由文件隐含、没有显式条目」的情况（理由同 `drop_font`）。
fn remove_if_present(tx: &mut Tx<'_>, path: &str) -> Result<Outcome, AromError> {
    if !tx.has_prefix(path)? {
        return Ok(Outcome::default());
    }
    tx.remove(path)?;
    Ok(Outcome {
        changed: 1,
        notes: vec![format!("removed {path}")],
        ..Outcome::default()
    })
}

/// 旧 `delete_horse_folder`（`converters/textures/drop_horse.rs`）。
#[path = "eraser/drop_horse.rs"]
pub mod drop_horse;

/// 旧 `delete_shaders_folder`（`converters/textures/drop_shaders.rs`）。
#[path = "eraser/drop_shaders.rs"]
pub mod drop_shaders;

/// 旧 `delete_enchanted_item_glint`（`converters/textures/drop_enchanted_glint.rs`）——删单个文件。
#[path = "eraser/drop_glint.rs"]
pub mod drop_glint;

/// 旧 `delete_blockstates_models`（`converters/textures/drop_blockstates_models.rs`）——一次删两个目录。
#[path = "eraser/drop_blockstates.rs"]
pub mod drop_blockstates;

#[path = "native/rename_blocks_tables.rs"]
mod rename_blocks_tables;

/// 旧 `rename_blocks_items`（`converters/textures/rename_blocks.rs`）**连同**它在末尾调用的
/// `process_blocks::rename_and_process_blocks(&block_path, false)`——两者是同一条注册任务，
/// 必须一起迁移，否则产物不同。
///
/// 语义要点（逐条对应旧实现）：
/// 1. `items → item`、`blocks → block`：目标不存在则整体改名；目标已存在则**逐文件移动、
///    同名时源覆盖目标**，最后删掉源目录（旧的 `merge_or_rename_dir`）；
/// 2. 旧表的 154 条重命名（`rename_with_mcmeta`：顺带搬 `.png.mcmeta`）；
/// 3. `rename_items` 的两遍：先搬 png（连带 `.png.mcmeta`），再搬「不带 .png」的 `{name}.mcmeta`；
/// 4. 红石粉十字/线贴图派生、木板与矿石的色相/明度派生、`nether_gold_ore` 的白→黄。
///
/// 派生图走 `tx.put_image`，它与旧实现的 `img.save()` 是同一条编码路径
/// （`DynamicImage::write_to(.., Png)`），因此产物字节可期一致；这一点由闸门实测把关。
#[path = "eraser/rename_blocks.rs"]
pub mod rename_blocks;

/// 旧 `process_chest_folder`（`converters/ui/process_chest_folder.rs`，**ui 模块第一个迁移任务**）。
///
/// 纯图像几何变换：单胸按宽度定缩放做 4 组「交换+镜像」与 8 组镜像；双胸用旧实现的
/// overlay 表生成 `{prefix}_left.png` / `{prefix}_right.png`。旧实现里的纯函数
/// （`swap_and_mirror` / `mirror_region` / `generate_double_chest_images`）**直接复用**
/// （放开到 `pub(crate)`），避免把这张几十行的 overlay 表抄第二遍。
#[path = "eraser/chest.rs"]
pub mod chest;

/// 旧 `reverse_process_chest_folder`（`converters/reverse/chest_folder.rs`）——**反向转换的第一个任务**。
///
/// 它是正向 `process_chest_folder` 的镜像版：同一张缩放表、同样 4 组「交换+镜像」与 8 组镜像，
/// 但**不处理双胸**（反向只把单胸的变换倒回去）。纯函数同样直接复用：交换用反向模块自己的
/// `swap_and_mirror`，镜像用正向模块的 `mirror_region`（两者语义逐字相同：先水平再垂直翻转后 overlay）。
#[path = "reverse/chest_reverse.rs"]
pub mod chest_reverse;

/// 反向侧的**空操作**与**删单路径**两类任务，成批迁移。
///
/// - 空操作类：旧实现有明确文档说明「无法从产物反推原状」（`cut_gui` 抽走了图集、`horse_ui`
///   与 `overlay_icons` 把外部贴图永久合成进去、`sub_hand` 同理），因此旧实现就是 `Ok(())`；
///   原生实现同样什么都不做，声明里**读写都为空**（精确反映事实）。
/// - 删单路径类：与正向的 `drop_*` 同构，直接复用 `remove_if_present`。
#[path = "reverse/reverse_trivial.rs"]
pub mod reverse_trivial;

/// 旧 `reverse_rename_mcpatcher_to_optifine`（`converters/reverse/mcpatcher_to_optifine.rs`）：
/// 把 `optifine/` 改回 `mcpatcher/`，**仅当目标不存在**（旧实现带守卫，不合并）。
#[path = "reverse/mcpatcher_optifine_reverse.rs"]
pub mod mcpatcher_optifine_reverse;

/// 反向「延迟删除」批次的第二批（复用 §9.24 的延迟删除机制）。
///
/// 三个任务都只有字面路径、无循环：
/// - `reverse_generate_shulker_box_ui`：延迟删一个 gui 文件；
/// - `reverse_fix_sign_entities`：延迟删一整棵 `entity/signs` 子树；
/// - `reverse_fix_smithing2_villager2_ui`：**立即**把 `villager_backup.png` 改名回 `villager.png`
///   （旧实现是 `fs::rename`，不是延迟），再延迟删 `smithing.png`。
#[path = "reverse/reverse_defer.rs"]
pub mod reverse_defer;

/// 旧 `reverse_rename_blocks_items`（`converters/reverse/rename_blocks.rs`）。
///
/// 与正向的差异（逐条对应旧实现）：
/// 1. 目录改名的**方向相反**且**不做合并**——只有目标目录不存在时才整体改名；
/// 2. 一张**只作用于 `items/`** 的 128 对反向表（脚本抽取），全部走同一套 `rename_with_mcmeta`；
/// 3. **不调用** `process_blocks::rename_and_process_blocks`（正向才调）。
#[path = "eraser/rename_blocks_reverse.rs"]
pub mod rename_blocks_reverse;

/// 旧贴图路径复制（对应旧 `convert_old_texture_paths`）。
#[path = "eraser/old_paths.rs"]
pub mod old_paths;

/// 动画贴图 mcmeta 升级（对应旧 `convert_animated_textures`）。
#[path = "eraser/animated.rs"]
pub mod animated;

/// 目录整体改名（对应旧 `rename_mcpatcher_to_optifine`）：用前缀规则，不物化子条目。
#[path = "architect/mcpatcher_optifine.rs"]
pub mod mcpatcher_optifine;

/// 只读视图里的一次性读（试点的公共小工具）。
pub fn read_dimensions(tx: &Tx<'_>, path: &str) -> Result<(u32, u32), AromError> {
    let img: Arc<RgbaImage> = tx.image(path)?;
    Ok((img.width(), img.height()))
}

/// **原生实现签名**：§9.127 分组后它住在 `natives::native::all`（那里还有 `all()` 清单）。
///
/// 这里显式转出，使 `crate::natives::PilotFn` 与分组前**同路径可用**
/// （外部多处按这个路径引用；glob 不会穿透子模块，因此必须显式 `pub use`）。

/// 反向「延迟删除」的第三批：手写模块（不复用 `reverse_defer` 内部的宏，
/// 因为 `macro_rules!` 只在定义它的模块及其子模块可见）。
#[path = "reverse/reverse_defer_extra.rs"]
pub mod reverse_defer_extra;

/// 反向「延迟删除」的第四批：铜/下界合金族共 8 个任务（都是路径表）。
#[path = "reverse/reverse_defer_metal.rs"]
pub mod reverse_defer_metal;

/// 反向「延迟删除」的第五批：`sign` 与 `machinery`（都是「延迟列表 + 立即改名」）。
///
/// 与 `boat` 的差别：这里的改名**没有目标守卫**（旧实现直接 `fs::rename`，会覆盖），
/// 因此原生实现也照抄「覆盖」，不擅自加守卫。
#[path = "reverse/reverse_defer_ui.rs"]
pub mod reverse_defer_ui;


/// 反向「像素回退」批次：`brewing_stand_ui` 与 `ui_creative`。
///
/// 两者都是对 GUI 贴图做像素级回退（填色 + 区域搬移），逐行照抄旧实现的坐标与缩放表；
/// `paste_region` 直接复用 `crate::image_utils`（旧实现用的就是它）。
#[path = "reverse/reverse_pixels.rs"]
pub mod reverse_pixels;
/// 反向「合成」批次：`clock_compass`（把逐帧贴图并回一张 + 生成 mcmeta）与
/// `particles`（用瓦片重建图集）。两者的源文件都在**延迟删除**里登记，与旧实现一致。
#[path = "reverse/reverse_compose.rs"]
pub mod reverse_compose;
/// 反向「像素回退」：`reverse_fix_ui_survival`（六步：透明填充 → 19 个状态图标缩放粘贴
/// → 填充 → 区域搬移 → 两处填充）。坐标、缩放表与顺序逐行照抄旧实现。
#[path = "reverse/reverse_survival.rs"]
pub mod reverse_survival;
/// 反向 `reverse_fix_armor_models`：把 `entity/equipment/humanoid(_leggings)/*.png` 改名回
/// `models/armor/*_layer_{1,2}.png`（8+8 条，**覆盖**语义，与旧实现逐条对应）。
///
/// 阶段是 **Surgeon**（注册表：`TaskType::Hybrid` / `Tier::Surgeon`）——§9.40 的教训：
/// 声明错阶段会让它被放到 Eraser 段之前，从而被清理点的延迟删除一并带走。
/// 它与仍为旧实现的 `adapt_java_shaders`（同属 Surgeon）在驱动里位置相同，
/// 但两者作用路径不相交（armor 贴图 vs shaders）；安全性由反向整包对照实测。
#[path = "reverse/reverse_armor.rs"]
pub mod reverse_armor;
/// 前向 Architect 批次（真生成逻辑，不能表驱动）——逐个移植。
///
/// `generate_furnace`：把 `gui/container/furnace.png` **复制**成 `blast_furnace.png` 与
/// `smoker.png`（旧实现是两次 `fs::copy`：源保留、目标若存在则覆盖）。
#[path = "architect/arch_gen.rs"]
pub mod arch_gen;
/// `generate_potion_lingering`：把 `items/potion.png` / `potion_bottle_drinkable.png`
/// **拷贝**成 lingering 版本，并把「上三分之一」透明化（方形图整幅；纵向条带逐格），
/// 最后连带拷贝 `.png.mcmeta`。模块名带 `_gen` 后缀以区别于反向的同名删除任务。
#[path = "architect/potion_lingering_gen.rs"]
pub mod potion_lingering_gen;

/// 前向 Architect 批次（续）：依赖**外部 `UImage` 覆盖图**的两个任务。
///
/// 这两个任务的旧实现都有「资源不可用则跳过」的分支，**移植时逐条照抄**——把跳过
/// 写成照做或报错都会改变产物（§9.44 记的正是这条）：
/// - `generate_smithing_ui`：`UImage/smithing/smithing_{width}.png` **存在才**叠加；解析不到
///   `UImage` 目录时只打日志、继续写出；
/// - `generate_fish_bucket`：水桶**先拷贝**，之后**每个**鱼桶各自独立地判断覆盖图是否存在，
///   存在才叠加（不存在就保留拷贝出来的水桶）。
#[path = "architect/arch_gen2.rs"]
pub mod arch_gen2;

/// `generate_shulker_box_ui`：由 `gui/container/generic_54.png` 派生 `shulker_box.png`。
///
/// 语义逐条照抄（**已完成，但暂不派发**——见 §9.51/§9.52）：
/// 1. `determine_scale_factor`：在 {1,2,4,8} 里取 `candidate*256` 与 `max(w,h)` **最接近**者
///    （`exact` 标志在旧实现里**没被使用**，因此这里也不需要）；
/// 2. 清空 `x ∈ [0, 176s)`、`y ∈ [71s, 127s)`（越界处按 `x<width && y<height` 跳过）；
/// 3. 把 `y ∈ [127s, 222s)` 整体**上移 56s**（`saturating_sub`），并把原区间清空——
///    **所有读取都来自原图**（旧实现读 `img` 写 `new_img`），因此源区与目标区重叠时不会自我覆盖。
///
/// **为什么暂不派发**：它的输入 `generic_54.png` 被**更高阶段**（Surgeon 的 GUI 切片链）的旧任务消费，
/// 而它的输出又要在那之前就位——即它需要「Architect 旧任务之后、Surgeon 旧任务之前」这个**中间位置**。
/// 旧批次不可拆分（§9.42），所以这个位置在 Surgeon 也原生化之前并不存在（§9.51）。
/// 实现与语义已就绪，等 Surgeon 就绪后随该阶段一起验收派发。
#[path = "architect/shulker_box_gen.rs"]
pub mod shulker_box_gen;

/// 前向 Architect 批次（续）：铜族与下界合金族 8 个任务。
///
/// 两个模块（`converters/textures/copper.rs`、`netherite.rs`）的形态一致，逐条照抄的要点：
/// - **源缺失 → 整任务跳过**（`iron_ingot` / `diamond_block` 等基准贴图不在包里时什么都不做）；
/// - **拷贝产物 = 同一张 RGBA 重新编码**，不做 `fs::copy`（`tx.put_image`，与旧实现解码后重存等价）；
/// - **`.mcmeta` 附属**：源有 `{src}.png.mcmeta` 才写 `{dst}.png.mcmeta`（旧实现用 `fs::copy`，
///   失败静默忽略——这里保持「有才写」）；
/// - **回退链顺序**：`copper_tools` 是「iron 优先，否则 diamond→gold→stone→netherite」，
///   `copper_armor_models` 是「iron 优先，否则 diamond→gold→chainmail→leather」——**顺序即产物**。
#[path = "architect/arch_gen_metal.rs"]
pub mod arch_gen_metal;

/// 前向 Architect 批次（续）：新木种 3 个任务（`converters/textures/planks.rs`）。
///
/// 三个任务共用两种变换，逐条照抄：
/// - **`process_block_image` / `recolor_rel`**：源存在才做，产物是
///   `adjust_hue_brightness(源, h, b, s)`，并且**源有 `.png.mcmeta` 才写附属**；
/// - **`leaves`**（poplar 专用）：读源 → `force_hue_saturation(源, hue, 6.0, sat, 0.22, 0.88)`
///   → 写目标 → **源有 `.png.mcmeta` 才写附属**（注意它**不**先拷贝源）。
///
/// 参数表（色相/明度/饱和度）全部逐字照抄旧实现——它们在注释里都有来源（原版均色差），
/// 改一个数字就会改变产物。
#[path = "architect/arch_gen_planks.rs"]
pub mod arch_gen_planks;

/// 前向 Architect 批次（续）：`generate_tricky_trials_breeze`（1.21 旋风系贴图）。
///
/// 逐条照抄 `converters/textures/breeze.rs` 的规则表与**三条容易漏的语义**：
/// 1. **`recolor_skip_existing`**：源缺失 → 跳过；**目标已存在 → 跳过**（不覆盖玩家/原版自定义）；
///    只从源**拷贝**再染色，附属 `{src}.png.mcmeta` 存在才一并拷贝；
/// 2. **候选链是「第一个存在者胜」**：刷怪蛋、风充能图标、重核、flow 模板、不祥之瓶各有一条
///    候选列表，命中即 `break`；
/// 3. **不祥试炼钥匙的源是条件选择**：`trial_key.png` 存在就用它，否则用 `gold_ingot.png`。
#[path = "architect/arch_gen_breeze.rs"]
pub mod arch_gen_breeze;

/// 前向 Architect 批次（收尾）：依赖外部 `UImage` 覆盖图的最后三个任务。
///
/// 三个任务的旧实现**都是「取不到覆盖图就整任务跳过」**，而这正是本轮的关键：
/// - `generate_crossbow`：**先要求 `UImage/crossbow/` 解析成功**（失败即整任务返回），
///   且只在 `UImage/crossbow/crossbow_{bow 的宽度}.png` **存在**时才写 `crossbow_standby`；
///   拉弓组只由 `bow_pulling_0` 的宽度决定基准图；
/// - `generate_tipped_arrow_images`：源在**1.9 路径** `textures/items/arrow.png`；
///   缺 `UImage/tipped_arrow_head/tipped_arrow_head_{size}.png` 即整任务跳过；
///   裁头用 `zip` —— 即**任一图短了就在那里停**；
/// - `generate_snow_bucket`：源 `item/milk_bucket.png`；覆盖图**可缺**（缺了就只留拷贝）。
#[path = "architect/arch_gen3.rs"]
pub mod arch_gen3;

/// **Surgeon 早期组**（计划里排在最前面的几个 Surgeon 旧任务）。
///
/// 为什么这组能**逐个**边迁边验（而不像 §9.51 说的「必须整阶段」）：它们的槽位在旧批次**之前**，
/// 所以「放进前阶段」与生产顺序一致——这正是 §9.53 之后把早期原生任务放进
/// `EARLY_NATIVES` 的那条路的自然延伸。
///
/// 本批两个任务的语义要点：
/// - `fix_slider`：由 `gui/widgets.png` **裁两条**贴到一张同尺寸的**全透明**新图——
///   注意目标画布是 `ImageBuffer::new`（全 0，含 alpha），不是原图副本；复制逐像素且**越界即跳过**；
/// - `fix_clock_compass`：把 `items/{clock,compass}.png` 纵向**均分抽帧**成 `{prefix}_{NN}.png`
///   （`num_splits > retain_num` 时按 `floor(i*step)` 取帧并夹到 `num_splits-1`），
///   然后**删掉原图与其 `.mcmeta`**。
#[path = "surgeon/surgeon_early.rs"]
pub mod surgeon_early;

/// **Surgeon 早期组（续）**：`overlay_icons` 与 `fix_brewing_stand_ui`。
///
/// 两个任务都是「整体重写一张 GUI 图」的形态，但**覆盖图的叠加方式不同**，必须分别照抄：
/// - `overlay_icons`：用**覆盖图自己的 alpha 当蒙版**逐像素混合（等价 PIL `paste(overlay, (0,0), overlay)`），
///   且**无论覆盖图是否存在都会重写 `icons.png`**（存在性判定只决定是否混合）；
/// - `fix_brewing_stand_ui`：由 `shulker_box.png` 派生（填 `cover_box` + 把 18×18 区域贴到 5 个位置），
///   覆盖图走 `imageops::overlay`（**源 alpha 混合，非蒙版语义**）。
///
/// 两者当前的**放置**都留在后阶段（见 §9.59/§9.60）：`fix_brewing_stand_ui` 依赖
/// `generate_shulker_box_ui` 的产物，而后者在旧计划里**还没有**（阶段 2–3 在阶段 1–2 之后）——
/// 也就是说它在生产管线里目前也是**空操作**，必须保持这个行为。
#[path = "surgeon/surgeon_early2.rs"]
pub mod surgeon_early2;

/// **Surgeon 组（续）**：`fix2_horse_ui` 与 `fix_sign_entities`。
///
/// 两个任务都留在**后阶段**（实测通过）：它们的读写都在自己独占的目录里
/// （`gui/sprites/container/{horse,slot}` 与 `entity/signs`），不被更早/更晚的旧任务触碰。
///
/// 照抄的重点：
/// - `fix_sign_entities`：11 个木种变体由 `entity/sign.png` 各自 `adjust_hue_brightness` 而来，
///   最后**把原图改名成 `signs/spruce.png`**（云杉就是原图本身；目标已存在时改为直接删源）——
///   漏掉这一步会少一个变体、多一个 `sign.png`；
/// - `fix2_horse_ui`：三个槽位 sprite 的**改名拷贝**（`*_slot.png` → 去掉 `_slot`），源保留。
#[path = "surgeon/surgeon_mid.rs"]
pub mod surgeon_mid;

/// **Surgeon 组（续）**：`fix_horse_ui` 与 `fix_sign`。
///
/// 两者都是「原位重写一张图」+「派生一组变体」，照抄要点：
/// - `fix_horse_ui`：`gui/container/horse.png` **原地**做四步（裁剪 18×18 → 贴到 (18,220)；
///   用 (7,16) 的颜色填回原区域；再把 (36,202) 那块拷到 (36,220)；最后可选叠加
///   `UImage/horse/horse_{width}.png`）。尺寸不在 {256,512,1024,2048} 即整任务跳过；
/// - `fix_sign`：先删已存在的 `spruce_sign.png`、把 `oak_sign.png` **改名**成它，再以它为底
///   生成 11 个木种变体（**其中又包含一个 `oak_sign.png`**——净效果是「橡木 = 原图按 +15 明度再染一次」）。
///
/// **与 `generate_poplar_planks` 的交互（值得单独记）**：poplar 会**写 `item/oak_sign.png`**，
/// 而 `fix_sign` 又**读它**。两者在计划里的顺序是 `fix_sign`（阶段 3–4）**早于** poplar（阶段 88–97），
/// 而当前 `generate_poplar_planks` 在 `EARLY_NATIVES` 里（前阶段）——若 `fix_sign` 留在旧批次里，
/// 它就会读到**尚未被 poplar 改写**的 `oak_sign.png`，与生产顺序相反。
/// 因此本批把 `fix_sign` 也一并原生化并放**后阶段**（= 旧批次之后），使其与生产的相对顺序一致。
#[path = "surgeon/surgeon_mid2.rs"]
pub mod surgeon_mid2;

/// **Surgeon 组（续）**：`fix_particles` —— 把 `particle/particles.png` 切成具名小图。
///
/// 逐条照抄 `converters/textures/particles.rs` 的两个要点：
/// 1. **尺寸守卫**：`w != h || w % 16 != 0` → 整任务跳过（不切、也不删源）；
/// 2. **映射表**：16×16 网格里只有部分格子有名字（`generic_0..7`、`splash_0..3`、
///    `bubble`、`fishing_hook`（写到 **`entity/`** 而不是 `particle/`）、`flame`、`lava`、
///    `note/critical_hit/enchanted_hit`、`heart/angry/glint`、`drip_hang/fall/land`、
///    `effect_0..7`、`spell_0..7`、`spark_0..7`），**其余格子丢弃**；
/// 3. 最后**删掉原 `particles.png`**。
#[path = "surgeon/surgeon_mid3.rs"]
pub mod surgeon_mid3;

/// **Surgeon 组（续）**：`fix_tabs` —— 原位搬移 `gui/container/creative_inventory/tabs.png` 的区域。
///
/// 逐条照抄 `converters/ui/tabs.rs` 的四步（缩放因子走共享的 `determine_scale_factor`，
/// **不是**「必须等于标准尺寸」那套）：
/// 1. 把 (168,0)-(196,128) **右移 14**；
/// 2. 六组区域各自**左移**固定像素（(15,0)-(41,128) 左移 2、(43,…) 左移 4、(71,…) 6、
///    (99,…) 8、(127,…) 10、(155,0)-(168,128) 12），左移用 `saturating_sub`（**不会为负**）；
/// 3. 把 (0,0)-(26,128) **拷到** (156,0)；
/// 4. **无论哪一步都没写**，最后都会把 `tabs.png` 重写一次（旧实现如此）。
///
/// 搬移的语义是「先整体裁剪出源区域、再**逐像素覆盖**贴到目标」——读取全部来自裁剪副本，
/// 因此源区与目标区重叠时不会自我污染，且**越界写入跳过**、**越界读取留透明**。
#[path = "surgeon/surgeon_mid4.rs"]
pub mod surgeon_mid4;

/// **Surgeon 组（续）**：`fix_armor_models` —— 把旧路径的盔甲模型**改名搬走**到新路径。
///
/// 逐条照抄 `converters/textures/armor.rs`：
/// `models/armor/{chainmail,diamond,iron,gold,leather,leather_overlay,netherite,copper}_layer_{1,2}.png`
/// → `entity/equipment/humanoid/`（layer_1）与 `entity/equipment/humanoid_leggings/`（layer_2），
/// 目标名去掉 `_layer_N` 后缀；**源被移走**。`models/armor/` 不存在即整任务跳过。
///
/// **它正是 §9.53 的另一半**：两个 `generate_*_armor_models` 之所以必须放前阶段，
/// 就是因为本任务会把它们的**源**搬走。本任务自己不改写被下游消费的图，因此**放后阶段**即可。
#[path = "surgeon/surgeon_late.rs"]
pub mod surgeon_late;

/// **Surgeon 组（续）**：`fix_ui_sub_hand` 与 `fix_ui_creative` —— 两个「原位改写 GUI 图」的任务。
///
/// 与 `fix_horse_ui`/`fix_slider` 同族，**计划槽位都在最前面**（阶段 1–2），因此放前阶段与生产一致。
/// 两者共用同一套区域原语，逐条照抄：
/// - **搬移/拷贝都是「先裁剪出源区域、再逐像素覆盖贴到目标」**：读取全部来自裁剪副本，
///   源区与目标区重叠也不会自我污染；越界写入跳过、越界读取留透明；
/// - `fix_ui_sub_hand`：`gui/widgets.png` 的 (1,23)-(23,45) 各拷到 (24,23) 与 (60,23)；
/// - `fix_ui_creative`：`…/creative_inventory/tab_inventory.png` 三步——(6,0)-(84,53) 拷到 (51,0)；
///   左区 (6,0)-(53,53) 用 **(164,27)** 的颜色填充（**注意取色发生在第一步拷贝之后**）；
///   再把 (53,5) 起 18×18 拷到 (34,19)（**同理，源取自被第一步改过的图**）。
#[path = "surgeon/surgeon_ui.rs"]
pub mod surgeon_ui;

/// **Surgeon 组（续）**：`fix_machinery_ui` —— 五个「机械方块 GUI」子步骤。
///
/// 逐条照抄 `converters/ui/machinery.rs`。四个子步骤共用 `process_ui_from_shulker`：
/// 读 **`container/shulker_box.png`**（尺寸必须是 256/512/1024/2048 之一，否则跳过），
/// 填 `cover_box` (6,16)-(170,72)（色取自 **(5,4)**），把 18×18 区域（取自 **(7,83)**）**逐像素覆盖**
/// 贴到各自位置表，再可选叠加 `UImage/{subdir}/{prefix}_{width}.png`，最后可选**贴 anvil 区域**
/// （(176,0) 起 28×21；尺寸不符时先把整张 anvil 按 `Nearest` 缩放到 (width,width)）。
///
/// **两种叠加语义不同，必须分开**：UImage 覆盖图用 `imageops::overlay`（按源 alpha 混合），
/// 而区域搬移/贴上用**原始覆盖**（等价 `Image.paste`，alpha 也照抄）——`copy_from` 在越界时会
/// 直接报错，所以这里手写「越界即跳过」的循环，与旧实现一致。
///
/// `process_villager2_machinery` 单独一支：读 `villager.png` → 生成**双宽**新图 → … →
/// **先把原图备份成 `villager_backup.png`（已存在则跳过备份）**，再把新图写回 `villager.png`。
/// 那个备份是反向任务 `reverse_fix_machinery_ui` 还原的依据，**不能漏**。
#[path = "surgeon/surgeon_machinery.rs"]
pub mod surgeon_machinery;

/// **Surgeon 收尾项：`adapt_java_shaders`（第一步：纯文本改写）**。
///
/// 背景（§9.77/§9.78）：真实包里没有 `shaders/`，该任务在第一个判断就返回——
/// **真实包闸门对它给的是假绿灯**，因此迁移必须靠夹具正题（第 12 个忽略用例已钉住三个分支的基线）。
///
/// 本模块是**分步移植的第一步**：只做**纯文本**翻译，不碰文件系统的删改与表格逻辑。
/// 函数与旧实现一一对应，符号名保持一致便于对照：
///
/// | 本模块 | 旧实现 `converters/shaders/java.rs` |
/// |---|---|
/// | [`convert_moj_import_to_include`] | `fn convert_moj_import_to_include` |
/// | [`rewrite_import_path`] | `fn rewrite_import_path` |
/// | [`namespace_moj_imports`] | `fn namespace_moj_imports` |
/// | [`needs_globals_import`] / [`has_globals_import`] / [`inject_globals_import`] | 同名 |
///
/// **尚未移植**（下一步）：表驱动的 `prune_and_rename_core`（4 张表 + 2 个 allowlist）、
/// `ensure_core_json`、`rewrite_json_matrix_types`、`strip_json_uniforms_for_ubo`、
/// `adapt_post_paths`、`walk_dir` 的遍历骨架。**因此本模块暂不派发**（生产路径不变）。
///
/// **§9.85 起已派发**：生产入口是 [`shader_adapt::run_from_pack`]（驱动派发表已登记），
/// 因此移植期用来压住死代码警告的 `#[cfg(test)]` **已移除**。
#[path = "surgeon/shader_adapt.rs"]
pub mod shader_adapt;
/// **Surgeon 组（续）**：`fix_ui_survival` —— 生存背包界面的四步修复。
///
/// 逐条照抄 `converters/ui/survival.rs`：
/// 1. **抽状态图标**：把 `gui/container/inventory.png` 里 y=198 起的三行 18×18 图标
///    （8+8+3 共 19 个）裁出来写成 `mob_effect/*.png`——**这正是 `generate_tricky_trials_breeze`
///    的源**（§9.55 记过这条顺序依赖），所以本任务必须留在**前阶段之前/以内**的正确位置；
/// 2. **移动区域**：把 (86,24)-(162,62) 移到 (+10,-8)。旧实现的搬移有个**特意保留的怪癖**：
///    先用**目标位置**的颜色填掉源区，而目标位置此时尚未被粘贴——照抄，不"顺手修正"；
/// 3. **填充两处背景**（取色点 (90,10)）与**拷贝一块**（(152,26)-(172,46) → (75,60)，**原始覆盖**）；
/// 4. **可选叠加** `UImage/inventory/inventory_{width}.png`（尺寸不符时按 `Lanczos3` 缩放整张）；
///    再抽出两张 1.21 药水背景 sprite 写到 `gui/sprites/container/inventory/`。
///
/// **不复刻旧执行器的纹理缓存**——那是内存优化，与产物无关；
/// 原生实现从 `Tx` 读、写回 `Tx`（缓存类型本身已在 §9.130 随 `hurray` 退场）。
#[path = "surgeon/surgeon_survival.rs"]
pub mod surgeon_survival;
/// **Surgeon 阶段**：`fix_smithing2_villager2_ui` —— 铁砧/村民 GUI 的第二步重排。
///
/// 逐条照抄 `converters/ui/smithing_villager.rs`（364 行，两个子过程）。
/// 与反向任务 `reverse_fix_smithing2_villager2_ui`（已迁移，见 `reverse_defer::smithing_villager`）配对。
///
/// **`s` 的语义**（两个子过程各自判定，互不共享）：
/// - `process_smithing2` 用 **`(width, height)`** 匹配 `(256,256)→1 / (512,512)→2 / (1024,1024)→4 / (2048,2048)→8`，
///   非方形或其它尺寸**跳过**；
/// - `process_villager2` 先要求 **`width == height`**，再用 **`width`** 匹配 `256/512/1024/2048`。
///
/// **刻意照抄的两处**：
/// 1. `get_pixel` 一律用 `*img.get_pixel(..)`（越界会 panic），不做"顺手加保护"；
/// 2. 写回用 `DynamicImage::write_to(.., Png)`——与旧 `img.save(path)` 同一编码路径（§9.87 由闸门实测把关）。
#[path = "surgeon/surgeon_smithing2.rs"]
pub mod surgeon_smithing2;
/// **M2 收尾：`cut_gui` 的 workdir 形态入口**（§9.99）。
///
/// `cut_gui` 是**唯一一个「位置敏感 + 必须直接读盘」**的转换任务：它在计划里的位置是
/// `(15,18)`（夹在旧批次中间），而 `GuiSurgeon` 按设计**直接读写工作目录**。
///
/// `Tx` 形态（`PilotFn = fn(&mut Tx)`）给不了它工作目录——`Tx::origin()` 是**标签**不是路径，
/// 公开方法里也没有任何文件系统路径。因此本模块提供**另一个形态**：由驱动在
/// **旧批次循环的同一位置**调用它，workdir 由驱动传入（驱动本来就有）。
///
/// **边界（如实说明）**：这里**没有**把 `GuiSurgeon` 的 1100+ 行本地化到 `Tx`，
/// 只是把「调用点」从旧适配层搬到了原生模块。代价是它仍直接读写磁盘；
/// 收益是 `cut_gui` 的**计划槽位先原生化**（计入原生任务），且**接口不变**——
/// 将来把本函数内部换成逐函数移植（§9.96 的替换表）即可，无需再改驱动。
///
/// **分辨率**：旧 `cut_gui` 调 `detect_resolution` 后**忽略其返回值**（`GuiSurgeon` 不读它），
/// 因此探测结果对产物无影响；此处照样探测，保持与旧实现同样的日志与副作用。
#[path = "surgeon/surgeon_cut_gui.rs"]
pub mod surgeon_cut_gui;
/// **`GuiSurgeon` 的 `Tx` 本地化（阶段 1：`SPRITE_MAP` 主循环）**（§9.105）。
///
/// 背景（§9.96/§9.104）：`GuiSurgeon` 是 M3 依赖链的**链首**——它是唯一「必须直接读盘」的转换器，
/// 只要它还在读磁盘，`temp_dir` / 旧闭包路径 / `Foray Rom` 三项都无法动。
/// 本模块按 §9.96 的替换表把它的**主循环**（产出 Late 那 62 项 sprite 的那一段）
/// 从「`&Path` + `TexturePool`」改写成「`Tx` + `PackView`」，**不动其余 7 个 `process_*`**。
///
/// **机械替换**（与 §9.96 的表一致）：
/// | 旧 | 新 |
/// |---|---|
/// | `base_path.join(x)` | `x`（`x` 本就是包内相对路径） |
/// | `pool.load_texture(&p)`（失败静默跳过） | `tx.image(x).ok()`（`PackView::image` 缺失即 `Err`，`.ok()` 等价） |
/// | `pool.store_texture(&p, img)` | `tx.put_image(x, &img)` |
/// | `pool.commit_all()` | 不需要（Tx 写层） |
///
/// **本阶段刻意不做**：7 个 `process_*`（`slider`/`icons`/`widgets`/`tabs`/`resource_packs`/
/// `server_selection`/`title`）与 20 个 atlas 文件的**延迟删除**。因此本模块**暂不派发**——
/// 它只用于夹具对照，证明主循环的原生实现与旧实现逐像素一致。
#[path = "surgeon/gui_surgeon_tx.rs"]
pub mod gui_surgeon_tx;


/// 全部试点：`(名称, 声明, 执行体)`，按阶段顺序排列（与旧管线一致）。

/// 全部试点：`(名称, 声明, 执行体)`，按阶段顺序排列（与旧管线一致）。
pub type PilotFn = fn(&mut Tx<'_>) -> Result<Outcome, AromError>;

pub fn all() -> Vec<(&'static str, TaskDecl, PilotFn)> {
    vec![
        ("drop_font", drop_font::decl(), drop_font::run as PilotFn),
        (
            "drop_blockstates",
            drop_blockstates::decl(),
            drop_blockstates::run as PilotFn,
        ),
        ("drop_horse", drop_horse::decl(), drop_horse::run as PilotFn),
        (
            "drop_shaders",
            drop_shaders::decl(),
            drop_shaders::run as PilotFn,
        ),
        ("drop_glint", drop_glint::decl(), drop_glint::run as PilotFn),
        (
            "rename_blocks",
            rename_blocks::decl(),
            rename_blocks::run as PilotFn,
        ),
        ("old_paths", old_paths::decl(), old_paths::run as PilotFn),
        (
            "mcpatcher_optifine",
            mcpatcher_optifine::decl(),
            mcpatcher_optifine::run as PilotFn,
        ),
        ("animated", animated::decl(), animated::run as PilotFn),
    ]
}
