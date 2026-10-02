<!-- markdownlint-disable MD033 MD036 -->

<p align="center">
  <img src="./src/assets/logo-256.png" width="160" alt="2-Pyramid logo">
</p>

<h1 align="center">2-Pyramid</h1>

<p align="center">
  <strong>跨任意版本转换 Minecraft 资源包 · Modern Universal Multi-Version Resource Pack Converter</strong>
</p>

<p align="center">
  <img alt="Version" src="https://img.shields.io/badge/version-2.6.0-007bff?style=flat-square">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows-0078D4?style=flat-square">
  <img alt="License" src="https://img.shields.io/badge/license-MIT-22c55e?style=flat-square">
  <img alt="Tauri" src="https://img.shields.io/badge/built%20with-Tauri%202-FFC131?style=flat-square">
  <img alt="Vue" src="https://img.shields.io/badge/Vue-3.5-4FC08D?style=flat-square">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-orange?style=flat-square">
</p>

---

## 中文

**2-Pyramid** 是一款 Windows 桌面端的 Minecraft 资源包版本转换器，覆盖从 1.6 到最新 26.3 的 Java 目标版本区间，支持任意两个版本之间的相互转换，并支持 **Java ↔ Bedrock（基岩）** 结构互转（实验性）。

### ✨ 特性

- **广覆盖** — 26 个 Java 目标版本区间，六个时代（Classic / Modern / Caves & Cliffs / Trails & Tales / Tricky Trials / Bundles of Bravery）
- **Java ↔ Bedrock（实验性）**
  - **j2b**：任意 Java 包 → 先转到最新 **Java 26.3（pack_format 97）** → 结构重组 → `.mcpack`
  - **b2j**：检测 `.mcpack` / 基岩 zip → 先落到 Java 26.3 树 → 再转到所选 Java 目标 → `.zip`
  - 贴图别名对齐 vanilla Bedrock（药水瓶、床、桶、木板/原木、盔甲层等）
  - 生成 `gui/icons.png` HUD 图集、`textures_list.json`、容器界面扁平化与 POT 适配
  - 平台独有内容按方向剥离（不互转 Java model ↔ Bedrock geometry）
- **完全本地** — 无云端、无账号、无遥测，**转换全程不联网**（更新检查与可选的 Foray AI 分析会联网，详见「已知限制」）
- **批量处理** — 一次拖入多个资源包，1–4 线程并行转换
- **目录规整防呆** — 自动把嵌套的 `pack.mcmeta` 提升到压缩包根目录；`pack.mcmeta.txt` 之类多扩展名文件只要内容是合法 mcmeta（能解析出 format 数值）也会统一改名为 `pack.mcmeta`
- **动态贴图转换** — 老版 `{"animation": {}}` 的 `.png.mcmeta` 自动按贴图尺寸推导帧数，改写为高版本 `frametime` + `interpolate` 格式
- **输出命名模板** — `[Ver]` / `[Name]` / `[Time]` / `[Date]` 占位符自由组合，再转换时自动替换名称中的旧版本前缀
- **Overlay 母包叠加** — 在不修改原包的前提下，把自定义覆盖包叠加到任意母包上
- **界面动画** — 页面切换交叉淡入淡出 + 模糊衔接；设置可选开启 / 跟随系统 / 关闭；速率三档作用于完整页切换
- **深度定制 UI** — 自定义背景（自动提取主题色）、玻璃 / 磨砂控件皮肤、中英双语
- **自研安装器** — 无需管理员权限的 HKCU 安装，OOBE 分步向导，可选桌面 / 开始菜单快捷方式；应用内更新时进入覆盖更新流（锁定原目录、保留用户数据）；重装时安装目录自动对齐现有位置
- **智能更新** — 按 major.minor.patch 判定：major 与 `Safe-*` 强制更新，minor / patch 可选
- **日志脱敏** — 导出日志（会话日志或磁盘日志）默认自动隐藏 Windows 用户名、机器名、IP、邮箱、API Key/token，并把包路径目录压成 `<path>\文件名`；仅导出文件脱敏，可在开发者选项中关闭
- **Beta 双渠道** — 正式版与 Beta 版可并存安装（独立注册表、独立目录、Beta 标识），`betabuild` 一键构建

### ⚠️ 已知限制（请先读）

2-Pyramid 是**结构转换器**，不是原版素材库。以下几点是设计取舍，不是待修 bug：

