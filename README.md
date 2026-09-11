<!-- markdownlint-disable MD033 MD036 -->

<p align="center">
  <img src="./src/assets/logo-256.png" width="160" alt="2-Pyramid logo">
</p>

<h1 align="center">2-Pyramid</h1>

<p align="center">
  <strong>跨任意版本转换 Minecraft 资源包 · The Nextgen Multi-Version Universal Resource Pack Converter</strong>
</p>

<p align="center">
  <img alt="Version" src="https://img.shields.io/badge/version-2.1.1-007bff?style=flat-square">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows-0078D4?style=flat-square">
  <img alt="License" src="https://img.shields.io/badge/license-MIT-22c55e?style=flat-square">
  <img alt="Tauri" src="https://img.shields.io/badge/built%20with-Tauri%202-FFC131?style=flat-square">
  <img alt="Vue" src="https://img.shields.io/badge/Vue-3.5-4FC08D?style=flat-square">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-orange?style=flat-square">
</p>

---

## 中文

**2-Pyramid** 是一款 Windows 桌面端的 Minecraft 资源包版本转换器，覆盖从 1.6 到最新 26.2 的 Java 目标版本区间，支持任意两个版本之间的相互转换，并支持 **Java ↔ Bedrock（基岩）** 结构互转（实验性）。

### ✨ 特性

- **广覆盖** — 26 个 Java 目标版本区间，六个时代（Classic / Modern / Caves & Cliffs / Trails & Tales / Tricky Trials / Bundles of Bravery）
- **Java ↔ Bedrock（实验性）**
  - **j2b**：任意 Java 包 → 先转到最新 **Java 26.2（pack_format 88）** → 结构重组 → `.mcpack`
  - **b2j**：检测 `.mcpack` / 基岩 zip → 先落到 Java 26.2 树 → 再转到所选 Java 目标 → `.zip`
  - 贴图别名对齐 vanilla Bedrock（药水瓶、床、桶、木板/原木、盔甲层等）
  - 生成 `gui/icons.png` HUD 图集、`textures_list.json`、容器界面扁平化与 POT 适配
  - 平台独有内容按方向剥离（不互转 Java model ↔ Bedrock geometry）
- **完全本地** — 无云端、无账号、无遥测，文件全程不离开你的电脑
- **批量处理** — 一次拖入多个资源包，1–4 线程并行转换
- **目录规整防呆** — 自动把嵌套的 `pack.mcmeta` 提升到压缩包根目录；`pack.mcmeta.txt` 之类多扩展名文件只要内容是合法 mcmeta（能解析出 format 数值）也会统一改名为 `pack.mcmeta`
- **动态贴图转换** — 老版 `{"animation": {}}` 的 `.png.mcmeta` 自动按贴图尺寸推导帧数，改写为高版本 `frametime` + `interpolate` 格式
- **输出命名模板** — `[Ver]` / `[Name]` / `[Time]` / `[Date]` 占位符自由组合，再转换时自动替换名称中的旧版本前缀
- **Overlay 母包叠加** — 在不修改原包的前提下，把自定义覆盖包叠加到任意母包上
- **界面动画** — 页面切换交叉淡入淡出 + 模糊衔接；设置可选开启 / 跟随系统 / 关闭；速率三档作用于完整页切换
- **深度定制 UI** — 自定义背景（自动提取主题色）、玻璃 / 磨砂控件皮肤、中英双语
- **自研安装器** — 无需管理员权限的 HKCU 安装，OOBE 分步向导，可选桌面 / 开始菜单快捷方式，重装时安装目录自动对齐现有位置；卸载器走完动画自动退出并自行清理
- **Beta 双渠道** — 正式版与 Beta 版可并存安装（独立注册表、独立目录、Beta 标识），`betabuild` 一键构建

### 🚀 快速开始

#### 用户（直接使用）

