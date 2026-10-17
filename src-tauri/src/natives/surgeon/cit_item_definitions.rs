    //! `generate_cit_item_definitions`：把包里的 **OptiFine CIT**（`optifine/cit/**/*.properties`）
    //! 转成 1.21.4+ 的**物品模型定义**（`assets/minecraft/items/<物品>.json`）。
    //!
    //! ## OptiFine CIT 是什么（本任务处理的输入）
    //!
    //! CIT = Custom Item Textures。一个 `.properties` 文件描述「哪些物品在什么条件下用哪张贴图」：
    //!
    //! ```properties
    //! # assets/minecraft/optifine/cit/ruby/ruby_sword.properties
    //! type=item
    //! items=diamond_sword
    //! texture=ruby_sword.png
    //! nbt.CustomModelData=7
    //! ```
    //!
    //! 语义与字段的**权威出处**（本文件的每条映射都对着它写）：
    //!
    //! * `OptiFineDoc/doc/cit_single.properties`（OptiFine 官方仓库里的注释模板）——
    //!   `items` / `texture` / `texture.<name>` / `model` / `damage` / `enchantments` /
    //!   `enchantmentLevels` / `hand` / `weight` / `components.*` 的官方语义**全在这里**，
    //!   包括「`enchantmentLevels` 只写等级不写附魔名 = **所有附魔等级之和**在区间内」这条
    //!   （原文：`Match any enchantment, with 8-10 total levels across all enchantments`），
    //!   以及「`model=item/mymodel` → `models/item/mymodel.json`、`model=./mymodel` → 与
    //!   properties 同目录的 `mymodel.json`」这两条解析规则。
    //! * `minecraft.wiki/w/Items_model_definition` + `/w/Data_component_predicate`——
    //!   目标侧的字段名与取值（`condition` / `select` / `range_dispatch` / `composite` 的
    //!   字段，`display_context` 的全部取值，`enchantments` predicate 的 `levels` 两种写法）。
    //! * 四个**真实 1.21.5 / 1.21.10 材质包**的 `assets/minecraft/items/*.json`（实测统计见下）。
    //! * 本机官方客户端 jar（`.minecraft/versions/1.21.4|1.21.8|1.21.9|1.21.10|1.21.11|26.2`）里的
    //!   `assets/minecraft/items/*.json`——用来钉**原版默认链条**（弓/弩）。
    //!
    //! ## 目标格式侧的硬边界（这一条决定了「哪些能转」）
    //!
    //! **`minecraft:component` 这个 condition property 是 1.21.5（pack_format 55）才有的。**
    //! 它是「读任意物品组件并与 predicate 精确比较」的唯一入口，因此：
    //!
    //! | 目标格式 | 附魔 / 组件条件 |
    //! |---|---|
    //! | ≥ 55 | 可转（`condition` + `property: minecraft:component`） |
    //! | 46–54 | **不可表达**，如实跳过并在 note 里写明「需要 1.21.5+（pack_format 55）」 |
    //!
    //! 证据是**三方独立**的：① 四个样本包里 format **55** 的那个（《妈妈再也不怕我分不清药水了》，
    //! `min_format: [55,0]`）已经在大规模用这个 property——179 个 `condition(component,
    //! predicate=potion_contents)` 节点；② 三个 format **69** 的样本包合计再用 153 次
    //! `condition(component, predicate=enchantments)`；③ wiki 把它列为 `condition` 的一种
    //! property 类型（带 `predicate` / `value` 两个字段）。
    //!
    //! 反转的事实：**官方客户端数据从来没用过它**——1.21.4 / 1.21.8 / 1.21.9 / 1.21.10 /
    //! 1.21.11 / 26.2 六个 jar 的 `items/*.json`（各 1385–1537 个定义）里
    //! `property: minecraft:component` 出现 **0** 次，`levels` 出现 **0** 次。原版自己不需要
    //! 「按附魔换模型」，所以从 jar 侧**无法**交叉验证这个 property 的存在性；55 / 69 两档能
    //! 工作，靠的是那四个真实包。
    //!
    //! ## 映射规则（本任务实际实现的）
    //!
    //! | OptiFine | 1.21.4+ 物品定义 | 需要 ≥55 |
    //! |---|---|---|
    //! | `items=a,b` + `nbt.CustomModelData=N` + `texture=T` | `items/<a>.json` 与 `<b>.json` 各插一个 `select(custom_model_data)` 分支 `when: N` → 生成的 `cit` 模型 | 否（46 即可） |
    //! | `enchantments=` / `enchantmentIDs=` + `enchantmentLevels=N` | `condition(component, predicate=enchantments)`，`value:[{enchantments, levels}]` | **是** |
    //! | 同上但没写 `enchantmentLevels=` | 同上但 `levels` 整个省略（=「有这附魔就行，任何等级」，与样本包的写法一致） | **是** |
    //! | `hand=main\|off` | `select(display_context)` 的 `cases[].when` 用**数组**列出该手对应的渲染上下文 | 否 |
    //! | `model=M`（`M` 是包内模型 json） | 把该 json **复制**到 `models/cit/<相对路径>.json` 并把其中 `textures` 的相对引用改写成搬好的 `cit/...` 贴图 | 否 |
    //! | `texture=T` | 生成 `models/cit/<相对路径>.json`（`parent` 按贴图名后缀选 handheld/generated），贴图搬到 `textures/cit/<相对路径>.png` | 否 |
    //! | `matchItems=bow` + `texture.bow_pulling_*` | 按**原版弓的定义**重建 `using_item` → `range_dispatch(use_duration)` 链条，叶子换成 CIT 模型 | 否 |
    //! | `type=enchantment\|armor\|elytra` / `items=` 空 / 没有 CMD 也没有其它条件 / `nbt.*`（非 CMD）/ `damage=` / `damageMask=` / `stackSize=` / `components.*` / 数字附魔 id / 只有 `enchantmentLevels` 没有附魔名 / `model.<状态>=` / 弩的多状态贴图 | **跳过**（记 note，写清是哪一条拦下的、为什么） | — |
    //!
    //! ## 为什么以前写「附魔转不了」，现在能转了（这条是**改了结论**，不是改了措辞）
    //!
    //! 旧版本的文件头写：
    //!
    //! > 硬把 `enchantments=` 翻成 `has_component(minecraft:enchantments)` 是错的——会变成
    //! > 「只要有任何附魔就换皮」。
    //!
    //! **推理是对的，结论过时了。** `has_component` 只能问「这个组件在不在」，确实会把
    //! 「锋利 5」放宽成「有附魔就行」。但 1.21.5 起有 `condition(component)`，它带
    //! `predicate` + `value` 两个字段，而 `minecraft:enchantments` 那个 predicate 的
    //! `levels` 既能写区间 `{"min":4}` 也能写**精确值** `{"min":4,"max":4}`：
    //!
    //! ```json
    //! { "type": "minecraft:condition", "property": "minecraft:component",
    //!   "predicate": "minecraft:enchantments",
    //!   "value": [{ "enchantments": "minecraft:sharpness", "levels": { "min": 5, "max": 5 } }],
    //!   "on_true": {...}, "on_false": {...} }
    //! ```
    //!
    //! 也就是说「附魔 + 等级」这个二维条件是**精确可表达**的，一个等级都不会放宽。
    //! 实测出处：`[1.21.10]锋利显示+金剑耐久(1).zip` 的 7 个 `items/*.json` 里用了
    //! 30 个 `condition(component, enchantments)`，等级链是 `{min:5}`→`{min:4}`→…→`{min:1}`
    //! （同一个附魔按等级从高到低嵌套，靠 `on_false` 串起来——本任务照这个次序生成）。
    //!
    //! ## 目标侧形状的实测数字（四个样本包，递归统计 `items/*.json`）
    //!
    //! | building block | 合计 | 用在哪 |
    //! |---|---|---|
    //! | `minecraft:model` | 1148 | 叶子 |
    //! | `minecraft:condition` | 334 | 全部是 `property: minecraft:component` |
    //! | `minecraft:composite` | 92 | 叠加（HUD 数字） |
    //! | `minecraft:select` | 95 | `display_context` 37 / `trim_material` 58 |
    //! | `minecraft:dye` | 96 | 皮革染色 tint |
    //! | `minecraft:range_dispatch` | 3 | `damage` 1 / `use_duration` 2 |
    //! | `minecraft:empty` | 30 | 「不画」 |
    //!
    //! predicate 侧：`potion_contents` 179 次、`enchantments` 153 次。
    //!
    //! ## `composite` 为什么**不**用在本任务里（和最初设想相反，有实测依据）
    //!
    //! 四个样本包里的 `composite` 都是「原模型 + HUD 叠加层」，那是**新创作**（在图标上叠一个
    //! 等级数字），不是 OptiFine 语义的翻译。OptiFine CIT 的 `texture=` / `model=` 是
    //! **整张换掉**，证据是验证包里 `model=` 指向的模型 json 本身：
    //!
    //! ```json
    //! // optifine/cit/sharpdiasword/s2.json
    //! { "parent": "item/handheld", "textures": { "layer0": "./s2" }, "display": {...} }
    //! ```
    //!
    //! 它是一套**自带 parent 与 layer0 的完整外观**（`item/handheld` 的手持变换都在里面），
    //! 不是叠加层。真按 composite 处理，玩家会看到原版贴图与 CIT 贴图**重叠**（验证包里
    //! 32×32 的 CIT 图有 20%–25% 不透明像素，双份必然穿帮）。所以本任务一律**替换**，
    //! 与原实现处理 `nbt.CustomModelData` 的方式一致。
    //!
    //! 「只想在图标角上叠一个小标记」这种**纯覆盖层**意图的 CIT 无法与「替换」区分
    //! （实测：验证包里最薄的弓贴图 72% 全透明，一张正常的手持剑也有 75% 透明，
    //! 用透明度比例判形状不可靠），因此不做自动判定——如实按替换处理。
    //!
    //! ## 优先级（OptiFine 官方文档的原文顺序）
    //!
    //! > If multiple properties files match the same item, the highest weighted one is used.
    //! > In the event of a tie, the properties filenames are compared next.
    //!
    //! 因此规则先按 **`weight` 降序、路径升序**排序，再按这个顺序从外到内嵌套
    //! （最外层 = 最高优先级）。同一物品的多条规则因此都活着，而不是互相覆盖。
    //!
    //! ## 与 CMD 数字的对接点
    //!
    //! 1.20.5 起 `CustomModelData` 变成 `minecraft:custom_model_data` **组件**，
    //! 1.21.4 起它有 `floats` / `flags` / `strings` 三个列表，而**老式数字**留在 `main` 字段。
    //! `select(property = custom_model_data, index = 0)` 读的就是 `main`——
    //! 因此 `nbt.CustomModelData=7` 与 `CustomModelData:7` 的旧地图/命令**逐字节兼容**。

    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    /// OptiFine CIT 的**两个**可能根目录，按优先级排列。
    ///
    /// 为什么是两个：`optifine/` 是现行名字，`mcpatcher/` 是 1.7 时代 MCPatcher 的旧名字，
    /// 老包至今仍在用。`rename_mcpatcher_to_optifine` 会把 `mcpatcher/` 改成 `optifine/`，
    /// 但它挂在 `(7,8)` 边上——而**现代转换路径根本不经过那条边**
    /// （34→55 的路径是 34→42→46→55）。所以 `mcpatcher/cit/**` 会原样留在包里。
    ///
    /// 实测教训：本任务最初只扫 `optifine/`，端到端用例里 CIT 一条都没转出来，
    /// 产物里 `mcpatcher/` 原封不动——**而任务自己报告"没有 CIT"**，一声不响。
    /// 因此这里把两个根都扫，**不依赖那条永远不跑的改名边**。
    const ROOTS: [&str; 2] = [
        "assets/minecraft/optifine/cit",
        "assets/minecraft/mcpatcher/cit",
    ];

    /// 生成的模型放在这个目录下（用 `cit/` 而不是 `item/`，避免与作者自己的模型撞名）。
    const GENERATED_MODELS: &str = "assets/minecraft/models/cit";

    /// 物品定义目录。
    const ITEMS: &str = "assets/minecraft/items";

    /// 物品模型定义出现的第一个格式。
    const FIRST_MODERN_FORMAT: u32 = 46;

    /// **`minecraft:component` condition property 出现的第一个格式**（1.21.5）。
    ///
    /// 为什么它值得一个专门的常量：46–54 与 55+ 的可转集合**在这里分叉**。
    /// 46–54 只能转「模型选择」类条件（CMD / 显示上下文），
    /// 附魔这种「读组件」的条件要到 55 才有入口。
    const COMPONENT_CONDITION_FORMAT: u32 = 55;

    const CMD_COMPONENT: &str = "minecraft:custom_model_data";

    /// **手持类物品的模型后缀**：这些物品在**第一人称/第三人称**里有专门的握持变换，
    /// 用 `item/generated` 会显得"平放在手里"。OptiFine 自己没有这个概念（它直接换贴图，
    /// 沿用物品原有的显示变换），因此这里**必须**替它补上，否则 CIT 转出来的剑会是错的姿势。
    ///
    /// 判定只看**文件名后缀**（OptiFine CIT 的惯例），不看物品名——这样作者自己的
    /// `ruby_sword.png` 也能被正确识别成手持类。
    const HANDHELD_SUFFIXES: [&str; 16] = [
        "_sword", "_pickaxe", "_axe", "_shovel", "_hoe", "_rod", "_stick", "_wand", "_blade",
        "_dagger", "_spear", "_hammer", "_scythe", "_bow", "_crossbow", "_trident",
    ];

    /// **弓的四个状态**（原版 `models/item/` 里的模型名）。
    ///
    /// 只有弓进了这张表，原因写在 [`state_chain`] 上。
    const BOW_STATES: [&str; 4] = ["bow", "bow_pulling_0", "bow_pulling_1", "bow_pulling_2"];

    // ---------------------------------------------------------------------
    // 解析结果的数据模型
    // ---------------------------------------------------------------------

    /// `enchantmentLevels=` 解析出的**一个连续区间**。
    ///
    /// OptiFine 的写法是「值的列表」，每项可以是 `5`（精确）、`3-`（≥3）、`2-4`（闭区间）。
    /// 目标侧 `levels` 是 `MinMaxBounds.Ints`，正好也是这两种边界，所以一对一。
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct LevelRange {
        min: Option<u32>,
        max: Option<u32>,
    }

    impl LevelRange {
        /// `{min, max}`，**两个都写**。
        ///
        /// 为什么不像样本包那样只写 `{min:5}`：样本包写的是「≥5」（它的等级链从 5 递减到 1，
        /// 靠顺序实现「精确」，见文件头）。本任务一次能拿到全部规则，没必要时序依赖，
        /// 直接用 `{min:N,max:N}` 精确锁等级——wiki 对 `levels` 的说明是
        /// 「Matches an exact [Int]value, or checks if the value is between a range」，
        /// 而 `{min,max}` 就是那个 range 写法。
        ///
        /// 这条也回答了「`enchantmentLevels=255` 怎么写」：写 `{"min":255,"max":255}`。
        /// 只写 `{"min":255}` 会变成「≥255」，把 300 级的命令物品也算进来。
        fn to_json(self) -> serde_json::Value {
            let mut map = serde_json::Map::new();
            if let Some(min) = self.min {
                map.insert("min".to_string(), serde_json::Value::from(min));
            }
            if let Some(max) = self.max {
                map.insert("max".to_string(), serde_json::Value::from(max));
            }
            serde_json::Value::Object(map)
        }

        fn describe(self) -> String {
            match (self.min, self.max) {
                (Some(min), Some(max)) if min == max => format!("{min}"),
                (Some(min), Some(max)) => format!("{min}-{max}"),
                (Some(min), None) => format!("{min}-"),
                (None, Some(max)) => format!("-{max}"),
                (None, None) => "任意".to_string(),
            }
        }

        /// 从已生成的 `levels` JSON 反解（幂等去重要用）。
        fn from_json(value: &serde_json::Value) -> Option<Self> {
            let object = value.as_object()?;
            let read = |key: &str| object.get(key).and_then(|v| v.as_u64()).map(|v| v as u32);
            Some(LevelRange {
                min: read("min"),
                max: read("max"),
            })
        }
    }

    /// `hand=any|main|off`。
    ///
    /// 目标侧**没有**「手」这个 property（`minecraft:main_hand` 读的是玩家的主手设置，
    /// 不是物品被画在哪只手上），只能用 `display_context` 枚举上下文来近似：
    /// OptiFine 文档明写「When rendered in the GUI the item is considered to be in the main hand」，
    /// 所以 `main` 要带上 `gui`。
    ///
    /// **未确认的部分**：地面上的掉落物（`ground`）、展示框（`fixed`）、戴在头上（`head`）
    /// 这些「没有手」的上下文，OptiFine 到底还看不看 `hand=` 没有实测依据。本任务让它们
    /// 一律落到 `fallback`（= 原版外观），也就是「只在第一/第三人称区分手」——
    /// 宁可少给一张自定义贴图，也不把条件放宽到不该命中的上下文。
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Hand {
        Any,
        Main,
        Off,
    }

    impl Hand {
        /// 该手对应的 `display_context` 取值（数组形式，`select` 的 `when` 支持数组）。
        fn contexts(self) -> &'static [&'static str] {
            match self {
                Hand::Any => &[],
                Hand::Main => &["gui", "firstperson_righthand", "thirdperson_righthand"],
                Hand::Off => &["firstperson_lefthand", "thirdperson_lefthand"],
            }
        }

        fn describe(self) -> &'static str {
            match self {
                Hand::Any => "any",
                Hand::Main => "main",
                Hand::Off => "off",
            }
        }
    }

    /// 一条规则上的**可表达条件**（外层在前）。
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Cond {
        /// `enchantments=` / `enchantmentIDs=`（+ 可选 `enchantmentLevels=`）。
        Enchant {
            ids: Vec<String>,
            levels: Option<LevelRange>,
        },
        /// `hand=`。
        Hand(Hand),
    }

    /// 一条 CIT 规则里本任务用得上的字段。
    struct CitRule {
        /// `items=` / `matchItems=` 里的物品名（已去重、已排序）。
        items: Vec<String>,
        /// `texture=`：相对 properties 文件的 PNG 路径（可能不带扩展名）。
        texture: Option<String>,
        /// `texture.<状态>=`：多状态物品（弓/弩）的分状态贴图。
        state_textures: BTreeMap<String, String>,
        /// `model=`：直接给定的模型（包内 json 或绝对引用）。
        model: Option<String>,
        /// `nbt.CustomModelData=N`。
        custom_model_data: Option<u64>,
        /// `enchantments=` / `enchantmentIDs=`。
        enchantments: Vec<String>,
        /// `enchantmentLevels=`。
        levels: Option<LevelRange>,
        /// `hand=`。
        hand: Hand,
        /// `weight=`（只影响排序，不影响语义）。
        weight: i64,
        /// 无法转换的条件（用于 note，不是错误）。
        unsupported: Vec<String>,
    }

    impl CitRule {
        /// 把这条规则的条件折成 `Cond` 列表（外层在前）。
        fn conditions(&self) -> Vec<Cond> {
            let mut conds = Vec::new();
            if self.hand != Hand::Any {
                conds.push(Cond::Hand(self.hand));
            }
            if !self.enchantments.is_empty() {
                conds.push(Cond::Enchant {
                    ids: self.enchantments.clone(),
                    levels: self.levels,
                });
            }
            conds
        }

        /// 这条规则有没有「只能用 `component` 条件表达」的依赖（决定 46–54 要不要整条跳过）。
        fn needs_component_condition(&self) -> bool {
            !self.enchantments.is_empty()
        }
    }

    /// 解析 `.properties`：`key=value`，`#` 开头是注释，**首个出现的键优先**。
    ///
    /// 为什么"首个优先"：OptiFine 自己的解析就是"先读到的赢"，
    /// 而很多包会把同名键写两遍（作者手改留下的），照抄行为比"后者覆盖"更接近原貌。
    fn parse_properties(text: &str) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim().to_ascii_lowercase();
            if key.is_empty() {
                continue;
            }
            map.entry(key).or_insert_with(|| value.trim().to_string());
        }
        map
    }

    /// OptiFine 的「列表字段」分隔符**同时**是逗号和空白。
    ///
    /// 踩过的坑：旧实现只按 `,` 切，于是 `matchItems=diamond_sword iron_sword`
    /// （官方文档里 `enchantments=minecraft:silk_touch sharpness smite` 就是**空格**分隔）
    /// 会变成一个名叫 `diamond_sword iron_sword` 的假物品名，产物里凭空多出一个
    /// 永远加载不到的定义。
    fn parse_list(raw: &str) -> Vec<String> {
        raw.split(|c: char| c == ',' || c.is_whitespace())
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// `items=` / `matchItems=` → 物品名列表（去空、去命名空间、去重、排序）。
    fn parse_items(raw: &str) -> Vec<String> {
        let mut items: Vec<String> = parse_list(raw)
            .into_iter()
            .map(|part| part.trim_start_matches("minecraft:").to_string())
            .collect();
        items.sort();
        items.dedup();
        items
    }

    /// 附魔名归一化：OptiFine 允许 `flame` 和 `minecraft:flame` 两种写法。
    ///
    /// 返回值第二项是**数字 id**（`enchantmentIDs=33`）：那是 1.12 时代的数字附魔 id，
    /// 1.13 起附魔只有注册名。官方文档的示例里还留着 `enchantmentIDs=33`（Silk Touch），
    /// 那是历史文档没改，本任务碰到数字就如实跳过。
    fn parse_enchantment_ids(raw: &str) -> (Vec<String>, Vec<String>) {
        let mut ids = Vec::new();
        let mut numeric = Vec::new();
        for token in parse_list(raw) {
            if token.chars().all(|c| c.is_ascii_digit()) {
                numeric.push(token);
                continue;
            }
            let id = if token.contains(':') {
                token
            } else {
                format!("minecraft:{token}")
            };
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids.sort();
        (ids, numeric)
    }

    /// `enchantmentLevels=` → 一个连续区间；`Err` = 这一段无法用目标侧表达。
    ///
    /// OptiFine 是「列表」：`1 3 5` / `3-` / `8-10` / `2-4 6-8`。
    /// 目标侧 `levels` 只有一个 `{min,max}`，所以**只有当列表的并集本身连续时**才能一一对应；
    /// 有断档（`1 3 5`）就无法表达——这时报错跳过，绝不把 `{1,3,5}` 放宽成 `1-5`。
    fn parse_levels(raw: &str) -> Result<Option<LevelRange>, String> {
        let mut ranges: Vec<LevelRange> = Vec::new();
        for token in parse_list(raw) {
            let parse = |text: &str| -> Result<u32, String> {
                text.trim()
                    .parse::<u32>()
                    .map_err(|_| format!("enchantmentLevels= 里的 `{token}` 不是整数"))
            };
            let range = match token.split_once('-') {
                Some((low, high)) => {
                    let min = if low.trim().is_empty() {
                        None
                    } else {
                        Some(parse(low)?)
                    };
                    let max = if high.trim().is_empty() {
                        None
                    } else {
                        Some(parse(high)?)
                    };
                    if let (Some(min), Some(max)) = (min, max) {
                        if min > max {
                            return Err(format!("enchantmentLevels= 里的 `{token}` 区间上下限颠倒"));
                        }
                    }
                    LevelRange { min, max }
                }
                None => {
                    let value = parse(&token)?;
                    LevelRange {
                        min: Some(value),
                        max: Some(value),
                    }
                }
            };
            ranges.push(range);
        }
        if ranges.is_empty() {
            return Ok(None);
        }
        // 排序后尝试合并；只要出现断档就别假装能表达。
        ranges.sort_by_key(|range| range.min.unwrap_or(0));
        let mut merged = ranges[0];
        for next in ranges.iter().skip(1) {
            let next_min = next.min.unwrap_or(0);
            let touches = match merged.max {
                None => true,
                Some(max) => next_min <= max.saturating_add(1),
            };
            if !touches {
                return Err(
                    "enchantmentLevels= 是断开的列表（并集不连续），单个 {min,max} 表达不了".to_string(),
                );
            }
            merged.max = match (merged.max, next.max) {
                (_, None) | (None, _) => None,
                (Some(a), Some(b)) => Some(a.max(b)),
            };
        }
        Ok(Some(merged))
    }

    /// `hand=any|main|off`。
    fn parse_hand(raw: &str) -> Result<Hand, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "any" => Ok(Hand::Any),
            "main" | "right" => Ok(Hand::Main),
            "off" | "left" => Ok(Hand::Off),
            other => Err(format!("hand=`{other}`（只认 any/main/off）")),
        }
    }

    /// 从 `.properties` 的键值对里抽出本任务的规则；`None` = 这不是一条能转的 CIT。
    fn cit_rule(properties: &BTreeMap<String, String>) -> Option<CitRule> {
        // `type` 缺省值是 `item`（OptiFine 文档如此），因此不能因为"没写"就跳过。
        let kind = properties
            .get("type")
            .map(String::as_str)
            .unwrap_or("item")
            .to_ascii_lowercase();
        if kind != "item" {
            return None;
        }
        let raw_items = properties
            .get("items")
            .or_else(|| properties.get("matchitems"))
            .map(String::as_str)
            .unwrap_or("");
        let mut unsupported = Vec::new();
        // **必须**有 `items=`：没有它就无从知道该挂到哪个物品上，
        // 而 OptiFine 的"空 items + 别的条件 = 同类目全物品"在原版里没有对应写法。
        let items = parse_items(raw_items);
        if items.is_empty() {
            unsupported.push("items=（空）".to_string());
        }

        // ---- 附魔条件（1.21.5+ 可转，见文件头的边界说明） ----
        let mut enchantments = Vec::new();
        let raw_enchant = properties
            .get("enchantments")
            .or_else(|| properties.get("enchantmentids"))
            .map(String::as_str)
            .unwrap_or("");
        if !raw_enchant.trim().is_empty() {
            let (ids, numeric) = parse_enchantment_ids(raw_enchant);
            if !numeric.is_empty() {
                unsupported.push(format!(
                    "enchantmentIDs=（数字 id {}：1.13 起附魔只有注册名）",
                    numeric.join(",")
                ));
            }
            enchantments = ids;
        }
        let mut levels = None;
        if let Some(raw) = properties.get("enchantmentlevels") {
            match parse_levels(raw) {
                Ok(parsed) => levels = parsed,
                Err(reason) => unsupported.push(reason),
            }
        }
        // **只有等级、没有附魔名**：OptiFine 官方文档写得很清楚——
        // 「Match any enchantment, with 8-10 **total levels across all enchantments**」，
        // 即所有附魔的等级**之和**落在区间内。目标侧 `levels` 不写 `enchantments` 时是
        // 「任意单个附魔的等级落在区间内」（wiki 原文：「succeeds when any of enchantments
        // on the item matches the level」）——两者**不等价**：锋利 5 + 耐久 3 的和是 8，
        // 但没有任何单条附魔是 8..10。这种事只能如实跳过。
        if enchantments.is_empty() && levels.is_some() {
            unsupported.push(
                "enchantmentLevels=（没写 enchantmentIDs：OptiFine 指所有附魔等级之和，目标侧只能表达单条附魔的等级）"
                    .to_string(),
            );
            levels = None;
        }

        let hand = match properties.get("hand") {
            Some(raw) => match parse_hand(raw) {
                Ok(hand) => hand,
                Err(reason) => {
                    unsupported.push(reason);
                    Hand::Any
                }
            },
            None => Hand::Any,
        };

        let weight = properties
            .get("weight")
            .and_then(|raw| raw.trim().parse::<i64>().ok())
            .unwrap_or(0);

        // ---- 明确不可表达的条件 ----
        // `damage=` 单独说：目标侧确实有 `range_dispatch(property: minecraft:damage)`，
        // 但它的语义是「取**最大的 ≤ 值的阈值**」——只有「≥ 某值」这一半。
        // OptiFine 的 `damage=` 是**精确值/区间的列表**（`damage=50%` 是「正好一半耐久」，
        // `damage=0-50%` 是闭区间），要表达区间上限就得把「后续整条链」复制一份塞进
        // 阈值分支里，规则一多就指数膨胀。放宽成「≥」会让满耐久的剑也换皮，
        // 所以这里如实跳过。`damageMask=`（按位掩码取药水子类型）更是没有对应。
        for blocked in [
            "damage",
            "damagemask",
            "stacksize",
            "biome",
            "weather",
            "health",
            "food",
            "level",
            "fishingrod",
            "compass",
            "clock",
            "armor",
            "elytra",
            "collarcolor",
            "villager",
            "profession",
            "entity",
        ] {
            if properties.contains_key(blocked) {
                unsupported.push(format!("{blocked}="));
            }
        }
        // `nbt.*` 里**只有** `CustomModelData` 能被原版表达。其余（烟花、显示名、lore…）
        // 在物品定义里没有入口。注意 `custom_data` 那一个 predicate **不能**顶替它：
        // 它的 value 是 NBT 子集测试的固定语法，不是 OptiFine 的任意 NBT 路径 + 前缀正则。
        for (key, _) in properties.iter() {
            if let Some(tag) = key.strip_prefix("nbt.") {
                if !tag.eq_ignore_ascii_case("custommodeldata") {
                    unsupported.push(format!("nbt.{tag}="));
                }
            }
            // 1.21+ 的 OptiFine 还支持 `components.<名字>=<值>`，语义上**正好**能落到
            // `condition(component)` 上，但它的值语法是一整套（类型前缀 / 通配 / 正则 /
            // 负匹配 / 转义命名空间 `minecraft\:xxx`），解析错一处就是静默换错贴图。
            // 本任务不做，如实记下。
            if key.starts_with("components.") {
                unsupported.push(format!("{key}="));
            }
        }
        let custom_model_data = properties
            .get("nbt.custommodeldata")
            .and_then(|value| value.trim().parse::<u64>().ok());
        // `nbt.CustomModelData` 写了但解析不出数字（可能是表达式）→ 当作不支持
        if properties.contains_key("nbt.custommodeldata") && custom_model_data.is_none() {
            unsupported.push("nbt.CustomModelData=（非数字）".to_string());
        }

        // `texture.<状态>=`：多状态贴图。`model.<状态>=` 是同一机制的模型版，
        // 本任务只做贴图版（状态链见 state_chain 的说明）。
        let mut state_textures = BTreeMap::new();
        for (key, value) in properties.iter() {
            if let Some(state) = key.strip_prefix("texture.") {
                state_textures.insert(state.to_string(), value.to_string());
            }
            if key.starts_with("model.") {
                unsupported.push(format!("{key}="));
            }
        }
        // `model=` 与 `texture.<状态>=` 同时出现：前者给整张模型、后者给某个状态的贴图，
        // 两者叠起来到底谁盖谁没有实测样本，不猜。
        if properties.contains_key("model") && !state_textures.is_empty() {
            unsupported.push(
                "model= + texture.<状态>=（同时给整张模型和分状态贴图：叠加语义无实测样本）".to_string(),
            );
        }

        Some(CitRule {
            items,
            texture: properties.get("texture").cloned(),
            state_textures,
            model: properties.get("model").cloned(),
            custom_model_data,
            enchantments,
            levels,
            hand,
            weight,
            unsupported,
        })
    }

    /// 把**相对路径**解析成包内路径，处理 `.` 与 `..` 段。
    ///
    /// 为什么要自己写而不是拼字符串：老包里的 CIT 大量使用
    /// `texture=../../../textures/items/ruby.png` 这种写法来指回原版贴图目录，
    /// 直接拼接会得到一条带 `..` 的假路径，`Tx::exists` 永远查不到。
    fn resolve_relative(base: &str, relative: &str) -> String {
        let mut parts: Vec<&str> = base.split('/').filter(|p| !p.is_empty()).collect();
        for segment in relative.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                other => parts.push(other),
            }
        }
        parts.join("/")
    }

    /// 判断一张 CIT 贴图该用哪个原版父模型。
    fn parent_for(texture_or_model: &str) -> &'static str {
        let stem = texture_or_model
            .rsplit('/')
            .next()
            .unwrap_or(texture_or_model)
            .trim_end_matches(".png");
        if HANDHELD_SUFFIXES
            .iter()
            .any(|suffix| stem.to_ascii_lowercase().ends_with(suffix))
        {
            "minecraft:item/handheld"
        } else {
            "minecraft:item/generated"
        }
    }

    /// 贴图名 → 候选包内路径（含「补 `.png`」这一步）。
    ///
    /// 为什么必须补扩展名：OptiFine 文档的 `texture=` 示例就**不带**扩展名
    /// （`texture=excalibur -> optifine/cit/excalibur.png`），验证包里的
    /// `texture=bow` / `texture=q3` 同样不带。旧实现只按原样查，于是这类规则
    /// **一条都找不到贴图**而被静默跳过（验证包 7 条弓/弩规则全中）。
    fn texture_candidates(directory: &str, raw: &str) -> Vec<String> {
        let names: Vec<String> = if raw.to_ascii_lowercase().ends_with(".png") {
            vec![raw.to_string()]
        } else {
            vec![raw.to_string(), format!("{raw}.png")]
        };
        let mut out: Vec<String> = Vec::new();
        for name in names {
            if name.starts_with("assets/") {
                out.push(name);
                continue;
            }
            if name.starts_with("optifine/") || name.starts_with("mcpatcher/") {
                out.push(format!("assets/minecraft/{name}"));
                continue;
            }
            // ① 与 properties 同目录（MCPatcher/OptiFine 的实际行为，验证包的
            //    `bow0/0.properties` + `bow0/bow.png` 就是靠这条命中）。
            out.push(resolve_relative(directory, &name));
            // ② 旧式 `textures/items/` ↔ 新式 `textures/item/`（1.13 改过名）。
            out.push(resolve_relative(
                directory,
                &name.replacen("textures/item/", "textures/items/", 1),
            ));
            out.push(resolve_relative(
                directory,
                &name.replacen("textures/items/", "textures/item/", 1),
            ));
            // ③ OptiFine 文档：「All paths are relative to assets/minecraft」。
            out.push(format!("assets/minecraft/{name}"));
            out.push(format!("assets/minecraft/textures/{name}"));
        }
        out.dedup();
        out
    }

    /// 找一张贴图在包里的真实位置。
    fn resolve_texture(
        tx: &Tx<'_>,
        directory: &str,
        raw: &str,
    ) -> Result<Option<String>, AromError> {
        for candidate in texture_candidates(directory, raw) {
            if tx.exists(&candidate) {
                return Ok(Some(candidate));
            }
        }
        Ok(None)
    }

    /// `model=` 的解析结果。
    enum ModelTarget {
        /// 包内确实有这个模型 json：要复制到 `models/cit/` 并改写其中的相对引用。
        InPack(String),
        /// 可直接引用的模型（带命名空间的作者引用，或 `item/…` 这种原版路径）。
        Reference(String),
    }

    /// `model=` → 候选路径。
    ///
    /// OptiFine 官方文档给了两条规则：
    /// `model=item/mymodel -> assets/minecraft/models/item/mymodel.json`、
    /// `model=./mymodel -> 与 properties 同目录的 mymodel.json`。
    /// 验证包里大量出现的是**第三条没写进文档的**：裸名 `model=s2`，而 `s2.json`
    /// 就在 properties 旁边（`optifine/cit/sharpdiasword/s2.json`）——所以同目录必须排第一，
    /// 否则这 6 条规则（`255axe/*`、`sharpdiasword/*`）一条都转不出来。
    ///
    /// 踩过的坑：`255axe/gold_axe.properties` 写 `model=golden_axe`，而包里**同时**有
    /// `optifine/cit/255axe/golden_axe.json`（CIT 自己的大斧模型，layer0=`./mrf0`）和
    /// `models/item/golden_axe.json`（包作者给出的原版斧模型）。按 OptiFine 的
    /// 「路径相对 properties 文件」规则，前者才是作者指的那张；顺序写反就会静默用错模型。
    fn model_candidates(directory: &str, raw: &str) -> Vec<String> {
        let name = raw.trim();
        let mut out: Vec<String> = Vec::new();
        if let Some(rest) = name.strip_prefix("./") {
            out.push(format!("{}{rest}.json", suffix_dir(directory)));
            return out;
        }
        if let Some((namespace, path)) = name.split_once(':') {
            out.push(format!("assets/{namespace}/models/{path}.json"));
            return out;
        }
        // 同目录优先（见上面的踩坑说明）。
        out.push(format!("{}{name}.json", suffix_dir(directory)));
        // 文档里的 `item/mymodel` 写法。
        out.push(format!("assets/minecraft/models/{name}.json"));
        // 裸名但作者指的是原版 `models/item/` 目录。
        out.push(format!("assets/minecraft/models/item/{name}.json"));
        out
    }

    fn suffix_dir(directory: &str) -> String {
        if directory.is_empty() {
            String::new()
        } else {
            format!("{directory}/")
        }
    }

    fn resolve_model(
        tx: &Tx<'_>,
        directory: &str,
        raw: &str,
    ) -> Result<Option<ModelTarget>, AromError> {
        for candidate in model_candidates(directory, raw) {
            if tx.exists(&candidate) {
                return Ok(Some(ModelTarget::InPack(candidate)));
            }
        }
        let name = raw.trim();
        if name.contains(':') {
            // 作者写的是绝对引用（可能来自别的包/原版）→ 原样引用，不猜。
            return Ok(Some(ModelTarget::Reference(name.to_string())));
        }
        if name.starts_with("item/") || name.starts_with("block/") {
            // 文档语义：这是 `assets/minecraft/models/<path>.json` 的短写，包里没有再交给原版。
            return Ok(Some(ModelTarget::Reference(format!("minecraft:{name}"))));
        }
        // 裸名且包内找不到同目录 json：指向不存在的模型只会得到紫黑格。
        Ok(None)
    }

    /// 把一段名字压成可做文件名的片段（`./a/b` → `a_b`）。
    fn sanitize(raw: &str) -> String {
        raw.trim_start_matches("./")
            .chars()
            .map(|c| if c == '/' || c == '\\' { '_' } else { c })
            .collect::<String>()
            .trim_end_matches(".png")
            .to_string()
    }

    /// 把模型 json 里的 `textures` 相对引用改写成搬好的绝对引用。
    ///
    /// 为什么非要改：模型的 `textures.layer0` 是**贴图引用**，客户端只在
    /// `assets/<ns>/textures/<path>.png` 下找，且 `./` 这种段在资源路径里没有意义
    /// （验证包里 4 个模型都写 `"layer0": "./s2"`，原样复制过去就是紫黑格）。
    fn rewrite_textures(
        tx: &mut Tx<'_>,
        model: &mut serde_json::Value,
        model_dir: &str,
        namespace: &str,
    ) -> Result<(), AromError> {
        let Some(textures) = model.get_mut("textures").and_then(|t| t.as_object_mut()) else {
            return Ok(());
        };
        let keys: Vec<String> = textures.keys().cloned().collect();
        for key in keys {
            let Some(value) = textures
                .get(&key)
                .and_then(|v| v.as_str())
                .map(str::to_string)
            else {
                continue;
            };
            if value.contains(':') {
                continue; // 已是绝对贴图引用
            }
            let name = value.trim_start_matches("./").to_string();
            // ① 与模型 json 同目录（`./s2` / `s2`）。找不到就留原样：裸名按
            //    `minecraft:` 命名空间解析到 `assets/minecraft/textures/<name>.png`，
            //    原版贴图就是这么引用的，不用动。
            let Some(found) = resolve_texture(tx, model_dir, &name)? else {
                continue;
            };
            // ② 已经躺在标准贴图目录里的就别搬，原地引用更省。
            if let Some(inside) = found.strip_prefix("assets/minecraft/textures/") {
                textures.insert(
                    key.clone(),
                    serde_json::Value::from(inside.trim_end_matches(".png")),
                );
                continue;
            }
            let file = sanitize(&name);
            let target = format!("assets/minecraft/textures/cit/{namespace}/{file}.png");
            if !tx.exists(&target) {
                let bytes = tx.read(&found)?.unwrap_or_default();
                tx.put(&target, bytes)?;
            }
            textures.insert(
                key.clone(),
                serde_json::Value::from(format!("cit/{namespace}/{file}")),
            );
        }
        Ok(())
    }

    /// 递归复制一个模型 json（含 `./` 形式的 parent），返回模型引用。
    fn copy_model(
        tx: &mut Tx<'_>,
        source: &str,
        dest: &str,
        namespace: &str,
        depth: u32,
    ) -> Result<Option<String>, AromError> {
        let Some(bytes) = tx.read(source)? else {
            return Ok(None);
        };
        let Ok(mut model) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            return Ok(None);
        };
        let model_dir = source.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
        // `overrides` 是 1.20 及以前的物品模型字段，1.21.4+ 的模型里已经不存在；
        // 留着就是把一份旧 CMD 表塞进一个不再被读的字段（而 CMD 表由
        // `generate_item_definitions` 单独负责），所以剥掉。
        if let Some(object) = model.as_object_mut() {
            object.remove("overrides");
        }
        rewrite_textures(tx, &mut model, model_dir, namespace)?;
        // `./` 形式的 parent 也要一起搬（否则复制出来的模型指着一个不存在的 parent）。
        if let Some(parent) = model
            .get("parent")
            .and_then(|p| p.as_str())
            .map(str::to_string)
        {
            if parent.starts_with("./") && depth > 0 {
                let parent_source =
                    resolve_relative(model_dir, parent.trim_start_matches("./")) + ".json";
                let parent_dest = format!("{dest}.parent{depth}");
                if let Some(parent_ref) =
                    copy_model(tx, &parent_source, &parent_dest, namespace, depth - 1)?
                {
                    model["parent"] = serde_json::Value::from(parent_ref);
                }
            }
        }
        let text = serde_json::to_string_pretty(&model)
            .map_err(|e| AromError::internal(format!("序列化模型 `{dest}` 失败：{e}")))?;
        tx.put(dest, text.into_bytes())?;
        Ok(Some(model_reference(dest)))
    }

    /// 包内模型路径 → 模型引用（`assets/minecraft/models/cit/x.json` → `minecraft:cit/x`）。
    fn model_reference(path: &str) -> String {
        format!(
            "minecraft:{}",
            path.strip_prefix("assets/minecraft/models/")
                .unwrap_or(path)
                .trim_end_matches(".json")
        )
    }

    /// 一个 `minecraft:model` 叶子。
    fn leaf(reference: &str) -> serde_json::Value {
        serde_json::json!({ "type": "minecraft:model", "model": reference })
    }

    /// **弓的状态链**，逐字段照抄官方 1.21.4 / 1.21.10 / 26.2 的 `items/bow.json`
    /// （三个 jar 的弓定义**逐字节同构**，所以这张模板跨版本安全）：
    ///
    /// ```text
    /// condition(using_item)
    ///   ├─ on_true  → range_dispatch(use_duration, scale 0.05)
    ///   │               ├─ 0.65 → bow_pulling_1
    ///   │               ├─ 0.90 → bow_pulling_2
    ///   │               └─ fallback → bow_pulling_0
    ///   └─ on_false → bow
    /// ```
    ///
    /// **为什么只有弓进了这张表**：弩的链路在 46 与 69 之间**改过结构**——
    /// 1.21.4 是 `condition(using_item) → select(charge_type)`（拉弦时绕过 charge_type），
    /// 1.21.9+ 反过来是 `select(charge_type) → condition(using_item)`（本机 1.21.4 /
    /// 1.21.9 / 1.21.10 / 26.2 四个 jar 实测）。55–68 这几档没有 jar 可查，
    /// 按错的模板重建会让整张弩的渲染走偏，所以弩的 `texture.crossbow_*` 一律如实跳过。
    fn state_chain(
        item: &str,
        model: &dyn Fn(&str) -> serde_json::Value,
    ) -> Option<serde_json::Value> {
        match item {
            "bow" => Some(serde_json::json!({
                "type": "minecraft:condition",
                "property": "minecraft:using_item",
                "on_true": {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:use_duration",
                    "scale": 0.05,
                    "entries": [
                        { "threshold": 0.65, "model": model("bow_pulling_1") },
                        { "threshold": 0.9, "model": model("bow_pulling_2") },
                    ],
                    "fallback": model("bow_pulling_0"),
                },
                "on_false": model("bow"),
            })),
            _ => None,
        }
    }

    /// 原版某个状态的模型引用（`bow` → `minecraft:item/bow`）。
    fn vanilla_state_model(state: &str) -> serde_json::Value {
        leaf(&format!("minecraft:item/{state}"))
    }

    /// 没有既有定义时，物品的**默认外观**。
    ///
    /// 为什么不能一律用「自指模型」：`items/bow.json` 一旦被我们写出来，就**接管**了原版
    /// 那张定义（客户端不会把包里的定义与原版的合并）。若默认分支写成
    /// `{"type":"model","model":"minecraft:item/bow"}`，未附魔的弓会永远停在静态贴图上，
    /// **拉弦动画整段消失**。所以对有已知状态链的物品，默认分支用**原版那张链**。
    fn default_base(item: &str) -> serde_json::Value {
        if let Some(chain) = state_chain(item, &vanilla_state_model) {
            return chain;
        }
        leaf(&format!("minecraft:item/{item}"))
    }

    /// 把一条条件包在 `on_true` 上，`on_false` 接后续链路。
    fn wrap_condition(
        cond: &Cond,
        on_true: &serde_json::Value,
        on_false: &serde_json::Value,
    ) -> serde_json::Value {
        match cond {
            Cond::Enchant { ids, levels } => {
                // `minecraft:enchantments` 这个 data component predicate 的格式（wiki 原文）：
                // 数组，每项 `{enchantments, levels}`，**各项之间是 AND**；
                // 单项里的 `enchantments` 可以给数组，**元素之间是 OR**。
                // 所以「多个附魔名」用一项搞定（与 OptiFine 的「任一命中」一一对应），
                // 而「多个不同等级段」才需要嵌套（`parse_levels` 已保证只有一段）。
                let mut predicate = serde_json::Map::new();
                let enchantments = if ids.len() == 1 {
                    serde_json::Value::from(ids[0].clone())
                } else {
                    serde_json::Value::Array(
                        ids.iter().cloned().map(serde_json::Value::from).collect(),
                    )
                };
                predicate.insert("enchantments".to_string(), enchantments);
                if let Some(levels) = levels {
                    predicate.insert("levels".to_string(), levels.to_json());
                }
                serde_json::json!({
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:enchantments",
                    "value": [serde_json::Value::Object(predicate)],
                    "on_true": on_true,
                    "on_false": on_false,
                })
            }
            Cond::Hand(hand) => serde_json::json!({
                "type": "minecraft:select",
                "property": "minecraft:display_context",
                // `when` 可以是**数组**，一项就能表达「这只手的全部上下文」。
                "cases": [{ "when": hand.contexts(), "model": on_true }],
                "fallback": on_false,
            }),
        }
    }

    /// 把若干条件按外层到内层包起来。
    ///
    /// 注意 `rest` 在**每条**条件的失败分支上都要出现：`A? (B? leaf : rest) : rest`。
    /// 一条条件时就是 `A? leaf : rest`，不复制；两条（`hand` + 附魔）时才复制一份。
    /// 这是 `condition` 只有 `on_true`/`on_false` 两个出口的必然结果，语义上是对的。
    fn apply_conditions(
        conds: &[Cond],
        leaf: &serde_json::Value,
        rest: &serde_json::Value,
    ) -> serde_json::Value {
        match conds.split_first() {
            None => leaf.clone(),
            Some((first, tail)) => {
                let inner = apply_conditions(tail, leaf, rest);
                wrap_condition(first, &inner, rest)
            }
        }
    }

    /// 一个附魔条件 + 它的叶子的**指纹**，用于「跑第二遍不重复嵌套」。
    ///
    /// 为什么要它：本任务会把整条链重建一遍。若对同一份输入跑两次（重跑、或驱动重放
    /// 同一个任务），没有去重就会得到 `condition(A) → condition(A) → …` 的两层嵌套。
    /// 只认附魔节点——`hand` 那层用 `select`，不会有别的任务写，重复风险低。
    fn enchant_fingerprint(
        ids: &[String],
        levels: Option<LevelRange>,
        leaf: &serde_json::Value,
    ) -> String {
        format!(
            "{}|{}|{}",
            ids.join("+"),
            levels.map(LevelRange::describe).unwrap_or_default(),
            subtree_model_refs(leaf)
        )
    }

    /// 子树里出现的全部模型引用（排序去重后拼起来）。
    fn subtree_model_refs(node: &serde_json::Value) -> String {
        fn walk(node: &serde_json::Value, out: &mut Vec<String>) {
            match node {
                serde_json::Value::Object(map) => {
                    if let Some(model) = map.get("model").and_then(|m| m.as_str()) {
                        out.push(model.to_string());
                    }
                    for value in map.values() {
                        walk(value, out);
                    }
                }
                serde_json::Value::Array(items) => {
                    for item in items {
                        walk(item, out);
                    }
                }
                _ => {}
            }
        }
        let mut refs = Vec::new();
        walk(node, &mut refs);
        refs.sort();
        refs.dedup();
        refs.join(",")
    }

    /// 已有定义树里的全部附魔条件指纹。
    fn existing_enchant_fingerprints(root: &serde_json::Value) -> BTreeSet<String> {
        fn walk(node: &serde_json::Value, out: &mut BTreeSet<String>) {
            match node {
                serde_json::Value::Object(map) => {
                    if map.get("property").and_then(|p| p.as_str()) == Some("minecraft:component")
                        && map.get("predicate").and_then(|p| p.as_str())
                            == Some("minecraft:enchantments")
                    {
                        let mut ids: Vec<String> = Vec::new();
                        let mut levels = None;
                        if let Some(first) = map
                            .get("value")
                            .and_then(|v| v.as_array())
                            .and_then(|list| list.first())
                        {
                            match first.get("enchantments") {
                                Some(serde_json::Value::String(id)) => ids.push(id.clone()),
                                Some(serde_json::Value::Array(list)) => {
                                    ids = list
                                        .iter()
                                        .filter_map(|v| v.as_str().map(str::to_string))
                                        .collect()
                                }
                                _ => {}
                            }
                            levels = first.get("levels").and_then(LevelRange::from_json);
                        }
                        if !ids.is_empty() {
                            ids.sort();
                            let on_true = map.get("on_true").cloned().unwrap_or_default();
                            out.insert(enchant_fingerprint(&ids, levels, &on_true));
                        }
                    }
                    for value in map.values() {
                        walk(value, out);
                    }
                }
                serde_json::Value::Array(items) => {
                    for item in items {
                        walk(item, out);
                    }
                }
                _ => {}
            }
        }
        let mut out = BTreeSet::new();
        walk(root, &mut out);
        out
    }

    /// 一条待并入物品定义的 `nbt.CustomModelData` 规则。
    struct CmdEntry {
        threshold: u64,
        model: serde_json::Value,
        reference: String,
        source: String,
    }

    /// 一条待并入物品定义的条件规则。
    struct NodeEntry {
        conds: Vec<Cond>,
        leaf: serde_json::Value,
        /// 叶子指向的模型引用（写 note 用）。
        reference: String,
        describe: String,
        source: String,
        /// 幂等指纹（只有附魔条件才有）。
        fingerprint: Option<String>,
    }

    #[derive(Default)]
    struct ItemPlan {
        cmds: Vec<CmdEntry>,
        nodes: Vec<NodeEntry>,
    }

    /// 把收集到的规则并进某个物品的定义。返回 (新定义, 实际并进去的条数, notes)。
    fn merge_into_definition(
        item: &str,
        existing: Option<serde_json::Value>,
        plan: &mut ItemPlan,
    ) -> Option<(serde_json::Value, usize, Vec<String>)> {
        let mut notes = Vec::new();
        let mut definition = match existing {
            Some(value) => value,
            // 物品定义还不存在（比如作者只想用 CIT，没写过 `models/item/`）：
            // 造一个语义上等价于原版默认外观的定义。
            None => serde_json::json!({ "model": default_base(item) }),
        };
        let mut base = definition.get("model")?.clone();
        let mut applied = 0usize;

        // ---- 第一层（内）：CMD 表 ----
        if !plan.cmds.is_empty() {
            plan.cmds.sort_by_key(|entry| entry.threshold);
            let mut merged_into_existing = false;
            if base.get("property").and_then(|p| p.as_str()) == Some(CMD_COMPONENT)
                && base.get("cases").map(|c| c.is_array()).unwrap_or(false)
            {
                // 已经有一个 CMD `select`（可能是 `generate_item_definitions` 留的）就并进去。
                let cases = base.get_mut("cases")?.as_array_mut()?;
                for entry in plan.cmds.drain(..) {
                    let duplicated = cases.iter().any(|case| {
                        case.get("when").and_then(|w| w.as_u64()) == Some(entry.threshold)
                    });
                    if duplicated {
                        // 同一物品同一 CMD 被两条 CIT 命中：**保留先写的那条**，
                        // 不覆盖（OptiFine 的同类冲突行为本就未定义，静默取一个总比来回抖好）。
                        continue;
                    }
                    cases.push(serde_json::json!({
                        "when": entry.threshold,
                        "model": entry.model,
                    }));
                    applied += 1;
                    merged_into_existing = true;
                    notes.push(format!(
                        "CIT {} → items/{item}.json（CustomModelData={} → {}）",
                        entry.source, entry.threshold, entry.reference
                    ));
                }
            }
            if !merged_into_existing {
                let mut cases: Vec<serde_json::Value> = Vec::new();
                for entry in plan.cmds.drain(..) {
                    let duplicated = cases.iter().any(|case| {
                        case.get("when").and_then(|w| w.as_u64()) == Some(entry.threshold)
                    });
                    if duplicated {
                        continue;
                    }
                    cases.push(serde_json::json!({
                        "when": entry.threshold,
                        "model": entry.model,
                    }));
                    applied += 1;
                    notes.push(format!(
                        "CIT {} → items/{item}.json（CustomModelData={} → {}）",
                        entry.source, entry.threshold, entry.reference
                    ));
                }
                if !cases.is_empty() {
                    base = serde_json::json!({
                        "type": "minecraft:select",
                        "property": CMD_COMPONENT,
                        "index": 0,
                        "cases": cases,
                        "fallback": base,
                    });
                }
            }
        }

        // ---- 第二层（外）：附魔 / 手 条件 ----
        //
        // 为什么条件在**外层**、CMD 在内层：`generate_item_definitions` 可能已经在该
        // 物品的根上套了一层 CMD 表，而 CIT 是作者后来写的规则，应当优先于旧
        // `overrides` 里留下的数字表（与 OptiFine 里 CIT 优先于 overrides 一致）。
        //
        // **倒序折**：`plan.nodes` 已按 OptiFine 优先级（weight 降序、路径升序）排好，
        // 而每折一次都是「把当前树塞进新条件的 `on_false`」——最后折的会在**最外层**。
        // 所以要让优先级最高的那条最后折，也就是从列表尾部开始。
        // 踩过的坑：正序折会让**优先级最低**的规则跑到最外层，于是锐利 1 会把锐利 5 挡住。
        let entries: Vec<NodeEntry> = plan.nodes.drain(..).collect();
        for entry in entries.into_iter().rev() {
            if let Some(fingerprint) = &entry.fingerprint {
                if existing_enchant_fingerprints(&base).contains(fingerprint) {
                    continue; // 第二遍跑：这条已经在了
                }
            }
            base = apply_conditions(&entry.conds, &entry.leaf, &base);
            applied += 1;
            notes.push(format!(
                "CIT {} → items/{item}.json（{} → {}）",
                entry.source, entry.describe, entry.reference
            ));
        }

        if applied == 0 {
            return None;
        }
        definition["model"] = base;
        Some((definition, applied, notes))
    }

    pub fn decl() -> TaskDecl {
        // **`exclusive(true)`**：本任务的读范围跨**两个** CIT 根目录，写范围又跨
        // `items/`（别的任务刚写过）与 `models/cit/`、`textures/cit/`，而且它要**读回并改写**
        // 那些 `items/*.json`。声明成独占是保守但明确的选择：CIT 是低频任务
        // （多数包根本没有），让它单独占一个波次换来的确定性远比那点并发收益值钱。
        //
        // 读范围里必须**包含 `models/`**：`model=` 的解析（OptiFine 文档的
        // `model=item/mymodel` 短写）要去看 `assets/minecraft/models/**.json` 在不在。
        let mut reads = ScopeSet::prefix(ITEMS)
            .union(&ScopeSet::prefix("assets/minecraft/textures"))
            .union(&ScopeSet::prefix("assets/minecraft/models"));
        for root in ROOTS {
            reads = reads.union(&ScopeSet::prefix(root));
        }
        TaskDecl::new("generate_cit_item_definitions", Tier::Surgeon)
            .reads(reads)
            .writes(
                ScopeSet::prefix(ITEMS)
                    .union(&ScopeSet::prefix(GENERATED_MODELS))
                    .union(&ScopeSet::prefix("assets/minecraft/textures/cit")),
            )
            .exclusive(true)
    }

    /// 把一张贴图搬进标准目录并生成配套模型，返回模型引用。
    fn emit_model_for_texture(
        tx: &mut Tx<'_>,
        texture_path: &str,
        cit_stem: &str,
        raw_name: &str,
        state_suffix: &str,
    ) -> Result<String, AromError> {
        let stem = if state_suffix.is_empty() {
            cit_stem.to_string()
        } else {
            format!("{cit_stem}_{state_suffix}")
        };
        // **贴图必须搬进 `assets/minecraft/textures/` 才引用得到。**
        //
        // 这是本任务最容易踩的坑：模型里的 `textures.layer0` 是**贴图引用**，
        // 客户端只在 `assets/<namespace>/textures/<path>.png` 下找它，并且
        // **不做任何相对路径解析**（`..` 在资源路径里是非法的）。
        // 而 OptiFine 的 CIT 贴图惯例是放在 properties 文件旁边
        // （`optifine/cit/...` 之下）——那个位置客户端**根本不索引**。
        let texture_reference = match texture_path.strip_prefix("assets/minecraft/textures/") {
            // 已经在标准贴图目录里（作者写的相对路径指到了 `textures/`）→ 原地引用。
            Some(inside) => inside.trim_end_matches(".png").to_string(),
            None => {
                let target = format!("assets/minecraft/textures/cit/{stem}.png");
                if !tx.exists(&target) {
                    let bytes = tx.read(texture_path)?.unwrap_or_default();
                    tx.put(&target, bytes)?;
                }
                format!("cit/{stem}")
            }
        };
        let model_path = format!("{GENERATED_MODELS}/{stem}.json");
        let model = serde_json::json!({
            "parent": parent_for(raw_name),
            "textures": { "layer0": texture_reference },
        });
        let text = serde_json::to_string_pretty(&model)
            .map_err(|e| AromError::internal(format!("序列化模型 `{model_path}` 失败：{e}")))?;
        tx.put(&model_path, text.into_bytes())?;
        Ok(model_reference(&model_path))
    }

    /// 把一条规则解析成「叶子模型引用」（必要时写出模型/贴图文件）。
    ///
    /// `Ok(None)` 表示这条规则被跳过，原因已写进 `notes`。
    fn build_leaf(
        tx: &mut Tx<'_>,
        rule: &CitRule,
        cit_stem: &str,
        path: &str,
        notes: &mut Vec<String>,
    ) -> Result<Option<(serde_json::Value, String)>, AromError> {
        let directory = path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");

        // ---- ① `model=`：包内模型 → 复制并改写；否则直接引用 ----
        if let Some(raw) = rule.model.as_deref() {
            return Ok(match resolve_model(tx, directory, raw)? {
                Some(ModelTarget::Reference(reference)) => Some((leaf(&reference), reference)),
                Some(ModelTarget::InPack(source)) => {
                    let dest = format!("{GENERATED_MODELS}/{cit_stem}.json");
                    match copy_model(tx, &source, &dest, cit_stem, 3)? {
                        Some(reference) => Some((leaf(&reference), reference)),
                        None => {
                            notes.push(format!("跳过 {path}：`{source}` 不是合法模型 json"));
                            None
                        }
                    }
                }
                None => {
                    notes.push(format!(
                        "跳过 {path}：model=`{raw}` 在包里找不到对应的模型 json，也不像模型引用"
                    ));
                    None
                }
            });
        }

        // ---- ② 多状态贴图的**可行性检查放在最前面** ----
        //
        // 踩过的坑：先搬贴图/写模型再判断「这条能不能转」，跳过时就会在产物里留下
        // 孤儿 `models/cit/crossbow/q3.json` + `textures/cit/crossbow/q3.png`
        // （实测第一版就是这样，31 个模型里多出一个没人引用的弩模型）。
        // 跳过必须是**零副作用**的。
        if !rule.state_textures.is_empty() {
            let chain_ok = rule.items.len() == 1 && rule.items[0] == "bow";
            if !chain_ok {
                notes.push(format!(
                    "跳过 {path}：`texture.<状态>=` 多状态贴图只支持弓（实测 1.21.4 / 1.21.10 / 26.2 \
                     三个官方 jar 的 items/bow.json 逐字节同构）；`{}` 的状态链没有可对照的官方模板\
                     （弩的链路在 46 与 69 之间改过结构），不硬凑",
                    rule.items.join(",")
                ));
                return Ok(None);
            }
        }

        // ---- ③ `texture=` / 同名 PNG ----
        let generic = rule
            .texture
            .clone()
            .unwrap_or_else(|| format!("{cit_stem}.png"));
        let Some(generic_path) = resolve_texture(tx, directory, &generic)? else {
            notes.push(format!(
                "跳过 {path}：texture 指向的 `{generic}` 在包里找不到（试过 {}）",
                texture_candidates(directory, &generic).join(" / ")
            ));
            return Ok(None);
        };
        let generic_ref = emit_model_for_texture(tx, &generic_path, cit_stem, &generic, "")?;

        // ---- ④ 多状态贴图（目前只有弓） ----
        if rule.state_textures.is_empty() {
            return Ok(Some((leaf(&generic_ref), generic_ref)));
        }

        // **`texture.bow_standby` 是旧写法里的静态状态名**（OptiFine 文档原文：
        // 「the vanilla bow has four possible textures … bow_standby, bow_pulling_0…」），
        // 也就是本模板里的 `bow` 那一格。
        let mut by_state: BTreeMap<String, String> = BTreeMap::new();
        for state in BOW_STATES {
            let raw = if state == "bow" {
                rule.state_textures
                    .get("bow_standby")
                    .or_else(|| rule.texture.as_ref())
            } else {
                rule.state_textures.get(state)
            };
            match raw {
                Some(raw) if raw != &generic => match resolve_texture(tx, directory, raw)? {
                    Some(found) => {
                        let reference =
                            emit_model_for_texture(tx, &found, cit_stem, raw, state)?;
                        by_state.insert(state.to_string(), reference);
                    }
                    None => {
                        // OptiFine 文档：「If no texture.<name> property matches, the generic
                        // texture property is used instead.」找不到就回落到通用贴图。
                        notes.push(format!(
                            "CIT {path}：`texture.{state}={raw}` 的贴图在包里找不到，回落到通用贴图"
                        ));
                        by_state.insert(state.to_string(), generic_ref.clone());
                    }
                },
                _ => {
                    by_state.insert(state.to_string(), generic_ref.clone());
                }
            }
        }
        let chain = state_chain("bow", &|state| {
            leaf(by_state.get(state).map(String::as_str).unwrap_or(&generic_ref))
        })
        .expect("bow 一定有状态链");
        Ok(Some((chain, generic_ref)))
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let target = tx
            .view()
            .mcmeta()
            .ok()
            .and_then(|m| m.effective_format())
            .unwrap_or(FIRST_MODERN_FORMAT);
        if target < FIRST_MODERN_FORMAT {
            return Ok(Outcome::default());
        }
        let component_ok = target >= COMPONENT_CONDITION_FORMAT;

        // 收集**两个根目录**下的全部 `.properties`（**排序**保证产物确定）。
        //
        // 用绝对路径（`根/相对` 合起来）而不是裸相对名，因为两个根都可能存在，
        // 而它们的相对路径可能重名——那时两条规则都得各自被处理。
        let mut files: Vec<String> = Vec::new();
        for root in ROOTS {
            if !tx.has_prefix(root)? {
                continue;
            }
            files.extend(
                tx.list(root)?
                    .into_iter()
                    .filter(|res| !res.is_dir && res.path.ends_with(".properties"))
                    .map(|res| res.path),
            );
        }
        files.sort();
        files.dedup();
        if files.is_empty() {
            return Ok(Outcome::default());
        }

        let mut outcome = Outcome::default();
        // 先解析出全部规则，再按 OptiFine 的优先级（weight 降序、路径升序）排序。
        // 这样「同一物品的多条规则」能正确地由外到内嵌套，而不是后写的覆盖先写的。
        let mut rules: Vec<(String, CitRule)> = Vec::new();
        for path in &files {
            let text = tx.text(path)?;
            let properties = parse_properties(&text);
            let Some(rule) = cit_rule(&properties) else {
                outcome.skipped += 1;
                continue;
            };
            rules.push((path.clone(), rule));
        }
        rules.sort_by(|a, b| b.1.weight.cmp(&a.1.weight).then_with(|| a.0.cmp(&b.0)));

        let mut per_item: BTreeMap<String, ItemPlan> = BTreeMap::new();
        for (path, rule) in &rules {
            if !rule.unsupported.is_empty() {
                outcome.skipped += 1;
                outcome.notes.push(format!(
                    "跳过 {path}：条件 {} 在 1.21.4+ 的物品定义里没有等价表达",
                    rule.unsupported.join(" / ")
                ));
                continue;
            }
            // 附魔条件需要 `minecraft:component`（1.21.5+）。目标更低时**整条**跳过：
            // 只保留 CMD 那半会把「CMD=7 **且** 锋利 5」放宽成「CMD=7」，正是
            // 文件头反复强调不能做的那种放宽。
            if rule.needs_component_condition() && !component_ok {
                outcome.skipped += 1;
                outcome.notes.push(format!(
                    "跳过 {path}：附魔条件需要 `minecraft:component` 这个 condition property，\
                     它是 1.21.5（pack_format 55）才引入的，本次目标格式 {target} 表达不了"
                ));
                continue;
            }
            if rule.items.is_empty() {
                outcome.skipped += 1;
                continue;
            }

            // **CIT 内的相对路径**（去掉根前缀、去掉 `.properties`）——产物路径都跟着它走，
            // 这样"一个 properties 文件对应一个模型文件"，排查时一眼能对上。
            let cit_stem = ROOTS
                .iter()
                .find_map(|root| path.strip_prefix(&format!("{root}/")))
                .unwrap_or(path)
                .trim_end_matches(".properties")
                .to_string();

            let conds = rule.conditions();
            if conds.is_empty() && rule.custom_model_data.is_none() {
                // 既没有 CMD、也没有任何可表达条件 = 无条件换皮。原版没有这种写法：
                // 那本来就是这个物品的默认外观，应该在 `items/<name>.json` 里直接写。
                // 强行接成 CMD 会要求玩家先给物品打标签，**改变作者意图**，因此不转。
                outcome.skipped += 1;
                outcome.notes.push(format!(
                    "跳过 {path}：没有 nbt.CustomModelData，也没有可表达的条件，\
                     无条件的换皮在 1.21.4+ 没有等价写法"
                ));
                continue;
            }

            let Some((leaf_model, reference)) =
                build_leaf(tx, rule, &cit_stem, path, &mut outcome.notes)?
            else {
                outcome.skipped += 1;
                continue;
            };

            let describe = conds
                .iter()
                .map(|cond| match cond {
                    Cond::Enchant { ids, levels } => format!(
                        "附魔 {} {}",
                        ids.join("|"),
                        levels
                            .map(LevelRange::describe)
                            .unwrap_or_else(|| "任意等级".to_string())
                    ),
                    Cond::Hand(hand) => format!("手 {}", hand.describe()),
                })
                .collect::<Vec<_>>()
                .join(" + ");

            for item in &rule.items {
                let plan = per_item.entry(item.clone()).or_default();
                if let Some(threshold) = rule.custom_model_data {
                    plan.cmds.push(CmdEntry {
                        threshold,
                        model: leaf_model.clone(),
                        reference: reference.clone(),
                        source: path.clone(),
                    });
                }
                if !conds.is_empty() {
                    // 附魔指纹：只有附魔条件参与去重（`hand` 那层是 select，重复风险低）。
                    let fingerprint = conds.iter().find_map(|cond| match cond {
                        Cond::Enchant { ids, levels } => {
                            Some(enchant_fingerprint(ids, *levels, &leaf_model))
                        }
                        Cond::Hand(_) => None,
                    });
                    plan.nodes.push(NodeEntry {
                        conds: conds.clone(),
                        leaf: leaf_model.clone(),
                        reference: reference.clone(),
                        describe: describe.clone(),
                        source: path.clone(),
                        fingerprint,
                    });
                }
            }
        }

        // 每个物品读一次、写一次。
        let mut converted = 0usize;
        for (item, mut plan) in per_item {
            let path = format!("{ITEMS}/{item}.json");
            let existing = match tx.read(&path)? {
                Some(bytes) => match serde_json::from_slice::<serde_json::Value>(&bytes) {
                    Ok(value) => Some(value),
                    Err(_) => {
                        // 已有定义不是合法 JSON —— 不猜、不覆盖。
                        outcome.skipped += 1;
                        outcome.notes.push(format!(
                            "跳过 items/{item}.json：既有定义不是合法 JSON，拒绝改写"
                        ));
                        continue;
                    }
                },
                None => None,
            };
            let Some((definition, applied, notes)) =
                merge_into_definition(&item, existing, &mut plan)
            else {
                continue;
            };
            let text = serde_json::to_string_pretty(&definition)
                .map_err(|e| AromError::internal(format!("序列化 `{path}` 失败：{e}")))?;
            tx.put(&path, text.into_bytes())?;
            converted += applied;
            outcome.notes.extend(notes);
        }

        outcome.changed = converted;
        Ok(outcome)
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "generate_cit_item_definitions" => Some((decl(), run)),
            _ => None,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::arom::source::MemSource;

        fn pack(format: u32, extra: Vec<(&str, &str)>) -> crate::arom::Pack {
            let mut files: Vec<(String, Vec<u8>)> = vec![(
                "pack.mcmeta".to_string(),
                format!(r#"{{"pack":{{"pack_format":{format}}}}}"#).into_bytes(),
            )];
            for (path, body) in extra {
                files.push((path.to_string(), body.as_bytes().to_vec()));
            }
            let src = MemSource::new(files).expect("mem source");
            crate::arom::Pack::from_source(Box::new(src), None).expect("pack")
        }

        fn run_on(pack: &mut crate::arom::Pack) -> Outcome {
            let mut tx = pack.tx("cit-test");
            let outcome = run(&mut tx).expect("run");
            let layer = tx.into_layer();
            pack.commit(layer);
            outcome
        }

        const CIT_DIR: &str = "assets/minecraft/optifine/cit/ruby";

        /// `.properties` 解析：注释跳过、键转小写、**首次出现者胜**。
        #[test]
        fn properties_parser_is_first_wins_and_skips_comments() {
            let map = parse_properties(
                "# 注释\n\
                 // 另一种注释\n\
                 type=item\n\
                 Type=enchantment\n\
                 items = diamond_sword , minecraft:stick\n\
                 \n\
                 没有等号的行\n\
                 texture=ruby_sword.png\n",
            );
            assert_eq!(map.get("type").map(String::as_str), Some("item"), "首个 type 胜出");
            assert_eq!(
                parse_items(map.get("items").expect("items")),
                vec!["diamond_sword".to_string(), "stick".to_string()],
                "去命名空间、去空白、排序"
            );
            assert_eq!(map.get("texture").map(String::as_str), Some("ruby_sword.png"));
            assert!(!map.contains_key("没有等号的行"));
        }

        /// **空格分隔也是合法列表分隔符**（OptiFine 文档里的示例就是空格分隔）。
        ///
        /// 踩过的坑：旧实现只按 `,` 切，`matchItems=diamond_sword iron_sword`
        /// 会变成一个名叫 `diamond_sword iron_sword` 的假物品。
        #[test]
        fn space_separated_lists_are_split_not_glued() {
            assert_eq!(
                parse_items("minecraft:diamond_sword iron_sword\tstone_sword"),
                vec![
                    "diamond_sword".to_string(),
                    "iron_sword".to_string(),
                    "stone_sword".to_string()
                ]
            );
            assert_eq!(
                parse_enchantment_ids("flame minecraft:fire_aspect").0,
                vec!["minecraft:fire_aspect".to_string(), "minecraft:flame".to_string()],
                "短名补 minecraft:，长名原样，且排序"
            );
        }

        /// `type!=item` 一律不碰（附魔/盔甲/鞘翅是别的机制）。
        #[test]
        fn non_item_cit_is_not_a_rule() {
            let map = parse_properties("type=enchantment\nitems=diamond_sword\ntexture=x.png\n");
            assert!(cit_rule(&map).is_none());
        }

        /// 无法表达的条件必须被**列出来**（而不是静默丢掉）。
        ///
        /// 注意 `enchantments=` 已经**不在**这张名单里了——1.21.5 起它能精确表达，
        /// 见 [`enchantment_level_becomes_an_exact_range`]。
        #[test]
        fn unsupported_conditions_are_reported() {
            let map = parse_properties(
                "type=item\nitems=diamond_sword\ntexture=x.png\n\
                 nbt.CustomModelData=7\nnbt.display.Name=hi\ndamage=50%\ncomponents.minecraft\\:custom_name=x\n",
            );
            let rule = cit_rule(&map).expect("rule");
            assert_eq!(rule.custom_model_data, Some(7), "CMD 是支持的条件");
            let joined = rule.unsupported.join(" ");
            for expected in [
                "nbt.display.name",
                "damage=",
                "components.minecraft\\:custom_name",
            ] {
                assert!(joined.contains(expected), "漏报 {expected}：{joined}");
            }
            assert!(!joined.contains("nbt.custommodeldata"), "CMD 不该被当成不支持");
        }

        /// 手持类父模型按**文件名后缀**判定（CIT 惯例看的是贴图名，不是物品名）。
        #[test]
        fn handheld_parent_is_chosen_by_texture_suffix() {
            assert_eq!(parent_for("ruby_sword.png"), "minecraft:item/handheld");
            assert_eq!(parent_for("textures/ruby_pickaxe.png"), "minecraft:item/handheld");
            assert_eq!(parent_for("ruby_bow.png"), "minecraft:item/handheld");
            assert_eq!(parent_for("ruby_gem.png"), "minecraft:item/generated");
            assert_eq!(parent_for("ruby_sword"), "minecraft:item/handheld", "无扩展名也要认");
        }

        /// **端到端**：`texture=` 形式 → 生成 `models/cit/...` + 物品定义里的 `select` 分支。
        #[test]
        fn texture_rule_generates_a_model_and_a_select_case() {
            let mut p = pack(
                55,
                vec![
                    (
                        &format!("{CIT_DIR}/ruby_sword.properties"),
                        "type=item\nitems=diamond_sword\ntexture=ruby_sword.png\nnbt.CustomModelData=7\n",
                    ),
                    (&format!("{CIT_DIR}/ruby_sword.png"), "png-bytes"),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 1, "应当转出一条：{:?}", outcome.notes);

            // 生成的模型：父模型按 `_sword` 后缀选手持类；layer0 引用**被搬进标准贴图目录**的那份。
            let model = p
                .view()
                .json("assets/minecraft/models/cit/ruby/ruby_sword.json")
                .expect("生成的模型应当存在");
            assert_eq!(model["parent"], "minecraft:item/handheld");
            assert_eq!(model["textures"]["layer0"], "cit/ruby/ruby_sword");
            // 贴图被复制到 `textures/cit/...`——直接引用 `optifine/cit/...` 客户端是加载不到的
            assert!(
                p.view()
                    .resolve("assets/minecraft/textures/cit/ruby/ruby_sword.png")
                    .is_some(),
                "贴图必须被搬进 assets/minecraft/textures/ 才引用得到"
            );

            // 物品定义：select(custom_model_data) + when=7
            let def = p
                .view()
                .json("assets/minecraft/items/diamond_sword.json")
                .expect("物品定义应当存在");
            let root = &def["model"];
            assert_eq!(root["type"], "minecraft:select");
            assert_eq!(root["property"], CMD_COMPONENT);
            assert_eq!(root["index"], 0);
            assert_eq!(root["cases"][0]["when"], 7);
            assert_eq!(
                root["cases"][0]["model"]["model"],
                "minecraft:cit/ruby/ruby_sword"
            );
            // 没有默认定义可包时，fallback 是"自指默认定义"
            assert_eq!(root["fallback"]["model"], "minecraft:item/diamond_sword");
        }

        /// `texture=` **不带扩展名**也要找到图（OptiFine 文档与验证包都是这么写的）。
        #[test]
        fn texture_without_extension_is_found() {
            let mut p = pack(
                55,
                vec![
                    (
                        &format!("{CIT_DIR}/ruby.properties"),
                        "type=item\nitems=stick\ntexture=ruby\nnbt.CustomModelData=1\n",
                    ),
                    (&format!("{CIT_DIR}/ruby.png"), "png"),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 1, "{:?}", outcome.notes);
        }

        /// `model=` 形式：**不生成**模型文件，分支直接指向作者给的引用。
        #[test]
        fn model_rule_reuses_the_authors_model() {
            let mut p = pack(
                55,
                vec![(
                    &format!("{CIT_DIR}/given.properties"),
                    "type=item\nitems=stick\nmodel=mypack:item/ruby_stick\nnbt.CustomModelData=3\n",
                )],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 1, "{:?}", outcome.notes);
            assert!(
                p.view()
                    .resolve("assets/minecraft/models/cit/ruby/given.json")
                    .is_none(),
                "给了 model= 就不该再生成模型文件"
            );
            let def = p
                .view()
                .json("assets/minecraft/items/stick.json")
                .expect("定义");
            assert_eq!(def["model"]["cases"][0]["model"]["model"], "mypack:item/ruby_stick");
        }

        /// **多条 CIT 命同一个物品**：并进同一个 `select`，不互相覆盖。
        #[test]
        fn multiple_rules_merge_into_one_select() {
            let mut p = pack(
                55,
                vec![
                    (
                        &format!("{CIT_DIR}/a.properties"),
                        "type=item\nitems=stick\nmodel=mypack:item/a\nnbt.CustomModelData=1\n",
                    ),
                    (
                        &format!("{CIT_DIR}/b.properties"),
                        "type=item\nitems=stick\nmodel=mypack:item/b\nnbt.CustomModelData=2\n",
                    ),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 2, "{:?}", outcome.notes);
            let def = p.view().json("assets/minecraft/items/stick.json").expect("定义");
            let cases = def["model"]["cases"].as_array().expect("cases");
            assert_eq!(cases.len(), 2, "两条规则必须并进同一个 select：{def}");
            assert_eq!(cases[0]["when"], 1);
            assert_eq!(cases[1]["when"], 2);
        }

        /// `texture=` 指向的 PNG 不存在 → 跳过整条规则（**不写半成品**）。
        #[test]
        fn missing_texture_skips_the_rule() {
            let mut p = pack(
                55,
                vec![(
                    &format!("{CIT_DIR}/ghost.properties"),
                    "type=item\nitems=stick\ntexture=nowhere.png\nnbt.CustomModelData=1\n",
                )],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 0);
            assert_eq!(outcome.skipped, 1);
            assert!(p.view().resolve("assets/minecraft/items/stick.json").is_none());
        }

        /// 没有 `nbt.CustomModelData`、也没有其它条件的 CIT（无条件换皮）**必须跳过**：
        /// 强行接成 CMD 会要求玩家先打标签，**改变作者意图**。
        #[test]
        fn unconditional_cit_is_skipped_not_guessed() {
            let mut p = pack(
                55,
                vec![
                    (
                        &format!("{CIT_DIR}/always.properties"),
                        "type=item\nitems=stick\ntexture=always.png\n",
                    ),
                    (&format!("{CIT_DIR}/always.png"), "png"),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 0, "无条件换皮没有等价写法");
            assert_eq!(outcome.skipped, 1);
            assert!(
                p.view().resolve("assets/minecraft/items/stick.json").is_none(),
                "跳过就不能留下半成品"
            );
        }

        /// 目标 < 46：整任务空转（`optifine/cit` 里的东西在旧客户端上由 OptiFine 自己读，不用动）。
        #[test]
        fn does_nothing_below_the_modern_boundary() {
            let mut p = pack(
                42,
                vec![(
                    &format!("{CIT_DIR}/x.properties"),
                    "type=item\nitems=stick\nmodel=mypack:item/x\nnbt.CustomModelData=1\n",
                )],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 0);
            assert_eq!(outcome.skipped, 0, "低于边界应当完全空转");
            assert!(p.view().resolve("assets/minecraft/items/stick.json").is_none());
        }

        /// 声明的范围必须覆盖真正写入的两处目录，以及 `model=` 解析要读的 `models/`。
        #[test]
        fn declared_scope_covers_both_output_directories() {
            let decl = decl();
            assert_eq!(decl.tier, Tier::Surgeon);
            assert!(decl.exclusive, "跨目录读写 + 读他人刚写过的 items/ ⇒ 必须独占");
            assert!(decl.writes.contains("assets/minecraft/items/stick.json"));
            assert!(decl.writes.contains("assets/minecraft/models/cit/ruby/x.json"));
            assert!(decl.reads.contains("assets/minecraft/optifine/cit/ruby/x.properties"));
            assert!(
                decl.reads.contains("assets/minecraft/models/item/x.json"),
                "`model=item/xxx` 的解析要读 models/，读范围里必须有它"
            );
        }

        // -----------------------------------------------------------------
        // 附魔条件（本任务的核心扩展）
        // -----------------------------------------------------------------

        /// **附魔 + 等级 → 精确的 `condition(component)`**，这是整个改造的核心断言。
        ///
        /// 形状逐字对齐真实样本（`[1.21.10]锋利显示+金剑耐久(1).zip`）：
        /// `value` 是数组，每项 `{enchantments, levels}`。
        #[test]
        fn enchantment_level_becomes_an_exact_range() {
            let mut p = pack(
                55,
                vec![
                    (
                        &format!("{CIT_DIR}/sharp3.properties"),
                        "type=item\nmatchItems=diamond_sword\nenchantments=sharpness\nenchantmentLevels=3\n\
                         texture=sharp3.png\n",
                    ),
                    (&format!("{CIT_DIR}/sharp3.png"), "png"),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 1, "{:?}", outcome.notes);
            let def = p
                .view()
                .json("assets/minecraft/items/diamond_sword.json")
                .expect("定义");
            let root = &def["model"];
            assert_eq!(root["type"], "minecraft:condition");
            assert_eq!(root["property"], "minecraft:component");
            assert_eq!(root["predicate"], "minecraft:enchantments");
            assert_eq!(root["value"][0]["enchantments"], "minecraft:sharpness");
            assert_eq!(root["value"][0]["levels"]["min"], 3);
            assert_eq!(root["value"][0]["levels"]["max"], 3, "必须锁死等级，不能只写 min");
            assert_eq!(root["on_true"]["type"], "minecraft:model");
            assert_eq!(root["on_true"]["model"], "minecraft:cit/ruby/sharp3");
            // 不命中 → 原版默认外观
            assert_eq!(root["on_false"]["model"], "minecraft:item/diamond_sword");
        }

        /// `enchantmentLevels=255` → `{"min":255,"max":255}`。
        ///
        /// 255 是「命令刷出来的 255 级附魔」这种真实玩法（验证包 `255axe/mrf` 就是
        /// 锋利 255 的斧头），所以它**不是**通配符，必须精确。只写 `{min:255}` 会把
        /// 300 级的物品也算进来。
        #[test]
        fn level_255_is_written_as_min_and_max() {
            assert_eq!(
                parse_levels("255").expect("ok"),
                Some(LevelRange {
                    min: Some(255),
                    max: Some(255)
                })
            );
            let json = parse_levels("255").expect("ok").expect("some").to_json();
            assert_eq!(json["min"], 255);
            assert_eq!(json["max"], 255);
        }

        /// 没写 `enchantmentLevels=` 就**整个省略 `levels`**（=「有任何等级都算」）。
        ///
        /// 这也是真实样本的写法：`[1.21.10]弓箭等级覆盖(1).zip` 里
        /// `{"enchantments":"minecraft:flame"}` 就不带 `levels`，只有 power 那条带 `{min:N}`。
        #[test]
        fn enchantment_without_levels_omits_the_levels_field() {
            let node = wrap_condition(
                &Cond::Enchant {
                    ids: vec!["minecraft:flame".to_string()],
                    levels: None,
                },
                &leaf("minecraft:cit/a"),
                &leaf("minecraft:item/bow"),
            );
            assert_eq!(node["value"][0]["enchantments"], "minecraft:flame");
            assert!(
                node["value"][0].get("levels").is_none(),
                "没写等级就不该出现 levels：{node}"
            );
        }

        /// `enchantmentLevels=3-` → 只有下界。
        #[test]
        fn open_level_range_uses_min_only() {
            let json = parse_levels("3-").expect("ok").expect("some").to_json();
            assert_eq!(json["min"], 3);
            assert!(json.get("max").is_none(), "开区间不该有 max：{json}");
        }

        /// 多个附魔名 → **一项里的数组**。
        ///
        /// 依据（wiki 原文）：`enchantments` 给数组时「succeeds when any one of them is
        /// present」，正好等于 OptiFine 的「任一命中」；而数组里的**各项之间是 AND**，
        /// 所以多个名字绝不能拆成多项。
        #[test]
        fn multiple_enchantment_ids_share_one_predicate() {
            let node = wrap_condition(
                &Cond::Enchant {
                    ids: vec![
                        "minecraft:fire_aspect".to_string(),
                        "minecraft:flame".to_string(),
                    ],
                    levels: Some(LevelRange {
                        min: Some(3),
                        max: None,
                    }),
                },
                &leaf("minecraft:cit/a"),
                &leaf("minecraft:item/bow"),
            );
            let value = node["value"].as_array().expect("数组");
            assert_eq!(value.len(), 1, "一个 predicate 项就够，不能拆成多项（那是 AND）：{node}");
            assert_eq!(
                value[0]["enchantments"],
                serde_json::json!(["minecraft:fire_aspect", "minecraft:flame"])
            );
            assert_eq!(value[0]["levels"]["min"], 3);
        }

        /// 断开的等级列表（`1 3 5`）**不可表达** → 跳过，绝不并成 `1-5`。
        #[test]
        fn broken_level_list_is_skipped() {
            assert!(parse_levels("1 3 5").is_err(), "并集不连续，不能假装能表达");
            assert_eq!(
                parse_levels("2-4 5-8").expect("连续的列表可以合并"),
                Some(LevelRange {
                    min: Some(2),
                    max: Some(8)
                })
            );
            let mut p = pack(
                55,
                vec![
                    (
                        &format!("{CIT_DIR}/odd.properties"),
                        "type=item\nitems=stick\nenchantmentIDs=sharpness\nenchantmentLevels=1 3 5\n\
                         texture=odd.png\n",
                    ),
                    (&format!("{CIT_DIR}/odd.png"), "png"),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 0);
            assert_eq!(outcome.skipped, 1);
            assert!(p.view().resolve("assets/minecraft/items/stick.json").is_none());
        }

        /// **只有 `enchantmentLevels`、没有附魔名** → 跳过。
        ///
        /// OptiFine 的语义是「所有附魔等级**之和**」（官方文档原文：
        /// `Match any enchantment, with 8-10 total levels across all enchantments`），
        /// 而目标侧不写 `enchantments` 时是「任意**单条**附魔的等级」。
        /// 锋利 5 + 耐久 3 的和是 8，但没有任何一条是 8..10 —— 两者不等价，只能跳过。
        #[test]
        fn total_levels_without_ids_is_skipped() {
            let mut p = pack(
                55,
                vec![
                    (
                        &format!("{CIT_DIR}/axe8.properties"),
                        "type=item\nmatchItems=diamond_axe\nenchantmentLevels=8\nmodel=axe8\n",
                    ),
                    (&format!("{CIT_DIR}/axe8.json"), r#"{"parent":"item/handheld"}"#),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 0, "总等级和没有等价表达");
            assert_eq!(outcome.skipped, 1);
            let joined = outcome.notes.join(" ");
            assert!(joined.contains("之和"), "note 必须说清是『等级之和』：{joined}");
        }

        /// 数字附魔 id（`enchantmentIDs=33`）→ 跳过（1.13 起附魔只有注册名）。
        #[test]
        fn numeric_enchantment_id_is_skipped() {
            let mut p = pack(
                55,
                vec![
                    (
                        &format!("{CIT_DIR}/legacy.properties"),
                        "type=item\nitems=stick\nenchantmentIDs=33\ntexture=s.png\n",
                    ),
                    (&format!("{CIT_DIR}/s.png"), "png"),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 0);
            assert!(outcome.notes.join(" ").contains("数字 id"));
        }

        /// **目标 46–54：附魔规则整条跳过，并写明需要 1.21.5+。**
        ///
        /// 这一条是「不破坏既有行为」的关键：46–54 只能转 CMD / 显示上下文，
        /// 而附魔条件要等 `minecraft:component`（55）。
        #[test]
        fn enchantment_rules_are_skipped_below_format_55() {
            let mut p = pack(
                46,
                vec![
                    (
                        &format!("{CIT_DIR}/sharp.properties"),
                        "type=item\nitems=diamond_sword\nenchantments=sharpness\nenchantmentLevels=5\n\
                         texture=s.png\n",
                    ),
                    (&format!("{CIT_DIR}/s.png"), "png"),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 0, "46 没有 component 条件");
            assert_eq!(outcome.skipped, 1);
            let joined = outcome.notes.join(" ");
            assert!(
                joined.contains("55") && joined.contains("1.21.5"),
                "note 必须点明需要 1.21.5+ / pack_format 55：{joined}"
            );
            assert!(p.view().resolve("assets/minecraft/items/diamond_sword.json").is_none());

            // **CMD + 附魔同一条规则**：46 也不能只转 CMD 那一半。
            // 只留 CMD 会把「CMD=7 且锋利 5」放宽成「CMD=7」，正是不能做的那种放宽。
            let mut both = pack(
                46,
                vec![
                    (
                        &format!("{CIT_DIR}/mixed.properties"),
                        "type=item\nitems=stick\nenchantments=sharpness\nenchantmentLevels=5\n\
                         nbt.CustomModelData=7\ntexture=m.png\n",
                    ),
                    (&format!("{CIT_DIR}/m.png"), "png"),
                ],
            );
            let outcome = run_on(&mut both);
            assert_eq!(outcome.changed, 0, "不能只转 CMD 那一半");
            assert_eq!(outcome.skipped, 1);
            assert!(
                both.view().resolve("assets/minecraft/items/stick.json").is_none(),
                "整条跳过才不留半成品"
            );
        }

        /// ……而 46–54 的 **CMD 规则照旧能转**（既有行为不变）。
        #[test]
        fn cmd_rules_still_convert_below_format_55() {
            let mut p = pack(
                46,
                vec![
                    (
                        &format!("{CIT_DIR}/cmd.properties"),
                        "type=item\nitems=stick\ntexture=t.png\nnbt.CustomModelData=4\n",
                    ),
                    (&format!("{CIT_DIR}/t.png"), "png"),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 1, "{:?}", outcome.notes);
            let def = p.view().json("assets/minecraft/items/stick.json").expect("定义");
            assert_eq!(def["model"]["property"], CMD_COMPONENT);
        }

        /// 同一物品多条附魔规则：**weight 大的在外层**（OptiFine 文档规定的优先级）。
        #[test]
        fn higher_weight_is_nested_outside() {
            let mut p = pack(
                55,
                vec![
                    (
                        &format!("{CIT_DIR}/a_low.properties"),
                        "type=item\nitems=diamond_sword\nenchantments=sharpness\nenchantmentLevels=1\n\
                         texture=low.png\n",
                    ),
                    (&format!("{CIT_DIR}/low.png"), "png"),
                    (
                        &format!("{CIT_DIR}/z_high.properties"),
                        "type=item\nitems=diamond_sword\nenchantments=sharpness\nenchantmentLevels=5\n\
                         texture=high.png\nweight=10\n",
                    ),
                    (&format!("{CIT_DIR}/high.png"), "png"),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 2, "{:?}", outcome.notes);
            let def = p
                .view()
                .json("assets/minecraft/items/diamond_sword.json")
                .expect("定义");
            let root = &def["model"];
            assert_eq!(root["value"][0]["levels"]["min"], 5, "weight=10 的那条必须在最外层");
            assert_eq!(root["on_true"]["model"], "minecraft:cit/ruby/z_high");
            let inner = &root["on_false"];
            assert_eq!(inner["value"][0]["levels"]["min"], 1, "低优先级的排在 on_false 链上");
            assert_eq!(inner["on_false"]["model"], "minecraft:item/diamond_sword");
        }

        /// 跑第二遍**不重复嵌套**（幂等）。
        #[test]
        fn second_run_does_not_nest_the_same_condition_again() {
            let mut p = pack(
                55,
                vec![
                    (
                        &format!("{CIT_DIR}/sharp3.properties"),
                        "type=item\nitems=diamond_sword\nenchantments=sharpness\nenchantmentLevels=3\n\
                         texture=sharp3.png\n",
                    ),
                    (&format!("{CIT_DIR}/sharp3.png"), "png"),
                ],
            );
            let first = run_on(&mut p);
            assert_eq!(first.changed, 1, "{:?}", first.notes);
            let second = run_on(&mut p);
            assert_eq!(second.changed, 0, "第二遍不该再改：{:?}", second.notes);
            let def = p
                .view()
                .json("assets/minecraft/items/diamond_sword.json")
                .expect("定义");
            // 只有一层 condition：外层直接就是那个附魔条件，叶子还是同一个模型
            assert_eq!(def["model"]["value"][0]["levels"]["max"], 3);
            assert_eq!(def["model"]["on_true"]["model"], "minecraft:cit/ruby/sharp3");
        }

        // -----------------------------------------------------------------
        // `model=` 的解析（验证包里 6 条规则靠它）
        // -----------------------------------------------------------------

        /// `model=s2`：**同目录**的 `s2.json` 被复制到 `models/cit/`，`"./s2"` 改写成搬好的贴图。
        ///
        /// 这是验证包 `sharpdiasword/*` / `sharp8axe/*` 的真实形状：
        /// `model=` 是裸名、json 就在 properties 旁边、json 里 `layer0` 写 `./s2`。
        /// 直接复制过去必然紫黑格（`./` 在资源路径里没有意义），所以必须改写。
        #[test]
        fn model_equals_sibling_json_is_copied_and_textures_rewritten() {
            let mut p = pack(
                55,
                vec![
                    (
                        &format!("{CIT_DIR}/s2.properties"),
                        "type=item\nmatchItems=diamond_sword\nmodel=s2\nenchantments=sharpness\n\
                         enchantmentLevels=2\n",
                    ),
                    (
                        &format!("{CIT_DIR}/s2.json"),
                        r#"{"parent":"item/handheld","textures":{"layer0":"./s2"},"overrides":[]}"#,
                    ),
                    (&format!("{CIT_DIR}/s2.png"), "png-bytes"),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 1, "{:?}", outcome.notes);

            let model = p
                .view()
                .json("assets/minecraft/models/cit/ruby/s2.json")
                .expect("模型必须被复制进 models/cit/");
            assert_eq!(
                model["parent"], "item/handheld",
                "parent 原样保留（没有命名空间就是 minecraft:，不用画蛇添足）"
            );
            assert_eq!(
                model["textures"]["layer0"], "cit/ruby/s2/s2",
                "`./s2` 必须改写成搬好的贴图引用（模型自带贴图按 `cit/<相对路径>/<名字>` 摆放，\
                 一个模型可能引用好几张图，用目录隔开才不会撞名）"
            );
            assert!(
                model.get("overrides").is_none(),
                "`overrides` 是 1.20 的字段，1.21.4+ 不该留在模型里"
            );
            assert!(
                p.view()
                    .resolve("assets/minecraft/textures/cit/ruby/s2/s2.png")
                    .is_some(),
                "模型引用的贴图必须一起搬过去"
            );
            let def = p
                .view()
                .json("assets/minecraft/items/diamond_sword.json")
                .expect("定义");
            assert_eq!(def["model"]["on_true"]["model"], "minecraft:cit/ruby/s2");
        }

        /// `model=` 指向包内不存在的裸名 → 跳过（**不写指向紫黑格的定义**）。
        #[test]
        fn dangling_relative_model_is_skipped() {
            let mut p = pack(
                55,
                vec![(
                    &format!("{CIT_DIR}/ghost.properties"),
                    "type=item\nitems=stick\nmodel=nope\nenchantments=sharpness\n",
                )],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 0);
            assert_eq!(outcome.skipped, 1);
            assert!(outcome.notes.join(" ").contains("model=`nope`"));
            assert!(p.view().resolve("assets/minecraft/items/stick.json").is_none());
        }

        // -----------------------------------------------------------------
        // 手 + 多状态（弓）
        // -----------------------------------------------------------------

        /// `hand=off` → `select(display_context)`，`when` 用**数组**列左手上下文。
        #[test]
        fn hand_off_uses_display_context_cases() {
            let node = wrap_condition(
                &Cond::Hand(Hand::Off),
                &leaf("minecraft:cit/a"),
                &leaf("minecraft:item/stick"),
            );
            assert_eq!(node["type"], "minecraft:select");
            assert_eq!(node["property"], "minecraft:display_context");
            assert_eq!(
                node["cases"][0]["when"],
                serde_json::json!(["firstperson_lefthand", "thirdperson_lefthand"])
            );
            assert_eq!(node["fallback"]["model"], "minecraft:item/stick");
        }

        /// `hand=main` 必须带上 `gui`。
        ///
        /// 依据是 OptiFine 文档原文：「When rendered in the GUI the item is considered
        /// to be in the main hand」——漏掉 `gui` 会让物品栏里的图标变回原版。
        #[test]
        fn hand_main_includes_gui() {
            let node = wrap_condition(
                &Cond::Hand(Hand::Main),
                &leaf("minecraft:cit/a"),
                &leaf("minecraft:item/stick"),
            );
            let when = node["cases"][0]["when"].as_array().expect("数组");
            assert!(when.iter().any(|v| v == "gui"), "main 手必须包含 gui：{node}");
        }

        /// **弓的多状态贴图**：按官方 `items/bow.json` 的形状重建链条。
        ///
        /// 模板出处是本机 1.21.4 / 1.21.10 / 26.2 三个 jar（逐字节同构），
        /// 阈值 0.65 / 0.9、`scale: 0.05`、`property: minecraft:use_duration`。
        #[test]
        fn bow_state_textures_rebuild_the_vanilla_chain() {
            let mut p = pack(
                55,
                vec![
                    (
                        "assets/minecraft/optifine/cit/bow0/0.properties",
                        "type=item\nmatchItems=bow\ntexture=bow\ntexture.bow_pulling_0=1\n\
                         texture.bow_pulling_1=2\ntexture.bow_pulling_2=3\n\
                         enchantmentIDs=minecraft:punch\nenchantmentLevels=3\n",
                    ),
                    ("assets/minecraft/optifine/cit/bow0/bow.png", "png"),
                    ("assets/minecraft/optifine/cit/bow0/1.png", "png1"),
                    ("assets/minecraft/optifine/cit/bow0/2.png", "png2"),
                    ("assets/minecraft/optifine/cit/bow0/3.png", "png3"),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 1, "{:?}", outcome.notes);

            let def = p.view().json("assets/minecraft/items/bow.json").expect("定义");
            let root = &def["model"];
            assert_eq!(root["property"], "minecraft:component", "外层是附魔条件");
            assert_eq!(root["value"][0]["enchantments"], "minecraft:punch");

            // 命中：换成 CIT 的弓状态链
            let hit = &root["on_true"];
            assert_eq!(hit["property"], "minecraft:using_item");
            assert_eq!(hit["on_false"]["model"], "minecraft:cit/bow0/0", "静态状态用通用贴图");
            let pull = &hit["on_true"];
            assert_eq!(pull["property"], "minecraft:use_duration");
            assert_eq!(pull["scale"], 0.05);
            assert_eq!(pull["entries"][0]["threshold"], 0.65);
            assert_eq!(pull["entries"][0]["model"]["model"], "minecraft:cit/bow0/0_bow_pulling_1");
            assert_eq!(pull["entries"][1]["threshold"], 0.9);
            assert_eq!(pull["entries"][1]["model"]["model"], "minecraft:cit/bow0/0_bow_pulling_2");
            assert_eq!(pull["fallback"]["model"], "minecraft:cit/bow0/0_bow_pulling_0");

            // 不命中：**原版那条链**（否则未附魔的弓会失去拉弦动画）
            let miss = &root["on_false"];
            assert_eq!(miss["property"], "minecraft:using_item");
            assert_eq!(miss["on_false"]["model"], "minecraft:item/bow");
            assert_eq!(miss["on_true"]["entries"][0]["model"]["model"], "minecraft:item/bow_pulling_1");

            // 四个状态各生成了一个模型 + 一张贴图
            for suffix in ["", "_bow_pulling_0", "_bow_pulling_1", "_bow_pulling_2"] {
                assert!(
                    p.view()
                        .resolve(&format!("assets/minecraft/models/cit/bow0/0{suffix}.json"))
                        .is_some(),
                    "缺模型 0{suffix}.json"
                );
            }
        }

        /// 弩的多状态贴图 → 跳过（链路在 46 与 69 之间改过结构，不硬凑）。
        #[test]
        fn crossbow_state_textures_are_skipped() {
            let mut p = pack(
                55,
                vec![
                    (
                        "assets/minecraft/optifine/cit/crossbow/q3.properties",
                        "type=item\nmatchItems=crossbow\ntexture=q3\ntexture.crossbow_pulling_0=q31\n\
                         enchantmentIDs=quick_charge\nenchantmentLevels=3\n",
                    ),
                    ("assets/minecraft/optifine/cit/crossbow/q3.png", "png"),
                    ("assets/minecraft/optifine/cit/crossbow/q31.png", "png"),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 0);
            assert_eq!(outcome.skipped, 1);
            let joined = outcome.notes.join(" ");
            assert!(joined.contains("弩"), "note 必须点明是弩的链路问题：{joined}");
            assert!(p.view().resolve("assets/minecraft/items/crossbow.json").is_none());
            // **跳过必须零副作用**：第一版这里留过孤儿模型 + 孤儿贴图。
            assert!(
                p.view()
                    .resolve("assets/minecraft/models/cit/crossbow/q3.json")
                    .is_none(),
                "跳过的规则不该留下没人引用的模型"
            );
            assert!(
                p.view()
                    .resolve("assets/minecraft/textures/cit/crossbow/q3.png")
                    .is_none(),
                "跳过的规则不该留下没人引用的贴图"
            );
        }
    }