- **目标版本的新增内容为「近似生成」**：升级时若目标版本引入了原包不存在的新方块 / 新物品（例如 netherite、copper、breeze、pale / poplar 木板、部分树叶），程序会从**最接近的现有贴图**按**手工标定的色相/饱和度/明度参数**派生一张近似贴图（部分树叶额外钉住色相以避免漂色）。**这些贴图不是原版资源，与原版视觉存在差异** —— 想要 1:1 原版观感，请在转换后自行替换这些文件。
- **结构转换优先，不做像素重绘**：物品 / 方块贴图分辨率、GUI 布局、模型与动画按版本规则改写（含九宫格等厚、sprite 切割、图集生成等），但不会重画美术资源。
- **Bedrock 双向转换为实验性**：j2b / b2j 未完成且存在严重问题，仅用于测试；选中该目标会弹出警示，材质包选择也不接受 `.mcpack` 作为输入。
- **发行版默认只记录 Warn / Error 日志**：release 构建下 Info 级日志（含转换进度与 OKAY 明细）默认不写入日志文件；若需完整日志用于排查，请在**设置 → 开发者模式**中开启后再复现问题。
- **更新检查会访问网络**：仅访问 GitHub 官方源（曾经的国内镜像已停止维护、已移除）；转换过程本身全程离线。
- **Editor Mode / Foray 的 AI 功能会把内容发给第三方**：该功能使用**你自己填写的** OpenAI 兼容 endpoint 与密钥，你选择分析的贴图/文本会被发送到该 endpoint。默认关闭。

### 🚀 快速开始

#### 用户（直接使用）

