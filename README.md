<!-- markdownlint-disable MD033 MD036 -->

<p align="center">
  <img src="./src/assets/logo-256.png" width="160" alt="2-Pyramid logo">
</p>

<h1 align="center">2-Pyramid</h1>

<p align="center">
  <strong>跨任意版本转换 Minecraft 资源包 · Modern Universal Multi-Version Resource Pack Converter</strong>
</p>

<p align="center">
  <img alt="Version" src="https://img.shields.io/badge/version-2.200.0-007bff?style=flat-square">
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
- **批量处理** — 一次拖入多个资源包；**性能档位**（设置里二选一）：**平衡**用一半核心（下限 2、上限 8）并同时转换 2 个包，机器可继续正常使用；**性能**吃满核心（上限 32）并同时最多 6 个包（可用内存低于 1500 MB 时自动降为 1 个包）。设置项下方实时显示本机解析结果（核数 · 线程预算 · 并发包数）
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
- **Bedrock 双向转换为实验性**：j2b / b2j 未完成且存在严重问题，仅用于测试；选中该目标会弹出警示（`.mcpack` 可直接作为 b2j 的输入）。
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
| `npm run buildrelease:noinstaller` | 跳过自研安装器 EXE；**但默认仍会为 MSIX 再编译一次**（`--features store`）并打包，所以想真正只构建主程序需同时跳过 MSIX：`python tools/build_release.py --skip-installer --skip-msix` |
| `python tools/build_release.py --skip-msix` / `--sign-msix` | 跳过 MSIX / 签名 MSIX（sideload 测试） |
| `npm run bump:build:show` / `bump:build:set` | 查看 / 手动设置 BUILD 构建号 |

> 构建号说明：`BUILD` 文件由主程序 `build.rs` 在 release 编译时**唯一递增一次**；`--no-bump` 通过环境变量 `2PYR_NO_BUMP=1` 完全跳过递增。

### 🔄 更新机制

应用内的「检查更新」读取本仓库的 Releases（仅 GitHub 官方 API）。Release tag 约定：

| Tag 前缀 | 含义 | 可见通道 |
|---|---|---|
| `Safe-{版本}` | 重要安全更新（强制提醒） | 全部 |
| `Stable-{版本}` | 稳定版 | 稳定通道 / 全部 |
| `v{版本}` | 稳定版（无前缀） | 稳定通道 / 全部 |
| `UnStable-{版本}` / `Beta-{版本}` | 测试版更新 | 测试通道 / 全部 |

更新通道为三态：**稳定版**（仅稳定发布）/ **测试版**（仅测试发布）/ **全部**（同时接受两个通道的更新内容，取最高版本）。
更新源：**仅 GitHub 官方**——国内镜像 `cdn.5eggpack.top` 已于 2026-10 **彻底移除**（镜像作者停止维护），设置里不再有「更新源」选项，也没有测速 / 切换入口。

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

- `src-tauri/src/bedrock_convert/` 模块：`mapping` / `textures` / `ui` / `potions` / `metadata` / `shaders` / `skybox` / `j2b` / `b2j`
- 任务 `bedrock_java_to_bedrock` / `bedrock_bedrock_to_java`（`Exclusive` + `Surgeon`）
- 边：j2b `(84|88|97 → 1000)`、b2j `(1000 → 97|88|84)`，均在 `arom/engine/conversion_maps.rs` 声明；Java 中间态统一 **format 97（26.3）**

### 🧪 命令行工具（排查 / 压测 / 质量闸门）

主程序支持三个无界面模式，全部**只读或独立运行**、不启动 GUI：

| 命令 | 作用 |
|---|---|
| `2-pyramid.exe --convert <包\|目录> [--to 26.3] [--out <目录>] [--report <报告.json>] [--fast]` | 跑**完整转换管线**并输出结构化报告：结构分析 + 纯转换/总时间（含 IO 分解）+ 逐任务耗时 + 体积变化。目录会递归处理其中所有 `.zip`/`.mcpack`。`--fast` 关闭 GUI sprite 手术（`run_gui_surgeon`），用于隔离"手术"那部分的耗时。 |
| `2-pyramid.exe --analyze <包\|目录>` | 只读**结构分析**：官方分层（overlays）、`supported_formats` 区间、非标准版本折叠目录、一包多根，含每层文件数与覆盖计数。 |
| `2-pyramid.exe --pack-diff <A> <B> [--strict] [--json <报告>]` | **质量闸门**：PNG 像素级、JSON 语义级、其余字节级比对；默认允许"像素相同仅编码不同"，`--strict` 要求字节一致。退出码 0/1，可用于提速改动的回归验证。 |

