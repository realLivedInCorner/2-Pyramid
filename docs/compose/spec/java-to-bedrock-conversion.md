---
feature: java-to-bedrock-conversion
status: in-progress
updated: 2026-08-23
branch: feat/java-bedrock-convert
commits: b7524b5..3e90121
---

# Java ↔ Bedrock 资源包双向转换

## Report

**What was built** — 在既有模块化 j2b/b2j 上补齐网络调研差异：j2b 生成 `textures_list.json`、`terrain_texture.json`、`item_texture.json`（shortname=文件名 stem），`colormap→colormaps`，音效定义双写 `sounds/sound_definitions.json` 与包根 `sounds.json`。b2j 删除索引文件、`colormaps→colormap`，音效优先读包根再 `sounds/`。仍经 Scheduler Exclusive/Surgeon 边任务挂载。

**Verification** — `cargo test --offline --manifest-path src-tauri/Cargo.toml`：89 passed（含 bedrock 12）。`npm run build`：PASS。独立 review（general-2）：T6–T10 全 PASS，无 critical；review 后补了扩展名大小写、包根 `item_texture.json` 剥离、根 sounds 解析失败回退 `sounds/sound_definitions.json`。

**Journey log**
- worktree 创建被环境拦截 → `feat/java-bedrock-convert` 功能分支隔离 master。
- 废弃巨型 `bedrock.rs`，按职责拆目录并用 Scheduler 边任务挂载。
- j2b 须在删除 `pack.mcmeta` **之前**读取 description；font 须在 textures 提升前抽出。
- GuiSurgeon 在 Bedrock 中间态跳过（`invoke_conversion_ex`）。
- atlas shortname / flipbook `atlas_tile` 统一用文件名 stem；sounds 可能在包根或 `sounds/`。
- 实测修正：netherite 在 Bedrock 与 Java **同名**，先前 `*_netherite` 改写是错误的；药水/床/草/UI container 需按 vanilla item_texture 对齐。

## [S1] Problem

「Bedrock Latest」目标目前只是薄层目录重组，且 **拒绝 `.mcpack` 输入**。用户需要：

1. **j2b**：Java 资源包 → 可用的 Bedrock `.mcpack`
2. **b2j**：Bedrock `.mcpack`/zip → 可用的 Java 资源包（任意 Java pack_format 目标）
3. **平台独有内容直接丢弃**：不尝试保留/翻译对端不存在的系统（模型 JSON、geometry、flipbook 专用结构、Ore UI、OptiFine 等）

`min_engine_version` 仅用于固定可用性基线（本设计取 `[1, 20, 0]`），不承担功能语义。

## [S2] Design

### 共同原则

| 原则 | 含义 |
|---|---|
| 对称中间态 | 两端都先落到「Java 1.21.11 / pack_format 75」可识别的纹理树，再走版本边或打包 |
| 丢弃独有 | 源端独有、目标端无对应运行时的目录/文件 **删除**，不伪造半成品 |
| 保守改名 | 仅使用下表中的确定性别名；未列出的文件名保持原样 |
| 成对改名 | `foo.png` / `foo.png.mcmeta` / `foo.tga` 必须一起改，避免拆散动画对 |

### j2b（Java → Bedrock）

入口：`pack_format2 == 1000`，或用户选择 Bedrock Latest。

```
zip → 规整 → Java 流水线 → 75 → convert_java_to_bedrock → .mcpack
```

| 步骤 | 行为 |
|---|---|
| 图标 | `pack.png → pack_icon.png`；删除 `pack.mcmeta` |
| 字体 | `textures/font → font/`；`ascii.png → default8.png` |
| 贴图 | `assets/minecraft/textures → textures/`（合并） |
| 目录 | `item→items`（目标不存在时）；`block→blocks`（同上） |
| 改名 | items/blocks 下按下表改名 |
| GUI | `gui/**` 全部合并进 `textures/ui/`；`ui/creative_inventory/*` 上提 |
| 动画 | `**/*.png.mcmeta` 的 `animation` → `textures/flipbook_textures.json`；删除 Java mcmeta |
| 语言 | `lang/*.json` → `texts/<Lang_REGION>.lang`（前缀映射 + 其余原样） |
| 音效 | `assets/minecraft/sounds/** → sounds/**`；`sounds.json` → `sounds/sound_definitions.json` |
| 剥离 | 见「j2b 剥离」 |
| manifest | format 2、UUID、`min_engine_version [1,20,0]`、`metadata.generated_with` |

**j2b 剥离（丢弃 Java 独有）**  
`blockstates`, `models`, `shaders`, `atlases`, `particles`, `equipment`, `items`(json), `font` providers, `post_effect`, `waypoint_style`, `optifine`, `mcpatcher`, `cit`, `emissive`；根级 `optifine`/`mcpatcher`。

### b2j（Bedrock → Java）

