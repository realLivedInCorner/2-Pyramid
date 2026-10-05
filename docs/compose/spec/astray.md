---
feature: astray
status: draft
updated: 2026-10-05
branch: feat/astray
commits: c750ede
---

> **阅读须知**：本文是**设计与实施记录**，按时间顺序追加，**保留了被推翻的中间结论**。
> 标注为「已撤回 / 纠正」的段落仅用于记录**推理过程与负结果**（为什么某条路走不通），
> **不是现行做法**——现行结论以文中后出的节与交叉引用为准。
> 代码注释里引用的 `§9.NN` 均可在本文找到对应小节。

# Astray / A-ROM — 转换内核从「文件流」到「对象模型」的破坏性重构

> **本文档是一项内部设计的破坏性变更的裁决记录。**
> 它不是叠加特性：转换引擎的模块边界、转换器契约、中间产物形态、缓存键与命令层参数都会被重写。
> Astray 是下一版本的更新代号；A-ROM = **A**stray **R**esourcePacks **O**bject **M**odel。
> **A-ROM 是重新设计，不是 Foray ROM 的推广**（理由见 2.2）。
> 事实基础：`src-tauri/src` 全量盘点（144 个 `.rs` / 30,704 行，行数含空行）与影响面清单（`A-ROM-impact-map.md`）。本文引用均已核对到行。
> 子设计：[astray-arom-model.md](./astray-arom-model.md) —— L0–L4 的类型、签名、所有权与逐步推进阶梯。

## [S0] 变更定性与治理

### 0.1 这是破坏性变更的确切含义

| 维度 | 结论 |
|---|---|
| 破坏等级 | **内部设计级**：`converters/`（108 文件 / 18,428 行）与 `hurray/`（7 文件 / 1,592 行）的执行形态全部改写；Foray 的 `Rom` 类型退役（`foray/` 2,351 行转为消费者） |
| 破坏范围 | 转换器签名与任务体、`HurrayContext` 契约、「临时目录是唯一真源」这一假设、贴图缓存键、任务间通信、解压/打包主路径、调度与并行模型 |
| 对外目标 | **零变化**：用户可见行为与产物 zip 字节不变（证明方式见 2.8，注意其中的闸门缺口） |
| 允许的对外破坏 | 仅限 2.10 显式列出并裁决过的条目；未列出的即视为对外承诺，不得破 |

### 0.2 解除 `foray.md` 的锁定条款（治理前提）

| 位置 | 原文 | Astray 的处理 |
|---|---|---|
| foray.md:48 | 「ROM：**Foray 侧独立 ROM**，不改写 Hurray 104 个转换器；Hurray 继续文件流」 | **解除**：A-ROM 成为唯一内核 |
| foray.md:72 | 「**禁止**在本特性内把转换器改成对象操作」 | **解除**：本变更的全部内容就是这个 |
| foray.md:73 | 「未来 Master Enzyme 实时编辑走 ROM 时，另开迁移特性（Out of Scope）」 | **认领**：Astray 即该迁移特性 |
| foray.md:154 | Out of Scope：「改写 Hurray 104 个转换器 / 全量测试重写」 | **移入 In Scope**（分期见 2.9） |

**解除的边界**：只解除「转换器与文件操作模块的**执行形态**」；不解除产品语义、不解除用户可见行为、不解除 Foray 已立的安全边界（Zip Slip / Bomb / 符号链接 / 深度 / 体积上限这套校验规则继续有效，见 2.2）。

### 0.3 已裁决

**第一轮（2026-10-03）**

| 编号 | 问题 | 裁决 |
|---|---|---|
| D2 | 内核归属 | 新建 `src-tauri/src/arom/`；**第二轮修订为「A-ROM 全新设计，Foray `Rom` 类型退役，Foray 改消费者」** |
| D3 | 内存与 I/O | 惰性 + 受管后端：源包句柄常开按需读取，变更驻内存、超预算溢出，输出流式写 |
| D5 | 迁移方式 | `PathView` 适配层 + 分批迁移：M1–M2 允许双视图，M3 强制删除 |
| — | 保真红线 | **硬红线**：产物与迁移前逐字节一致；未触碰条目字节保留，仅被写入的对象重编码 |

**第二轮（内核形状）**

| 编号 | 问题 | 裁决 |
|---|---|---|
| D10 | 变更表示 | **CoW 层 + 路径覆盖映射**：源包是不可变 base 层，任务/阶段写自己的层，提交时合并 |
| D11 | 对象 API | **条目 + 按需类型化视图**：存储只管条目与字节，`mcmeta()` / `image(path)` / `model()` 按需解析、缓存、写入即失效 |
| D12 | 任务契约 | **任务声明读写范围**（路径前缀/通配），引擎据此并行、定序与冲突检测 |
| D13 | 版本与结构 | **进核心**：多根、overlays 分层、版本折叠目录、`pack_format(min/max)`、目标版本维度是一等结构 |

### 0.4 交付判据

- **G1 单一模型**：一次转换全程只有一个受管对象模型；临时目录不再是语义载体，最多是溢出后端。
- **G2 类型化契约**：转换器不再接收 `temp_dir`/`&Path`；任务间不再用字符串字典传隐含语义。
- **G3 保真可证**：未触碰条目字节保留；产物与迁移前逐字节一致 —— 2.8 指出的闸门缺口必须先补。
- **G4 可观测**：每个条目的来源、被哪个任务改过、改成什么，由层结构直接给出，无需额外埋点。
- **G5 单一内核**：Foray 与引擎共用同一模型，Foray 只持有投影与编辑能力，不再维护第二棵树。

## [S1] Problem

### 1.1 现状：中间形态自始至终是磁盘路径

```
输入 zip
  → 建临时目录 .2pyr-work-*            version_converter.rs:637 / zip.rs:114
  → 解压落盘（并行写盘）                zip.rs:322 extract_zip_to_dir（写盘 :365，大文件 io::copy :442）
  → 预检全部读盘                       version_converter.rs:661–686（结构分析 :686 → pack_analysis.rs:197）
  → 建上下文（只有路径 + 两个字符串）    invoke_conversion.rs:604（set_data :609/:611）
  → 注册 88 个任务闭包                 invoke_conversion.rs:140–593
  → 分层执行，任务各自读写临时树         scheduler.rs:257 / :296 execute_tasks
  → 延迟清理清单                        invoke_conversion.rs:632 → context.rs:96
  → 重新打包                           version_converter.rs:756 → zip.rs:493（WalkDir :507）
```

矛盾集中在两个结构体里：

```rust
// hurray/context.rs:62
pub struct HurrayContext {
    temp_dir: PathBuf,
    shared_data: RwLock<HashMap<String, String>>,            // 任务间唯一通信渠道，字符串键值
    texture_cache: RwLock<HashMap<PathBuf, Arc<RgbaImage>>>, // 缓存键是磁盘路径
    cleanup: RwLock<CleanupList>,                            // 删除靠延迟清单，顺序敏感
}

// hurray/scheduler.rs:28 —— 没有 converter trait，是裸闭包
type TaskFn = Arc<dyn Fn(&HurrayContext) -> Result<(), String> + Send + Sync>;
```

转换器的三种输入形态（全部直接从磁盘取数）：`&HurrayContext` 48 处（内部只取 `temp_dir()`）、`&Path` 89 个函数（converters 内 `: &Path` 参数 189 处）、少数纯像素函数（`color/*`、`image_utils.rs`、`scale_factor.rs`）。典型：`bedrock/j2b.rs:18 convert_java_to_bedrock(temp_dir: &Path, pack_name)`、`audio/sounds.rs:6 convert_sound_files(temp_dir: &str)`、`ui/survival.rs:30 image::open → :121 cache_texture → :124 img.save`。

### 1.2 实测规模

| 指标 | 数值 |
|---|---|
| `src-tauri/src` | 144 个 `.rs` / 30,704 行 |
| `converters/` | 108 文件 / 18,428 行（ui 20/4,345 · bedrock 11/3,971 · textures 26/3,301 · reverse 34/1,592 · shaders 2/1,261 · color 3/219 · audio 2/167 · 根 10/3,572） |
| 运行时任务 | **92 个**：`invoke_conversion.rs` 88（Eraser 39 / Architect 21 / Surgeon 28）+ `bedrock/mod.rs:40` 2 + `shaders/java.rs:989` 1 + `textures/alpha_layers.rs:191` 1 |
| `temp_dir()` 调用 | **143 处**（全仓） |
| converters 内 `fs::*` 变更类调用 | **510 处**（全仓 `fs::write` 187 / `create_dir_all` 181 / `copy` 89 / `rename` 35） |
| 图片编解码 | `image::open` 120（converters 内 117）· `.save()` 113（converters 内 110） |
| JSON 读写 | `fs::read_to_string` 80（converters 内 42）· `serde_json::from_str` 49 |
| 测试 | `#[test]` 215 / `#[cfg(test)]` 63 处 / 无集成测试目录 / **无 CI**（`.github/workflows/` 仅 feishu-notify.yml） |

### 1.3 为什么「再加一层」解决不了

1. **语义无处表达** — 任务之间只能传 `String → String`（context.rs:107–115）。「某文件被谁改过、改成了什么、是否应当原样保留」没有类型化表示。
2. **磁盘是唯一真源** — 每个任务自己拼路径读写；贴图是否已解码要靠 `is_texture_cached(path)`（context.rs:127）人工挡。全仓唯一接近对象模型的是 `pack_diff.rs:95 read_container`（→ `BTreeMap<路径, 字节>`）与 `ui/gui_surgeon.rs:262` 的 `TexturePool` 直调（invoke_conversion.rs:616–624，甚至不在任务注册表内）。
3. **正确性依赖顺序自觉** — 删除是延迟清单（context.rs:14–59、82–101）；转换链**完全没有**备份/原子替换（全仓只有 `foray/export.rs:134–149`），因为现状改的是临时副本 —— 一旦源包参与就地写，这条保障必须补齐。
4. **可验证性只在端到端**，而端到端闸门本身有证明力缺口（见 2.8）。
5. **样板重复** — 拼路径 → 读 → 改 → 写 → 登记清理 → 想缓存失效，154 个注册点各抄一遍。
6. **两棵树 + 两套 mcmeta** — 检索 `Rom|RomFile|RomDir|SafeArchive|SafeEntry|PackMeta` 在 `converters/`、`hurray/` 命中 **0**；mcmeta 也是两套独立实现（`version_converter.rs:294/:341` vs `foray/mcmeta.rs:82`）；结构知识还散在第三处（`pack_analysis.rs`）。
7. **死代码占位** — **54 个** `pub fn register_task(engine: &mut HurrayEngine)` 与从未被实例化的 `hurray/engine.rs`（89 行，全仓 0 引用）。

## [S2] Design

### 2.1 设计驱动（来自引擎负载，不是来自 UI）

| 驱动 | 证据 / 约束 |
|---|---|
| 92 个任务、三级分层与并行/独占混合 | scheduler.rs:257 / :296；`TaskType`/`TaskTier` |
| 体量上限 50k 条目 / 1 GiB（现实样本 3.6k 文件 / 18 MB） | foray/zip_safe.rs:20–31 |
| 确定性：同输入必须同字节 | 否则 2.8 的闸门无意义 |
| 保真：未触碰条目字节保留 | 现状靠「不写就不变」的惯例，无结构保证 |
| 双消费者 + CLI | 引擎（并行、写重）· Foray（单线程、读多、要 UI 投影）· CLI（只读分析/diff）· overlay 工作区（目录形态） |
| 结构复杂 | 多根、overlays、版本折叠目录、Java↔Bedrock 整树重排（bedrock/fsutil.rs:6 merge_dir / :68 rename_stems_in_dir） |
| 目录级改名是常态 | textures/rename_blocks.rs:67、bedrock 整树搬迁 —— 逐条目物化会爆 |
| 包外资源 | `converters/mod.rs:38 get_uimage_path()`、ui/survival.rs:100–106 从磁盘读模板 |
| 可观测 | 逐任务计时（`TaskTiming`）、转换报告、CLI 报告 JSON（cli.rs:21–86） |

### 2.2 为什么 Foray ROM 不能作为种子

它是为「单包 · 单线程 · 读多写少 · UI 展示」设计的，形态与引擎需求正面冲突：

| 冲突 | 证据 |
|---|---|
| **生命周期错**：读完即关闭 zip 句柄，导出时必须重新打开源包逐条 `raw_copy` | zip_safe.rs:87–102、export.rs:58–65 —— 这个「对象模型」无法独立成立，始终挂着磁盘 |
| **内存形态错**：每个条目全量常驻 `Vec<u8>` 且**急切计算 SHA-256**；`SafeArchive{entries: Vec<SafeEntry>}` 无索引 | zip_safe.rs:34–46 —— 1 GiB 包不可能，且为用不到的哈希付费 |
| **真源形态错**：嵌套 `RomDir{dirs,files}` + 节点重复存 `path` 字符串 + `find_file` 全树递归 O(n) | rom.rs:47–52、114–140 —— 无索引、无稳定 ID；92 个任务各查几次就是平方级 |
| **结构表达错**：overlays 只有 `has_overlays: bool`；空目录条目直接丢弃；不携带 unix mode | mcmeta.rs:30、zip_safe.rs:125–127、probe.rs:130 —— 转换链要保真的东西恰好都表示不了 |
| **变更模型错**：`dirty: bool` + 导出时比对，无层、无事务、无来源追溯 | rom.rs:41、export.rs:32 |
| **消费假设错**：`Rom` 连 `Serialize` 都没有，前端拿的是临时投影 | commands/foray.rs:60 等 |
| **质量信号**：`classify()` 有一个两支返回相同的冗余分支 | rom.rs:151–158 —— 从未被当引擎内核压过 |

**只借鉴三样东西**：① `zip_safe` 的**校验规则**（Zip Slip / 重复路径 / bomb / 深度 / 体积上限，zip_safe.rs:49–169）—— 安全知识而非数据结构；② 原子替换范式（`.bak` + 临时文件 + 替换/回滚，export.rs:134–149）；③ 「未改动条目 raw_copy」的纪律（export.rs:78）。

**Foray 的归宿**：`Rom`/`RomFile`/`RomDir` 类型**退役**；Foray 改为 A-ROM 的消费者，`RomDir`/`RomFile` 的 JSON 由投影生成，**前端契约不变**。

### 2.3 内核分层

```
L4 Facade     引擎（&PackView + Tx）  ·  Foray 编辑投影  ·  CLI 只读
L3 Structure  多根 · overlays 分层 · 版本折叠目录 · pack_format(min/max) · 目标版本维度
L2 Views      按需类型化视图：mcmeta() · image(path) · model() · lang() · manifest()
L1 Store      arena<EntryId> + 路径驻留 + path→id 索引 + ContentRef + Layer/Tx
L0 Sources    ZipSource（句柄常开，可取原始压缩字节） · BlobSource（内存/溢出） · DirSource（overlay 工作区）
```