> **拖放即转换**：把资源包（或含包的文件夹）拖到 **`tools/convert-report.bat`** 上——脚本会找到支持 `--convert` 的程序（仓库构建优先，其次已安装版本）、让你选目标版本、跑转换、打印两个耗时口径与慢任务，并把 JSON 报告存到资源包旁边同时打开所在文件夹。
>
> 也可脚本化调用（`.bat` 只是纯 ASCII 启动器，逻辑在 `convert-report.ps1` 里）：
> ```powershell
> .\tools\convert-report.bat "D:\packs\my.zip" -Target 1.21.4      # 指定版本
> .\tools\convert-report.bat "D:\packs"        -MenuChoice 7 -Yes  # 菜单序号 7 = 1.21.4，跳过确认
> ```
> 参数：`-Target`（版本或 pack_format）、`-MenuChoice`（菜单 1–16）、`-Yes`（跳过确认与结尾等待）、`-NoOpen`（不自动打开文件夹）。
>
> **性能基准页**：`tools/benchmark/index.html`（单文件、零依赖）——横向对比各优化节点的耗时构成，含双计时口径切换与质量校验结论；数据更新方式见 `tools/benchmark/README.md`。
>
> 维护者文档：发布清单 `tools/release/README.md`；商店提交文案 `tools/store/`。

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
│   │   ├── arom/              A-ROM 对象模型 + 执行引擎（engine/：版本映射与按阶段调度）
│   │   ├── natives/           46 个任务的**原生实现**（eraser / architect / surgeon / reverse 分组）
│   │   ├── pack/              资源包 I/O 与分析工具（io / diff / analysis / version_converter）
│   │   ├── bedrock_convert/   基岩 ↔ Java 结构转换（j2b / b2j）
│   │   ├── commands/          Tauri 命令（config / background / overlay / misc …）
│   │   ├── foray/             资源包结构分析与轻量编辑（只读侧）
│   │   ├── overlay/           Overlay 模板生成
│   │   └── lib.rs             入口：窗口创建 / 单实例 / 退出策略
│   ├── UImage/                内置贴图模板
│   ├── overlay/               Overlay 模板（模型 / shader / lang）
│   └── tauri.conf.json
├── installer-app/             自研安装器项目（内嵌 payload.zip，HKCU 注册表）
├── tools/                     构建与发布工具（见下）
│   ├── build_release.py       发布流水线（版本校验 / 安装器 / MSIX / .sha256）
│   ├── set_version.py         版本号 10 处统一设置与 --check 守门
│   ├── convert-report.bat/.ps1 拖放即转换的启动器与逻辑
│   ├── benchmark/             性能基准页（单文件 HTML，零依赖）
│   ├── release/  store/       发布清单 / 商店提交文案
│   ├── msi/  msix/            MSI 与 MSIX 打包资源与包身份
│   ├── gen-task-registry.ps1  校验 task_registry 的 88 项元数据不被格式化改动
│   └── arom-baseline.txt      冻结内容基线的逐条目清单（4018 行）
├── BUILD                      构建号（release 编译自动递增）
├── CHANGELOG.md               更新日志
├── legal/                     EULA / 隐私 / 免责 / 第三方声明 / 安全 / 贡献（中英双语）
└── release/                   构建产物（已 gitignore）
```

### 🤝 贡献

欢迎 PR —— 见 [legal/CONTRIBUTING.md](./legal/CONTRIBUTING.md)。推送前请先跑：

```bash
npm test                                                    # Rust 单测
npm run build                                               # 前端构建
```

### ⚖️ 法律

| 文件 | 用途 |
|------|------|
| [LICENSE](./LICENSE) | MIT —— 源代码 |
| [legal/EULA.md](./legal/EULA.md) | 最终用户须知；**不**限制 MIT 权利 |
| [legal/DISCLAIMER.md](./legal/DISCLAIMER.md) | 免责声明；非 Mojang / Microsoft 官方工具 |
| [legal/PRIVACY.md](./legal/PRIVACY.md) | 本地优先隐私；无遥测 |
| [legal/THIRD-PARTY-NOTICES.md](./legal/THIRD-PARTY-NOTICES.md) | 开源依赖声明 |
| [legal/SECURITY.md](./legal/SECURITY.md) | 漏洞报告 |
| [legal/CONTRIBUTING.md](./legal/CONTRIBUTING.md) | 贡献指南 |

**致谢：** 2-Pyramid 是独立项目。Minecraft 资源包转换领域的早期原型与社区工具为部分设计提供了参考，我们感谢那些作者的工作。此类引用不代表共同著作权、雇佣关系或背书。应用内致谢见 **设置 → 版本信息**。法律文本随包内置于 `legal/`，可在 **设置 → 法律声明** 中阅读（界面语言为英文时优先读取 `legal/en/`，缺失自动回落中文）。

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
- **Batch processing** — Multiple packs at once, with a **performance tier** (choose one in Settings): **Balanced** uses half the cores (min 2, max 8) and converts 2 packs at a time so the machine stays usable; **Performance** uses all cores (cap 32) and up to 6 packs at once (auto-reduced to 1 when available memory drops below 1500 MB). Settings show the resolved numbers for your machine (cores · thread budget · concurrent packs) below the option
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
- **Bedrock conversion is experimental**: j2b / b2j are incomplete and known to be broken — testing only; selecting that target shows a warning (`.mcpack` is accepted directly as b2j input).
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
| `npm run test` / `npm run test:offline` | Run the Rust unit tests (`test:offline` forces offline) |
| `npm run buildrelease:noinstaller` | Skips the self-owned installer EXE, but still compiles a second time for MSIX (`--features store`) and packs it. To build only the app, skip both: `python tools/build_release.py --skip-installer --skip-msix` |
| `python tools/build_release.py --skip-msix` / `--sign-msix` | Skip MSIX / sign MSIX (sideload testing) |
| `npm run bump:build:show` / `bump:build:set` | Show / manually set the `BUILD` number |

> `BUILD` is incremented exactly once per release build by `build.rs`; `--no-bump` skips it via `2PYR_NO_BUMP=1`.

### 🔄 Updates

In-app update checks read this repository's Releases (official GitHub API only). Tag conventions:

| Tag prefix | Meaning | Visible to |
|---|---|---|
| `Safe-{version}` | Important security update | All channels |
| `Stable-{version}` | Stable | Stable / Both |
| `v{version}` | Stable (no prefix) | Stable / Both |
| `UnStable-{version}` / `Beta-{version}` | Test / Beta update | Test / Both |

The update channel has three states: **Stable** (stable releases only) / **Pre-Release** (test releases only) / **Both** (accepts updates from both channels at once, highest version wins).
Update source: **official GitHub only** — the China mirror `cdn.5eggpack.top` was **fully removed in 2026-10** (its maintainer stopped maintaining it); Settings no longer offers an update-source option, speed test or switch.

Release requirements:
1. Attach the `.exe` installer plus a **matching `.sha256` file** (`build_release.py` writes it). **Verification is mandatory**: a missing checksum, a failed checksum fetch, or a hash mismatch makes the updater refuse to download/install;
2. For in-app updates the updater launches the installer with `--from-app`, which opens the **overwrite-update wizard** (keeps user data, locks the existing install dir) instead of the fresh-install flow;
3. For Microsoft Store, attach `release/2-Pyramid-{version}.msix` as well (produced by default; skip with `--skip-msix`).

### 🏗️ Architecture

The core is a hand-rolled **DTD Pipeline** driven by a **BFS Scheduler**, built on the in-house **A-ROM** object model (`Pack` / `Tx` / `Layer` / `PackView`):

```
        ┌───────────────────────────────────────────────────────┐
        │          Resource Pack (zip / folder)                │
        └─────────────────────────┬─────────────────────────────┘
                                  │
                                  ▼  directory normalization (runs first, highest priority)
        ┌───────────────────────────────────────────────────────┐
        │     locate / promote pack.mcmeta to the zip root      │
        │     (multi-extension foolproofing)                    │
        └─────────────────────────┬─────────────────────────────┘
                                  │
                                  ▼
        ┌───────────────────────────────────────────────────────┐
        │              DTD Pipeline (Scheduler)                │
        │      ┌──────────┐   ┌──────────┐   ┌──────────┐      │
        │      │  Eraser  │ → │ Architect│ → │ Surgeon  │      │
        │      │ tear the │   │ BFS the  │   │ run the  │      │
        │      │ pack down│   │ path     │   │ rewrites │      │
        │      └──────────┘   └──────────┘   └──────────┘      │
        └─────────────────────────┬─────────────────────────────┘
                                  │
                                  ▼
        ┌───────────────────────────────────────────────────────┐
        │            Target Version (1.6 → 26.3)                │
        └───────────────────────────────────────────────────────┘