1. 前往 [Releases](https://github.com/realLivedInCorner/2-Pyramid/releases) 下载 `2-Pyramid-Installer-{版本}.exe`（Beta 版为 `2-Pyramid-Installer-{版本}-beta.{BUILD}.exe`）
2. 或从 **Microsoft Store** 安装（`2-Pyramid-{版本}.msix`，与应用内更新互不影响）
3. 安装后启动，首次运行跟随 OOBE 引导配置即可

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
| `npm run test` / `npm run test:offline` | 运行 Rust 单测（`test:offline` 强制离线） |
| `npm run buildrelease:noinstaller` | 只构建主程序，跳过安装器（更快） |
| `python tools/build_release.py --skip-msix` / `--sign-msix` | 跳过 MSIX / 签名 MSIX（sideload 测试） |
| `npm run bump:build:show` / `bump:build:set` | 查看 / 手动设置 BUILD 构建号 |

> 构建号说明：`BUILD` 文件由主程序 `build.rs` 在 release 编译时**唯一递增一次**；`--no-bump` 通过环境变量 `2PYR_NO_BUMP=1` 完全跳过递增。

### 🔄 更新机制

应用内的「检查更新」读取本仓库的 Releases（仅 GitHub 官方 API）。Release tag 约定（`{版本}` 例如 `2.5.0`）：

| Tag 前缀 | 含义 | 可见通道 |
|---|---|---|
| `Safe-{版本}` | 重要安全更新（强制提醒） | 全部 |
| `Stable-{版本}` | 稳定版 | 稳定通道 / 全部 |
| `v{版本}` | 稳定版（无前缀） | 稳定通道 / 全部 |
| `UnStable-{版本}` / `Beta-{版本}` | 测试版更新 | 测试通道 / 全部 |

更新通道为三态：**稳定版**（仅稳定发布）/ **测试版**（仅测试发布）/ **全部**（同时接受两个通道的更新内容，取最高版本）。
更新源：**仅 GitHub 官方**。国内镜像 `cdn.5eggpack.top` 曾作为可选项，**已于 2026-10 移除**（镜像作者停止维护），设置页中该选项显示为「已停止维护，不可用」。

发版要求：
1. Release 附带 `.exe` 安装包（自研安装器）及**同名 `.sha256` 校验文件**（`build_release.py` 自动生成）。**校验是强制的**：缺少 `.sha256`、校验文件拉取失败或哈希不匹配，更新器一律拒绝下载/安装；
2. 应用内更新时，更新器以 `--from-app` 拉起安装器的**覆盖更新向导**（保留用户数据、锁定原安装目录），而不是全新安装流程；
3. 走 Microsoft Store 时另附 `release/2-Pyramid-{版本}.msix`（`build_release.py` 默认产出，可用 `--skip-msix` 跳过）。

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
        │            Target Version (1.6 → 26.3)                │
        └───────────────────────────────────────────────────────┘
```

- **目录规整** — 最先执行：定位真正的 `pack.mcmeta`（含多扩展名防呆）并提升到根目录
- **Eraser** — 拆解源资源包，统一中间表示
- **Architect** — 在“目标版本图”上 BFS 规划最短转换路径
- **Surgeon** — 按序执行每个 converter（各含 reverse 配对），输出目标资源包

**Bedrock 路径（实验性）** 在同一 Scheduler 上以版本边挂载：

- `converters/bedrock/` 模块：`mapping` / `textures` / `ui` / `potions` / `metadata` / `j2b` / `b2j`
- 任务 `bedrock_java_to_bedrock` / `bedrock_bedrock_to_java`（`Exclusive` + `Surgeon`）
- 边：`(97, 1000)` j2b、`(1000, 97)` b2j；Java 中间态统一 **format 97（26.3）**

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

**2-Pyramid** is a Windows desktop Minecraft resource-pack version converter. It covers 26 Java target version ranges from 1.6 to the latest 26.3, with conversion between any two versions, plus experimental **Java ↔ Bedrock** structural conversion.

### ✨ Features

- **Wide coverage** — 26 Java target ranges across six eras
- **Java ↔ Bedrock (experimental)**
  - **j2b**: any Java pack → latest **Java 26.3 (pack_format 97)** → restructure → `.mcpack`
  - **b2j**: detects `.mcpack` / Bedrock zip → Java 26.3 tree → chosen Java target → `.zip`
  - Vanilla-aligned texture aliases (potions, beds, buckets, planks/logs, armor layers)
  - Builds `gui/icons.png` HUD atlas, `textures_list.json`; flattens container UI and pads to power-of-two
  - Platform-exclusive assets are dropped per direction (no Java model ↔ Bedrock geometry conversion)
- **Fully local** — No cloud, no account, no telemetry; **conversion never touches the network** (update checks and the optional Foray AI analysis do — see Known Limitations)
- **Batch processing** — Multiple packs at once, 1–4 parallel workers
- **Directory normalization** — Nested `pack.mcmeta` is promoted to the zip root; `pack.mcmeta.txt`-style files are renamed to `pack.mcmeta` when they contain a valid `format` value
- **Animated texture conversion** — Legacy `{"animation": {}}` mcmeta files are upgraded to explicit `frametime` + `interpolate` (frame count derived from texture dimensions)
- **Output naming template** — `[Ver]` / `[Name]` / `[Time]` / `[Date]` placeholders; old version prefixes are replaced on re-conversion
- **Overlay parent packs** — Layer custom content on top of any base pack without modifying it
- **Interface motion** — Crossfade page swaps with soft blur; settings for on / follow system / off; speed tiers scale the full page transition
- **Deep UI customization** — Custom background with auto theme color, glass / frosted control skins, zh / en
- **Self-owned installer** — No-admin HKCU install, OOBE wizard, optional desktop / start-menu shortcuts; in-app updates enter an overwrite-update flow (locks the original dir, keeps user data); install dir auto-aligns to the existing location on reinstall
- **Smart updates** — Compared by major.minor.patch: major and `Safe-*` force the update; minor / patch are optional
- **Redacted log export** — exporting logs (session or on-disk) hides the Windows user name, machine name, IPs, e-mails and API keys/tokens, and collapses pack paths to `<path>\file.ext`; only the exported file is redacted, and it can be turned off in developer options
- **Beta channel** — Stable and Beta installs coexist (separate registry, directory and badges); built with `betabuild`

### ⚠️ Known Limitations (read first)

2-Pyramid is a **structure converter**, not a vanilla asset library. The following are design trade-offs, not bugs to be fixed:

- **New content in newer target versions is approximated**: when upgrading, blocks/items introduced by the target version but absent from your pack (e.g. netherite, copper, breeze, pale / poplar planks, some leaves) are derived from the **closest existing texture** using **hand-calibrated hue/saturation/value parameters** (some leaves additionally pin hue to avoid drifting). **These are not vanilla assets and will differ visually** — replace them manually if you need a 1:1 vanilla look.
- **Structure first, no pixel repainting**: item/block resolutions, GUI layout, models and animations are rewritten per version rules (nine-slice equalization, sprite slicing, atlas generation, …), but art is never redrawn.
- **Bedrock conversion is experimental**: j2b / b2j are incomplete and known to be broken — testing only; selecting that target shows a warning, and `.mcpack` is rejected as input.
- **Release builds log Warn / Error only**: Info-level logs (conversion progress and OKAY details) are not written in release builds; enable **Settings → Developer Mode** first if you need full logs for a bug report.
- **Update checks reach the network**: official GitHub only (the former China mirror is retired and has been removed). Conversion itself is fully offline.
- **Editor Mode / Foray AI sends content to a third party**: it uses the OpenAI-compatible endpoint and key **you provide**, and the textures/text you choose to analyze are sent there. Off by default.

### 🚀 Quick Start

#### Users

1. Download `2-Pyramid-Installer-{version}.exe` (or `...-beta.{BUILD}.exe` for Beta) from [Releases](https://github.com/realLivedInCorner/2-Pyramid/releases)
2. Or install from the **Microsoft Store** (`2-Pyramid-{version}.msix`; independent of in-app updates)
3. Install, launch, and follow the OOBE setup on first run

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

In-app update checks read this repository's Releases (official GitHub API only). Tag conventions (`{version}`, e.g. `2.5.0`):

| Tag prefix | Meaning | Visible to |
|---|---|---|
| `Safe-{version}` | Important security update | All channels |
| `Stable-{version}` | Stable | Stable / Both |
| `v{version}` | Stable (no prefix) | Stable / Both |
| `UnStable-{version}` / `Beta-{version}` | Test / Beta update | Test / Both |

The update channel has three states: **Stable** (stable releases only) / **Pre-Release** (test releases only) / **Both** (accepts updates from both channels at once, highest version wins).
Update source: **official GitHub only**. The China mirror `cdn.5eggpack.top` was an optional source and was **removed in 2026-10** (its maintainer stopped maintaining it); the option is shown as “retired / unavailable” in Settings.

Release requirements:
1. Attach the `.exe` installer plus a **matching `.sha256` file** (`build_release.py` writes it). **Verification is mandatory**: a missing checksum, a failed checksum fetch, or a hash mismatch makes the updater refuse to download/install;
2. For in-app updates the updater launches the installer with `--from-app`, which opens the **overwrite-update wizard** (keeps user data, locks the existing install dir) instead of the fresh-install flow;
3. For Microsoft Store, attach `release/2-Pyramid-{version}.msix` as well (produced by default; skip with `--skip-msix`).

### 🏗️ Architecture

The core is a hand-rolled **DTD Pipeline** driven by a **BFS Scheduler**. The very first step is **directory normalization** (locate & promote `pack.mcmeta` to the zip root, with `pack.mcmeta.txt` foolproofing), followed by Eraser → Architect → Surgeon converter tiers.

**Bedrock path (experimental)** mounts on the same scheduler as version edges: modular `converters/bedrock/` (`mapping` / `textures` / `ui` / `potions` / `metadata` / `j2b` / `b2j`), tasks `bedrock_java_to_bedrock` / `bedrock_bedrock_to_java` (`Exclusive` + `Surgeon`), edges `(97, 1000)` / `(1000, 97)`. The Java intermediate is always **format 97 (26.3)**.

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
├── legal/                     EULA / Privacy / Disclaimer / Notices / Security / Contributing
└── release/                   Build artifacts (gitignored)
```

### 🤝 Contributing

PRs welcome — see [legal/CONTRIBUTING.md](./legal/CONTRIBUTING.md). Before pushing, please run:

```bash
npm test                                                    # Rust unit tests
npm run build                                               # frontend build
```

### ⚖️ Legal

| File | Purpose |
|------|---------|
| [LICENSE](./LICENSE) | MIT — source code |
| [legal/EULA.md](./legal/EULA.md) | End-user notice; does **not** restrict MIT rights |
| [legal/DISCLAIMER.md](./legal/DISCLAIMER.md) | Warranty disclaimer; unofficial Mojang/Microsoft tool |
| [legal/PRIVACY.md](./legal/PRIVACY.md) | Local-first privacy; no telemetry |
| [legal/THIRD-PARTY-NOTICES.md](./legal/THIRD-PARTY-NOTICES.md) | Open-source dependency notices |
| [legal/SECURITY.md](./legal/SECURITY.md) | Vulnerability reporting |
| [legal/CONTRIBUTING.md](./legal/CONTRIBUTING.md) | Contribution guide |

**Credits:** 2-Pyramid is an independent project. Early prototypes and community tools in the Minecraft pack-conversion space informed parts of the design; we thank those authors for their work. Such references do not imply joint copyright, employment, or endorsement. In-app credits live in **Settings → Version Info**. Legal texts are bundled under `legal/` and readable in **Settings → Legal Notices**.

---

## License / 许可

[MIT](./LICENSE) © 2025–2026 2-Pyramid Studio

使用可执行程序时另见 [EULA](./legal/EULA.md) 与 [DISCLAIMER](./legal/DISCLAIMER.md)。请同时遵守 Minecraft EULA 与资源包原作者授权。

## Foray / 安装包（2.5.0）

- **Foray**：设置 → Editor Mode 开启后，主页拖入 zip 进入资源包分析工作台（探针 / 轻量编辑 / 可选 AI）。
- **静默安装**：2-Pyramid-Installer.exe --silent [--dir <path>] [--relaunch] [--shortcuts]。
- **产物**：版本化 exe → GitHub Releases；.msix（makeappx）→ Microsoft Store。**不再产出 MSI**。
- 法律与隐私：`legal/`（含外部 AI API 说明）。