入口：检测到 Bedrock 包（见检测规则）且目标为 Java pack_format；或未来显式源选择。

```
mcpack/zip → 解包 → convert_bedrock_to_java（→ 伪 Java 75 树）→ Java 流水线到目标 format → .zip
```

| 步骤 | 行为 |
|---|---|
| 图标 | `pack_icon.png → pack.png` |
| 元数据 | 从 `manifest.json` 读 name/description → 生成 `pack.mcmeta`（`pack_format` 由后续流水线写入目标值） |
| 贴图 | `textures → assets/minecraft/textures` |
| 目录 | `items→item`；`blocks→block` |
| 改名 | 反向别名表 |
| 字体 | `font/default8.png → assets/minecraft/textures/font/ascii.png`；其余 `font/*` 合并进 `textures/font/` |
| GUI | `textures/ui → assets/minecraft/textures/gui/container`（无 container 语义时仍放此目录，Java 能加载 gui 贴图） |
| 语言 | `texts/*.lang` → `assets/minecraft/lang/<java_code>.json`（反向前缀映射） |
| 音效 | `sounds/** → assets/minecraft/sounds/**`；`sound_definitions.json` → 尽力生成 `assets/minecraft/sounds.json`（无定义则跳过） |
| 剥离 | 见「b2j 剥离」 |

**b2j 剥离（丢弃 Bedrock 独有）**  
`manifest.json`, `pack_icon.png`（已改名）, `flipbook_textures.json`, `textures_list.json`, `terrain_texture.json`, `blocks.json`, `biomes_client.json`, `splashes.json`, `bug_pack_icon.png`, `animation_controllers/`, `animations/`, `attachables/`, `entity/`(json 定义), `fogs/`, `particles/`(json), `materials/`, `models/`(geo), `render_controllers/`, `items/`(json 定义，注意与 textures/items 区分), `ui/*.json`（UI 脚本，非贴图）, `texts/languages.json`, `texts/language_names.json`, PBR/`atmospherics`/`color_grading`/`cubemaps`/`lighting`/`local_lighting`/`shadows`/`water`/`pbr` 目录。

### 检测规则（b2j 触发）

解压后满足任一即视为 Bedrock 源：

1. 根目录存在 `manifest.json`，且 JSON 含 `modules` 且任一 `type == "resources"`
2. 根目录存在 `pack_icon.png` 且 **不存在** `pack.mcmeta`，且存在 `textures/` 或 `manifest.json`

`.mcpack` 与 `.zip` 均允许作输入（前端去掉拒绝逻辑）。

### 别名表

| Java | Bedrock |
|---|---|
| `golden_apple` | `apple_golden` |
| `golden_carrot` | `carrot_golden` |
| `golden_*` | `gold_*` |
| `wooden_*` | `wood_*` |
| `music_disc_*` | `record_*` |
| `netherite_<part>` | `<part>_netherite` |
| `golden_horse_armor` | `horsearmor_gold` |
| `iron_horse_armor` | `horsearmor_iron` |
| `diamond_horse_armor` | `horsearmor_diamond` |
| `leather_horse_armor` | `horsearmor_leather` |
| `recovery_compass` | `compass_recovery` |

b2j 使用同一表的逆映射（冲突时 Java 名优先作为规范形）。

### 语言键映射

| Java | Bedrock |
|---|---|
| `block.minecraft.X` | `tile.X.name` |
| `item.minecraft.X` | `item.X.name` |
| `entity.minecraft.X` | `entity.X.name` |
| `enchantment.minecraft.X` | `enchantment.X.name` |

代码：`en_us ↔ en_US`，`zh_cn ↔ zh_CN`。无法映射的键：j2b 原样写入；b2j 原样写入 JSON。

### 流水线接线（version_converter::process_zip）

1. 解压到 temp
2. 若检测为 Bedrock 源 → `convert_bedrock_to_java`，得到 `pack.mcmeta`（临时 format 75）+ Java 树；`source_version = 75`
3. 走现有 `invoke_conversion` 到 `java_target`
4. 若 `pack_format2 == 1000` → `convert_java_to_bedrock`，扩展名 `.mcpack`
5. 否则 `.zip`

### 测试边界

- j2b：现有夹具 + block/blocks、icons→ui、ascii→default8、flipbook、lang、sounds、剥离、manifest
- b2j：对称夹具（items→item、record→music_disc、default8→ascii、ui→gui/container、lang 反向、manifest→mcmeta、bedrock json 目录被删除）
- 检测：有 manifest resources 模块 → 识别为 Bedrock

### 网络调研补充（2026-08-23）

来源：Minecraft Wiki Resource pack、Bedrock Wiki pack-structure / texture-atlases / flipbook-textures / textures-list。