**L0 Sources** — `trait Source { fn read(&self, idx) -> Bytes; fn read_raw(&self, idx) -> RawBytes }`。`ZipSource` 持有 `ZipArchive` 句柄直到包生命周期结束（解决 export.rs:58 的「重开源包」），`read_raw` 走 `by_index_raw` 取得原始压缩字节 → 未改条目可以**零解压零重压**透传。

**L1 Store** — 条目放 arena，路径字符串驻留一次，索引 `PathId → EntryId`：

```rust
struct Store { entries: Slab<Entry>, paths: PathInterner, index: BTreeMap<PathId, EntryId> }
struct Entry  { parent: Option<EntryId>, name: PathId, len: u64, origin: Origin, content: ContentRef }
enum ContentRef { Base { entry_idx: u32 }, Blob(BlobId), Ref(EntryId), Tombstone }
```

- 字节**惰性**：`ContentRef::Base` 直接指向源 zip 条目，不驻留；`Blob` 是任务生成/改写后的内容（内存或溢出文件）。
- `Ref` 让复制与移动**不复制字节**（现状是 `fs::copy`：全仓 89 处）。
- 哈希**按需**计算并缓存（报告/闸门要时才做），不再是构建期成本。
- 索引用 `BTreeMap` 而非 `HashMap` → **迭代顺序确定**，这是产物确定性的前提。

**L1 Layers / Tx** — 一层 = 路径覆盖映射 + 前缀重写规则：

```rust
struct Layer { writes: BTreeMap<PathId, ContentRef>, renames: Vec<PrefixRule> }
```

- base 层不可变 = 源包；每个任务/阶段写自己的层；`commit` 按层序合并，`abort` 丢弃整层 → 回滚与并行隔离变成结构性的，「谁改了什么」由层直接给出（G4 不需要埋点）。
- **目录级改名用前缀规则**（rename_blocks.rs:67、bedrock 整树搬迁）：解析时惰性生效，不物化 3k 个条目。
- 冲突检测：同路径被两个并行层写入 → `commit` 时报冲突（策略可配：拒绝 / 后者胜 + 告警）。
- **`dirty` 概念消失**：路径不出现在任何非 base 层，就是「未触碰」，字节透传是默认行为（G3 从惯例变成结构）。

**L2 Views** — 类型化视图按需解析、缓存、写入即失效：`pack.mcmeta`（取代 `version_converter.rs:294/:341` 与 `foray/mcmeta.rs:82` 两套）、贴图（`RgbaImage`，取代路径键缓存 context.rs:66 与 `TexturePool` texture.rs:22）、模型/blockstate、lang、manifest、sounds.json。未识别的条目退化为原始字节视图，**不付解析成本**。

**L3 Structure / 版本** — 多根、`overlays.entries` 分层（把 `pack_analysis.rs` 的只读识别升级为一等结构）、版本折叠目录、`pack_format`/`min_format`/`max_format` 规则（`version_converter.rs:363–390` 的语义收进核心），转换目标作为模型的一维（`Pack { source_format, target_format }`）。

**L4 Facade** — 三个消费者同一内核：引擎走 `&PackView` + `Tx`；Foray 走可变投影（单线程）；CLI 走只读（`--analyze` / `--pack-diff` 直接吃 A-ROM，不再各自实现读容器）。

### 2.4 任务契约与调度（D12）

```rust
struct TaskDecl { name: &'static str, tier: TaskTier, reads: ScopeSet, writes: ScopeSet }
type TaskFn = Arc<dyn Fn(&PackView, &mut Tx) -> Result<TaskOutcome, TaskError> + Send + Sync>;
```

- 引擎据 `reads/writes` 建冲突图：**写集不相交的任务才并行**；独占任务（现状 39 个 Eraser 型）作为屏障。这取代「靠 tier 表 + 注册顺序」的隐式约定。
- **调试模式断言**：写入未声明的路径即报错 —— 把「忘记登记」从静默错误变成可测失败。
- 保留现状骨架：三级分层、`calculate_path` 拓扑、`ProgressTracker` 进度、`TaskTiming` 计时（scheduler.rs）—— 只换任务签名与并行判据。
- `PathView` 适配器（D5）：未迁移任务继续吃 `&Path`，但**其读写视为对该前缀的整段声明**，因此适配器不破坏冲突检测；M3 删除。

### 2.5 并发与确定性

- 视图不可变共享（`&PackView`），写只经 `Tx` → 不需要用锁包住整个模型（现状 `HurrayContext` 内四把锁）。
- 每任务独立视图缓存（或共享只读缓存 + 层局部缓存），避免为并行安全给贴图缓存加全局锁。
- 确定性来源（写进文档并作为回归项）：条目迭代顺序 = 源 zip 条目顺序；层合并顺序 = 阶段与任务注册顺序；blob 命名与溢出位置不影响产物字节。
- 批处理并发 6 包 → 每包一个源句柄 + 一份预算配额（现状 `concurrent_packs` 只限包数，不做内存配额：commands/conversion.rs:36）。

### 2.6 内存与 I/O（D3 细化，M0 必须定参）

| 参数 | 建议默认 | 说明 |
|---|---|---|
| 内存预算 | 512 MiB / 包，可配 | 超预算 blob 溢写到受管后端 |
| 溢出位置 | `.2pyr-work-<id>/spill/` | 复用 `WORK_DIR_PREFIX`（zip.rs:114）与三档清理（zip.rs:159–169）、启动清扫 >2h（zip.rs:120–150） |
| 与性能档位的关系 | 预算并入 `PerfPlan` | 现状 `MEMORY_GUARD_MB = 1500`（perf.rs:105）只做粗降档 |
| 大条目策略 | >8 MiB 或不可解析类型默认不进内存 | 音频、大 PNG |
| 基准 | `TapL 16x.zip`（3631 文件 / 18.41 MB）+ 逼近 1 GiB 的合成包 | 测内存峰值、耗时、溢出次数 |

### 2.7 源包只读与写回

源包全程只读；输出走「临时文件 + 原子替换 + 失败回滚」（export.rs:134–149 的范式上移到内核）；Foray 的「原地覆盖」与转换的「另存」共用同一条序列化路径。

### 2.8 保真与闸门证明力边界（重要）

**保真现在是结构性的**：未出现在非 base 层的条目，其字节从源 zip 直接透传、不经过任何转换。**但「原始压缩字节透传」只对 `Stored` 条目成立**——`Deflated` 条目若原样透传，方法与压缩字节都会与现状产出不同（现状会解压后用本仓默认级别重压）。Step 0 的容器级闸门当场抓出了这一点，策略修正见 [astray-arom-model.md](./astray-arom-model.md) §1.1（D23）：**仅在「源方法 == 白名单方法 == `Stored`」时透传，其余解压后按现状设置重压**。

**但「保真」的判据必须先钉死现状契约**（实测；细则见 [astray-arom-model.md](./astray-arom-model.md) §1）：

| # | 现状输出契约 | 证据 |
|---|---|---|
| 1 | 条目集合 = 文件 ∪ **磁盘上的目录条目**（空目录会保留，且**文件隐含的父目录也会成为目录条目**——解压在磁盘上建出它们，重新打包时写进包） | zip.rs:411–413、541–545；测试断言 zip.rs:707；**Step 4 验收用例实测确认**（首版漏了隐含父目录，闸门当场报红） |
| 2 | 文件内容 = 磁盘字节（未触碰者 = 源字节） | PNG 等走 `Stored`，zip.rs:568–581 |
| 3 | 压缩方法 = 扩展名白名单（`Stored` / `Deflated`） | zip.rs:568–581 |
| 4 | 时间戳 = `FileOptions::default()`（固定默认值，**不是源时间**） | zip.rs:518 |
| 5 | **unix mode 不保留**：`unix_mode()` 只在 `#[cfg(unix)]` 下用于拒绝符号链接，Windows 下整段编译掉，全仓无 `set_permissions` | zip.rs:399–406 |
| 6 | 条目顺序 = `WalkDir` **未排序**的枚举顺序（无契约保证） | zip.rs:507–510 |

由此两条纪律：**①「模型保留信息」≠「输出策略」** —— A-ROM 必须能携带 unix mode / 时间戳 / 原始压缩字节，但转换输出策略要与现状一致（不写 mode、写默认时间戳、按扩展名选方法），否则红线当场变红；Foray 导出路径可另选策略。**② 顺序不是现状承诺** —— 容器级「逐字节一致」要可判定，必须先定义 A-ROM 的确定顺序（建议目录序 + 路径排序，D20），并把与现状顺序的差异在闸门里作为**信息项**而非失败项。

**闸门证明不了这件事**：`pack_diff` 把 zip/目录读成 `BTreeMap<相对路径, 内容字节>`（pack_diff.rs:95–129），据此分级 `identical` / `encoding-only` / `json-equivalent` / `CONTENT-DIFF` / `only-in-A|B`，而它**不比对**：

- **目录条目**（pack_diff.rs:118–119 直接跳过）→ 空目录条目的增删检不出；
- **zip 容器属性**：条目顺序、压缩方法、外部属性（unix mode）、时间戳、注释；
- 其内存占用与包体成正比（`read_container` 全量载入）。

所以「`--strict` 通过」= 「两边**文件内容**一致」，**不等于**「产物 zip 字节一致」；而 2.2 列出的两个 Foray 缺口（丢空目录 zip_safe.rs:125–127、无 unix mode probe.rs:130）恰好落在这个盲区里。**结论：红线要成立，闸门必须先升级为两层** —— (a) 现有内容级比对；(b) 新增 **zip 容器级比对**（条目顺序 / 压缩方法 / 外部属性 / 空目录 / 时间戳策略）。这是 **M-1** 的交付内容，也是整个重构的判据基础。

### 2.9 分期与双轨

| 里程碑 | 内容 | 闸门（acceptance） |
|---|---|---|
| **M-1 闸门先行** | 升级 `pack_diff`（或新增 zip 级对照）覆盖容器属性与空目录；合并 `zip.rs:9–11` 与 `SafeLimits` 两套限额 | 用「空目录 / unix mode / 压缩方法」三种人为扰动验证：升级后的闸门必须报红 |
| **M0 内核** | L0–L3：Source/Store/Layer/Tx/Views/Structure + 惰性后端 + 序列化；**不改任何转换器** | 「读 → 原样写」等价性：样本包过升级后的闸门；单测覆盖 arena/索引/层合并/前缀重写/预算溢出/确定性顺序 |
| **M1 契约与试点** | `TaskDecl` + 冲突感知调度 + `PathView`；删除 54 个死注册与 `engine.rs`；迁 2–3 个低风险转换器 | 试点双轨对照（旧路径 vs A-ROM）产物一致；未声明写入断言可触发；并行度不劣化 |
| **M2 分批迁移** | textures(26) → ui(20) → bedrock(11) → reverse(34) → shaders / audio / color | 每批跑全量闸门；每批可独立回退；`cargo test --lib` 全绿 |
| **M3 收口** | 删除 `PathView`、`temp_dir` 主路径、`shared_data`、`PathBuf` 缓存键；Foray `Rom` 类型退役、切投影 | Foray 前端契约不变；删除代码净减少可核查；全量闸门 + 基准不劣化 |

### 2.10 影响面

**内部（必然破坏）**：`converters/**` 全部任务体与签名 · `hurray/context.rs` / `texture.rs` / `scheduler.rs`（签名与并行判据）· `invoke_conversion.rs:111–593` · `converters/version_converter.rs`（解压/打包/mcmeta 写盘）· `converters/zip.rs:322/:493` · `bedrock/fsutil.rs` · `foray/rom.rs`(退役) / `probe.rs` / `paint.rs` / `export.rs` / `ai.rs` / `commands/foray.rs`（改消费者）· 入口层（`commands/conversion.rs`、`cli.rs`、`lib.rs:109–116`）。

**可保留（几乎不动）**：调度骨架（三级 tier / 拓扑 / 进度 / 计时）· `error.rs` / `logger.rs` / `perf.rs` · 纯像素层 `color/*`、`image_utils.rs`、`scale_factor.rs` · `overlay/*`（二期再对象化）· 其余 `commands/*`。

**对外承诺清单（默认不破，逐条核对）**

| 面 | 证据 |
|---|---|
| 产物 zip 内容与命名 | 命名模板 version_converter.rs:413–425 · 冲突永远 ` (n)` 不覆盖 :492–500 · target 1000 → `.mcpack` :471 · mcmeta 写法 :363–390 |
| 前端断点 | `ConversionPage.vue:512–513` 直读 `result.status` / `result.output`；历史写入 commands/conversion.rs:345–352 → 必须继续提供「对象 → 路径」映射（投影层职责） |
| Tauri 命令 | 87 个 `#[tauri::command]`，84 个注册于 lib.rs:261；含路径参数者约 22 个 |
| CLI | main.rs:148–169（`--pack-diff` / `--convert` / `--analyze`）+ 报告结构 cli.rs:21–86（`input`/`output`/`source` 是路径字符串）；`tools/convert-report.ps1` 依赖「`--convert` 是否被识别」 |
| 用户数据 | `~/.2pyr/configs/settings.json`（commands/config.rs:87–90，22 个键）· `history.json`（commands/history.rs:26）· 日志 logger.rs:36–48 |
| 配置迁移范式（照抄） | `conversion_threads` → `performance_mode`：读取时惰性迁移、不写回（perf.rs:111–118，调用点 commands/conversion.rs:27–33，测试 perf.rs:149–199） |
| 临时目录 | `.2pyr-work-`（zip.rs:114）· 启动清扫 >2h（zip.rs:120–150）· 三档清理（zip.rs:159–169）· `2PYR_SYNC_CLEANUP=1`（:184–187） |

**需显式裁决才允许破的对外面（D4）**：临时目录位置与生命周期、转换过程中间产物布局、内部错误文案。

### 2.11 回滚、灰度与发布

- **灰度（D6）**：编译期开关 + 每里程碑可独立回退；不做运行期双内核（状态互相污染）。
- **发布注意**：CHANGELOG 需单列 **Breaking** 段；版本改动**必须**走 `tools/set_version.py`（10 处 + `--check`，发版守门 build_release.py:408）—— 仓库根的 `code/set_version.py` 是**过期脚本**（只覆盖 4 处、不含 installer-app，且被 `.gitignore:23` 忽略），误用会复现 2.7.0 的漏改事故。不可改身份：MSIX `identity_name`/`publisher`、MSI `UpgradeCode`、两个 Tauri `identifier`。

## [S3] Out of Scope

- 不改**用户可见**的转换语义与产物内容。
- 不改转换器的**转换规则**（版本映射表、命名规则、贴图生成算法）——只改它们的执行形态。
- 不新增版本支持；不追求性能提升（性能只作回归门槛）。
- Foray 的编辑/AI 功能不再扩展（仅做消费者适配）。
- Android / TUI / Enzyme / Ifaso 互通 / 插件市场。
- 不做「双内核长期并存」。

## 待裁决（Open Decisions）