```

- **Directory normalization** — runs first: locate the real `pack.mcmeta` (multi-extension foolproofing) and promote it to the root
- **Eraser** — tear down the source pack into one intermediate representation
- **Architect** — BFS the shortest conversion path over the target-version graph
- **Surgeon** — run each converter in order (each with its reverse pair) and emit the target pack

**Bedrock path (experimental)** mounts on the same scheduler as version edges:

- Module: `src-tauri/src/bedrock_convert/` (`mapping` / `textures` / `ui` / `potions` / `metadata` / `shaders` / `skybox` / `j2b` / `b2j`)
- Tasks `bedrock_java_to_bedrock` / `bedrock_bedrock_to_java` (`Exclusive` + `Surgeon`)
- Edges: j2b `(84|88|97 → 1000)`, b2j `(1000 → 97|88|84)`, both declared in `arom/engine/conversion_maps.rs`; the Java intermediate is always **format 97 (26.3)**

### 🧪 Command-line tools (diagnostics / benchmarking / quality gate)

The binary has three headless modes; all are **read-only or self-contained** and never start the GUI:

| Command | Purpose |
|---|---|
| `2-pyramid.exe --convert <pack\|dir> [--to 26.3] [--out <dir>] [--report <report.json>] [--fast]` | Runs the **full conversion pipeline** and writes a structured report: structure analysis + pure/total timings (with the IO split) + per-task profile + size delta. A directory is walked recursively for every `.zip` / `.mcpack`. `--fast` turns off the GUI sprite surgery (`run_gui_surgeon`) to isolate its cost. |
| `2-pyramid.exe --analyze <pack\|dir>` | Read-only **structure analysis**: official overlays, `supported_formats` range, non-standard version-folded directories, multi-root packs, with per-layer file counts and override counts. |
| `2-pyramid.exe --pack-diff <A> <B> [--strict] [--json <report>]` | **Quality gate**: PNG compared per pixel, JSON semantically, everything else byte-wise; by default "same pixels, different encoding" is allowed, `--strict` demands identical bytes. Exit code 0/1, usable as a regression check for speed work. |

> **Drag-and-drop conversion**: drop a pack (or a folder of packs) onto **`tools/convert-report.bat`** — the script finds a binary that supports `--convert` (repo build first, then an installed release), asks for the target version, converts, prints both timing figures and the slowest tasks, and saves the JSON report next to the pack while opening its folder.
>
> It can also be scripted (the `.bat` is a pure-ASCII launcher; the logic lives in `convert-report.ps1`):
> ```powershell
> .\tools\convert-report.bat "D:\packs\my.zip" -Target 1.21.4      # explicit version
> .\tools\convert-report.bat "D:\packs"        -MenuChoice 7 -Yes  # menu 7 = 1.21.4, skip confirmation
> ```
> Arguments: `-Target` (version or pack_format), `-MenuChoice` (menu 1–16), `-Yes` (skip confirmation and the final pause), `-NoOpen` (do not open the folder).
>
> **Performance baseline page**: `tools/benchmark/index.html` (single file, zero dependencies) compares the timing breakdown across optimisation steps, with a switch between the two timing methods and the quality-check verdict; how to update the data is in `tools/benchmark/README.md`.
>
> Maintainer docs: release checklist `tools/release/README.md`; Store listing copy `tools/store/`.

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
│   │   ├── arom/              A-ROM object model plus the execution engine (engine/: version maps and tier bucketing)
│   │   ├── natives/           Native implementations of the 46 tasks (eraser / architect / surgeon / reverse)
│   │   ├── pack/              Pack I/O and analysis tools (io / diff / analysis / version_converter)
│   │   ├── bedrock_convert/   Bedrock <-> Java structure conversion (j2b / b2j)
│   │   ├── commands/          Tauri commands
│   │   ├── foray/             Pack structure analysis and light editing (read-only side)
│   │   ├── overlay/           Overlay template generation
│   │   └── lib.rs             Entry: window / single-instance / exit policy
│   ├── UImage/                Built-in texture templates
│   ├── overlay/               Overlay templates
│   └── tauri.conf.json
├── installer-app/             Self-owned installer (embeds payload.zip)
├── tools/                     Build & release tooling (see below)
│   ├── build_release.py       Release pipeline (version check / installer / MSIX / .sha256)
│   ├── set_version.py         Sets all 10 version sites; `--check` gate
│   ├── convert-report.bat/.ps1 Drag-and-drop convert launcher and its logic
│   ├── benchmark/             Perf baseline page (single-file HTML, zero deps)
│   ├── release/  store/       Release checklist / Store listing copy
│   ├── msi/  msix/            MSI & MSIX packaging resources and package identity
│   ├── gen-task-registry.ps1  Guards the 88 registry entries against format churn
│   └── arom-baseline.txt      Per-entry list of the frozen content baseline (4018 lines)
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

**Credits:** 2-Pyramid is an independent project. Early prototypes and community tools in the Minecraft pack-conversion space informed parts of the design; we thank those authors for their work. Such references do not imply joint copyright, employment, or endorsement. In-app credits live in **Settings → Version Info**. Legal texts are bundled under `legal/` and readable in **Settings → Legal Notices** (when the UI language is English the `legal/en/` copy is preferred, falling back to the Chinese original).

---

## License / 许可

[MIT](./LICENSE) © 2025–2026 2-Pyramid Studio

使用可执行程序时另见 [EULA](./legal/EULA.md) 与 [DISCLAIMER](./legal/DISCLAIMER.md)。请同时遵守 Minecraft EULA 与资源包原作者授权。

## 其它 / Misc

- **Foray（Editor Mode）**：在 **设置 → 开发者选项** 开启 Editor Mode 后，主页拖入 zip 进入资源包分析工作台（结构探针 / 轻量像素编辑 / 可选 AI 分析）。默认关闭，说明见 [`docs/compose/spec/foray.md`](./docs/compose/spec/foray.md)。
- **静默安装**：`2-Pyramid-Installer.exe --silent [--dir <路径>] [--relaunch] [--shortcuts]`。
- **发行形式**：版本化 EXE → GitHub Releases；MSIX → Microsoft Store。**不再产出 MSI。**
- **可复现性能验证**：测量方法与已证伪的方向见 [`docs/compose/spec/perf-verification.md`](./docs/compose/spec/perf-verification.md)。

**Misc** — the same points in English:

- **Foray (Editor Mode)**: enable Editor Mode in **Settings → Developer options**, then drop a zip on the home page to open the pack-analysis workbench (structure probes / light pixel editing / optional AI). Off by default; see [`docs/compose/spec/foray.md`](./docs/compose/spec/foray.md).
- **Silent install**: `2-Pyramid-Installer.exe --silent [--dir <path>] [--relaunch] [--shortcuts]`.
- **Distribution**: versioned EXE → GitHub Releases; MSIX → Microsoft Store. **No MSI is produced any more.**
- **Reproducible performance verification**: the measurement method and the directions already disproved are in [`docs/compose/spec/perf-verification.md`](./docs/compose/spec/perf-verification.md).

