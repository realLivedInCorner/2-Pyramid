    //! `generate_cit_item_definitions`：把包里的 **OptiFine CIT**（`optifine/cit/**/*.properties`）
    //! 转成 1.21.4+ 的**物品模型定义**。
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
    //! 要点（**决定了下面的实现为什么这样写**）：
    //!
    //! * `type=item` 是 CIT；`type=enchantment` / `type=armor` / `type=elytra` 是别的机制，本任务不碰。
    //! * `items=`（旧名 `matchItems=`）是**逗号分隔的物品列表**，可带命名空间，也可以**省略**——
    //!   省略时 OptiFine 按**同类目全物品**处理（例如 `items=` 留空 + `enchantments=` 表示"所有附魔物品"）。
    //!   本任务对"省 list"的情形**只处理能确定唯一物品名的写法**，其余如实计入跳过。
    //! * `texture=` 是**相对本 properties 文件的 PNG 路径**；`model=` 则直接给一个模型引用。
    //! * 文件名即身份：`foo.properties` 在没写 `texture=` 时会去找同目录的 `foo.png`。
    //! * `nbt.<标签>=<值>` 是任意 NBT 条件。**`nbt.CustomModelData=N` 是唯一能被原版表达的**
    //!   （它就是 `minecraft:custom_model_data` 组件的 `main` 字段），本任务把它翻成
    //!   `select` 的一个 `when: N` 分支。**其它 `nbt.*` 原版表达不了**——如实跳过，不假装转换。
    //! * `damage=` / `stackSize=` / `enchantments=` / `biome=` / `weather=` / `hand=` 等
    //!   同样没有等价 property，一律计入跳过。
    //!
    //! ## 映射规则（本任务实际实现的）
    //!
    //! | OptiFine | 1.21.4+ 物品定义 |
    //! |---|---|
    //! | `items=a,b` + `nbt.CustomModelData=N` + `texture=T` | `assets/minecraft/items/<a>.json` 与 `<b>.json` 各插一个 `select` 分支 `when: N` → 生成的 `cit` 模型 |
    //! | 同上但用 `model=M` | 分支直接指向 `M`（不生成模型文件） |
    //! | 生成模型 | `assets/minecraft/models/cit/<cit 相对路径>.json`，`parent` 为 `minecraft:item/handheld`（贴图名含 `_sword`/`_pickaxe`/… 等"手持类"后缀）或 `minecraft:item/generated`，`textures.layer0` 指向**被搬进标准贴图目录**的那份（`textures/cit/<cit 相对路径>`） |
    //! | 贴图 | CIT 的 PNG 惯例放在 properties 旁边，而那个位置客户端**不索引** ⇒ 原样复制到 `assets/minecraft/textures/cit/<cit 相对路径>.png` |
    //! | `damage=` / `stackSize=` / `enchantments=` / 其它 `nbt.*` / `type≠item` | **跳过**（记 note），因为 1.21.4 的物品定义没有等价 property |
    //!
    //! ## 为什么"跳过"是正确答案而不是缺陷
    //!
    //! 物品模型定义能表达的**只有**：模型选择（`select` / `range_dispatch` / `condition`）与
    //! 染色。它读不到附魔、读不到任意 NBT、读不到生物群系。**硬把 `enchantments=` 翻成
    //! `has_component(minecraft:enchantments)` 是错的**——那会变成"只要有任何附魔就换皮"，
    //! 把作者精确的条件放宽成近似条件，比不转换更糟（玩家看到错误的贴图，且无从察觉）。
    //!
    //! ## 与 CMD 数字的对接点
    //!
    //! 1.20.5 起 `CustomModelData` 变成 `minecraft:custom_model_data` **组件**，
    //! 1.21.4 起它有 `floats` / `flags` / `strings` 三个列表，而**老式数字**留在 `main` 字段。
    //! `select(property = custom_model_data, index = 0)` 读的就是 `main`——
    //! 因此 `nbt.CustomModelData=7` 与 `CustomModelData:7` 的旧地图/命令**逐字节兼容**。

    use super::*;
    use std::collections::BTreeMap;

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

    const FIRST_MODERN_FORMAT: u32 = 46;
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

    /// 一条 CIT 规则里本任务用得上的字段。
    struct CitRule {
        /// `items=` / `matchItems=` 里的物品名（已去重、已排序）。
        items: Vec<String>,
        /// `texture=`：相对 properties 文件的 PNG 路径。
        texture: Option<String>,
        /// `model=`：直接给定的模型引用。
        model: Option<String>,
        /// `nbt.CustomModelData=N`。
        custom_model_data: Option<u64>,
        /// 无法转换的条件（用于 note，不是错误）。
        unsupported: Vec<String>,
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

    /// `items=` / `matchItems=` → 物品名列表（去空、去重、排序）。
    fn parse_items(raw: &str) -> Vec<String> {
        let mut items: Vec<String> = raw
            .split(',')
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
            .map(|part| part.trim_start_matches("minecraft:").to_string())
            .collect();
        items.sort();
        items.dedup();
        items
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
        for blocked in [
            "damage",
            "stacksize",
            "enchantments",
            "enchantmentlevels",
            // **`enchantmentIDs=` 必须单列。** 它是 `enchantments=` 的旧名，
            // 属性键在这里会 to_ascii_lowercase，所以实际比较的是
            // `enchantmentids` —— 只写 `enchantments` 拦不住它。
            // 实测：服主材质包普遍只写 `enchantmentIDs=` 而**不写** Levels，
            // 漏掉这一项会把"附魔某等级才换皮"静默当成"无条件换皮"，
            // 于是该物品的所有实例都变成那张贴图 —— 条件被丢弃且无人察觉。
            "enchantmentids",
            "biome",
            "weather",
            "hand",
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
        // `nbt.*` 里**只有** `CustomModelData` 能被原版表达。
        for (key, _) in properties.iter() {
            if let Some(tag) = key.strip_prefix("nbt.") {
                if !tag.eq_ignore_ascii_case("custommodeldata") {
                    unsupported.push(format!("nbt.{tag}="));
                }
            }
        }
        let custom_model_data = properties
            .get("nbt.custommodeldata")
            .and_then(|value| value.trim().parse::<u64>().ok());
        // `nbt.CustomModelData` 写了但解析不出数字（可能是表达式）→ 当作不支持
        if properties.contains_key("nbt.custommodeldata") && custom_model_data.is_none() {
            unsupported.push("nbt.CustomModelData=（非数字）".to_string());
        }
        Some(CitRule {
            items,
            texture: properties.get("texture").cloned(),
            model: properties.get("model").cloned(),
            custom_model_data,
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

    pub fn decl() -> TaskDecl {
        // **`exclusive(true)`**：本任务的读范围跨**两个** CIT 根目录，写范围又跨
        // `items/`（别的任务刚写过）与 `models/cit/`、`textures/cit/`，而且它要**读回并改写**
        // 那些 `items/*.json`。声明成独占是保守但明确的选择：CIT 是低频任务
        // （多数包根本没有），让它单独占一个波次换来的确定性远比那点并发收益值钱。
        let mut reads = ScopeSet::prefix(ITEMS)
            .union(&ScopeSet::prefix("assets/minecraft/textures"));
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

    /// 把一条 CIT 规则并进某个物品的定义（没有定义就现造一个自指定义）。
    ///
    /// ## 为什么**不**去"钻进去"找默认分支，而是往外再包一层
    ///
    /// `generate_item_definitions` 可能已经在该物品的根上套了一层
    /// `condition(has_component, on_true: range_dispatch(旧的 custom_model_data 表))`。
    /// 本可以沿 `on_false` / `fallback` 钻进去、把 CIT 的 `select` 插在**更内层**，
    /// 但那样得到一个"旧表优先、CIT 兜底"的次序——而 CIT 是作者后来写的规则，
    /// **应当优先于**旧 `overrides` 里留下的数字表。
    ///
    /// 因此在**最外层**包一个 `select`：CIT 命中就走 CIT，否则整条旧链原样生效。
    /// 这与 OptiFine 里 CIT 的优先级高于 `overrides` 是一致的。
    fn merge_rule(
        tx: &mut Tx<'_>,
        item: &str,
        threshold: u64,
        model_reference: &str,
    ) -> Result<bool, AromError> {
        let path = format!("{ITEMS}/{item}.json");
        let mut definition = match tx.read(&path)? {
            Some(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|e| {
                AromError::path(format!("已有的 `{path}` 不是合法 JSON，拒绝改写：{e}"))
            })?,
            // 物品定义还不存在（比如作者只想用 CIT，没写过 `models/item/`）：
            // 造一个"自指默认定义"，语义上等价于原版默认外观。
            None => serde_json::json!({
                "model": { "type": "minecraft:model", "model": format!("minecraft:item/{item}") }
            }),
        };
        let Some(root) = definition.get_mut("model") else {
            return Ok(false);
        };
        let case = serde_json::json!({
            "when": threshold,
            "model": { "type": "minecraft:model", "model": model_reference },
        });
        // 已经有一个 CMD `select` 就并进去（作者可能有多条 CIT 命同一个物品）。
        if root.get("property").and_then(|p| p.as_str()) == Some(CMD_COMPONENT) {
            if let Some(cases) = root.get_mut("cases").and_then(|c| c.as_array_mut()) {
                let exists = cases
                    .iter()
                    .any(|existing| existing.get("when").and_then(|w| w.as_u64()) == Some(threshold));
                if exists {
                    // 同一物品同一 CMD 被两条 CIT 命中：**保留先写的那条**，
                    // 不覆盖（OptiFine 的同类冲突行为本就未定义，静默取一个总比来回抖好）。
                    return Ok(false);
                }
                cases.push(case);
                let text = serde_json::to_string_pretty(&definition)
                    .map_err(|e| AromError::internal(format!("序列化 `{path}` 失败：{e}")))?;
                tx.put(&path, text.into_bytes())?;
                return Ok(true);
            }
        }
        // 否则把整棵默认定义包进一个新的 `select`。
        let base = root.clone();
        let wrapped = serde_json::json!({
            "type": "minecraft:select",
            "property": CMD_COMPONENT,
            "index": 0,
            "cases": [case],
            "fallback": base,
        });
        *root = wrapped;
        let text = serde_json::to_string_pretty(&definition)
            .map_err(|e| AromError::internal(format!("序列化 `{path}` 失败：{e}")))?;
        tx.put(&path, text.into_bytes())?;
        Ok(true)
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
        let mut converted = 0usize;
        for path in &files {
            let text = tx.text(path)?;
            let properties = parse_properties(&text);
            let Some(rule) = cit_rule(&properties) else {
                outcome.skipped += 1;
                continue;
            };
            if !rule.unsupported.is_empty() {
                outcome.skipped += 1;
                outcome.notes.push(format!(
                    "跳过 {path}：条件 {} 在 1.21.4+ 的物品定义里没有等价表达",
                    rule.unsupported.join(" / ")
                ));
                continue;
            }
            if rule.items.is_empty() {
                outcome.skipped += 1;
                continue;
            }
            let Some(threshold) = rule.custom_model_data else {
                // 没有 `nbt.CustomModelData` 的 CIT 表示"只要拿着这个物品就换贴图"。
                // 原版**没有**这种写法：物品定义无法表达"看物品 id 就换皮"——
                // 那本来就是这个物品的默认外观，应该在 `items/<name>.json` 里直接写。
                // 强行接成 CMD 会要求玩家先给物品打标签，**改变作者意图**，因此不转。
                outcome.skipped += 1;
                outcome.notes.push(format!(
                    "跳过 {path}：没有 nbt.CustomModelData，无条件的换皮在 1.21.4+ 没有等价写法"
                ));
                continue;
            };

            // **CIT 内的相对路径**（去掉根前缀、去掉 `.properties`）——产物路径都跟着它走，
            // 这样"一个 properties 文件对应一个模型文件"，排查时一眼能对上。
            let cit_stem = ROOTS
                .iter()
                .find_map(|root| path.strip_prefix(&format!("{root}/")))
                .unwrap_or(path)
                .trim_end_matches(".properties")
                .to_string();

            // 解析出模型引用：`model=` 直接用；否则生成一个 `models/cit/...` 模型。
            let model_reference = match (&rule.model, &rule.texture) {
                (Some(given), _) => given.clone(),
                (None, texture) => {
                    // `texture=` 缺省时 OptiFine 按"同名 PNG"找。
                    let relative = texture
                        .clone()
                        .unwrap_or_else(|| format!("{cit_stem}.png"));
                    // **贴图在包里的实际位置**。OptiFine 的解析顺序不是"只有一处"：
                    //
                    // 1. `texture=` 如果写成**包内绝对路径**（带 `assets/`），直接用；
                    // 2. 否则按**相对 properties 所在目录**解析，并且要处理 `..`
                    //    （作者常用 `texture=../../../textures/item/ruby_sword.png` 指到原版贴图目录）；
                    // 3. 相对路径若以 `optifine/` 开头，按 `assets/minecraft/` 之下解析。
                    //
                    // 三处都不在 → 跳过整条规则（**不写半成品**：指向不存在贴图的模型会变成紫黑格）。
                    let directory = path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
                    let candidates: Vec<String> = if relative.starts_with("assets/") {
                        vec![relative.clone()]
                    } else if relative.starts_with("optifine/") {
                        vec![format!("assets/minecraft/{relative}")]
                    } else {
                        // 相对路径可能写成旧式 `textures/items/...`，新式 `textures/item/...`，
                        // 也可能干脆只有文件名；把这两种也当候选，覆盖旧包写法。
                        let legacy = relative.replacen("textures/item/", "textures/items/", 1);
                        let mut list = vec![
                            resolve_relative(directory, &relative),
                            resolve_relative(directory, &legacy),
                        ];
                        list.dedup();
                        list
                    };
                    let found = candidates.iter().find(|candidate| tx.exists(candidate));
                    let Some(texture_path) = found.cloned() else {
                        outcome.skipped += 1;
                        outcome.notes.push(format!(
                            "跳过 {path}：texture 指向的 `{relative}` 在包里找不到（试过 {}）",
                            candidates.join(" / ")
                        ));
                        continue;
                    };
                    // 模型路径跟着 CIT 的相对路径走（`cit/ruby/sword`），保持可读、可排查。
                    let model_stem = cit_stem.as_str();
                    let model_path = format!("{GENERATED_MODELS}/{model_stem}.json");
                    // **贴图必须搬进 `assets/minecraft/textures/` 才引用得到。**
                    //
                    // 这是本任务最容易踩的坑：模型里的 `textures.layer0` 是**贴图引用**，
                    // 客户端只在 `assets/<namespace>/textures/<path>.png` 下找它，并且
                    // **不做任何相对路径解析**（`..` 在资源路径里是非法的）。
                    // 而 OptiFine 的 CIT 贴图惯例是放在 properties 文件旁边
                    // （`optifine/cit/...` 之下）——那个位置客户端**根本不索引**。
                    //
                    // 因此：同目录/相对路径解析到的图**原样复制**到
                    // `textures/cit/<CIT 相对路径>.png`，模型再引用它。
                    // 直接引用 `optifine/cit/...` 会得到一个永远加载不出来的贴图（紫黑格）。
                    let texture_reference = match texture_path.strip_prefix("assets/minecraft/textures/") {
                        // 已经在标准贴图目录里（作者写的相对路径指到了 `textures/`）→ 原地引用。
                        Some(inside) => inside.trim_end_matches(".png").to_string(),
                        None => {
                            let target =
                                format!("assets/minecraft/textures/cit/{model_stem}.png");
                            if !tx.exists(&target) {
                                let bytes = tx.read(&texture_path)?.unwrap_or_default();
                                tx.put(&target, bytes)?;
                            }
                            format!("cit/{model_stem}")
                        }
                    };
                    let model = serde_json::json!({
                        "parent": parent_for(&relative),
                        "textures": { "layer0": texture_reference },
                    });
                    let text = serde_json::to_string_pretty(&model).map_err(|e| {
                        AromError::internal(format!("序列化模型 `{model_path}` 失败：{e}"))
                    })?;
                    tx.put(&model_path, text.into_bytes())?;
                    format!(
                        "minecraft:{}",
                        model_path
                            .strip_prefix("assets/minecraft/models/")
                            .unwrap_or(&model_path)
                            .trim_end_matches(".json")
                    )
                }
            };

            for item in &rule.items {
                if merge_rule(tx, item, threshold, &model_reference)? {
                    converted += 1;
                    outcome.notes.push(format!(
                        "CIT {path} → items/{item}.json（CustomModelData={threshold} → {model_reference}）"
                    ));
                }
            }
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

        /// `type!=item` 一律不碰（附魔/盔甲/鞘翅是别的机制）。
        #[test]
        fn non_item_cit_is_not_a_rule() {
            let map = parse_properties("type=enchantment\nitems=diamond_sword\ntexture=x.png\n");
            assert!(cit_rule(&map).is_none());
        }

        /// 无法表达的条件必须被**列出来**（而不是静默丢掉）。
        #[test]
        fn unsupported_conditions_are_reported() {
            let map = parse_properties(
                "type=item\nitems=diamond_sword\ntexture=x.png\n\
                 nbt.CustomModelData=7\nnbt.display.Name=hi\nenchantments=sharpness\ndamage=50%\n",
            );
            let rule = cit_rule(&map).expect("rule");
            assert_eq!(rule.custom_model_data, Some(7), "CMD 是唯一支持的条件");
            let joined = rule.unsupported.join(" ");
            for expected in ["nbt.display.name", "enchantments=", "damage="] {
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

        /// 没有 `nbt.CustomModelData` 的 CIT（无条件换皮）**必须跳过**：
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

        /// **`enchantmentIDs=` 必须被拦下。**
        ///
        /// 它是 `enchantments=` 的旧名，而属性键在这里会被 `to_ascii_lowercase`，
        /// 所以实际比较的是 `enchantmentids` —— 只写 `enchantments` 拦不住它。
        ///
        /// 为什么这条值得单独钉：漏掉它时规则会被当成**无条件换皮**转换，
        /// 于是该物品的每一个实例都变成那张贴图，而"附魔某等级才换皮"的条件
        /// 被静默丢弃。玩家看到错误的贴图且无从察觉，比不转换糟得多。
        ///
        /// 真实来源：一份 1.20.1 服务端材质包的 24 条 CIT 规则里 18 条用
        /// `enchantmentIDs=` + `enchantmentLevels=` 触发。
        #[test]
        fn enchantment_ids_alone_is_blocked_not_treated_as_unconditional() {
            let mut p = pack(
                55,
                vec![
                    (
                        &format!("{CIT_DIR}/sharp.properties"),
                        // 注意：**故意不写 enchantmentLevels**，只留 enchantmentIDs
                        "type=item\nmatchItems=diamond_sword\nenchantmentIDs=sharpness\nmodel=s2\n",
                    ),
                    (&format!("{CIT_DIR}/s2.json"), "{}"),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(
                outcome.changed, 0,
                "带附魔条件的规则不能当成无条件换皮转换"
            );
            assert_eq!(outcome.skipped, 1);
            assert!(
                p.view().resolve("assets/minecraft/items/diamond_sword.json").is_none(),
                "跳过就不能留下半成品——留下就等于把条件丢了"
            );
            assert!(
                outcome.notes.iter().any(|n| n.contains("enchantmentids")),
                "跳过必须写清是哪一条条件拦下的，实际 notes = {:?}",
                outcome.notes
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

        /// 声明的范围必须覆盖真正写入的两处目录。
        #[test]
        fn declared_scope_covers_both_output_directories() {
            let decl = decl();
            assert_eq!(decl.tier, Tier::Surgeon);
            assert!(decl.exclusive, "跨目录读写 + 读他人刚写过的 items/ ⇒ 必须独占");
            assert!(decl.writes.contains("assets/minecraft/items/stick.json"));
            assert!(decl.writes.contains("assets/minecraft/models/cit/ruby/x.json"));
            assert!(decl.reads.contains("assets/minecraft/optifine/cit/ruby/x.properties"));
        }
    }
