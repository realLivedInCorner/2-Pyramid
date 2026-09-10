---
feature: java-to-bedrock-conversion
status: designed
updated: 2026-08-23
branch: (pending)
commits: (pending)
---

# Java → Bedrock 资源包转换完善

## Report

（实现并验证后填写）

## [S1] Problem

当前「Bedrock Latest」目标（`pack_format=1000`）是实验性流水线：先把包转到 Java 1.21.11（75），再做一层很薄的目录重组。CHANGELOG 与 UI 均标明「未完成、存在严重问题」。对照 Minecraft Wiki / Bedrock 资源包约定后，主要缺口如下：

1. **路径映射不全**
   - 只处理了 `textures/item → textures/items`，未处理 `textures/block → textures/blocks`。
   - `textures/gui` 只搬了 `container`，其余（`icons.png`、`options_background.png` 等）仍留在 `gui/`，基岩不会加载。
   - 字体：`font/ascii.png` 未映射为基岩的 `default8.png`。

2. **物品/方块 id 命名差异覆盖过窄**
   - 仅有 `golden_apple→apple_golden`、`golden_*→gold_*`、`wooden_*→wood_*`。
   - 缺常见别名：`music_disc_*→record_*`、`netherite_*→*_netherite`、马铠、`recovery_compass` 等。

3. **Java 专用内容仍被打进 `.mcpack`**
   - `blockstates` / `models` / `shaders` / `atlases` / `particles` / `font` providers json / `optifine` / `mcpatcher` 等基岩不会读，徒增体积与混淆。

4. **动画与元数据未转换**
   - `.png.mcmeta` 原样保留；基岩动画走 `textures/flipbook_textures.json`。
   - `pack.mcmeta` 保留在包内（基岩不识别）。
   - `assets/*/lang/*.json` 未转为 `texts/*.lang`。
   - `sounds.json` + `sounds/` 未映射为 `sounds/sound_definitions.json`。

5. **manifest 过旧**
   - `min_engine_version: [1,16,2]`，无 `metadata.generated_with`，不利于识别与排查。

## [S2] Design

### 总体流程（不变）

```
输入 zip
  → 目录规整（pack.mcmeta 提升）
  → Java 流水线 → pack_format 75
  → Bedrock 阶段（本设计增强 converters/bedrock.rs）
  → 打包 .mcpack
```

入口仍在 `version_converter::process_zip`：`pack_format2 == 1000` 时先转 Java 75，再调用 `bedrock::convert_java_to_bedrock`。

### Bedrock 阶段步骤（目标行为）