| 编号 | 问题 | 建议 |
|---|---|---|
| **D1** | 版本号：`2.8.0` 还是 `3.0.0`？ | **发布前再定**（2026-10-04 裁决）。已知约束：major 升位会触发强制更新（updater.rs:241–249），走 `3.0.0` 等于把纯内部重构变成全员强制升级 |
| **D4** | 是否允许破「次要对外面」（临时目录、中间产物、错误文案） | 尽量不破；确需破则逐条列出并写进 CHANGELOG Breaking 段 |
| **D6** | 开关粒度与灰度 | **编译期开关、按模块**（与 M2 的迁移批次一致，每批可单独回退） |
| **D7** | 文档粒度 | 已按「总纲 + 子设计」执行：总纲（本文）+ `astray-arom-model.md`（L0–L4 接口与阶梯）；`astray-migration.md`（分批清单）待 M2 |
| **D18–D24** | 接口级裁决：结构层范围 / 并行层冲突策略 / 输出条目顺序 / `ZipSource` 并发读 / 测试夹具 / 透传策略 / 限额取值 | 见 [astray-arom-model.md](./astray-arom-model.md) §10；**D24 已裁决为「取宽松者 + 新限制默认关闭」** |
| **M2 驱动** | 迁移怎么推进 | **逐任务/逐批替换 + 适配层兜底**：未迁移任务经 `PathView` 继续以路径形态工作，其读写计为整段范围声明（2026-10-04 裁决） |
| **D8** | **文档放哪**：`docs/` 被 `.gitignore:39` 整目录忽略 | **保持仅本机**，经飞书开发群分发，不入 GitHub（2026-10-04 裁决；`.gitignore` 规则不动） |
| **D9** | 是否先做 M-1 闸门升级再动内核 | **是**（见 2.8） |
| **D14** | 层粒度：任务级层 vs 阶段级层 | 阶段级层 + 任务级写入集：层数可控，冲突检测仍精确 |
| **D15** | `PathView` 与 A-ROM 同时写入时的可见性规则 | 适配器任务只见挂载点快照 + 自己的写入；跨适配器可见性由 commit 保证 |
| **D16** | 内存预算默认值与是否并入 `PerfPlan` | 512 MiB/包；并入 `PerfPlan` |
| **D17** | Foray 迁移方式：`Rom` 一次性退役 vs 适配层过渡 | M3 一次性退役（它没有仓外消费者，只有本仓命令层） |

## 未确认（不当作事实）

- 无调用点的模块（`legacy_processor` / `legacy_eraser` / `blockstate_adapter` / `main_converter` / `audio/sounds` / `anims_folder` / `old_paths`）是否为将来版本预留。
- GUI 前端实际调用 `convert_zip` / `convert_resource_pack` / `convert_resource_packs_batch` 中的哪一个。
- `docs/` 被整目录忽略是有意还是遗漏。
- `--nogui` 的 `--setting` / `--exit` 是否有外部使用者。

## Tasks（骨架，待 D1/D4/D6–D9/D14–D17 裁决后展开）