| 差异 | Java | Bedrock | 转换策略 |
|---|---|---|---|
| 贴图缓存 | 无 | `textures/textures_list.json` 数组（无扩展名路径） | j2b **生成**；b2j 删除 |
| 方块 atlas | 路径即用 | `terrain_texture.json` shortname → path | j2b 为 blocks 下 png 写入 shortname=文件名 stem |
| 物品 atlas | 路径即用 | `item_texture.json` shortname → path | j2b 为 items 下 png 写入 |
| flipbook 锚点 | `.png.mcmeta` | `atlas_tile` 须对齐 shortname | 现用文件名 stem；生成 atlas 后一致 |
| 色图目录 | `textures/colormap/` | `textures/colormaps/` | j2b 改名；b2j 反向 |
| 音效定义 | `assets/.../sounds.json` | `sounds/sound_definitions.json` **且** 包根 `sounds.json` | j2b 写 `sounds/sound_definitions.json` + 根 `sounds.json`（同内容）；b2j 优先读根再读 sounds/ |
| 字体 | `textures/font` png + font json | `font/` png/ttf/metadata | 已做 png；json 继续剥离 |

## [S3] Out of Scope

- Java model/blockstate ↔ Bedrock geometry/attachables 完整互转
- Ore UI 覆盖；PBR/Vibrant Visuals
- 语言/音效 100% 键级对照
- UI 去掉「未完成」警示的文案大改（可保留 beta 标识）
- 自动选择「最新」Java pack_format 以外的中间版本策略（固定 75）
- 为**自定义**新方块生成完整 block definition（仅覆盖原版贴图路径）

### 模块与调度（实现约束）

- 代码在 `converters/bedrock/{mod,mapping,textures,metadata,fsutil,j2b,b2j}.rs`，各子模块自带 `#[cfg(test)]`。
- `invoke_conversion` 仅调用 `bedrock::register_tasks`，**不写转换逻辑**。
- 任务名 `bedrock_java_to_bedrock` / `bedrock_bedrock_to_java`，`TaskType::Exclusive` + `TaskTier::Surgeon`。
- 版本边：forward `(84,1000)`、reverse `(1000,84)`。
- `process_zip`：Bedrock 源先 `execute_version_conversion(1000,84)`；目标 1000 时 Java 管线到 75 后 `execute_version_conversion(84,1000)`。

## Tasks

已完成（T1–T5）：

- [x] T1: `converters/bedrock/` 模块化 + 各子模块单测 — acceptance: bedrock 过滤测试 12 绿 (covers: S2)
- [x] T2: Scheduler 注册 Exclusive/Surgeon 任务与版本边 — acceptance: invoke_conversion 仅 register_tasks (covers: S2)
- [x] T3: process_zip 检测 Bedrock 源并经 Scheduler 跑 b2j/j2j — acceptance: 无直调 convert_java_to_bedrock (covers: S2)
- [x] T4: 前端允许 `.mcpack` 拖入/选择 — acceptance: zip+mcpack 过滤 (covers: S2)
- [x] T5: `cargo test --offline` 89 passed；`npm run build` 成功 — acceptance: 退出码 0 (covers: S2)

待实现（调研增补，**已批准 ALL**）：

- [x] T6: j2b 生成 `textures/textures_list.json`（扫描 textures 下 png/tga，写无扩展名相对路径）— acceptance: j2b 夹具含列表且路径正确；b2j 删除该文件 (covers: S2 网络调研)
- [x] T7: j2b 生成 `textures/terrain_texture.json` + `item_texture.json`（shortname=文件名 stem，path 相对包根）— acceptance: stone→`textures/blocks/stone`；flipbook `atlas_tile` 与 shortname 一致 (covers: S2 网络调研)
- [x] T8: colormap 目录互转 — acceptance: j2b `colormap→colormaps`；b2j 反向 (covers: S2 网络调研)
- [x] T9: 音效双写/双读 — acceptance: j2b 同时写 `sounds/sound_definitions.json` 与包根 `sounds.json`；b2j 优先根目录再 sounds/ (covers: S2 网络调研)
- [x] T10: 回归验证 — acceptance: `cargo test --offline` 全绿；`npm run build` PASS (covers: S2)

用户实测反馈修正（T11）：

- [x] T11: 修复可见贴图 — 药水 bottle 命名、bed 颜色前缀、grass_block_*、**撤销**错误的 netherite→*_netherite、gui/container 扁平到 ui/、totem/slimeball 等别名 — acceptance: mapping 单测 + bedrock 过滤测试通过 (covers: S2 网络调研)

模块约束不变：逻辑落在 `textures.rs` / `metadata.rs`（或新增小函数），各模块保留 `#[cfg(test)]`；Scheduler 挂载方式不变。

## 审查时请确认

1. 是否批准 T6–T10 全部实现，或只要子集（例如只要 T6+T8）。
2. atlas shortname 用「文件名 stem」是否足够（不引入 namespace: 前缀）。
3. 根 `sounds.json` 与 `sounds/sound_definitions.json` 双写是否可接受。
