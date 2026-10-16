    //! `generate_item_definitions`：把旧包的 `models/item/*.json` **`overrides`** 迁移到
    //! 1.21.4+（pack_format 46）的**物品模型定义** `assets/minecraft/items/<name>.json`。
    //!
    //! ## 问题的形状
    //!
    //! pack_format 42（1.21.3）及以前，一个物品的「自定义外观」是这样表达的：
    //!
    //! ```json
    //! // assets/minecraft/models/item/bow.json
    //! { "parent": "item/bow",
    //!   "overrides": [
    //!     { "predicate": { "pulling": 1 },          "model": "item/bow_pulling_0" },
    //!     { "predicate": { "custom_model_data": 7 }, "model": "mypack:item/ruby_bow" } ] }
    //! ```
    //!
    //! 1.21.4（46）起，**`overrides` 不再被读取**。物品到模型的选择被搬进了新文件
    //! `assets/<ns>/items/<name>.json`，用一棵「模型节点树」描述。因此旧包一旦转到 46+，
    //! **弓的拉弦、时钟 / 指南针的帧动画、皮革甲染色、盔甲纹饰、以及所有
    //! `CustomModelData` 自定义物品全部失效**——本任务就是把它们接回去。
    //!
    //! ## 两件事，两种做法
    //!
    //! | 旧写法 | 新版对应 | 本任务怎么接 |
    //! |---|---|---|
    //! | `overrides` 只用了**原版谓词**（`pulling` / `pull` / `cast` / `time` / `angle` / `trim_type` / `damage` …） | 官方定义里本来就是这些 `condition` / `range_dispatch` / `select` 节点 | **用官方默认定义**（从客户端 jar 提取，见 [`TABLE`]） |
    //! | `overrides` 用了 **`custom_model_data`** 谓词 | `minecraft:select` / `minecraft:range_dispatch` 的 `property: "minecraft:custom_model_data"` | **就地生成**（[`custom_model_data_dispatch`]），因为官方定义里当然没有作者自己的 CMD 表 |
    //!
    //! 第二行是**这一块能力的核心价值**：作者的 CMD 表是包自己的数据，任何参考表都不可能预先收录。
    //!
    //! ## 参考数据从哪来
    //!
    //! 「原版默认定义长什么样」是 **Mojang 的数据**（时钟 64 帧、指南针 32 帧、纹饰 11 材质 × N 件），
    //! 手写必错。数据落在同目录的 `item-definitions.json`，由 `tools/gen-item-definitions.py`
    //! **从官方客户端 jar 重新提取**（脚本读 jar 自报的 `pack_version.resource` 当格式号）。
    //!
    //! **为什么不抄现成实现的数据文件**：那是**许可**问题——本项目是 MIT，
    //! 而那份实现是 GPL-3.0-only，其数据文件随其仓库分发、受同一许可覆盖。
    //! 从官方 jar 重生成同时满足「权威」「可复现」「无许可风险」。
    //!
    //! ## 已知边界（不回避）
    //!
    //! * **`overrides` 里的非 CMD 谓词**（`damage` 作为"受损"布尔、`nbt.*`、`enchantments`）
    //!   没有对应的新版 property。这类条目**不被转换**：模板原样来自官方，作者那部分条件
    //!   就丢了（物品退回官方默认外观，而不是变成紫黑格）。**这是有意的降级**，
    //!   因为把 `enchantments=sharpness` 硬翻成"有任何附魔"会把精确条件放宽成近似条件，
    //!   比不转换更糟（玩家看到错误贴图且无从察觉）。
    //! * **`minecraft:special` 定义不收录**（床 / 箱子 / 旗帜 / 盾 / 潜影盒 / 潮涌核心……）：
    //!   它们由客户端硬编码渲染器画，外观**无法**被物品模型定义改写。
    //! * **不生成模型几何**：`assets/minecraft/models/item/*` 一律不动（作者的模型文件
    //!   本来就还在包里，定义只是把它们重新引用回来）。
    //! * 若作者把 `overrides` 放在**别的文件名**的模型里（不是以物品命名），本任务按物品名对不上，
    //!   不会为它生成定义。
    //! * 本任务**不改写**物品定义里官方模板的内容——除了把作者的 CMD 表包在最外层。
    //!   也就是说"官方默认外观"永远是逐字节的官方数据，不存在"手写推导"的成分。

    use super::*;
    use std::collections::BTreeMap;
    use std::sync::OnceLock;

    /// 参考表内容（`include_str!` 的路径相对**本文件**，因此数据文件必须放在同目录）。
    const TABLE_JSON: &str = include_str!("item-definitions.json");

    /// 物品定义的包内目录（1.21.4+）。
    const ITEMS: &str = "assets/minecraft/items";

    /// 旧包里物品模型所在目录（**迁移的来源**）。
    const MODELS_ITEM: &str = "assets/minecraft/models/item";

    /// 1.21.4 的 pack_format：物品模型定义从这一刻起生效。
    ///
    /// 拿一个**具体**数字而不是"最高支持版本"是有意的：`conversion_maps` 里这条任务
    /// 只挂在 `(42, 46)` 这条边上，而 46 → 97 之间物品定义的 schema **没有第二次搬家**，
    /// 因此不需要重复执行。
    ///
    /// **这个值是从 `pack.mcmeta` 读出来的目标格式**，不是源格式——驱动在管线一开始
    /// 就把 `pack.mcmeta` 写成目标格式（`native_run` 里的 §9.155）。这一点很关键：
    /// 在 §9.155 之前，任务读到的其实是**源**格式，于是本任务在真实转换里
    /// **整段空转且不报错**（实测由 `modern_item_definitions_survive_the_whole_pipeline` 抓出）。
    const FIRST_MODERN_FORMAT: u32 = 46;

    /// `custom_model_data` 组件的名字（1.21.4+）。
    ///
    /// 1.20.5（32）起 `CustomModelData` 的 NBT 整数标签变成 `minecraft:custom_model_data`
    /// **组件**；1.21.4 起该组件有了 `floats` / `flags` / `strings` 三个列表，
    /// 而**老式的数字**仍写在 `main` 字段里，并被 `minecraft:custom_model_data`
    /// property 的 `index: 0` 读到——这就是下面那棵树能接住旧 CMD 数字的原因。
    const CMD_COMPONENT: &str = "minecraft:custom_model_data";

    /// 一份格式档：某个 pack_format 上的官方物品定义模板。
    struct Profile {
        /// 数据来源的官方版本号（只用于日志与报告）。
        reference: String,
        /// 物品名 → 该物品在新版里的**默认定义**（`{"model": ...}` 整体）。
        definitions: BTreeMap<String, serde_json::Value>,
    }

    /// 解析后的参考表：按 `pack_format` 升序，因此可以做"≤ 目标"的回落。
    struct Table {
        profiles: Vec<(u32, Profile)>,
    }

    /// **懒解析**：参考表 488 KB，只有真的跑到这个任务时才付解析成本。
    fn table() -> &'static Table {
        static PARSED: OnceLock<Table> = OnceLock::new();
        PARSED.get_or_init(|| {
            let raw: serde_json::Value = serde_json::from_str(TABLE_JSON)
                .expect("item-definitions.json 解析失败（构建期数据损坏）");
            let entries = raw
                .get("profiles")
                .and_then(|p| p.as_object())
                .expect("item-definitions.json 缺少 profiles");
            let mut profiles = Vec::with_capacity(entries.len());
            for (format, profile) in entries {
                let format: u32 = format.parse().unwrap_or_else(|_| {
                    panic!("profiles 的键必须是 pack_format 数字，实际 `{format}`")
                });
                let reference = profile
                    .get("reference")
                    .and_then(|r| r.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let definitions: BTreeMap<String, serde_json::Value> = profile
                    .get("definitions")
                    .and_then(|d| d.as_object())
                    .map(|d| d.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default();
                profiles.push((
                    format,
                    Profile {
                        reference,
                        definitions,
                    },
                ));
            }
            profiles.sort_by_key(|(format, _)| *format);
            Table { profiles }
        })
    }

    /// 目标格式 → 该用哪一档模板：**≤ 目标的最大已收录档**。
    ///
    /// 为什么是「≤ 目标」而不是「精确相等」：2pyr 支持的目标格式里 `55`（1.21.5）
    /// 与 `63`（1.21.6）**本机没有对应的客户端 jar**，而这两个版本的物品定义与
    /// 1.21.4 **逐字节相同**（Mojang 在这两个版本没动物品定义）——因此回落到 46
    /// 是正确的而不是将就。回落本身是单调的：格式只会向前。
    ///
    /// 返回 `None` = 目标格式下面没有任何模板 ⇒ 调用方**什么都不做**（而不是猜一个）。
    fn profile_for(table: &Table, target: u32) -> Option<&Profile> {
        table
            .profiles
            .iter()
            .rev()
            .find(|(format, _)| *format <= target)
            .map(|(_, profile)| profile)
    }

    pub fn decl() -> TaskDecl {
        // **不标 `exclusive`**：写入范围（`assets/minecraft/items`）与 Architect 阶段
        // 其它任务（贴图、GUI、方块）不相交，声明清楚就能同波并发
        // （`arom::task::plan` 按写集切波）。
        TaskDecl::new("generate_item_definitions", Tier::Architect)
            .reads(ScopeSet::prefix(MODELS_ITEM))
            .writes(ScopeSet::prefix(ITEMS))
    }

    /// 读包里 `assets/minecraft/models/item/` 下的**直接子文件**名（不含扩展名，已排序去重）。
    fn pack_item_models(tx: &Tx<'_>) -> Result<Vec<String>, AromError> {
        let mut names: Vec<String> = tx
            .list(MODELS_ITEM)?
            .into_iter()
            .filter(|res| !res.is_dir)
            .filter_map(|res| {
                let rest = res.path.strip_prefix(MODELS_ITEM)?.trim_start_matches('/');
                // 只认直接子文件：`models/item/foo/bar.json` 不是物品名
                // （`strip_prefix` 会把 `foo/bar` 也带出来，必须自己挡掉）。
                if rest.is_empty() || rest.contains('/') {
                    return None;
                }
                rest.strip_suffix(".json").map(str::to_string)
            })
            .collect();
        names.sort();
        names.dedup();
        Ok(names)
    }

    /// 从 `overrides` 里剔出一个物品的 `custom_model_data` 条目：
    /// `(CMD 阈值, 模型引用)`，**按阈值升序**。
    ///
    /// 只收**数字**型 `custom_model_data`（旧格式里它就是个数字）且**严格递增**——
    /// 同一个阈值出现多次时取**最后一个**（旧客户端的 `overrides` 是顺序求值，
    /// 后写的赢）。
    fn custom_model_data_entries(model: &serde_json::Value) -> Vec<(u64, String)> {
        let Some(overrides) = model.get("overrides").and_then(|o| o.as_array()) else {
            return Vec::new();
        };
        // BTreeMap 而不是 Vec：同阈值自动去重为"最后写入者胜"，且天然升序。
        let mut by_threshold: BTreeMap<u64, String> = BTreeMap::new();
        for entry in overrides {
            let Some(predicate) = entry.get("predicate").and_then(|p| p.as_object()) else {
                continue;
            };
            let Some(threshold) = predicate
                .get("custom_model_data")
                .and_then(serde_json::Value::as_u64)
            else {
                continue;
            };
            let Some(model) = entry.get("model").and_then(|m| m.as_str()) else {
                continue;
            };
            by_threshold.insert(threshold, model.to_string());
        }
        by_threshold.into_iter().collect()
    }

    /// 一个 `{"type":"minecraft:model","model":"<reference>"}` 节点。
    fn model_node(reference: &str) -> serde_json::Value {
        serde_json::json!({ "type": "minecraft:model", "model": reference })
    }

    /// 用作者的 CMD 表包住 `base`（该物品的**默认定义**里的 `model` 节点）。
    ///
    /// ## 生成的树
    ///
    /// ```text
    /// condition(property = has_component, component = minecraft:custom_model_data)
    ///   ├─ on_false → base                      // 没有该组件：默认外观
    ///   └─ on_true  → range_dispatch(property = custom_model_data, index = 0, scale = 1)
    ///                   ├─ threshold 0   → <作者模型 0>
    ///                   ├─ threshold 100 → <作者模型 100>
    ///                   └─ fallback      → base   // CMD 落在表外：默认外观
    /// ```
    ///
    /// ## 每一层为什么必须在
    ///
    /// * **外层 `condition` 不是装饰**：`range_dispatch` 的 `scale` 最小是 `0.01`，
    ///   而旧谓词是**精确相等**。用 `index: 0, scale: 1` 时阈值是整数、命中语义为
    ///   "最大的 `≤` 值"——与旧行为**只在整数 CMD 上一致**。为了不把
    ///   「`CMD=3.5` 落在 `3` 与 `4` 之间」这种旧实现里**不匹配任何条目**的情形
    ///   悄悄变成"匹配 `3`"，这里显式检查"组件到底在不在"：
    ///   在 → 走 range_dispatch（用户确实设了 CMD），不在 → 默认外观。
    /// * **`fallback` 指向 `base`**：旧 `overrides` 全不命中时客户端回落到基础模型，
    ///   语义一致。
    /// * **`index: 0`**：读 `custom_model_data` 组件 `main` 字段的第 0 个下标——
    ///   老式数字 CMD 就落在这里。
    fn custom_model_data_dispatch(
        base: &serde_json::Value,
        entries: &[(u64, String)],
    ) -> serde_json::Value {
        let dispatch = serde_json::json!({
            "type": "minecraft:range_dispatch",
            "property": CMD_COMPONENT,
            "index": 0,
            "scale": 1.0,
            "entries": entries
                .iter()
                .map(|(threshold, reference)| serde_json::json!({
                    "threshold": threshold,
                    "model": model_node(reference),
                }))
                .collect::<Vec<_>>(),
            "fallback": base.clone(),
        });
        serde_json::json!({
            "type": "minecraft:condition",
            "property": "minecraft:has_component",
            "component": CMD_COMPONENT,
            "on_true": dispatch,
            "on_false": base.clone(),
        })
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let target = tx
            .view()
            .mcmeta()
            .ok()
            .and_then(|m| m.effective_format())
            .unwrap_or(FIRST_MODERN_FORMAT);
        if target < FIRST_MODERN_FORMAT {
            // 目标还不是 1.21.4+：物品定义在那种客户端上**不存在**，写出去只是垃圾文件。
            // （这条分支同时是"不涉及 1.21.4+ 的目标产物逐字节不变"的第一道保证。）
            return Ok(Outcome::default());
        }

        let mut outcome = Outcome::default();
        let table = table();
        let Some(profile) = profile_for(table, target) else {
            // 目标格式下面没有模板。**不猜**：凭空写一份会盖掉官方定义。
            outcome.notes.push(format!(
                "目标格式 {target} 没有对应的物品定义参考档，跳过（已知档：{:?}）",
                table.profiles.iter().map(|(f, _)| *f).collect::<Vec<_>>()
            ));
            return Ok(outcome);
        };

        let models = pack_item_models(tx)?;
        if models.is_empty() {
            // 包里没有任何物品模型 = 没有 `overrides` 要迁移。
            // 真实包 TapL 16x 走的就是这条（它只有 `mcpatcher/` 与方块贴图）。
            return Ok(outcome);
        }

        let mut written = 0usize;
        let mut plain = 0usize;
        let mut with_cmd = 0usize;
        let mut skipped_existing = 0usize;
        for name in &models {
            let path = format!("{ITEMS}/{name}.json");
            if tx.exists(&path) {
                // 包里**已经有**新版定义（作者手工升级过，或上一次转换留下的）。
                // 尊重它：覆盖会把作者的东西弄丢。
                skipped_existing += 1;
                continue;
            }
            let Some(template) = profile.definitions.get(name) else {
                // 该物品在新版里是**平凡定义**（`{"type":"minecraft:model","model":"minecraft:item/<name>"}`），
                // 或它压根不是物品（`models/item/` 下也常放只给别的模型引用的中间件）。
                //
                // **刻意不为它写一个"自指定义"**：官方客户端里本来就有这个文件，
                // 而"自指"写不出比官方更正确的内容——凭空造一个只会把该物品的
                // 官方默认外观盖成"只有 self 引用"。
                plain += 1;
                continue;
            };
            let mut definition = template.clone();
            // 作者的 CMD 表接在默认定义**外面**（见 `custom_model_data_dispatch`）。
            let overrides = tx.json(&format!("{MODELS_ITEM}/{name}.json"))?;
            let entries = custom_model_data_entries(&overrides);
            if !entries.is_empty() {
                let base = definition
                    .get("model")
                    .cloned()
                    .unwrap_or_else(|| model_node(&format!("minecraft:item/{name}")));
                definition["model"] = custom_model_data_dispatch(&base, &entries);
                with_cmd += 1;
            }
            let text = serde_json::to_string_pretty(&definition).map_err(|e| {
                AromError::internal(format!("序列化物品定义 `{name}` 失败：{e}"))
            })?;
            tx.put(&path, text.into_bytes())?;
            written += 1;
        }

        if written > 0 {
            outcome.changed = written;
            outcome.notes.push(format!(
                "按官方 {} 结构写出 {written} 个物品模型定义到 {ITEMS}/（包内物品模型 {} 个；其中 {with_cmd} 个接了包的 CustomModelData 表）",
                profile.reference,
                models.len()
            ));
        }
        if plain > 0 {
            outcome.notes.push(format!(
                "{plain} 个包内物品模型在新版里是平凡定义或非物品，未生成（官方自带，无需覆盖）"
            ));
        }
        if skipped_existing > 0 {
            outcome.notes.push(format!("{skipped_existing} 个物品新版定义已存在，保留原样"));
        }
        Ok(outcome)
    }

    /// 任务名 → (声明, 实现)。
    pub fn lookup(name: &str) -> Option<(TaskDecl, PilotFn)> {
        match name {
            "generate_item_definitions" => Some((decl(), run)),
            _ => None,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::arom::source::MemSource;

        /// 造一个内存包。`format` 写进 `pack.mcmeta`——**任务是靠它判断目标版本的**。
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

        /// 一个最小但形状完整的旧物品模型：基础模型 + 两条 CMD 覆盖。
        const OLD_ITEM: &str = r#"{
          "parent": "item/generated",
          "textures": { "layer0": "item/ruby" },
          "overrides": [
            { "predicate": { "custom_model_data": 1000 }, "model": "mypack:item/ruby_1" },
            { "predicate": { "custom_model_data": 5 },    "model": "mypack:item/ruby_5" }
          ]
        }"#;

        fn run_on(pack: &mut crate::arom::Pack) -> Outcome {
            let mut tx = pack.tx("item-definitions-test");
            let outcome = run(&mut tx).expect("run");
            let layer = tx.into_layer();
            pack.commit(layer);
            outcome
        }

        /// **目标 < 46 必须什么都不写。**
        ///
        /// 这是「不涉及 1.21.4+ 的目标产物逐字节不变」的第一道保证：
        /// 物品定义在 1.21.3 及以前的客户端上**不存在**，写出去只是垃圾文件。
        #[test]
        fn does_nothing_below_the_modern_boundary() {
            for format in [1u32, 15, 34, 42, 45] {
                let mut p = pack(
                    format,
                    vec![("assets/minecraft/models/item/stick.json", OLD_ITEM)],
                );
                let outcome = run_on(&mut p);
                assert_eq!(outcome.changed, 0, "格式 {format} 不应写任何东西");
                assert!(
                    p.view().resolve("assets/minecraft/items/stick.json").is_none(),
                    "格式 {format} 下不该出现物品定义"
                );
            }
        }

        /// 没有物品模型的包必须**零改动**且**零 note**（真实包 TapL 16x 就是这条路径）。
        #[test]
        fn does_nothing_without_any_item_model() {
            let mut p = pack(46, vec![("assets/minecraft/textures/item/stick.png", "x")]);
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 0);
            assert!(
                outcome.notes.is_empty(),
                "无输入时不该有 note：{:?}",
                outcome.notes
            );
        }

        /// 官方参考表必须真的被解析出来，且覆盖 1.21.4 的 `(42,46)` 边界。
        #[test]
        fn reference_table_has_the_modern_profiles() {
            let t = table();
            assert!(
                t.profiles.iter().any(|(f, _)| *f == FIRST_MODERN_FORMAT),
                "参考表里必须有 46 档（1.21.4）"
            );
            assert!(
                t.profiles.windows(2).all(|w| w[0].0 < w[1].0),
                "档位必须严格升序（回落查找依赖它）"
            );
            for (format, profile) in &t.profiles {
                assert!(
                    !profile.definitions.is_empty(),
                    "格式 {format}（{}）的定义集为空",
                    profile.reference
                );
            }
        }

        /// **回落规则**：`55` / `63`（1.21.5 / 1.21.6）本机没有 jar，必须落到 46。
        #[test]
        fn profile_lookup_falls_back_monotonically() {
            let t = table();
            let at = |target: u32| profile_for(t, target).map(|p| p.reference.as_str());
            assert_eq!(at(46), Some("1.21.4"));
            assert_eq!(at(55), Some("1.21.4"), "1.21.5 无 jar，回落到 46");
            assert_eq!(at(63), Some("1.21.4"), "1.21.6 无 jar，回落到 46");
            assert_eq!(at(64), Some("1.21.8"));
            assert_eq!(at(69), Some("1.21.10"));
            assert_eq!(at(75), Some("1.21.11"));
            assert_eq!(at(88), Some("26.2"));
            assert_eq!(at(97), Some("26.2"), "97 没有更新存档，回落到 88");
            assert_eq!(at(45), None, "46 以下必须无档");
        }

        /// **CMD 条目抽取**：按阈值升序；同阈值**后者胜**（旧 `overrides` 顺序求值）。
        #[test]
        fn cmd_entries_are_sorted_and_last_wins() {
            let value: serde_json::Value = serde_json::from_str(
                r#"{"overrides":[
                     {"predicate":{"custom_model_data":9},"model":"a:x9"},
                     {"predicate":{"custom_model_data":3},"model":"a:x3"},
                     {"predicate":{"custom_model_data":9},"model":"a:x9_last"},
                     {"predicate":{"pulling":1},"model":"a:no_cmd"},
                     {"predicate":{"custom_model_data":7}}]}"#,
            )
            .expect("json");
            assert_eq!(
                custom_model_data_entries(&value),
                vec![(3, "a:x3".to_string()), (9, "a:x9_last".to_string())]
            );
        }

        /// 没有 `overrides` 时不该造出空表（否则会给每个物品套一层无用的 condition）。
        #[test]
        fn no_overrides_means_no_cmd_dispatch() {
            let value: serde_json::Value =
                serde_json::from_str(r#"{"parent":"item/generated"}"#).expect("json");
            assert!(custom_model_data_entries(&value).is_empty());
        }

        /// 生成的 CMD 调度树形状：外层 `condition(has_component)` + 内层 `range_dispatch`。
        #[test]
        fn cmd_dispatch_shape_is_component_guarded() {
            let base = model_node("minecraft:item/stick");
            let dispatch = custom_model_data_dispatch(
                &base,
                &[
                    (5, "mypack:item/a".into()),
                    (1000, "mypack:item/b".into()),
                ],
            );
            assert_eq!(dispatch["type"], "minecraft:condition");
            assert_eq!(dispatch["property"], "minecraft:has_component");
            assert_eq!(dispatch["component"], CMD_COMPONENT);
            assert_eq!(dispatch["on_false"], base, "没有组件时必须回落到默认外观");
            let inner = &dispatch["on_true"];
            assert_eq!(inner["type"], "minecraft:range_dispatch");
            assert_eq!(inner["property"], CMD_COMPONENT);
            assert_eq!(inner["index"], 0);
            assert_eq!(inner["scale"], 1.0);
            assert_eq!(inner["fallback"], base, "CMD 落在表外也要回落到默认外观");
            assert_eq!(inner["entries"][0]["threshold"], 5);
            assert_eq!(inner["entries"][0]["model"]["model"], "mypack:item/a");
            assert_eq!(inner["entries"][1]["threshold"], 1000);
        }

        /// **端到端**：带 CMD 的包在 46 下写出定义；`stick` 是平凡定义 ⇒ 不生成。
        #[test]
        fn writes_definitions_for_known_items_and_skips_trivial_ones() {
            let mut p = pack(
                46,
                vec![
                    ("assets/minecraft/models/item/bow.json", OLD_ITEM),
                    // `stick` 在新版里是平凡定义（自指 model），刻意不为它写自指文件
                    ("assets/minecraft/models/item/stick.json", "{}"),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(outcome.changed, 1, "只有 bow 该被写出：{:?}", outcome.notes);

            let bow = p
                .view()
                .json("assets/minecraft/items/bow.json")
                .expect("bow 定义应当存在");
            // 外层是包的 CMD 表（阈值 5 / 1000），内层才是官方默认定义
            assert_eq!(bow["model"]["property"], "minecraft:has_component");
            let inner = &bow["model"]["on_true"];
            assert_eq!(inner["entries"][0]["threshold"], 5);
            assert_eq!(inner["entries"][0]["model"]["model"], "mypack:item/ruby_5");
            assert_eq!(inner["entries"][1]["threshold"], 1000);
            // 官方默认定义被完整保留在 fallback 里（弓的拉弦调度不能丢）
            let fallback = inner["fallback"].to_string();
            assert!(
                fallback.contains("minecraft:use_duration") && fallback.contains("bow_pulling_2"),
                "官方默认定义必须原样保留：{fallback}"
            );
            assert!(
                p.view().resolve("assets/minecraft/items/stick.json").is_none(),
                "平凡定义不该生成"
            );
        }

        /// 包里**已有**的新版定义必须被保留（作者手工升级过 / 上次转换留下的）。
        #[test]
        fn existing_definitions_are_never_overwritten() {
            const MINE: &str = r#"{"model":{"type":"minecraft:model","model":"mypack:item/bow"}}"#;
            let mut p = pack(
                46,
                vec![
                    ("assets/minecraft/models/item/bow.json", OLD_ITEM),
                    ("assets/minecraft/items/bow.json", MINE),
                ],
            );
            let outcome = run_on(&mut p);
            assert_eq!(
                outcome.changed, 0,
                "已有定义不该被覆盖：{:?}",
                outcome.notes
            );
            assert_eq!(
                p.view()
                    .text("assets/minecraft/items/bow.json")
                    .expect("read"),
                MINE
            );
        }

        /// 声明的写入范围必须覆盖真正写出的路径——
        /// 否则 `strict_scopes` 下驱动会判「写了自己没声明的路径」。
        #[test]
        fn declared_scope_covers_every_written_path() {
            let decl = decl();
            assert_eq!(decl.tier, Tier::Architect);
            assert!(decl.writes.contains("assets/minecraft/items/bow.json"));
            assert!(decl.reads.contains("assets/minecraft/models/item/bow.json"));
            // 反向：不能顺手声明成整包（那会让它和一切任务冲突、退化成串行）
            assert!(!decl.writes.is_any(), "写入范围必须收窄到 items/");
            assert!(!decl.reads.is_any(), "读取范围必须收窄到 models/item/");
        }
    }