- [x] T1（M-1）闸门升级：pack_diff 容器级比对 — acceptance ✅：空目录 / unix mode / 压缩方法三种扰动必报红（`cargo test --lib` 221 passed / 0 failed，新增 6 例）。**限额合并延后到 Step 1**（见 [astray-arom-model.md](./astray-arom-model.md) §9.1）
- [x] T2（M0）L0 Sources ✅：`arom/` 模块（`mod` / `error` / `limits` / `source`）+ `ZipSource`（句柄池 + `read` / `copy_to` / `copy_raw_to`）+ `SafeLimits` 单一来源（双预设）+ 最小 `MemSource`；全量 `cargo test --lib` **231 passed / 1 ignored**。**DirSource 与 BlobSource 的溢出后端挪到 Step 2/4**（Step 1 只立接口与最小实现）；`converters/zip.rs:9–11` 与 `foray/zip_safe.rs` 未动，故现状行为零变化
- [x] T3（M0）L1a Store ✅：`arom/store.rs` —— arena + `PathInterner` + `BTreeMap` 索引 + 目录一等 + 合成祖先目录；7 例（含用计数来源证明**构建期零字节读取**）。`ContentRef` 的实际形态是 `Body::{Base,Blob,Alias,Dir}` + `Slot::Presence/Tombstone`，在 T4 落地
- [x] T4（M0）L1b Layer/Tx ✅：`arom/layer.rs` —— `BlobStore`（预算）/ `Layer`（前缀改名 + 写入）/ `Tx` / `PackView` / `commit_batch` 冲突检测；11 例。抓到三条语义坑：Move 必须隐藏源路径、Copy 必须参与反向映射、层键用字符串而非 `PathId`（理由见细则 §9.3）
- [x] T5（M0）L2 Views ✅：`arom/view.rs` —— `PackMeta` + `mcmeta()` / `image()` / `text()` / `json()` + 版本号驱动的缓存失效；9 例（含 26.x 数组/对象写法、`Arc::ptr_eq` 命中、提交即失效、失败降级）
- [x] T7（M0）序列化与保真 ✅：`arom/serialize.rs` —— 顺序策略（目录优先 + 路径排序，D20）/ 方法白名单 / **D23 透传策略** / 合成目录补齐 / 原子替换；7 例。**M0 验收达成**：与旧管线真实产出对照 `--strict` 通过（`containerEntrySetBlocking = 0`、`containerByteOnly = 0`）
- [x] T6（M0）L3 Structure ✅：`arom/structure.rs` —— `PackStructure::analyze(&PackView)`：多根（Java/Bedrock）、overlays 只读（含 `overrides` 计数）、折叠目录、`supported_formats`、版本写规则（复用 L2 规则不另写判断）；13 例，含**两个与 `pack_analysis.rs` 的交叉对照**与脏包容错
- [x] **M0（Step 0–6）全部完成** ✅：`cargo test --lib` **278 passed / 0 failed / 2 ignored**；`cargo check --bins` 通过；`arom/` 合计 4,572 行 / 9 文件，既有转换器与解压打包主路径**一行未改**。核心验收：读 → 原样写对旧管线真实产出 `--strict` 通过（合成夹具 + 真实基准包 `TapL 16x.zip` 双重复验，见细则 §9.3）
- [x] T8a（M1）任务契约与冲突感知调度 ✅：`arom/task.rs` —— `TaskDecl` / `ScopeSet`（精确 / 前缀 / 整包）/ `Tier` / `plan()` 波次划分 / `conflict_reason` / `check_write`；11 例，含「未迁移任务声明整包 → 强制串行」
- [x] T8b（M1）删除死注册路径 ✅：**56 个**从未被实例化的注册函数（54 个 `register_task(&mut HurrayEngine)` + `legacy_eraser::register` + `cut_gui::register_task_with_deps`）+ `hurray/engine.rs`，并清理 23 个文件因此失效的 `use`；删除后全仓 `HurrayEngine` 引用归零，回归不变。**教训**：并行委派子代理前先提交自己的改动（它用 `git checkout` 回退中间状态时连带抹掉了我未提交的 `lib.rs` 改动）
- [x] T8d（M1）`PathView` 兼容适配层 ✅：`arom/pathview.rs` —— `materialize`（落盘 + 基线哈希）/ `harvest`（产出层：增改 → Blob、删除 → Tombstone、新增目录 → `Body::Dir`）/ `run_legacy` / 越界写入诊断；4 例，含**旧任务经适配层与旧管线 `--strict` 一致**（`mcpatcher → optifine` 目录改名）。抓到两条真问题：① 目录改名类任务的**写入目标在源前缀之外**，因此新增 `decl_scopes` 支持不对称声明；② 试点 `drop_font` 首版只查目录条目，而 zip 里常无显式目录条目（目录由文件隐含）
- [x] T9（M1）试点迁移与双轨对照 ✅：`pilots/` **四个**试点覆盖子树删除 / 零字节复制 / **目录改名** / JSON 改写（对应目标要求的「删除 / 改名 / JSON 改写」三种机制），各自的 `decl()` 与实现同文件；双轨对照在夹具与**真实基准包 `TapL 16x.zip`** 上均通过（真实包 3630 文件 / 3723 条目 / `entry-set=0` / `byte-only=0` / `mtime-only=0`）
- [x] T9b（M1）闸门时间戳语义修正 ✅：真实包首跑报出 428 条 `MtimeDiff`，根因是 zip 0.6.6 的 `FileOptions::default()` 取**运行时刻**（`OffsetDateTime::now_utc()`）——两条管线都盖运行时刻，跨运行必然不同，此前 M0 能过是运气。修正为 `MtimeDiff` 单独计数、**只报告不判失败**，`strict_pass_label()` 明说「不构成逐字节一致」；细则 §9.6 F 有完整记录
- [x] T8c（M1）契约修正（实现时发现，已回写细则 §9.5）：任务签名改为 `Fn(&mut Tx)`；`Tx` 改持 `&Pack` + `Mutex<BlobStore>`，使**同波任务可并发持事务**，提交改为 `tx.into_layer()` + `Pack::commit`；顺带采纳 D14（阶段级层 + 任务级写入集）与 D19（冲突默认拒绝）
- [x] **M1（Step 7）全部完成** ✅：`cargo test --lib` **298 passed / 0 failed / 3 ignored**；`cargo check --bins` 通过；`arom/` 合计 5,538 行 / 11 文件。**仍未接线**：四个试点与适配层都没有改变生产路径，`invoke_conversion.rs` 的注册仍跑旧实现（切换属 M2，先定「编译期开关粒度」与「混合运行驱动方式」）
- [x] T10a（M2）混合运行驱动 ✅：`mixed_run.rs` —— A-ROM 接管读入与写出（`Pack` → `materialize` → 旧执行器 → `harvest` → `write_zip`），任务仍全部经适配层；**真实包对照通过**（TapL 16x，format 1 → 97：4018 文件 / 4138 条目 / `blocking=0 / entry-set=0 / byte-only=0`）。生产代码未接线，`write_pack_format` 可见性放宽为 `pub`（M2 驱动需要）
- [x] T10b（M2）任务注册共用化 ✅：`register_legacy_tasks(...)`（88 内联注册 + 3 外部注册器集中一处）+ `Scheduler::plan()`（有序选择，零逻辑改动）+ `Scheduler::run_named()`（按名字执行，未注册名字宽容）。全量 **302 passed**；真实包端到端复验数字与提取前逐项一致。**注**：任务「读写范围」仍按任务在迁移时声明（未迁移者默认整包 → 串行），不阻塞本步
- [x] T10c（M2）按模块批次迁移（先 textures）**前两批** ✅：已迁移 **6** 个任务（5 个删除类 + 目录改名），1→97 计划里 **5 原生 + 40 适配层**；夹具与真实包三种配置逐项一致（全量 303 passed）。**实测否掉了一个设计**：逐任务交错执行与生产不等价（真实包差 105 个文件，根因是 `TexturePool` 提交时机 / 延迟清理 / 阶段内并行分组的非局部耦合），改为「原生前阶段 + 旧任务一次性批量」。**并修掉一个基座保真问题**：输入里由文件隐含的祖先目录必须显式化为条目，否则「删文件」会顺带剪掉父目录（现状管线解压建目录、打包写条目）。详见细则 §9.10 / §9.11
- [x] T10c-2（M2）原生写入的声明范围检查 ✅：派发表携带 `TaskDecl`，驱动逐条校验原生层的写入路径，越界即报错（`strict_scopes`，默认开）；附带一条「必须能抓住越界」的用例，真实包复验 `undeclared: []`。**同时记录两个「注册了但从不执行」的任务**：`convert_old_texture_paths`（连注册都没有）、`convert_animated_textures`（已注册但不在任何版本映射段里）——即旧贴图路径转换与动画 mcmeta 升级今天在生产里都不发生；修复属对外行为变更，待裁决（细则 §9.12）
- [x] T10c-3（M2）`rename_blocks_items` 移植 ✅（**已派发**，§9.15：根因是 tombstone 盖住同层规则的目标，改用 move_path）：397 条重命名对（脚本抽取）+ 合并语义 + 5 类图像派生；**模型补上「先写后改名」**（`resolve` 同层回查与 `hides_path` 顺序、`entries` 正向映射），并证明 `put_image` 与旧 `img.save()` 编码一致。**但真实包上仍差 116/105 个文件，因此未进生产派发**，真实包对照按显式清单跳过（细则 §9.13）
- [x] T10c-4（M2）定位并修复真实包分歧 ✅：完整差异名单 9 条全为目标名 → `move_path`（读→写→删）；**textures 真任务全部迁移完毕**（7 个在派发中）。后续批次的前置条件（规则镜像）已在 §9.16 闭合
- [x] T10d（M2）限额接线 ✅：`converters/zip.rs` 三个常量改为从 `SafeLimits::preserving_current()` 取值，全仓只剩一个来源；全量回归与真实包混合运行均通过。**其中含一次有意的放宽**（总解压 500 MB → 1 GiB，见细则 §9.8）——按 D24 裁决执行，但需写进对外说明（CHANGELOG 的 Changed 段），不是静默改动
- [x] T10e（M2）ui 模块第一个任务 ✅：`process_chest_folder` 原生（复用旧纯函数 + `tx.image`/`put_image`），`NativeSwitches::ui` 按模块；专项双轨用例 + 真实包三种配置（**8 个原生任务**）全部一致。过程教训：worktree 是 CRLF，脚本多行替换须显式 ` `r
 `（§9.17）
- [x] T10f（M2）重测同阶段内交错 ✅（**负结果**）：逐任务收层与生产仍不等价（真实包 3146 vs 3150，差异集中在 GuiSurgeon/sprite 系列），实验开关保留但不断言等价。**Architect/Surgeon 任务不能靠交错迁移**，后续只剩「整体原生化」或「按阶段分段 + 段内单独验证」两条路，需裁决（§9.18）
- [x] T10g（M2）阶段/模块清单 ✅：活注册 88（Eraser 39 / Architect 21 / Surgeon 28 + 4 外部 Surgeon）；**forward Eraser 8 个活的全部迁移完毕**；剩下 49 个 forward 任务全属 Architect/Surgeon、全部被 §9.18 约束挡住。两条路的规模与建议见 §9.19，**待裁决**
- [x] T10h（M2）反向转换第一个任务 ✅：`reverse_process_chest_folder` 原生（复用正向 `mirror_region` + 反向 `swap_and_mirror`），`NativeSwitches::reverse` 按模块，专项双轨用例通过（§9.20）。反向侧 30 个 `reverse_*` 都是 Eraser 级 → **不受交错约束**，可在裁决前继续推进
- [x] T10i（M2）反向整包对照 ✅：把**正向产物当反向输入**（正向输出即反向所需形态），反向 97→1 三种配置对照一致（计划 43 个任务，on 派发 3 个原生）。反向侧 30 个 Eraser 任务从此有整包硬证据（§9.21）
- [x] T10j（M2）反向空操作批次 ✅：`reverse_cut_gui`/`reverse_fix_horse_ui`/`reverse_overlay_icons`/`reverse_fix_ui_sub_hand` 四个「有文档的空操作」原生并派发（读写声明为空）。**删除类已实现但暂不派发**：实测同阶段顺序约束（删除提前会让 `reverse_rename_blocks_items` 找不到源），须先迁移改名任务（§9.22）
- [x] T10k（M2）反向改名迁移 ✅：`reverse_rename_blocks_items` 原生并派发（128 对反向表脚本抽取、不做合并、不调 process_blocks），反向整包通过（8 个原生）。**新发现：删除是「延迟到最末」的时机语义**，立即删除会让更晚的改名找不到源 → 删除类须先给模型加「延迟删除」（§9.23）
- [x] T10l（M2）模型加上「延迟删除」✅：`Outcome::deferred_removals` + `defer_remove_if_present`，驱动在**清理点**统一 tombstone（复刻 `defer_remove_*` 时机）；三个反向删除类重新派发，反向整包 `rev on` 达 **11 个原生任务**、三种配置一致。至此三类全局时机（纹理池提交 / GuiSurgeon / 延迟删除）在模型里都有着落（§9.24）
- [x] T10m（M2）反向延迟删除批次 ✅：`reverse_generate_shulker_box_ui`/`reverse_fix_sign_entities`/`reverse_fix_smithing2_villager2_ui`（后者同时含**立即改名**与**延迟删除**两种时机）原生并派发；反向整包 `rev on` 达 **14 个原生任务**、三种配置一致（§9.25）。剩余反向任务多含循环，需逐个手工移植（机制已就位）
- [x] T10n（M2）反向批次 boat/tipped_arrows ✅：新增表驱动宏 `defer_list_pilot!`；`reverse_generate_boat` 含**有守卫**的立即改名（目标已存在不改，非覆盖语义）。反向整包 `rev on` 达 **16 个原生任务** / 27 适配层、三种配置一致（§9.26）
- [x] T10o（M2）五个反向删除任务 ✅：`furnace`/`potion_lingering`/`redwood_cherry_bamboo_planks`/`pale_planks`/`poplar_planks`（后者跨 22 个路径，含 `.png.mcmeta` 附属），新增宏 `defer_with_meta_pilot!`。反向 `rev on` 达 **21 原生 / 22 适配层**，超过一半任务已原生化（§9.27）
- [x] T10p（M2）反向改名 ✅：`reverse_rename_mcpatcher_to_optifine`（带守卫、不合并；复用逐文件读→写→删，不产生改名规则）。反向 `rev on` 达 **22 原生 / 21 适配层**——反向侧过半（§9.28）
- [x] T10q（M2）`crossbow`/`fish_bucket` ✅：两个纯路径表任务（各 6 张贴图）。反向 `rev on` 达 **24 原生 / 19 适配层**（§9.29）
- [x] T10r（M2）`horse_ui`/`breeze` ✅：两个路径表任务（3 条 / 36 条 + `.png.mcmeta` 附属）。反向 `rev on` 达 **26 原生 / 17 适配层**（§9.30）。教训：新增试点一律追加到文件末尾的独立模块，不要用脚本搬移既有代码块
- [x] T10s（M2）铜/下界合金族 8 个任务 ✅：新增 `reverse_defer_metal` 模块（显式路径表 + 名字表附属两种助手）。反向 `rev on` 从 26 跃到 **34 原生 / 9 适配层**（79% 原生化）（§9.31）
- [x] T10t（M2）`sign`/`machinery` ✅：延迟列表 + **无守卫**立即改名（覆盖）；`oak_sign.png` 既在删除列表又是改名目标的「怪」行为**刻意照抄**（净效果无该文件）。反向 `rev on` 达 **36 原生 / 7 适配层**（84%）（§9.32）
- [ ] T10u（M2）`armor_models`/`brewing_stand_ui`：实现已完成但**未派发**（已回退）。反向对照只差 2 个文件（`models/armor/netherite_layer_{1,2}.png`）——它是「移入目标」与「延迟删除目标」的交集，需按 §9.33 的三点聚焦排查（计划先后 / 登记时存在性判断 / 旧侧同刻磁盘状态）
- [x] T10v（M2）像素回退两任务 ✅：`brewing_stand_ui`/`ui_creative`（坐标与缩放表逐行照抄，复用 `image_utils::paste_region`）。反向 `rev on` 达 **38 原生 / 5 适配层**（88%），只剩 4 个未迁移（§9.37）
- [x] T10w（M2）合成两任务 ✅：`clock_compass`（逐帧并回 + 精确 mcmeta + 延迟删帧）与 `particles`（瓦片重建图集 + 延迟删瓦片）——「写入 + 延迟删除」组合的第二次实战检验，均一次通过。反向 `rev on` 达 **40 原生 / 3 适配层**（93%）（§9.38）
- [x] T10x（M2）`ui_survival` ✅ + **反向侧覆盖率核查**：`reverse_*` 共 40 个，只剩 1 个未覆盖（`reverse_fix_armor_models`，待 §9.36 定案）；`rev on` 达 **41 原生 / 2 适配层**（95%），另一个适配层是 Surgeon 阶段的 `adapt_java_shaders`（受 §9.18 约束）。**反向侧事实上已收尾**（§9.39）
- [x] T10y（M2）**方向裁决有数据了** ✅：实测「按阶段分段」**不可行**——仅把旧批次拆成 4 段（零原生任务）就与经典管线不一致（胸口贴图 6244 像素差），根因是「一次提交 + 一次清理」的整体性；耗时也从 ~80s 涨到 287s。**可行路径只剩「整阶段/整模块原生化」**（§9.42）
- [x] T10z（M2）**按阶段放置（不拆旧批次）** ✅：前阶段=Eraser 级原生、旧批次一次调用、后阶段=非 Eraser 级原生（§9.43，实测通过）。`reverse_fix_armor_models` 派发成功，反向 `rev on` 达 **42 原生 / 1 适配层**；**§9.41 的 9 个放置有误的试点也自动归位**。方向问题降级为「批量大小」
- [x] T10aa（M2）Architect 阶段盘点 ✅（§9.44）：21 个任务**全部有已迁移的反向孪生**，但前向是**真生成逻辑**（画贴图/合成），不能表驱动 → 需逐个移植（10–72 行/个，合计约 500 行），按 3–5 个一批推进；放置机制已就位
- [x] T10ab（M2）Architect 第一个任务 ✅：`generate_furnace`（纯拷贝）原生并派发；**首次在真实包上验证非 Eraser 原生任务的放置**（正向 `on` = 8 原生 / 37 适配层，三种配置一致）。同时修掉 §9.43 的计数瑕疵（它真的挡住了 `native_tasks + legacy_tasks == plan_len` 断言）（§9.45）。Architect 余下 20 个、约 480 行，逐个移植
- [x] T10ac（M2）Architect 第二个 ✅：`generate_boat`（5 个色相变体复用旧函数 `adjust_hue_brightness` + **把 `boat.png` 改名为 `spruce_boat.png`**——这一步易漏，产物会条目集合不等）。正向 `on` 达 9 原生 / 36 适配层；Architect 进度 2/21。下一批取舍已探明：`potion_lingering`/`shulker_box_ui` 可做；`crossbow`/`tipped_arrows`/`snow_bucket` 依赖外部 UImage 且缺资源时跳过（§9.46）
- [x] T10ad（M2）Architect 第三个 ✅：`generate_potion_lingering`（三种尺寸分支：方形/纵向条带/其它跳过 + 拷贝 mcmeta）。正向 `on` 达 **10 原生 / 35 适配层**，Architect 3/21（§9.47）。教训重申：同名模块要用唯一名并追加到文件末尾（锚点在多模块文件里会重复命中）
- [ ] T10ae（M2）`shulker_box_ui`：实现完成但**未派发**（已回退）。失败原因是**放置**：Architect 原生放后阶段**太晚**（输入 `generic_54.png` 已被更高阶段的旧任务消费，dded 掉 32）。**修正 §9.43 的过头结论**：非 Eraser 原生放后阶段仅在「旧批次中无更高阶段任务」时等价（§9.48）
- [x] T10af（M2）**放置规则精确化** ✅（§9.49）：原生阶段 < 旧批次最小阶段 → 前阶段；否则后阶段。实测正向 10 原生、反向 42 原生均与 legacy 一致。旧批次仍不可拆分，**同阶段混合**才需要整阶段迁移（限制被缩小到真正需要的那一类）
- [ ] T10ag（M2）**中间阶段零散迁移不可行**（§9.50）：`boat` 需后阶段（输入被更早旧任务改动）、`shulker_box` 需前阶段（输入被更晚旧任务消费）——**两个任务要求相反的放置侧**。结论：**Architect 必须整批迁移**（剩余 18 个全部原生化），迁移的最小单位是**阶段**
- [x] T10ah（M2）§9.50 推论**修正** ✅（§9.51）：整阶段 Architect **也不足以**救 `shulker_box`（消费它输入的仍是 Surgeon **旧**任务，需要的是「Eraser 旧之后、Surgeon 旧之前」的中间位置）→ 充分条件是**依赖链上的阶段全部原生化**。**排产规则**：不触碰跨阶段时序的任务边迁边验；依赖跨阶段时序的先实现、**暂不派发**，等依赖阶段就绪后一起验收
- [x] T10ai（M2）Architect 第四、五个 ✅（§9.52）：`generate_fish_bucket`（水桶→6 鱼桶，逐桶独立判断 `UImage` 覆盖图）与 `generate_smithing_ui`（`anvil.png`→`smithing.png`，含「存在才叠加」的覆盖图与静态透明框）原生并派发；正向 `on` 达 **12 原生 / 33 适配层**，Architect 5/21。**本轮真正的内容是揪出驱动两个真 bug**：
  - **缺陷 A**：后阶段原生层**漏了** `apply_layer_to_workdir`，而 GuiSurgeon/`cut_gui` 是直接读盘的 → 它们读到上一阶段的内容。已补上调用，并加契约检查 `check_layer_materialized`（磁盘大小 vs 层 blob 大小，不符即报错并**自报任务名与路径**）
  - **缺陷 B**：§9.49 的「阶段窗口」判据把 `generate_smithing_ui` 放到旧批次之后，而计划里它**早于** Surgeon 的 `fix_smithing2_villager2_ui`（后者会重新派生并覆盖 `container/smithing.png`）→ 必须在前。诊断读数：**原生 vs 旧 Architect = 0 像素差异**，差异全部来自「谁最后写该文件」
  - **被实测否决的判据**：「阶段同级或更早 → 前阶段」会让 `generate_boat`/`rename_blocks_items` 提前并使真实包分叉（8 项 OnlyInB）→ 改为**显式名单** `EARLY_NATIVES`（只收有实测证据的任务，逐条写明证据），阶段判据继续兜底
  - **验证缺口（如实记录）**：`fish_bucket` 的正题**未被真实包覆盖**——TapL 16x 是 1.9 路径，只有 `items/bucket_water.png`，没有 `item/water_bucket.png`，该任务走「跳过」分支；正题需夹具用例补
- [x] T10aj（M2）铜/下界合金族 8 个任务 ✅（§9.53）：新增 `arch_gen_metal`；逐条照抄的要点是「源缺失即跳过」「`.mcmeta` **源有才写**且**两个 armor 任务不写**」「回退链顺序即产物」「`netherite_tools` 附带 `arrow→spectral_arrow`」。首次派发只差 4 个 `OnlyInA`：旧任务 `fix_armor_models`（Surgeon）会把 `models/armor/{copper,netherite}_layer_*.png` **改名搬走**，原生放后阶段时源已不在 → 跳过 → 新路径下 4 个文件消失。处置：两个 `*_armor_models` 加入 `EARLY_NATIVES`（**第二个实例**，与 `smithing_ui` 同形）。正向 `on` 达 **20 原生 / 25 适配层**，Architect **13/21**；「同级一律提前」的判据再次被否决，名单式最小授权验证可行
- [x] T10ak（M2）新木种 3 个任务 ✅（§9.54）：新增 `arch_gen_planks`（redwood/cherry/bamboo、pale、poplar 全套）。首次派发只差 `item/poplar_sign.png` 一个文件，**最大通道差仅 13** = 换了源图而非算错：poplar 的「优先 jungle、缺失回退 oak」判据依赖当时磁盘内容，放后阶段时 jungle 源已被更早的删除类旧任务搬走。处置：`generate_poplar_planks` 加入 `EARLY_NATIVES`（第三个成员，与 `smithing_ui`、两个 `*_armor_models` 同形）。正向 `on` 达 **23 原生 / 22 适配层**，Architect **16/21**
- [x] T10al（M2）`generate_tricky_trials_breeze` ✅（§9.55）：新增 `arch_gen_breeze`，照抄规则表 + 三条易漏语义（目标已存在即跳过、候选链首个存在者胜、不祥钥匙的源为条件选择）。首次派发只差 `mob_effect/wind_charged.png` 且是 **OnlyInB**（我多生成了）：状态图标源 `mob_effect/{speed,jump_boost,absorption}.png` **是旧任务 `fix_ui_survival` 造出来的**，放后阶段时它们"凭空出现" → 该规则由跳过变执行。加入 `EARLY_NATIVES`（第四个成员）。**通则**：「源存在性」是随时间变化的输入，判据含 `exists()` 的任务放进错误一侧会静默换产物。正向 `on` 达 **24 原生 / 21 适配层**，Architect **17/21**
- [x] T10am（M2）Architect 剩余 3 个 ✅（§9.56）：新增 `arch_gen3`（`generate_crossbow` / `generate_tipped_arrow_images` / `generate_snow_bucket`），照抄三者**不同粒度**的跳过语义与 `zip` 语义（裁头、拉弓配对表、firework 的「先拷贝再叠加」）。**验证缺口已标注**：三者源都在 1.9 路径或根本不在包里 → 真实包上**都走跳过分支**，闸门只证明「无副作用」，正题需夹具用例（连同 `fish_bucket` 一起）。正向 `on` 达 **27 原生 / 18 适配层**，Architect **20/21**，只剩依赖 Surgeon 的 `shulker_box_ui`
- [x] T10an（M2）**夹具用例补四个任务的"正题"** ✅（§9.57）：`uimage_tasks_match_the_old_implementations_on_a_fixture`（第 8 个忽略用例）——**先断言 `UImage` 可用**（解析不到即红，避免用例退化成空跑），源贴图取自真实包、路径换成 1.13+ 的 `item/`，同一份输入分别跑旧转换器与原生实现，逐像素比对。首次即通过：crossbow 6/6、tipped_arrows 2/2、snow_bucket 1/1、fish_bucket 6/6。为此加了 `#[cfg(test)] native_for_probe`（只读派发表，不改生产路径）。**忽略用例 7 → 8**
- [x] T10ao（M2）`generate_shulker_box_ui` 实现完成 ✅（§9.58）：新增独立模块 `shulker_box_gen`（自适应缩放的「最接近 256 倍数」判定 + 清空区间 + 上移 56s，**读取全部取自原图**）。**算法当场用夹具正题验证**（1/1 逐像素一致），**但生产路径仍未派发**：它需要「Architect 旧任务之后、Surgeon 旧任务之前」的中间位置（§9.51），旧批次不可拆分 → 等 Surgeon 原生化后一起验收。夹具暴露走 `#[cfg(test)] native_for_probe`，**刻意不做环境变量开关**（同一提交两种行为会让闸门失去意义）。**Architect 阶段 21/21 实现完成**（20 派发 + 1 待 Surgeon）
- [x] T10ap（M2）**进入 Surgeon**：`fix_slider` + `fix_clock_compass` ✅（§9.59）：新增 `surgeon_early`。两条新教训：①`fix_slider` 第一版多写了一条目录条目（`tx.mkdir`），**契约检查 `strict_scopes` 当场报出**——旧实现从不创建该目录，多写即产物差异；②`fix_clock_compass` 在**生产管线里是空操作**（旧实现读 1.9 路径 `items/`，而阶段 3–4 的 `rename_blocks_items` 已把源改名到 `item/`），我把它提前后源"复活"→ 真的拆出 100 项差异，因此**从 `EARLY_NATIVES` 撤出**。这是 §9.55 通则的**反向实例**：提前会让本该跳过的旧任务复活，旧行为的 bug 也要一起固化。抽帧算法另立夹具正题（第 9 个忽略用例，44 个条目全一致）。正向 `on` 达 **29 原生 / 16 适配层**
- [x] T10aq（M2）`overlay_icons` + `fix_brewing_stand_ui` ✅（§9.60）：新增 `surgeon_early2`。关键差异是**覆盖图叠加语义不同**——`overlay_icons` 用**覆盖图 alpha 当蒙版**线性混合（且覆盖图不存在也会重写 `icons.png`），`fix_brewing_stand_ui` 用 `imageops::overlay`。`fix_brewing_stand_ui` 与 `fix_clock_compass` 同型：**在生产管线里是空操作**（它要读的 `shulker_box.png` 由阶段 2–3 的 `generate_shulker_box_ui` 生成，而它自己在阶段 1–2 → 源还不存在），M2 要求产物一致所以**必须保持跳过**；其正题要等 `shulker_box_ui` 派发后才被真实包覆盖，届时两者顺序成为硬约束（§9.51「中间位置」的又一处实例）。`overlay_icons` 在真实包上真的执行了（`modified` 10 → 20）。正向 `on` 达 **31 原生 / 14 适配层**
- [x] T10ar（M2）`fix_sign_entities` + `fix2_horse_ui` ✅（§9.61）：新增 `surgeon_mid`，两者留在后阶段即通过（读写各自独占目录）。要点：`fix_sign_entities` 的 11 个木种变体 + **把原图改名为 `signs/spruce.png`**（漏掉会少一变体多一 `sign.png`）；`fix2_horse_ui` 是三对槽位 sprite 的改名拷贝。正向 `on` 达 **33 原生 / 12 适配层**（占计划 73%），两者在真实包上都真的执行了。剩余 12 个适配层任务已列出顺序（`fix_tabs`/`fix_ui_creative`/`fix_ui_sub_hand`/`fix_sign`/`fix_horse_ui`/`fix_particles`/`fix_machinery_ui`/`fix_ui_survival`/`fix_armor_models`/`cut_gui` + 待派发的 `shulker_box_ui` + 单独立项的 `adapt_java_shaders`）
- [x] T10as（M2）`fix_horse_ui` + `fix_sign` ✅（§9.62）：新增 `surgeon_mid2`，**两头各验证一次**：①`fix_horse_ui` **必须提前**——它**原位改写** `horse.png`，而该图随后被 GUI 切片链消费成 `slot/*`，放后阶段时 `fix2_horse_ui` 会用旧图切出的槽位覆盖正确产物（`llama_armor` / `saddle` 各 88 / 118 像素差异）；这是「原位改写被下游消费的图必须提前」的**第三次**同型证据。②`fix_sign` 与 `generate_poplar_planks` 有顺序交互——poplar 写 `item/oak_sign.png` 而 `fix_sign` 读它，生产顺序是 fix_sign **早于** poplar，故 `fix_sign` 必须放**后阶段**（旧批次之后）才与生产一致。正向 `on` 达 **35 原生 / 10 适配层**（占计划 **78%**）
- [x] T10at（M2）`fix_particles` ✅（§9.64）：新增 `surgeon_mid3`（16×16 网格映射表 + 尺寸守卫生成具名小图、`fishing_hook` 写到 `entity/`、末尾删源），放后阶段即通过、真实包上真的执行了。**一次自我纠正**：第一版照抄旧实现的两次 `create_dir_all` 写成 `tx.mkdir`，因声明范围是**前缀**（非 `exact`）**契约检查抓不到**；按 §9.59 的教训直接去掉——**不写旧实现没写的东西**，闸门确认产物不变。正向 `on` 达 **36 原生 / 9 适配层**（占计划 **80%**）
- [x] T10au（M2）`cut_gui` 的**设计勘察** ✅（§9.63）：读清后确认它不是普通图元任务——`GuiSurgeon::execute_transformation(ctx, pool, res)` 需要**真实 workdir** 与**自带 `TexturePool`**（还有内部延迟删除列表），而原生钩子只给 `&mut Tx`。已写下可执行方案：给驱动加「**workdir 型原生任务**」钩子（执行前落层、执行后用现有 `harvest` 收层，与 `run_direct_steps` 同路），并指出它属于 §9.51 的「中间位置」类——**要等 `cut_gui` 之后的旧任务（尤其 `adapt_java_shaders`）也原生化后一起验**。本轮未动工（按纪律不半途插入驱动级改动）
- [x] T10av（M2）`fix_tabs` ✅（§9.65）：新增 `surgeon_mid4`（原位搬移 tabs.png：右移 14 + 六组左移 2/4/6/8/10/12 + 末段拷贝重写）。**关键发现：它不在本次验收路径上**——`scheduler` 有一条 `from==9 && to==12 && target>15 → 跳过 fix_tabs` 的规则，1→97 不含该段，所以派发后 `native_names` 里没有它（不是 bug）。**教训**：`native_names` 是「计划 ∩ 派发表」的交集，**不能**把「挂上实现」当成「被验证」；为此补第 10 个忽略用例（夹具正题，256/512 双缩放**均 0 像素差异**）。**排产提醒**：迁移前先确认任务是否出现在 `plan` 里，否则真实包闸门是**假绿灯**
- [x] T10aw（M2）`fix_armor_models` ✅（§9.66）：新增 `surgeon_late`（`models/armor/*_layer_{1,2}.png` → `entity/equipment/humanoid(_leggings)/`，源被移走、目标覆盖）；它是 §9.53 的另一半（两个 `generate_*_armor_models` 必须提前正因为本任务会搬走其源），自己放后阶段即通过、真实包上真的执行了。**契约检查连抓两处声明错误**：①**删除也是写入**（Tombstone 计入 `writes`，删源必须声明源目录）；②**`TaskDecl::writes()` 是覆盖不是累加**——连续两次 `.writes(...)` 会**静默丢掉**前一个范围（报错与上一次一模一样＝改动没生效的典型信号），必须用 `ScopeSet::with_prefix` 组合。正向 `on` 达 **37 原生 / 8 适配层**（占计划 **82%**）
- [x] T10ax（M2）`fix_ui_sub_hand` + `fix_ui_creative` ✅（§9.67）：新增 `surgeon_ui`，两者是 `fix_horse_ui` 同族（**原位改写 GUI 图**）且计划槽位在最前，故放**前阶段**。共用原语逐条照抄：**搬移/拷贝都是「先裁剪出源区域、再逐像素覆盖贴到目标」**（读取全来自裁剪副本 → 源区与目标区重叠也不会自我污染；越界跳过/留透明），`fill` 的取色取自**当次修改后的图**（顺序敏感）。**留意一处相邻关系**：`fix_slider` 读 `widgets.png` 而 `fix_ui_sub_hand` 原位改写它，两者区域不重叠所以顺序无关——**但这是实测的巧合，不是可依赖的性质**，已写进文档。正向 `on` 达 **39 原生 / 6 适配层**（占计划 **86.7%**）；剩余 6 项为 `fix_machinery_ui`、`fix_ui_survival`、`cut_gui`（需驱动扩展）、`generate_shulker_box_ui`（已实现待派发）、`adapt_java_shaders`（单独立项）、`fix_tabs`（不在验收路径）
- [x] T10ay（M2）**放置规则的回归测试** ✅（§9.68）：新增 `native_placement_rule_is_pinned_by_the_real_plan`（**标准路径、0.01 秒**、用夹具计划；全量单测 **307 → 308**）。钉住三件事：①`EARLY_NATIVES` 里的任务必须真判为 `Early`；②**实测边界**——真实包 `min_legacy = Architect`（Eraser 旧任务已全部原生化），故七个 Eraser 级原生任务由**阶段判据**判为 `Early`（`rename_blocks_items`、`delete_font_folder`、`process_chest_folder` 等，闸门一直在此放置下通过）；③曾被误判的 `fix_clock_compass`（§9.59 的 100 项差异）与 `generate_boat`（§9.53 的 8 项 OnlyInB）必须留在 `Late`。**附一次误判与撤回**：第一版把 `rename_blocks_items` 的断言写反，失败后误判为「判据偏松／已知缺口」；核对**真实包**读数后**撤回**该说法——它判 `Early` 是正确的，我把断言文案当成了被测对象。**教训（比误判更值钱）**：断言失败时除了分清「实现错／期望错」，还要**先核对读数属于哪个计划**——凡关于放置的判断都必须以**真实包**计划为准
- [x] T10az（M2）`fix_machinery_ui` **读清与排产** ✅（§9.69，未动工）：371 行、5 个子步骤。其中 4 个（grindstone / cartography_table / stonecutter / loom）都走 `process_ui_from_shulker`——**读 `container/shulker_box.png`**，因此**当前全走「源缺失 → 跳过」**；只有 `process_villager2_machinery` 会真的跑（双宽重写 `villager.png` + **把原图备份成 `villager_backup.png`（有存在性守卫）**，而该备份正是反向任务还原的依据）。**排产结论**：它又一次指向同一顺序——**先原生化消费 `shulker_box.png` 的 Surgeon 任务（`fix_brewing_stand_ui` 已完成，剩 `fix_machinery_ui`），再派发 `generate_shulker_box_ui`，最后才是 `cut_gui` / `adapt_java_shaders`**
- [x] T10ba（M2）`fix_machinery_ui` ✅（§9.70）：新增 `surgeon_machinery`，五个子步骤逐条照抄。**关键是两种叠加语义严格分开**：区域搬移/贴上用**原始覆盖**（等价 `Image.paste`，alpha 也照抄；**不能**用 `copy_from`——它越界会报错而旧实现静默裁剪，故手写「越界即跳过」循环），而 `UImage` 覆盖图 / anvil 区域 / villager2 的搬运用 `imageops::overlay`。真实包上四个 shulker 子步骤仍全走跳过，**真执行的是 `process_villager2_machinery`**（双宽重写 + **备份 `villager_backup.png`**）。正向 `on` 达 **40 原生 / 5 适配层**（占计划 **89%**）；剩余 5 项：`fix_ui_survival`、`cut_gui`（需驱动扩展）、`generate_shulker_box_ui`（已实现待派发）、`adapt_java_shaders`（单独立项）、`fix_tabs`（不在验收路径）
- [x] T10bb（M2）`fix_ui_survival` ✅（§9.71）：新增 `surgeon_survival`，四步逐条照抄——①抽 **19 个状态图标**写成 `mob_effect/*.png`（**这正是 `generate_tricky_trials_breeze` 的源**，§9.55 的顺序依赖，如今两段都是原生）；②搬移 (86,24)-(162,62) → (+10,−8)，**保留旧实现「先用目标位置颜色擦源区」的怪癖**（迁移第一原则是产物一致，不顺手修正）；③两处填充 + 一块**原始覆盖**拷贝；④可选叠加 `UImage/inventory/inventory_{width}.png`（**Lanczos3**）并抽两张药水背景 sprite。**刻意不复刻** `HurrayContext` 的纹理缓存（内存优化，与产物无关）。`decl()` 按 §9.66 的教训**用 `with_prefix` 组合三个写范围**（先想起教训再动笔）。正向 `on` 达 **41 原生 / 4 适配层**（占计划 **91%**）
- [x] T10bc（M2）`generate_shulker_box_ui` **为何仍不能派发** ✅（§9.72，顺序推演，未动代码）：消费它的 Surgeon 任务虽已全部原生化，但**把它们放在后阶段**——若现在把生成器加进前阶段，两个消费者会**突然真的开始生成** `brewing_stand.png` / `grindstone.png` 等**生产里本来不产生**的文件。**根因**：生产顺序是「消费者在前、生成器在后」，而「前阶段／后阶段」这个二分**表达不了中间切分**。**正确路径**：先原生化 **`cut_gui`** 并放进前阶段，使生成器与消费者落在**同一侧**（前阶段内部按计划顺序排列），链路才会被正确激活。**剩余项因此有依赖顺序**：`cut_gui`（关键路径）→ `generate_shulker_box_ui` 派发 → `adapt_java_shaders`
- [x] T10bd（M2）尝试派发 `generate_shulker_box_ui` → **测出放置规则的隐含耦合，已回退** ✅（§9.73）：
  - **推翻一个前提**：`cut_gui` **不是** `shulker_box` 的前置。`GuiSurgeon` 在生产里**跑两次**（注册表任务 + `run_direct_steps` 收尾），且其清理用 **`defer_remove_file` 延迟删除**（到末尾 `execute_cleanup` 才真删），因此第二次运行时 atlas 仍在盘上、切出的 sprite 相同 → **幂等**，`cut_gui` 的效果被收尾步骤**遮蔽**（也因此无法靠产物独立验证）。
  - **测到的真缺陷**：`native_placements` 用「**剩余旧任务**最小阶段」作基准 → 真实包上派发 `generate_shulker_box_ui` 后基准从 `Architect` 变 `Surgeon`，`generate_boat`/`potion_lingering`/`tipped_arrows` 被判 `Early`，**复现 §9.53 的 8 项 `OnlyInB`**。即**迁移本身会改变迁移的判据**。
  - **修法尝试也失败**：改成「整批计划最小阶段」后仍分叉（30+ 项 `OnlyInA`：poplar/铜灯泡族/`breeze_rod`，加 `clock_*`、船变体 `OnlyInB`）；诊断显示前阶段只剩 10 个 = 名单成员，**不在名单里的 smithing/horse_ui/shulker 掉到后阶段**，但按代码字面不该如此——**出现矛盾即停手**，两者一起回退。
  - **后续建议**：①先用「自报身份」办法打印**每个原生任务前后**的关键路径哈希，定位是哪一步先偏离；②判据应改为**不依赖迁移进度**的形式（如 §9.52 的相邻任务关系分析）；③`cut_gui` 可作**独立清理项**，但**不要**当作 `shulker_box` 的前置。
  - **本轮净改动**：仅两处文档注释 + §9.73；**代码行为与上一版一致**（41 原生 / 308 passed / 10 ignored 全过 / 警告 84）