| 步骤 | 行为 |
|---|---|
| 图标/元数据 | `pack.png → pack_icon.png`；**删除** `pack.mcmeta` |
| 字体 | `textures/font → font/`；`ascii.png → default8.png` |
| 贴图提升 | `assets/minecraft/textures → textures/`（合并，不覆盖已有） |
| 目录名单数化 | `item→items`（已存在则跳过）；**新增** `block→blocks` |
| id 改名 | 在 items/blocks 上对 png / png.mcmeta / tga 成对改名 |
| GUI | `gui/container → ui`；**其余 gui/* 合并进 ui/** 后删 gui |
| 创造栏 | `ui/creative_inventory/* → ui/` 后删目录 |
| 动画 | 扫描 `**/*.png.mcmeta` 的 `animation`，写入 `textures/flipbook_textures.json`（`flipbook_texture` / `atlas_tile` / `ticks_per_frame`），并删除 Java mcmeta |
| 语言 | `assets/minecraft/lang/*.json` → `texts/<Lang_REGION>.lang`；键前缀尽力映射（见下表） |
| 音效 | `assets/minecraft/sounds/** → sounds/**`；`sounds.json` → `sounds/sound_definitions.json` |
| 剥离 | 删除 Java 专用目录与根级 optifine/mcpatcher 等 |
| 收尾 | 清理空 `assets/`；写 `manifest.json` |

### id 改名表（保守，避免误伤）

| Java stem | Bedrock stem |
|---|---|
| `golden_apple` | `apple_golden` |
| `golden_carrot` | `carrot_golden` |
| `golden_*` / `wooden_*` | `gold_*` / `wood_*`（既有规则） |
| `music_disc_*` | `record_*` |
| `netherite_<tool/armor>` | `<tool/armor>_netherite` |
| `*_horse_armor` | `horsearmor_<mat>`（leather/iron/diamond/gold） |
| `recovery_compass` | `compass_recovery` |

不在表内的名字一律保持不动。

### 语言键映射（尽力）

| Java | Bedrock |
|---|---|
| `block.minecraft.X` | `tile.X.name` |
| `item.minecraft.X` | `item.X.name` |
| `entity.minecraft.X` | `entity.X.name` |
| `enchantment.minecraft.X` | `enchantment.X.name` |
| 其他键 | 原样写出（菜单/自定义） |

语言代码：`zh_cn → zh_CN`、`en_us → en_US`。

### sound_definitions 结构

```json
{
  "format_version": "1.14.0",
  "sound_definitions": {
    "<event>": {
      "category": "<java category or block>",
      "sounds": [{ "name": "sounds/<path>", "stream": false }]
    }
  }
}
```

无扩展名路径补 `sounds/` 前缀；已是 `sounds/` 开头则原样。

### manifest

- `format_version: 2`
- `min_engine_version: [1, 20, 0]`
- `header` / `modules`：`type: resources`，独立 UUID
- `metadata.generated_with["2-Pyramid"] = [CARGO_PKG_VERSION]`
- `metadata.authors = ["2-Pyramid"]`

### 剥离清单（目录）

`blockstates`, `models`, `shaders`, `atlases`, `particles`, `equipment`, `items`（Java 1.21 item model）, `font` providers json, `post_effect`, `waypoint_style`, `optifine`, `mcpatcher`, `cit`, `emissive`；以及根目录 `optifine` / `mcpatcher`。

### 测试边界（单元测试覆盖）

在 `converters/bedrock.rs` 现有测试上扩展夹具：

1. `block/` 提升为 `blocks/`，`item/` 为 `items/`
2. `ascii.png` → `default8.png`
3. `music_disc_13` → `record_13`
4. `water_still.png.mcmeta` → flipbook 条目且 mcmeta 删除
5. `gui/icons.png` → `textures/ui/icons.png`
6. `lang/zh_cn.json` → `texts/zh_CN.lang` 含 `tile.stone.name=…`
7. `sounds.json` + ogg → `sound_definitions.json` + `sounds/...`
8. `blockstates`/`models` 不出现在产物
9. `pack.mcmeta` 已删除；manifest 含 `min_engine_version [1,20,0]` 与 `metadata`

## [S3] Out of Scope

- **不做** Java JSON model → Bedrock geometry / attachables 完整转换（无通用算法，需人工建模）。
- **不做** Ore UI 覆盖（官方限制资源包无法修改 Ore UI）。
- **不做** Bedrock → Java 反向转换。
- **不做** PBR / Vibrant Visuals / MERS 材质生成。
- **不做** 语言与音效的 100% 键级对照表（仅常见前缀 + 原样透传）。
- **不改** 前端版本选择与 `.mcpack` 扩展名逻辑（已存在）。
- UI 警告文案是否从「未完成」改为「实验性、模型/UI 不完整」— 可作为可选后续，本规格不强制。

## Tasks

- [ ] T1: 扩展 `convert_java_to_bedrock` 路径映射（block→blocks、gui 其余→ui、ascii→default8、删 pack.mcmeta）— acceptance: 单测断言这些路径 (covers: S2)
- [ ] T2: 扩展 id 改名表并在 items+blocks 上成对处理 png/mcmeta/tga — acceptance: music_disc/netherite/horsearmor 夹具通过 (covers: S2)
- [ ] T3: 生成 flipbook_textures.json 并移除已消费的 .png.mcmeta — acceptance: water_still 夹具含 ticks_per_frame 且无残留 mcmeta (covers: S2)
- [ ] T4: lang JSON → texts/*.lang 与 sounds.json → sound_definitions.json — acceptance: zh_CN.lang 与 sound_definitions 夹具通过 (covers: S2)
- [ ] T5: 剥离 Java 专用目录 + 更新 manifest（min_engine 1.20.0 + metadata）— acceptance: blockstates/models 不在产物；manifest 字段断言 (covers: S2)
- [ ] T6: 跑 `cargo test --offline --manifest-path src-tauri/Cargo.toml` 与 `npm run build` — acceptance: 命令退出码 0，bedrock 模块测试全绿 (covers: S2)

## 审查时请确认

1. 改名表是否足够/过激（尤其 `netherite_*` 方向）。
2. `min_engine_version` 用 `[1,20,0]` 还是保持 `[1,16,2]`。
3. 动画：只写 flipbook 条目是否可接受（不生成完整 `terrain_texture.json` 映射时，部分贴图可能仍需用户手工 atlas 键）。
4. 是否允许在本仓库 `master` 上直接实现，或需要独立 worktree/分支。
