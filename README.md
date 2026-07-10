<!-- markdownlint-disable MD033 MD036 -->

<p align="center">
  <img src="./2pyr-logo.svg" width="160" alt="2-Pyramid logo">
</p>

<h1 align="center">2-Pyramid</h1>

<p align="center">
  <strong>跨任意版本转换 Minecraft 资源包 · The Nextgen Multi-Version Universal Resource Pack Converter</strong>
</p>

<p align="center">
  <img alt="Version" src="https://img.shields.io/badge/version-2.0.0-007bff?style=flat-square">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-1d1d1f?style=flat-square">
  <img alt="License" src="https://img.shields.io/badge/license-MIT-22c55e?style=flat-square">
  <img alt="Tauri" src="https://img.shields.io/badge/built%20with-Tauri%202-FFC131?style=flat-square">
  <img alt="Vue" src="https://img.shields.io/badge/Vue-3.5-4FC08D?style=flat-square">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-orange?style=flat-square">
  <img alt="Code Lines" src="https://img.shields.io/badge/code-12k%20Rust%20%2B%208k%20Vue-94a3b8?style=flat-square">
</p>

---

## 中文

**2-Pyramid** 是一款桌面端的 Minecraft 资源包版本转换器,从 1.6 一直覆盖到最新的 26.1+,支持任意两个版本之间的相互转换。

### ✨ 特性

- **广覆盖** — 支持 23 个目标版本区间,涵盖 Classic / Modern / Caves & Cliffs / Trails & Tales / Tricky Trials / Bundles of Bravery 六个时代
- **完全本地** — 无云端、无账号,文件全程不离开你的电脑
- **批量处理** — 一次拖入多个资源包,串行 / 并行转换
- **Overlay 母包叠加** — 在不修改原包的前提下,把自定义 HUD / 物品 / 描边叠加到任意母包上
- **现代 UI** — 主题色可定制、动画速率三档可调、中英双语界面
- **资源丰富** — 内置 26 个 3D 物品模型、附魔闪光、彩虹描边等多种 Overlay 模板

### 🚀 快速开始

#### 用户(直接使用)

1. 前往 [Releases](../../releases) 下载对应平台的安装包
2. 安装后启动,首次运行跟随 OOBE 引导配置即可

#### 开发者(本地运行)

```bash
git clone <repo-url>
cd Hurricane

# 前端依赖
npm install

# 启动 Tauri dev 模式(同时启动 Rust 后端 + Vite 前端)
npm run 2-pyramid
```

常用脚本:

| 命令 | 作用 |
|---|---|
| `npm run dev` | 仅启动 Vite 前端(无 Rust 后端) |
| `npm run 2-pyramid` / `npm run 2pyr` | Tauri dev 模式 |
| `npm run build` | 仅构建前端 |
| `npm run buildproject` / `npm run build2pyr` | 完整发布构建(Windows) |

### 🏗️ 架构

2-Pyramid 的核心是自研的 **DTD Pipeline** 与 **BFS Scheduler**:

```
        ┌───────────────────────────────────────────────────────┐
        │          Resource Pack (zip / folder)                │
        └─────────────────────────┬─────────────────────────────┘
                                  │
                                  ▼
        ┌───────────────────────────────────────────────────────┐
        │              DTD Pipeline (Scheduler)                │
        │                                                       │
        │      ┌──────────┐   ┌──────────┐   ┌──────────┐      │
        │      │  Eraser  │ → │ Architect│ → │ Surgeon  │      │
        │      │          │   │          │   │          │      │
        │      │ 拆解包,  │   │ BFS 规划 │   │ 应用模块 │      │
        │      │ 提取纹理 │   │ 转换路径 │   │ 改写输出 │      │
        │      └──────────┘   └──────────┘   └──────────┘      │
        │                                                       │
        │      85 个 converter 模块,每个都有 reverse 配对       │
        └───────────────────────────────────────────────────────┘
                                  │
                                  ▼
        ┌───────────────────────────────────────────────────────┐
        │            Target Version (1.6 → 26.1+)              │
        └───────────────────────────────────────────────────────┘
```

- **Eraser** — 拆解源资源包,统一为内存中的中间表示
- **Architect** — 用 BFS 在"目标版本图"上规划最短转换路径,把"1.20 → 1.21"分解为若干个相邻版本转换器
- **Surgeon** — 按 Architect 排好的顺序,依次执行每个 converter,输出目标资源包

### 🧰 技术栈

| 层 | 技术 |
|---|---|
| 桌面壳 | Tauri 2(Windows / Linux / macOS) |
| 前端 | Vue 3.5 + TypeScript 5 + Vite 5 + vue-i18n 10 |
| 后端 | Rust 1.x + image crate + tokio + serde |
| 资源 | `src-tauri/UImage/`(模板贴图) + `src-tauri/overlay/`(Overlay 模板) |
| 参考实现 | `Python_Script/pack.py` 1.0 版(Python,已停止维护) |

### 📁 项目结构

```
Hurricane/
├── src/                       Vue 3 前端
│   ├── components/            页面组件(Home / Conversion / Settings / Overlay)
│   ├── composables/           useLanguage / useNotification / useUpdater
│   └── locales/               zh-CN.json / en-US.json
├── src-tauri/                 Rust 后端
│   ├── src/
│   │   ├── converters/        85 个版本转换模块(各含 reverse)
│   │   ├── commands/          Tauri 命令(overlay / config / tray ...)
│   │   ├── lib.rs             入口 + Tray Icon + on_window_event
│   │   └── ...
│   ├── UImage/                内置贴图模板
│   ├── overlay/               Overlay 模板(模型 / shader / lang)
│   ├── icons/                 应用图标
│   └── tauri.conf.json        Tauri 配置
├── Python_Script/             Python 1.0 参考实现(只读)
├── tools/                     工具脚本(audit / logo 生成)
└── 2pyr-logo.svg              主 Logo 源文件
```