- [x] T10be（M2）**逐步读数工具** ✅（§9.74，第 11 个忽略用例）：新增 `MixedRunOptions::step_trace`（默认关）+ `MixedRunReport::step_trace`，读数点覆盖 `pre:<任务名>`、`before-legacy`、**`legacy:<任务名>`（旧批次逐个）**、`after-legacy`、`post:<任务名>`、`after-direct-steps`，共 **48 步**；指纹用「文件数 + 总字节 + 相对路径/大小序列的 FNV-1a」，**不读内容**所以开销可忽略。另备好细粒度 `digest_scope(prefix)`（逐文件内容哈希，暂未接线）。**一处刻意设计**：打开 `step_trace` 会把旧批次改为「逐任务执行+逐任务收层」（否则拿不到批次内的粒度），因此新用例**先断言读数不改变产物**——实测 `assert_equivalent(prod, trace)` 通过。**下一轮用法**：先存绿色状态的读数，再做派发/基准改动跑第二份，**两份 trace 从某标签起指纹不同 ⇒ 该标签即第一处偏离**
- [x] T10bf（M2）用逐步读数定位分叉 → **拿到关键读数，仍未收敛，已回退** ✅（§9.75）：三层结论——①**耦合被读数直接证实**：打开派发后读数里 `pre:` 段出现 `generate_boat`/`potion_lingering`/`tipped_arrows`/`furnace` 等**本应在后阶段**的任务（基准被从 `Architect` 推到 `Surgeon`）；②**改基准为「整批计划」后放置完全正确**（前阶段恰好 = `EARLY_NATIVES` 的 10 个成员），且**单独改它时闸门通过**（即该改动本身安全）；③但**两者同时打开仍分叉**（30+ 项 `OnlyInA`：`copper_bulb`/`poplar`/`breeze_rod`/`clock.png`，加 `clock_00..63`、船变体 `OnlyInB`），且观测到「逐任务旧批次 vs 一次性批量」在该新配置下**不再一致**——说明该配置下**旧批次内部的顺序敏感度变了**。**处置**：按纪律不叠加改动，两者一起回退。**后续抓手**：①**先单独落地"基准改整批计划"**（已验证单独安全），**再**单独评估派发，**成对验收**；②若仍分叉，用 §9.74 的工具对比同一配置下的两份读数（逐任务 vs 一次性批量），差值直接指出**旧批次里哪一步对顺序变敏感**
- [x] T10bg（M2）**放置判据定稿 + `generate_shulker_box_ui` 派发成功** ✅（§9.76）：抓手①执行后发现「单独改基准**也失败**」（产物多 **57** 个文件），而逐步读数**第一处偏离一眼可见**——`pre:delete_blockstates_models` 从首位消失。**根因**：新基准恒为 `Eraser`，于是 `stage < min` 对 Eraser 级**永远为假**，**六个删除/改名类原生任务被挪到后阶段**，早阶段生成的新格式产物（`poplar_*`/铜灯泡族/`breeze_rod`）**不再被删除**。即：旧判据一直正确只是因为基准恰好是 `Architect`、Eraser 级**碰巧**满足——这正是随迁移漂移的那个"碰巧"。**定稿**：把判据写成**不依赖任何全局基准**的显式形式 —— `is_eraser || EARLY_NATIVES.contains(name) → Early`，其余 `Late`；判据里再没有「最小阶段」，**与迁移进度彻底解耦**。**结果**：`generate_shulker_box_ui` 一并派发后**闸门一次通过**，正向 `on` 达 **42 原生 / 3 适配层（93%）**，产物与 legacy 逐项一致（4018 文件 / 19294643 字节）。回归测试同步改为「凡 Eraser 级已派发原生任务必须 `Early`」（并断言计划里至少有一个，避免空转）。**剩余 3 项**：`cut_gui`（独立清理项）、`adapt_java_shaders`（单独立项）、`fix_tabs`（不在验收路径）
- [x] T10bh（M2）`adapt_java_shaders` **立项勘察** ✅（§9.77，未派发、未动代码）：①**注册方式**不是直接注册，而是 `if adapt_shaders { shaders::java::register_scheduler_task(scheduler) }`（实验开关，默认开）；②**计划里 9 个正向前向段都登记了它，但按 `plan()` 去重只有 (6,7) 生效**；③**规模 1260 行 / 17 个顶层函数**，横跨 7 个 pack_format 里程碑（7/32/46/63/84/97）；④**关键读数：真实包里 `shaders` 相关条目 = 0**，所以它在第一个判断（`!shaders.is_dir()`）就返回——**真实包闸门对它给的是假绿灯**（与四个 `UImage` 任务、`fix_tabs` 同型）。**结论**：**不派发它产物不变**，因此"挪过去没分叉"**不能**用来验收；迁移前提是①有带 `shaders/` 的夹具能覆盖正题、②它的槽位能被闸门区分。**下一步已写明**：先造三分支夹具（`<7` legacy / `7≤t<63` / `≥97`）做「旧函数 vs 原生」逐文件比对；移植顺序建议**先表驱动**（4 张表 + 2 个 allowlist）**再文本改写**；派发**单独一轮**（属「中间位置」风险候选）
- [x] T10bi（M2）`adapt_java_shaders` **三分支夹具正题** ✅（§9.78，第 12 个忽略用例，未派发）：自造含 `shaders/{core,post,post_effect,include}` 的 1.20.1 风格包，对 `target ∈ {1, 34, 97}` 各跑一次**旧实现**并记录**逐文件快照**（增/删/改 + 内容）。**实测基线**：`target=1` 删 5（core 下 3 个 JSON + `post/`、`post_effect/` 各 1）、改 0、增 0；`target=34` 删 2（`unknown_thing.vsh`、`post_effect/blur.json`）、改 2（`rendertype_entity.json` mat3→mat4、`include/fog.glsl` 补空行）、增 0；`target=97` 删 9（旧名组 `rendertype_{entity,text}.*`、`rendertype_entity_translucent.fsh`、`post/`、`post_effect/`…）、改 2（`screenquad.json` 剥 uniforms、`fog.glsl`）、增 3（**改名产物** `core/entity.fsh`、`core/text.{vsh,json}`）。**两处"先测量"收益**：①我原断言 target=97 时 `#include` 出现在 `rendertype_entity.fsh`——读数显示该文件**被删**（属改名组，且 `_translucent` 不在现代白名单），改查**改名后**的 `core/entity.fsh`；②我原把 JSON 写成 `"matrix3x3"`，mat3→mat4 **没发生**——实现匹配的字面量是 `"type": "mat3"`（含空格），改对后该分支"修改"从 1 项变 2 项，**说明夹具写错会让闸门空转**。**这三条基线即后续原生移植的验收口径**
- [x] T10bj（M2）`adapt_java_shaders` **移植第一步：纯文本改写** ✅（§9.79，第 13 个忽略用例，仍不派发）：新增 `pilots::shader_adapt`，移植四个纯文本函数（`rewrite_import_path` / `convert_moj_import_to_include` / `namespace_moj_imports` / `needs_globals_import` + `has_globals_import`）。**留意一处非显然语义**：旧实现用 `lines()` 重组、每行补 `\n`，因此**顺手把 CRLF 归一为 LF 并给无结尾换行的文件补换行**——这是**产物的一部分**，照抄未"修正"。**验证方式**：新增逐函数对照用例（12 条语料 + 9 条路径输入 = **57 项**），原生与旧实现**逐字节**比较输出与计数，**全部一致**。为此在 `converters/shaders/java.rs` 加了一个**只读**的 `legacy_text_ops` 访问层（`pub(crate)`，仅转发四个私有函数供对照）。**尚未移植**（下一步）：表驱动的 `prune_and_rename_core`（4 张表 + 2 个 allowlist）、`ensure_core_json`、`rewrite_json_matrix_types`、`strip_json_uniforms_for_ubo`、`adapt_post_paths`、`walk_dir` 骨架与 `ensure_include_trailing_newline`。模块标 `#[allow(dead_code)]`，**派发后应移除**；真实包产物不变
- [x] T10bk（M2）`adapt_java_shaders` **移植第二步：四张表 + 常量** ✅（§9.80，第 14 个忽略用例，仍不派发）：把 `prune_and_rename_core` 依赖的全部纯数据移植进 `shader_adapt`（`modern_core_allowlist` / `legacy_core_allowlist` / `core_rename_table` / `core_removed_stems` + `SHARED_VERTEX_STEMS` + 六个 `FMT_*` 常量），并用第 14 个忽略用例在 **20 个 target**（覆盖 7/32/46/63/84/97 每个里程碑**两侧**）上逐项比对，共 **62 项全部一致**。**两处自我纠正**：①测试首跑报了 6 个 target 差异，加打印后读数显示「旧独有 `[]` / 原生独有 `[]`」——**集合相同只是长度不同**（旧 `HashSet` 去重，我的 `Vec` 把基础表里已有的 `"block"` 又 push 一次）；语义其实正确，改为比较**去重后集合**，并顺手证明移植连"冗余 push"都照抄了；②`#[allow(dead_code)]` **不合本仓约定**（仓库用 `#[cfg(test)]`，见 `native_for_probe`），已把 `legacy_text_ops` 与 `shader_adapt` 两处都改成 `#[cfg(test)]` 并注明派发时移除。**待做**：扫描/改写骨架（`prune_and_rename_core` 文件操作、`ensure_core_json`、`rewrite_json_matrix_types`、`strip_json_uniforms_for_ubo`、`adapt_post_paths`、`ensure_include_trailing_newline`、`walk_dir`），之后才能派发；验收口径见 §9.78 的三分支基线
- [x] T10bl（M2）`adapt_java_shaders` **移植第三步：JSON 文本变换** ✅（§9.81，第 15 个忽略用例，仍不派发）：移植 `rewrite_json_matrix_types_text`（`"type": "mat2"|"mat3"` → `"mat4"`，**只匹配带空格的字面量**）、`remove_json_key`（删顶层 `"key": …`，粗粒度）、`minimal_core_json`、`json_has_uniforms`。**`remove_json_key` 是最绕的一个**（按括号深度找值尾、再"吃后随空白与逗号，否则删前导逗号"），第 15 个用例专门覆盖值在中/末尾、对象/数组/字符串/布尔/数字值、嵌套、只有键没冒号、键不存在（须原样返回），共 **31 项对照全部一致**。**三个对照套件合计 150 项**（逐函数 57 + 表格 62 + JSON 31）。**只剩文件系统骨架**（`prune_and_rename_core` 遍历与删/改名、三个目录遍历、`adapt_post_paths`、`ensure_include_trailing_newline`、`walk_dir`）——它们没有纯函数可逐例对照，**验收要落到 §9.78 的三分支夹具快照**
- [x] T10bm（M2）`adapt_java_shaders` **移植第四步：globals/fog 文本 + 骨架** ✅（§9.82，第 16–17 个忽略用例，仍不派发）：
  - **文本收尾**：新增 `has_globals_import` / `inject_globals_import` / `count_args_likely_three` / `fog_note_if_needed`；第 16 个用例 64 项对照全一致。**抓出一处真实保真度缺口**：我第一版把 `has_globals_import` 写成**子串搜索**，而旧实现是**逐行**判定（`globals.glsl` 只出现在注释里不算），已修正。**四个文本/表格套件合计 214 项对照**
  - **骨架已能跑**：`shader_adapt::run` 实现改名组（`json/vsh/fsh` 成组、目标已存在则删源）、include 结尾空行、源码遍历（跳过 `include/`、仅 `core` 改导入指令、三段判定顺序照抄）
  - **尚未移植（代码与日志都显式写明，绝不静默跳过）**：`prune_and_rename_core` 的**删除**步骤、`ensure_core_json`、两个 JSON 目录遍历、`adapt_post_paths` —— **补齐前不得派发**
  - **骨架验收方式**：第 17 个用例在**同一夹具**上跑两侧并**按范围断言**（`target=1` 原生改1删0增0 / 旧删5；`34` 改1删0增0 / 旧删2；`97` 改1删3增3 / 旧删9）：①原生不得对它未移植的类别做删除；②旧侧多删的必须落在「已声明未移植类别」白名单里
  - **口径纠正**：上一版把忽略用例数写成 15，**实际当时是 16**；本轮后为 **17**（7 真实包 + 5 shader_adapt + 5 夹具/诊断），已用 `--ignored --list` 逐个核对
