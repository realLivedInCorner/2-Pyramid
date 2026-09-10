# Beta-2.1.0（BUILD 20035）

> 测试通道发布。可与正式版并存安装。产物命名示例：`2-Pyramid-Installer-2.1.0-beta.20035.exe`（请附带同名 `.sha256`）。

## 概要

本版本重点是 **界面动效体验** 与 **Java ↔ 基岩资源包双向转换**。基岩链路仍为实验性，模型/部分 UI 不可能 100% 等价；欢迎用真实资源包反馈问题。

## 新增

- 界面动画系统重做 + 设置项「界面动画」（开启 / 跟随系统 / 关闭）
- 页面切换交叉淡入淡出 + 模糊衔接；动画速率三档作用于完整页切换
- 基岩 **j2b / b2j** 模块化实现，Scheduler 边任务挂载
- 输入接受 `.mcpack` / `.zip`；检测基岩源自动先转 Java
- j2b 生成 `textures_list.json`、`terrain_texture.json`、`item_texture.json`
- 音效定义双写：`sounds/sound_definitions.json` + 包根 `sounds.json`

## 调整 / 修复

- 药水、床、草方块、桶、弓弩、合金（撤销错误 netherite 改名）等贴图路径对齐 vanilla Bedrock
- 床从 `blocks/` 复制到 `items/`；item atlas 使用 vanilla 数组 shortname
- UI：`gui/**` → `textures/ui/` 并扁平 `ui/container/*`
- Bedrock 中间态跳过 GuiSurgeon
- 快速切页叠影、动画速率未作用于 enter/leave 等 UI 问题

## 已知限制

- Java model / blockstate ↔ Bedrock geometry 不互转
- Ore UI 无法被资源包修改
- 语言 / 音效键级映射为尽力而为
- 请勿将 Bedrock 目标视为生产级输出

## 完整变更

见仓库 `CHANGELOG.md` 的 `[2.1.0]` 小节。

## 安装与通道

1. 下载 `2-Pyramid-Installer-2.1.0-beta.20035.exe`（及 `.sha256`）
2. 或在应用内将更新通道设为 **测试版** / **全部** 后检查更新
3. 正式版用户请继续使用 Stable / `v` 前缀发布

## 验证摘要（开发者）

- `cargo test --offline --manifest-path src-tauri/Cargo.toml`：91 passed
- `npm run build`：PASS
- 分支：`feat/java-bedrock-convert` → 已合入 `origin/master`（`ffe2ee3`）