### 🤝 贡献

欢迎 PR。改动前请先跑:

```bash
cargo test --manifest-path src-tauri/Cargo.toml    # Rust 单测(目前 36/37,UImage 模板问题见 issue)
npm run build                                      # 前端 build
```

Converter 新增或修改时,务必对照 `Python_Script/pack.py` 中同名的 py 函数 —— py 是唯一真源,rs 是它的翻译。

---

## English

**2-Pyramid** is a desktop Minecraft resource-pack version converter. It covers every release from 1.6 all the way to the latest 26.1+, and supports conversion between any two versions.

### ✨ Features

- **Wide coverage** — 23 target version ranges across six eras: Classic / Modern / Caves & Cliffs / Trails & Tales / Tricky Trials / Bundles of Bravery
- **Fully local** — No cloud, no account, no telemetry. Your files never leave your machine.
- **Batch processing** — Drag in multiple resource packs at once; serial or parallel conversion
- **Overlay parent pack** — Layer custom HUD / items / outlines on top of any base pack without modifying the original
- **Modern UI** — Customizable theme color, three animation speeds, bilingual (zh / en) interface
- **Rich resource library** — 26 built-in 3D item models, custom enchant glint, rainbow outline shaders, and more

### 🚀 Quick Start

#### Users (just want to run it)

1. Download the installer for your platform from [Releases](../../releases)
2. Install, launch, and follow the OOBE setup on first run

#### Developers

```bash
git clone <repo-url>
cd Hurricane

npm install              # frontend deps
npm run 2-pyramid        # Tauri dev mode (Rust backend + Vite frontend)
```

Common scripts:

| Script | Purpose |
|---|---|
| `npm run dev` | Vite-only (no Rust backend) |
| `npm run 2-pyramid` / `npm run 2pyr` | Tauri dev mode |
| `npm run build` | Build frontend only |
| `npm run buildproject` / `npm run build2pyr` | Full release build (Windows) |

### 🏗️ Architecture

The core of 2-Pyramid is a hand-rolled **DTD Pipeline** driven by a **BFS Scheduler**:

```
        ┌───────────────────────────────────────────────────────┐
        │          Resource Pack (zip / folder)                │
        └─────────────────────────┬─────────────────────────────┘
                                  │
                                  ▼
        ┌───────────────────────────────────────────────────────┐
        │              DTD Pipeline (Scheduler)                │
        │                                                       │
        │      ┌──────────┐   ┌──────────┐   ┌──────────┐      │
        │      │  Eraser  │ → │ Architect│ → │ Surgeon  │      │
        │      │          │   │          │   │          │      │
        │      │  Unpack  │   │ BFS path │   │ Apply    │      │
        │      │  parse   │   │ planning │   │ modules  │      │
        │      └──────────┘   └──────────┘   └──────────┘      │
        │                                                       │
        │      85 converters, each with a reverse partner      │
        └───────────────────────────────────────────────────────┘
                                  │
                                  ▼
        ┌───────────────────────────────────────────────────────┐
        │            Target Version (1.6 → 26.1+)              │
        └───────────────────────────────────────────────────────┘
```

- **Eraser** — Unpack the source pack and normalize it into an in-memory representation
- **Architect** — Run BFS on the version graph to find the shortest conversion path. `1.20 → 1.21` becomes a sequence of adjacent-version converters.
- **Surgeon** — Execute each converter in order, then write out the target pack

### 🧰 Tech Stack

| Layer | Tech |
|---|---|
| Shell | Tauri 2 (Windows / Linux / macOS) |
| Frontend | Vue 3.5 + TypeScript 5 + Vite 5 + vue-i18n 10 |
| Backend | Rust 1.x + image crate + tokio + serde |
| Resources | `src-tauri/UImage/` (texture templates), `src-tauri/overlay/` (overlay models / shaders / lang) |
| Reference | `Python_Script/pack.py` 1.0 (Python, frozen) |

### 📁 Project Layout

```
Hurricane/
├── src/                       Vue 3 frontend
│   ├── components/            Page components (Home / Conversion / Settings / Overlay)
│   ├── composables/           useLanguage / useNotification / useUpdater
│   └── locales/               zh-CN.json / en-US.json
├── src-tauri/                 Rust backend
│   ├── src/
│   │   ├── converters/        85 version-converter modules (each with reverse)
│   │   ├── commands/          Tauri commands (overlay / config / tray / ...)
│   │   ├── lib.rs             entrypoint + tray icon + on_window_event
│   │   └── ...
│   ├── UImage/                Built-in texture templates
│   ├── overlay/               Overlay templates (models / shaders / lang)
│   ├── icons/                 App icons
│   └── tauri.conf.json        Tauri config
├── Python_Script/             Python 1.0 reference implementation (read-only)
├── tools/                     Utilities (audit / logo generation)
└── 2pyr-logo.svg              Master logo source
```

### 🤝 Contributing

PRs welcome. Before pushing, please run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml    # Rust unit tests (currently 36/37; see UImage-template issue)
npm run build                                      # frontend build
```

When adding or modifying a converter, **always cross-check the homonymous function in `Python_Script/pack.py`** — the Python file is the single source of truth; Rust is its translation.

---

## License / 许可

[MIT](./LICENSE) © 2025–2026 2-Pyramid Studio