- [x] T10bn（M2）`adapt_java_shaders` **移植完成 + 夹具快照逐项一致** ✅（§9.83，第 17 个用例升级为完整对照，**仍不派发**）：补齐最后一块（**删除步骤**、core JSON 三态、**post 路径**），`shader_adapt::run` 已覆盖旧实现 `adapt_java_shaders_at` 的**全部步骤**（post 路径 → 旧 API 分支「删 post_effect + 递归删 JSON」→ core 改名+删除+include 空行 → core JSON 补齐/mat 升级/剥 uniforms → 源码遍历）。验收升级为**完整快照对照**（不再按范围断言）：`target=1` 两侧均 改0删5增0；`34` 均 改2删2增0；`97` 均 改2删9增3 —— 条目集合与**每个文件内容**逐项相同；并顺带钉住 §9.78 基线（97 删9增3）。**移植总账**：文本/表格 **214 项**逐项对照 + 夹具 **3 target 完整快照对照**。**为何仍未派发（如实说明）**：它的 `run` 需要**目标 pack_format** 参数（旧实现从 `ctx.get_data` 取，而原生任务只拿得到 `&mut Tx`），接入生产需一次**签名/接线改动**（`MixedRunOptions.target_version` 已有该值，成本小但属驱动接口改动，应单独一轮）；且**真实包没有 `shaders/`**（§9.77 实测 0 条），派发与否产物相同、**闸门区分不出**，故正确验收依据是本轮的夹具快照对照
- [x] T10bo（M2）`adapt_java_shaders` **接线尝试未落地（已回退）+ 一条硬约束** ✅（§9.84，**本轮无入库改动**）：目标是上一版留下的「传 `target_version` 并派发」，做法是在驱动里引入 `NativeImpl::Tx`/`Target` 两种形态。`native_for` 有**约 45 个 return 点**，用脚本做机械包裹**两次都没编译过**（第一次括号不配、第二次正则跨行吞掉尾随逗号），按纪律**立即 `git checkout` 回退**到绿色状态（HEAD `f7af951`）。**教训（与 §9.30 同族）**：本仓 `mixed_run.rs` **不适合脚本批量结构改写**——45 处多行元组、锚点跨行、形式不一；该手工逐个改，或**先加薄适配层再逐个迁移调用点**（本轮后半段的设计正是如此，只是不该用脚本去撞那 45 处）。**硬约束**：`adapt_java_shaders` 在生产路径上是「`shaders/` 不存在 → 立即返回」，**派发它对闸门完全无信号**（分叉看不见、正确也证明不了），故**派发价值仅限"去掉一个适配层"**，其正确性**只能由夹具证明**（§9.83 已做到）。**后续接线路径已写明**：①先加 `NativeImpl` + `native_for_tx`（不动现有 return 点）②改 4 个「拿实现去执行」的调用点 ③只把 shader 那一条改成 `Target` 并在前后阶段循环里传 `opts.target_version` ④夹具验证 + 真实包确认产物不变
- [x] T10bp（M2）`adapt_java_shaders` **接线并派发成功** ✅（§9.85）：改换思路——**让任务自己从包里读目标格式**（`run_from_pack`：`tx.view().mcmeta().effective_format()`，取不到则按旧实现缺省 88），因为**全仓只有一处设置 `target_pack_format`**（生产 `context.set_data("target_pack_format", target_version)`，而驱动收尾用**同一个** `target_version` 写 `pack.mcmeta`），两条路拿到的是**同一个数**，且"从包里读"更符合对象模型（任务声明需求、包是唯一真源）。**派发只改一处**：在 `native_for` 末尾 `match name` 里加一个**单行分支**（与既有 `generate_shulker_box_ui` 特判同形）——**未改返回类型、未动任何 return 点**，§9.84 的事故诱因被完全绕开；同时移除 `shader_adapt` 上的 `#[cfg(test)]`（现在是生产代码）。**结果**：正向 `on` 达 **43 原生 / 2 适配层（95.6%）**，`adapt_java_shaders` 落在后阶段（紧接 `generate_snow_bucket`，与计划一致），产物与 legacy 逐项一致（4018 文件 / 19294643 字节）。**如预判**：真实包上它仍是「`shaders/` 不存在 → 跳过」，闸门只能证明「派发未改变产物」，**正确性依据是 §9.83 的夹具完整快照对照**
- [x] T10bq（M2）**8c 收官总账** ✅（§9.86，目标三项逐条核对全部达成）：①**Architect 21/21 全部完成**（用 `git grep 'TaskTier::Architect'` 把注册表里的 Architect 任务**全数列出**再逐条核对，不凭记忆：boat / copper ×4 / crossbow / fish_bucket / furnace / netherite ×4 / pale·poplar·redwood planks / potion_lingering / shulker_box_ui / smithing_ui / snow_bucket / tipped_arrow_images / tricky_trials_breeze）；②**跨阶段时序任务按"只实现暂不派发"执行**（`generate_shulker_box_ui` 先夹具验证、待 §9.76 放置判据定稿后才派发）；③**`adapt_java_shaders` 单独立项并完成**（勘察 §9.77 → 分步移植+逐层对照 §9.79–§9.83 → 接线派发 §9.85）。**最终数字**：正向 **43 原生 / 2 适配（95.6%）**；反向 42/1；产物 4018 文件 / 19294643 字节逐项一致；全量 **308 passed / 17 ignored**；忽略用例 **17/17 通过**（含 **7 个真实包对照**，已按名字核对）；`check --bins` 通过；警告 **84** 全程未增。**剩余 2 个适配层经核对属 Surgeon 阶段、不在本目标范围**：`fix_smithing2_villager2_ui`（Surgeon，槽位 12→13）、`cut_gui`（Surgeon，槽位 15→18，且被非幂等的收尾 `GuiSurgeon` 遮蔽）
- [x] T10br（M2）**全部原生化（阶段一）**：`fix_smithing2_villager2_ui` 原生化 ✅（§9.87，第 18 个忽略用例）：新增 `pilots::surgeon_smithing2`（两个子过程逐条照抄，含 `s` 的两套判定口径不同这一细节：`process_smithing2` 用 `(w,h)` 匹配、`process_villager2` 用 `w` 匹配）；**刻意照抄** `get_pixel` 不越界保护与 `write_to(.., Png)` 编码路径。**验收**：夹具正题在 256/512 两档比对 `smithing.png`/`villager.png`/`villager_backup.png`，**全部 0 像素差**；真实包闸门正向 **44 原生 / 1 适配（97.8%）**，产物 4018 文件 / 19294643 字节逐项一致（它属「中间位置」风险候选，本次**一次通过**）。**口径修正**：正向注册是 **47**（非 88——此前把 `reverse_*` 重复算了），反向 41。**阶段一剩余**：`cut_gui`（闸门可观测，但被非幂等 `GuiSurgeon` 遮蔽，需先理清遮蔽关系）；**不可观测的 3 个**：`convert_animated_textures`（不在任何计划段）、`reverse_fix_tabs`（反向计划跳过）、`delete_shaders_folder`（仅在关闭着色器适配时注册）——按口径 A 下一步给它们补夹具逐任务对照
- [x] T10bs（M2）**发现并修复「注册了却永不执行」的生产任务** ✅（§9.88，**45 原生 / 1 适配层**）：用户提醒「TapL 里有动画部分（末影珍珠）」→ 查证确认这是**生产缺口**。**事实链**：①包里确有 17 个 `.png.mcmeta`，`items/ender_pearl.png.mcmeta` 正是那个（16×400 条带），但它**已含 `frametime`** → 旧实现第 3 步跳过；**17 个里有 7 个**是 `{"animation": {}}` 会被改写；②本任务只扫 `textures/item|items`，那 7 个里只有 `items/{clock,compass}.png.mcmeta` 落在范围内；③**决定性**：`convert_animated_textures` **不在 `scheduler.rs` 的任何 segment 里**，而 `plan()` 是按 path 遍历 segment 向量（`scheduler.rs:222–247`）——**不在 segment 就永不进 plan、永不执行**，尽管它在 `invoke_conversion.rs` 里是**无条件注册**的 Eraser 任务且注释写明必须在 `rename_blocks_items` 之后；④**实测**：临时加进 `(1,2)` 段后产物字节 **19294643 → 19294735（+92）**，文件数不变，闸门依然通过（两条路径走同一 plan，一起变）。**处置**：补 `(1,2)` 段（排在 `delete_blockstates_models` 前，因后者会删 `clock.png` 源图）+ 派发原生（`animated` 模块**早已移植完成**，只因不在计划里而从未派发）。**结果**：`plan_len` 45→46、正向 **45 原生 / 1 适配**、产物 19294735 字节；**这一项从「不可观测」变成「闸门可观测且已验证」**
- [x] T10bt（M2）**注册表 ↔ segment 双向一致性测试** ✅（§9.89）：§9.88 暴露了结构性风险——`plan()` 只从 **segment 向量**取任务，而**注册表是另一份独立清单**，两者原本**没有任何交叉检查**，于是「注册了却不在任何段」会**静默永不执行**（反方向「段里写了未注册任务」则被 `run_named` 只 `log_warn!` 跳过）。新增单测 `registry_and_segments_agree_in_both_directions` 钉住两个方向，并给 `Scheduler` 加 `registered_task_names()`（注册表原本私有，没它方向②会退化成空转断言）。**测试立刻报出 2 个孤儿并逐个定性**：①`delete_shaders_folder`——无条件注册的 Eraser，但新管线改用 `adapt_java_shaders` 适配、**无任何段登记「整目录删除」**，即已被取代，且真实包无 `shaders/` 无法验收 → **保留实现不进段**；②`reverse_fix_tabs`——正向 `fix_tabs` 在 `(9,12)` 但带**跳过规则**（`target_version>15`），反向 `(12,9)` 没有它**与之一致**，补上反而会在正向没做变换时执行逆变换 → **不进段是正确的**。两者入 `KNOWN_ORPHANS` 白名单，**其余新孤儿一律报红**。**非空转已实证**：临时拿掉 §9.88 的那一项，测试立刻报出 `convert_animated_textures` —— 即「若当初有此测试，第一天就会发现该漏洞」。全量 **309 passed**（+1）
- [x] T10bu（M2）`cut_gui` **定性：它是 `GuiSurgeon` 的重复入口，不按"再移植一个任务"处理** ✅（§9.90）：读代码确认——①`cut_gui`（116 行，其中 88 行是测试）**函数体只有 14 行**，就是 `GuiSurgeon::execute_transformation` 的**薄包装**；②**同一份** `execute_transformation` 在收尾的直接步骤（`run_direct_steps`，条件 `run_gui_surgeon && target_version>=34`）**又被调用一次**，而混合驱动注释写明这步不能漏（「漏掉会整片丢失 sprite 产物，实测少 3861 个文件」）；③故**一次转换里 `GuiSurgeon` 跑两遍**，闸门一直通过 ⇒ 两遍与一遍产物相同（**从闸门读数反推**，未见单独的"只跑一遍"隔离实验，故严格说是「实测等价」非「已证明幂等」）；④原生化它必须先把 `GuiSurgeon` 的**全部写入点**盘清（§9.72 教训：删除也是写、`decl()` 必须覆盖全部写入），否则 `strict_scopes` 当场报红。**结论**：要「让 cut_gui 原生化」等价于把 **1244 行的 `gui_surgeon.rs`** 整个搬进原生池，而它**已在跑且已过闸门**，投入产出比极差且风险高（含 `defer_remove_file` + `TexturePool` 两套机制）。**正确做法是反过来消掉重复**（让一处成为唯一入口），属 **M3 收口**范畴而非 M2 分批迁移。**故这 1 个适配层不是进度缺口**——它与 `reverse_cut_gui`（早已定性为空操作试点 `noop_pilot!`）同族：都是薄包装/不可逆操作，非独立业务逻辑。**重复调用记入 M3 待办**
- [x] T10bv（M2）**纠正 §9.90 + 补「绝对产物契约」**（第 19 个忽略用例）✅（§9.91）：**§9.90 说"两处是同一操作的两次调用、可去其一"——本轮动手去重，两次都失败，证明该判断是错的**。①**尝试 A**（让 `cut_gui` 成唯一入口：两处移除 `run_direct_steps`）→ 闸门报红 `OnlyInA` **62 项**（`blast_furnace/burn_progress`、`loom/*`、`stonecutter/*`…）+ `OnlyInB` 6 项 ⇒ **那批 sprite 只有直接步骤会产出**（`cut_gui` 在 `(15,18)` 跑得太早）；②**尝试 B**（让 `run_direct_steps` 成唯一入口：`cut_gui` 出计划）→ `native_switch` **通过**，但**产物绝对值掉了 4018→4015 文件、19294735→19293011 字节**（少 3 文件/1724 字节）⇒ `cut_gui` 也有独有贡献。**结论：两处各自产出不同的东西，去重前提不成立，两者都保留。** **更有价值的副产物**：尝试 B 暴露了测试体系**真盲区**——**「相对对照」（legacy vs mixed）看不见"两条路径一起变"**：两条路径走同一份注册表与计划，一起少 3 个文件后彼此仍相等，**全部 18 个忽略用例依然通过**。**修复**：新增第 19 个忽略用例 `real_pack_absolute_output_is_pinned`，在真实包上断言 `files == 4018` 且 `bytes == 19294735`，并在注释写明相对对照的盲区与"改动前须先解释产物为何变化"。**非空转已实证**：还原尝试 B 后新用例立刻报红「files=4015 bytes=19293011 … 偏离已确认基线 4018」——即「若当时有此用例，会在同一分钟发现少了 3 个文件」。全量 **309 passed / 19 ignored**（+1）
- [x] T10bw（M2）`cut_gui` 与直接步骤的**交叉证据：两处都不可去** ✅（§9.92，**本轮无入库改动**，诊断探针已移除）：两个方向都已实测——**A** 只有 `cut_gui`（两处移除直接步骤）→ 闸门报红 `OnlyInA` **62 项** sprite（直接步骤产出 `cut_gui` **产不出**的东西）；**B** 只有直接步骤（`cut_gui` 出计划）→ 产物 **4018→4015 文件 / 19294735→19293011 字节**（`cut_gui` 产出直接步骤**产不出**的 3 个文件）。**两个方向互为反证 ⇒ 两处各自产出不同产物 ⇒ "去重"前提不成立**，足以判定 **M2 收尾不能靠删除任一处完成**，无需再产出逐文件清单。**为何没拿到逐文件清单（如实说明）**：原计划用 `opts.run_gui_surgeon` 隔离，实测**不可行**——`invoke_conversion_ex`（被 `legacy_run` 调用）**自身无条件调用** `run_direct_steps`（`invoke_conversion.rs:163`），驱动侧关掉只是少跑一遍，探针实测两次配置条目集合**完全相同**（`both=4138 cut_only=4138`，差集 0）即隔离失败；要真隔离须**改生产代码**（给调度器加"跳过某任务"的测试开关，或拆出带额外开关的内部函数），为避免生产代码背诊断脚手架故未做。**一个明确标注为未证实的推断**：B 少 **3** 个文件，而 A 的 `OnlyInB` 有 **6** 项（`gui/container/{blast_furnace,cartography_table,grindstone,loom,smoker,stonecutter}.png`），数量不吻合，**不能**据此断定"少的 3 个就是这 6 个里的 3 个"；若后续要合并两处，第一步应是产出这份清单
- [ ] T11（M3）收口：删旧路径与适配层；Foray `Rom` 退役切投影
  - [x] **T11a（M3）删除 `shared_data`** ✅（§9.93）：先看清它承载什么——全仓只有**两个键**：`pack_name`（写入=转换入口；读取=Bedrock 决定 `.mcpack` 文件名 + scheduler 的**进度显示标签**）与 `target_pack_format`（读取=旧 `adapt_java_shaders`，而原生实现已改为直接读 `pack.mcmeta`，§9.85）。**关键判断：两个键都是只读的** ⇒ 这个「任务间可变共享表」**从未被当可变共享表用过**，只是伪装成侧信道的**构造期数据**。**处置**：①`pack_name` → `HurrayContext` 的**只读构造期字段**（新增 `with_pack_name`/`pack_name()`，`new()` 兜底 `"resource_pack"`），`scheduler` 侧改为**显式参数**（仅用于 `ProgressTracker` 标签）；②`target_pack_format` → **直接删除**，旧 `adapt_java_shaders` 改从 `pack.mcmeta` 读（与原生同源，同一输入得同一个数）；③删除 `shared_data` 字段、`set_data`、`get_data`。**验收**：全量 **309 passed / 19 ignored**、`check --bins` 通过、警告 **84** 无新增、**绝对产物契约未变**（4018 文件 / 19294735 字节）⇒ 纯结构清理、零产物影响；残留引用 6 处**全为注释**。**关于"净减少"**：`git diff --stat` 为 +63/−30（**净增**），原因是本仓在每个改动点都写中文设计注释，注释量超过被删的 30 行；**删掉的机制是实打实的**，行数不降反升是注释风格所致
  - [ ] T11b（M3）`PathView` / `temp_dir` 主路径 / `PathBuf` 缓存键 / Foray `Rom` 退役切投影
    - [x] **M3 阻塞点精确重测：`temp_dir` 不在生产任务路径上** ✅（§9.112，**无代码改动**）：用**当前状态**重测真实包——`on`（生产口径）**46 原生 / 0 适配层**；`off`（闸门基线口径）1 / 45。**⇒ 生产配置下 46 个任务全部走原生实现，一个都不走旧 `converters`。** **修正 §9.104 的措辞**：`temp_dir` 的 95 处挂载点**确实不能删**，但理由不是"生产还要用旧闭包"，而是——**`off` 配置是闸门用来定义「旧管线基线」的路径**（`assert_equivalent(&legacy, &off)`），**删掉旧闭包路径 = 拆掉验证自己正确性的基线**。这是**验证依赖**而非**功能依赖**：区别在于后者只能先迁移任务，而前者只要**换一种方式建立基线**（冻结已知正确的产物快照，或改用 `legacy` 路径本身作基线）就能逐步退场。**同时修正 `temp_dir` 的构成**：`invoke_conversion.rs` 71（旧闭包挂载点，仅基线用）、`commands/overlay.rs` **47（与本管线无关）**、`version_converter.rs` 27（**唯一的真 I/O 层** = 解压/重打包，即"输入 zip→工作目录→输出 zip"这条**管线本身的形状**，非迁移脚手架）、`context.rs` 9、其余 ~11。**本轮决定：不派发 `gui_surgeon_tx`**——它已纯 `Tx` 形态且验收完备（§9.111），但派发需让 `cut_gui` 在**旧批次内部**以 `Tx` 执行，而 (A) 把批次切三段**违反 §9.42「旧批次不可拆分」**、(B) 在 workdir 壳里走 `Tx` 再落盘**引入物化往返（正是 M3 想消除的）**，两者都需专门一轮，不在无人值守下贸然做
    - **对完成度的如实表述**：M2「全部任务原生化」✅ **达成**（46/46 原生、适配层 0）；`GuiSurgeon` 本地化到 `Tx` ✅ **代码完成且验收完备**（§9.111）；`gui_surgeon_tx`**上生产** ✅ **已接线**（§9.113：位置不变、实现走 `Tx`，双闸门确认产物未变）
    - [x] **M3 ②-a 完成：冻结内容基线** ✅（§9.115）：新增第 21 个忽略用例 `real_pack_content_baseline_is_frozen`——把真实包产物 zip 每个条目取 `(路径, 长度, 内容 FNV-1a)`，排序聚合成**一个 u64 指纹**并钉成常量。**比 §9.91 绝对契约强**：后者只钉 `files`/`bytes` 两个计数，若两文件**互换内容**则计数不变、契约看不见；本用例会红。**逐条目清单**经 `AROM_BASELINE_DUMP` 导出 4018 行，已入库 `tools/arom-baseline.txt`（330 KB），指纹变化时可逐行 diff 定位。**实测指纹 `0x75bb3260e7f578a6`（4018 条目）**；**非空转已实证**（篡改常量即报红）。全量 **310 passed / 21 ignored**。**提交 `c750ede`**。**②的进度**：②-a ✅ → ②-b（把 `mixed_run` 派发路径接到生产入口 `version_converter` 里的 `invoke_conversion_ex`）⏭ → ②-c（删旧闭包 95 处 + `legacy_processor`/`legacy_eraser`）⏭
    - [ ] **M3 待办（用户指示）：`pilots/` 目录改名**——它现在装着几乎所有转换器的**原生实现主体**（`mod.rs` 9 千余行），"试点"之名已名不副实；候选 `native/` 或 `arom_tasks/`。**改名会触及大量 `crate::pilots::…` 引用**（含 `mixed_run` 派发表与各测试），应**单独一轮**且**只改名不夹带行为改动**，以便"产物不变"可验证
    - [x] **M3 第②项勘察：生产入口就是旧调度器路径 + ② 的可行切法** ✅（§9.114，**无代码改动**）：测量**生产入口链**——`Tauri 命令 → version_converter.rs:725 → invoke_conversion::invoke_conversion_ex → register_legacy_tasks（注册 46 个任务）+ scheduler.execute_version_conversion（旧调度器逐任务执行）+ execute_cleanup`。**`invoke_conversion_ex` 就是生产路径，不是测试专用**。**准确图景**：①生产**怎么跑**＝旧调度器 `run_named` 逐任务，其中 **46/46** 经 A-ROM 派发表改派到原生实现；②**旧闭包还在**（注册表仍挂 `converters/**`，95 处 `ctx.temp_dir()` 形态），只是**生产上不再被调用**；③旧闭包**只作为验证基线**（`off` 配置跑一遍，闸门用 `legacy` 与 `off` 一致定义"旧管线基线"）；④`temp_dir` 仍在是因为旧闭包仍注册 + `mixed_run` workdir 形态在用。**⇒ 第②项"移除旧闭包路径"的真实含义**＝从"旧调度器 + 改派到原生"变成"**原生执行、不再有旧调度器参与**"。**② 的可行切法（三步，各可独立验证）**：**②-a 冻结基线**（把旧管线产物做成可复现对照物，使闸门不再**必须**每次跑旧闭包——解掉"验证依赖"这个真正阻塞）；**②-b** 用 `mixed_run` 派发路径**替换** `version_converter` 里的 `invoke_conversion_ex` 调用（生产改纯原生 + 层序列化，闸门改与冻结基线对照）；**②-c** 删除旧闭包（95 处）与 `legacy_processor`/`legacy_eraser`。**②-a 是关键前置**：没有它，②-c 就是"拆掉自己的尺子"。**可用起点**：`mixed_run::run_with_legacy_tasks` 已是那条完整路径（materialize → 跑 → harvest → 序列化），只需接到生产入口
    - [x] **依赖链勘察（纠正"M3 = 四个独立待办"的理解）** ✅（§9.104，**无代码改动**）：①**没有 `PathView` 这个类型**——`arom/pathview.rs` 只有 `Materialized`/`Harvest`/`materialize`/`harvest`/`run_legacy`；M3 说的"适配层"是**「物化到磁盘 → 跑旧任务 → 收回成层」这套机制**，而 `materialize`/`harvest` **生产在用**（`mixed_run.rs` 9 处 + 夹具正题 4 处）**不能删**。②`temp_dir` 165 处按文件分布：`invoke_conversion.rs` **71**（其中 **48 处**是 `ctx.temp_dir()` + ~47 处把 `Path::new(temp_dir)` 传给旧转换器 = **注册表里旧闭包的挂载点**）、`commands/overlay.rs` **47**（**与本管线无关**的 overlay 命令）、`version_converter.rs` **27**（解压/重打包 = 真 I/O 层）、`context.rs` 9、其余 ~11。**关键判断**：那 48+47 处**不能删**——`NativeSwitches` 全关（`off`）时的行为**就是"全部任务走旧闭包"，而 `off` 正是闸门定义"旧管线基线"的路径**（`assert_equivalent(&legacy, &off)`）⇒ **删旧闭包路径 = 拆掉验证自己正确性的基线**。③**⇒ 四项是一条链，顺序不可颠倒**：①本地化 `GuiSurgeon` 到 Tx → ②移除旧闭包路径（95 处挂载点）→ ③删 `temp_dir` 主路径 → ④Foray `Rom` 退役 → ⑤`PathBuf` 缓存键（§9.103 判定"有调用方但运行时不生效"，须先澄清意图）。**每项以前一项为前提**；①未完成前动②③只会拆掉闸门基线。**`shared_data`（§9.93）能干净删掉正是因为它不在链上**（真旁路）。**⇒ M3 的下一件事是 §9.96 的 (B)：把 `GuiSurgeon` 逐函数移植到 `Tx`**（8 读/12 写/3 处 `fs::`（全在测试）/33 处路径拼接 + `SPRITE_MAP` ~87 条），双闸门验收