1. 前往 [Releases](https://github.com/realLivedInCorner/2-Pyramid/releases) 下载 `2-Pyramid-Installer-{版本}.exe`（Beta 版为 `2-Pyramid-Installer-{版本}-beta.{BUILD}.exe`）
2. 安装后启动，首次运行跟随 OOBE 引导配置即可

#### 开发者（本地运行）

```bash
git clone git@github.com:realLivedInCorner/2-Pyramid.git
cd 2-Pyramid

npm install        # 前端依赖
npm run 2pyr       # Tauri dev 模式（Rust 后端 + Vite 前端）
```

常用脚本：

| 命令 | 作用 |
|---|---|
| `npm run dev` | 仅启动 Vite 前端（无 Rust 后端） |
| `npm run 2pyr` | Tauri dev 模式 |
| `npm run build` | 仅构建前端 |
| `npm run buildrelease` | 正式版完整构建（前端 + 主程序 + 自研安装器 → `release/`） |
| `npm run betabuild` | Beta 渠道构建（输出 `-beta.{BUILD}.exe`，与正式版可并存） |
| `npm run buildrelease:nobump` / `npm run betabuild:nobump` | 同上但不递增 BUILD 构建号 |

> 构建号说明：`BUILD` 文件由主程序 `build.rs` 在 release 编译时**唯一递增一次**；`--no-bump` 通过环境变量 `2PYR_NO_BUMP=1` 完全跳过递增。

### 🔄 更新机制

应用内的「检查更新」读取本仓库的 GitHub Releases。Release tag 约定：

| Tag 前缀 | 含义 | 可见通道 |
|---|---|---|
| `Safe-2.0.3` | 重要安全更新（强制提醒） | 全部 |
| `Stable-2.0.3` | 稳定版 | 稳定通道 / 全部 |
| `v2.0.3` | 稳定版（无前缀） | 稳定通道 / 全部 |
| `UnStable-2.0.3` / `Beta-2.0.3` | 测试版更新 | 测试通道 / 全部 |

更新通道为三态：**稳定版**（仅稳定发布）/ **测试版**（仅测试发布）/ **全部**（同时接受两个通道的更新内容，取最高版本）。
更新源可切换：**镜像源**（`cdn.5eggpack.top`，国内加速，第三方维护）/ **GitHub 官方**，设置页内置测速与「使用最快源」。

发版时记得给 Release 附带 `.exe` 安装包附件及**同名 `.sha256` 校验文件**（流水线自动生成，更新器会做完整性校验；更新会拉起图形安装向导）。

### 🏗️ 架构

2-Pyramid 的核心是自研的 **DTD Pipeline** 与 **BFS Scheduler**：

```
        ┌───────────────────────────────────────────────────────┐
        │          Resource Pack (zip / folder)                │
        └─────────────────────────┬─────────────────────────────┘
                                  │
                                  ▼  目录规整（最先执行、优先级最高）
        ┌───────────────────────────────────────────────────────┐
        │     定位 / 提升 pack.mcmeta 到压缩包根目录            │
        │     （含 pack.mcmeta.txt 多扩展名防呆）               │
        └─────────────────────────┬─────────────────────────────┘
                                  │
                                  ▼
        ┌───────────────────────────────────────────────────────┐
        │              DTD Pipeline (Scheduler)                │
        │      ┌──────────┐   ┌──────────┐   ┌──────────┐      │
        │      │  Eraser  │ → │ Architect│ → │ Surgeon  │      │
        │      │ 拆解包,  │   │ BFS 规划 │   │ 应用模块 │      │
        │      │ 提取纹理 │   │ 转换路径 │   │ 改写输出 │      │
        │      └──────────┘   └──────────┘   └──────────┘      │
        └─────────────────────────┬─────────────────────────────┘
                                  │
                                  ▼
        ┌───────────────────────────────────────────────────────┐
        │            Target Version (1.6 → 26.1+)              │
        └───────────────────────────────────────────────────────┘
```

- **目录规整** — 最先执行：定位真正的 `pack.mcmeta`（含多扩展名防呆）并提升到根目录
- **Eraser** — 拆解源资源包，统一中间表示
- **Architect** — 在“目标版本图”上 BFS 规划最短转换路径
- **Surgeon** — 按序执行每个 converter（各含 reverse 配对），输出目标资源包

**Bedrock 路径（实验性）** 在同一 Scheduler 上以版本边挂载：

- `converters/bedrock/` 模块：`mapping` / `textures` / `ui` / `potions` / `metadata` / `j2b` / `b2j`
- 任务 `bedrock_java_to_bedrock` / `bedrock_bedrock_to_java`（`Exclusive` + `Surgeon`）
- 边：`(88, 1000)` j2b、`(1000, 88)` b2j；Java 中间态统一 **format 88（26.2）**

### 🧰 技术栈

| 层 | 技术 |
|---|---|
| 桌面壳 | Tauri 2（仅 Windows，自绘无边框窗口） |
| 前端 | Vue 3.5 + TypeScript 5 + Vite + vue-i18n 10 |
| 后端 | Rust + image + zip + winreg + reqwest（更新检查） |
| 安装器 | `installer-app/`（Tauri 2 + Vue 3，内嵌 payload.zip 的 HKCU 免管理员安装器） |
| 发布流水线 | `tools/build_release.py`（无第三方打包工具） |
| 资源 | `src-tauri/UImage/`（模板贴图）+ `src-tauri/overlay/`（Overlay 模板） |

### 📁 项目结构

```
2-Pyramid/
├── src/                       Vue 3 前端
│   ├── components/            页面组件（Home / Conversion / Settings / Overlay …）
│   ├── composables/           useAppInfo / useUpdater / useI18n …
│   └── locales/               zh-CN.json / en-US.json
├── src-tauri/                 Rust 后端（主程序）
│   ├── src/
│   │   ├── converters/        版本转换模块（各含 reverse）+ bedrock/ 子模块 + 目录规整 / 打包
│   │   ├── commands/          Tauri 命令（config / background / overlay / misc …）
│   │   ├── hurray/            调度器与纹理池
│   │   ├── overlay/           Overlay 模板生成
│   │   └── lib.rs             入口：窗口创建 / 单实例 / 退出策略
│   ├── UImage/                内置贴图模板
│   ├── overlay/               Overlay 模板（模型 / shader / lang）
│   └── tauri.conf.json
├── installer-app/             自研安装器项目（内嵌 payload.zip，HKCU 注册表）
├── tools/                     发布流水线 / 构建号 / Logo 生成
├── BUILD                      构建号（release 编译自动递增）
├── CHANGELOG.md               更新日志
└── release/                   构建产物（已 gitignore）
```

### 🤝 贡献

欢迎 PR。改动前请先跑：

```bash
cargo test --offline --manifest-path src-tauri/Cargo.toml   # Rust 单测
npm run build                                               # 前端 build
```

---

## English

**2-Pyramid** is a Windows desktop Minecraft resource-pack version converter. It covers 26 Java target version ranges from 1.6 to the latest 26.2, with conversion between any two versions, plus experimental **Java ↔ Bedrock** structural conversion.

### ✨ Features

- **Wide coverage** — 26 Java target ranges across six eras
- **Java ↔ Bedrock (experimental)**
  - **j2b**: any Java pack → latest **Java 26.2 (pack_format 88)** → restructure → `.mcpack`
  - **b2j**: detects `.mcpack` / Bedrock zip → Java 26.2 tree → chosen Java target → `.zip`
  - Vanilla-aligned texture aliases (potions, beds, buckets, planks/logs, armor layers)
  - Builds `gui/icons.png` HUD atlas, `textures_list.json`; flattens container UI and pads to power-of-two
  - Platform-exclusive assets are dropped per direction (no Java model ↔ Bedrock geometry conversion)
- **Fully local** — No cloud, no account, no telemetry
- **Batch processing** — Multiple packs at once, 1–4 parallel workers
- **Directory normalization** — Nested `pack.mcmeta` is promoted to the zip root; `pack.mcmeta.txt`-style files are renamed to `pack.mcmeta` when they contain a valid `format` value
- **Animated texture conversion** — Legacy `{"animation": {}}` mcmeta files are upgraded to explicit `frametime` + `interpolate` (frame count derived from texture dimensions)
- **Output naming template** — `[Ver]` / `[Name]` / `[Time]` / `[Date]` placeholders; old version prefixes are replaced on re-conversion
- **Overlay parent packs** — Layer custom content on top of any base pack without modifying it
- **Interface motion** — Crossfade page swaps with soft blur; settings for on / follow system / off; speed tiers scale the full page transition
- **Deep UI customization** — Custom background with auto theme color, glass / frosted control skins, zh / en
- **Self-owned installer** — No-admin HKCU install, OOBE wizard, optional desktop / start-menu shortcuts, install dir auto-aligns to the existing location on reinstall; the uninstaller cleans up after itself on exit
- **Beta channel** — Stable and Beta installs coexist (separate registry, directory and badges); built with `betabuild`

### 🚀 Quick Start

#### Users

1. Download `2-Pyramid-Installer-{version}.exe` (or `...-beta.{BUILD}.exe` for Beta) from [Releases](https://github.com/realLivedInCorner/2-Pyramid/releases)
2. Install, launch, and follow the OOBE setup on first run

#### Developers

```bash
git clone git@github.com:realLivedInCorner/2-Pyramid.git
cd 2-Pyramid

npm install        # frontend deps
npm run 2pyr       # Tauri dev mode (Rust backend + Vite frontend)
```

Common scripts:

| Script | Purpose |
|---|---|
| `npm run dev` | Vite-only (no Rust backend) |
| `npm run 2pyr` | Tauri dev mode |
| `npm run build` | Build frontend only |
| `npm run buildrelease` | Full stable release build (frontend + app + self-owned installer → `release/`) |
| `npm run betabuild` | Beta channel build (`-beta.{BUILD}.exe`, coexists with stable) |
| `npm run buildrelease:nobump` / `npm run betabuild:nobump` | Same without bumping `BUILD` |

> `BUILD` is incremented exactly once per release build by `build.rs`; `--no-bump` skips it via `2PYR_NO_BUMP=1`.

### 🔄 Updates

In-app update checks read this repository's GitHub Releases. Tag conventions:

| Tag prefix | Meaning | Visible to |
|---|---|---|
| `Safe-2.0.3` | Important security update | All channels |
| `Stable-2.0.3` | Stable | Stable / Both |
| `v2.0.3` | Stable (no prefix) | Stable / Both |
| `UnStable-2.0.3` / `Beta-2.0.3` | Test / Beta update | Test / Both |

The update channel has three states: **Stable** (stable releases only) / **Pre-Release** (test releases only) / **Both** (accepts updates from both channels at once, highest version wins).

Attach the `.exe` installer to each release (updates launch the GUI installer wizard).

### 🏗️ Architecture

The core is a hand-rolled **DTD Pipeline** driven by a **BFS Scheduler**. The very first step is **directory normalization** (locate & promote `pack.mcmeta` to the zip root, with `pack.mcmeta.txt` foolproofing), followed by Eraser → Architect → Surgeon converter tiers.

**Bedrock path (experimental)** mounts on the same scheduler as version edges: modular `converters/bedrock/` (`mapping` / `textures` / `ui` / `potions` / `metadata` / `j2b` / `b2j`), tasks `bedrock_java_to_bedrock` / `bedrock_bedrock_to_java` (`Exclusive` + `Surgeon`), edges `(88, 1000)` / `(1000, 88)`. The Java intermediate is always **format 88 (26.2)**.

### 🧰 Tech Stack

| Layer | Tech |
|---|---|
| Shell | Tauri 2 (Windows-only, custom frameless window) |
| Frontend | Vue 3.5 + TypeScript 5 + Vite + vue-i18n 10 |
| Backend | Rust + image + zip + winreg + reqwest (update checks) |
| Installer | `installer-app/` (Tauri 2 + Vue 3, embeds `payload.zip`, no-admin HKCU) |
| Release pipeline | `tools/build_release.py` (no third-party packagers) |
| Resources | `src-tauri/UImage/` + `src-tauri/overlay/` |

### 📁 Project Layout

```
2-Pyramid/
├── src/                       Vue 3 frontend
│   ├── components/            Pages & dialogs
│   ├── composables/           useAppInfo / useUpdater / useI18n …
│   └── locales/               zh-CN.json / en-US.json
├── src-tauri/                 Rust backend (main app)
│   ├── src/
│   │   ├── converters/        Version converters + bedrock/ submodule + directory normalization
│   │   ├── commands/          Tauri commands
│   │   ├── hurray/            Scheduler & texture pool
│   │   ├── overlay/           Overlay template generation
│   │   └── lib.rs             Entry: window / single-instance / exit policy
│   ├── UImage/                Built-in texture templates
│   ├── overlay/               Overlay templates
│   └── tauri.conf.json
├── installer-app/             Self-owned installer (embeds payload.zip)
├── tools/                     Release pipeline / build number / logo generation
├── BUILD                      Build number (auto-incremented on release builds)
├── CHANGELOG.md               Changelog
└── release/                   Build artifacts (gitignored)
```

### 🤝 Contributing

PRs welcome. Before pushing, please run:

```bash
cargo test --offline --manifest-path src-tauri/Cargo.toml   # Rust unit tests
npm run build                                               # frontend build
```

---

## License / 许可

[MIT](./LICENSE) © 2025–2026 2-Pyramid Studio