- [ ] T13（M2 收尾）`cut_gui` 本地化·方案 1：**接口勘察完成** ✅（§9.94，**未改动代码**）
  - [x] 步骤 1：补绝对契约测试 ✅（§9.91，第 19 个忽略用例钉死 4018 文件 / 19294735 字节）
  - [x] 步骤 2–5 的**前置勘察** ✅：量清真实工作量——**好消息**：`GuiSurgeon` 的 I/O 面远比"1244 行"暗示的小：`pool.load_texture` **8 次** / `pool.store_texture` **12 次** / `pool.commit_all` **4 次**（实测只做 `save`，**无删除、无延迟删除**）/ `fs::` **3 处** / `.join(` 33 处（纯路径拼接）。**并纠正两处我先前的错误读数**：①`load_texture` **并非无调用者**（调用写作 `pool.load_texture(...)`，我的 grep 漏了 `pool.` 前缀）——这是"grep 读数也要先验证"的第四次教训；②§9.90 说 GuiSurgeon 带延迟删除**也不准确**，延迟删除在别的模块。**真正的阻塞点只有两个**：①**签名**——`execute_transformation(ctx, pool, res)` 需要 `&HurrayContext` 与 `&ResolutionTransducer`，而原生 `PilotFn` 只有 `&mut Tx`（故除 stage 参数外还需把 `res` 传进来）；②**分辨率探测**——`sample_scales` 用 `image::open` 读盘（`resolution.rs` 仅 245 行、磁盘面**只有这 1 处**），改为**注入式读尺寸**即可
  - [ ] **下一步（修订后的顺序，比原方案省）**：先解阻塞②——给 `ResolutionTransducer` 加**不碰磁盘**的构造入口（注入"宽高读取器"），现有 `detect_resolution(&Path)` 变成它的薄包装；**独立可验证**（单测：同夹具下注入式与读盘式得到相同 scale）且**不改产物**。之后解阻塞①（含 stage 参数），再移植 `GuiSurgeon`、验证双入口（Early 3 文件 + Late 62 项，总产物精确 4018/19294735）、最后删薄壳
  - [x] **步骤 2 完成：`ResolutionTransducer` 支持注入式读尺寸** ✅（§9.95）：按修订顺序先解**阻塞②**。改法（单文件 `hurray/resolution.rs`）：提为常量 `ITEM_PROBES`/`BLOCK_PROBES`/`GUI_PROBES`；抽出采样算法本体 `sample_scales_with(relatives, base_px, read_dims)`（读尺寸由调用方给）；旧 `sample_scales(root,...)` 变薄包装；新增**注入式入口 `detect_resolution_with(read_dims)`**（不碰磁盘）；旧 `detect_resolution(&Path)` 改为「读盘 → 交给 `apply_samples`」；后半段（众数/三路优先级/混合告警/兜底）提为共用 `apply_samples`。**关键设计点**：`sample_scales_with` 对不可用路径**静默跳过、不在函数内记日志**——因为旧实现区分「文件不存在=静默」与「解码失败=warn」，该区分只能在**知道原因的那一层**（闭包内）表达；我第一版把 warn 放进 `sample_scales_with` 会让"不存在"也刷告警，**已改掉**。**验收**：新增单测 `injected_and_disk_entries_agree` —— 造「32x 物品+32x 方块+512 GUI」假包，两入口四项倍率全等；**非空转**断言注入侧确实采到 `2.0`（而非两侧都落 1.0 兜底后假装相等）；再跑三路全缺的空目录验兜底。全量 **310 passed / 19 ignored**（+1）、`check --bins` 通过、警告 **84**、**绝对产物未变**（4018/19294735）⇒ **纯重构零产物影响**；变更 1 文件 +164/−60
  - [ ] **步骤 3**：解**阻塞①**——驱动侧让原生任务拿到 `ResolutionTransducer`（stage 参数可一并定）；之后移植 `GuiSurgeon` 本体（8 读 / 12 写 / 3 处 `fs::` / 33 处路径拼接）、验证双入口、删薄壳
  - [x] **纠正 §9.98：放置模型不是阻塞点** ✅（§9.99，**无代码改动**）：§9.98 判定"原生 cut_gui 无法用当前机制派发（放置模型不允许插入旧批次内部）"——**该结论错误**。漏看了 `legacy_names` 的构造：`cut_gui` 不在派发表里 ⇒ **本来就在 `legacy_names` 中**，由 `run_named` **按计划顺序**执行，位置**正是 `(15,18)`**（既不在前阶段也不在后阶段）⇒ **放置模型无需改动**，真正阻塞只有一个：**原生任务拿不到 workdir**。**可行设计（三处小改动，都不碰 `native_for` 返回类型）**：①`pilots` 末尾**追加** `surgeon_cut_gui`，提供 **workdir 形态入口**（内部照抄旧 `cut_gui` 的 14 行：`TexturePool::new()` → `detect_resolution(workdir)` → `GuiSurgeon::execute_transformation` → `commit_all`）；②驱动**旧批次循环**里特判 `cut_gui`，在**同一位置**直接调 ①（`legacy_one_by_one`/`step_trace` 两分支同样处理）；③**延迟删除归属**——`GuiSurgeon` 把 20 个 atlas 文件登记进 context 的 cleanup 列表，而驱动末尾**无条件**调 `execute_cleanup()`，故需给 `HurrayContext` 加 `take_cleanup_paths()`（取出并清空），驱动前后各取一次做差集、经 `Outcome.deferred_removals` 上报并从 context 移除以免重复。**定位**：让 `cut_gui` 的**计划槽位先原生化**（`native_names` 会出现它），算法仍是 `GuiSurgeon` 本体；**代价**是它仍直接读写磁盘（"IO 本地化"未做），**收益**是低成本可完整验证的一步，且**不堵死**全量本地化（将来把 ① 内部换成逐函数移植即可，接口不变）。**验收必须全跑**：绝对产物契约（4018/19294735）+ 三配置两两一致 + 19 忽略用例 + 警告不增（84）。**降低风险的一点**：`GuiSurgeon` 对分辨率**不敏感**（`let _ = detect_resolution(...)` 忽略返回值），故 ① 怎么探测对产物无影响
  - [x] **步骤 3 的接口结论 + 完整移植配方** ✅（§9.96，**无代码改动**，交接用）：**推翻"需要改驱动签名"的判断**——①`Tx` **已有** `image()`（`layer.rs:702`，返回 `Arc<RgbaImage>`）与 `put_image()`（`layer.rs:713`）；②上一步刚做的 `detect_resolution_with(read_dims)` 正好接受「路径→宽高」闭包，两者一拼 **原生任务可自己探测分辨率**（`res.detect_resolution_with(|rel| tx.image(rel).ok().map(|i| i.dimensions()))`；**注意借用顺序：先探测后写入**，闭包只在调用期间不可变借用 `tx`）；③**stage 参数也不需要**——`cut_gui` 在计划里只占 `(15,18)` **一个**位置，另一处是**计划之外**的 `run_direct_steps`（§9.92 只说明"两处各自载荷"，**不**说明任务要知道自己是 Early/Late）⇒ **无需改 `PilotFn`、无需 `NativeImpl`、无需 stage**。**机械替换表（已逐条核对）**：`base_path.join(x)`→`x`（33 处）；`pool.load_texture(&p)`→`tx.image(x).ok().map(|i| (*i).clone())`（8 处：275/353/421/723/867/992/1039/1085，**类型不同需解引用克隆**）；`pool.store_texture`→`tx.put_image`（12 处：255/284/359/381/403/786/846/900/909/1090/1113/1114）；`pool.commit_all()`→**删除**（4）；`ctx.temp_dir()`→删除；`ctx.defer_remove_file`→登记 `Outcome.deferred_removals`（§9.72）；唯一需改签名的辅助函数是 `save_slices`（210–261，用 `get_texture`+`store_texture`）。**两个必须照抄的语义**：①清理清单（310–331，20 文件）**不含 `container/inventory.png`**（1.21 客户端仍需它渲染生存背包背景）；②清理**必须走延迟**，立即删会让更晚任务看不到文件（§9.23）。**已核实**：该文件 3 处 `fs::` **全在测试里**，**生产路径 `fs::` 为 0**。**本轮因无人值守+即将关机，评估"一回合内完成移植并跑完全部验证"风险不可接受，故不新增代码**，仅落盘配方；仓库停在 `c131016`（绿色：310 passed / 19 ignored / 警告 84）
- [ ] T12（全程）影响面核对表：对外承诺逐条验证
