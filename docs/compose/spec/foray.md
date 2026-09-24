---
feature: foray
status: delivered
updated: 2026-09-18
branch: feat/foray
commits: 6723394..HEAD
---

# Foray — 资源包结构分析与轻量编辑工作台

## Report

**What was built** — Foray 独立模块（平行 Hurray）落地：`zip_safe` 安全门禁（Zip Slip / 路径深度 / 体积与条目上限）、`mcmeta` + `rom` 对象树、五探针、轻量 Paint（涂抹/HSV/撤销）、导出（默认另存；原地覆盖写 `.bak` + 临时文件原子替换，失败回滚）、OpenAI 兼容 AI（档位 0–5，0 为空载荷，探针 JSON 不默认附带）。Desktop 侧 `ForayPage` 工作台、设置 Editor Mode、主页/转换页拖入分流（事件时读 EM）。

**Verification** — `cargo test --lib` 171 passed（含 foray 23）；`npx vue-tsc -b` PASS；`npx vite build` PASS。独立 review 3 critical（原地导出截断风险 / 未改字节保留 / EM mount 短路）已修复并复测。

**Journey log**
- Hurray 104 转换器全量 ROM 化明确 Out of Scope，Foray 侧独立树。
- `@tauri-apps/plugin-notification` JS 路径走 `window.Notification` 的教训：跨层 API 要核对真实实现。
- PowerShell 写 UTF-8 易 BOM/乱码；版本与文案改动用 node/编辑工具。
- 原地导出须「.bak + 临时文件 + 替换/回滚」，不能 `File::create` 直接截断源包。

## [S1] Problem

材质包制作者在「做包 / 改包 / 审包」时缺少一体化工作台：

1. **看不清结构** — zip 内资源树、类型分布、异常文件需要外部解包，费时且易漏。
2. **风险不可见** — 加密、只读、可执行脚本、Zip Slip / Zip Bomb 等恶意内容，导入前往往事后才发现。
3. **改包成本高** — 只为改几个像素也要开 Photoshop；着色器 / JSON 是否适配新版本只能靠试装。
4. **AI 分析不可控** — 若接入大模型，用户担心数据外传与黑盒提示词。

**用户画像**：做包、改包、审查包的制作者。  
**产品定位**：Foray 是资源包结构分析与轻量编辑工作台，平行于 Hurray 转换引擎，默认关闭。

**触发条件（用户已确认）**：

- 设置中开启 **Editor Mode（EM）** 后，将 zip 拖入 **主页面** 进入 Foray；
- 关闭 EM 时拖入主页面仍是普通转换流程，**不会**误入 Foray。
- 实现注记：拖放监听常驻，**事件发生时**读取 `localStorage.editorMode`，避免开关后误分流。

## [S2] Design

### 2.0 已锁定决策（2.5.0 范围）

| 轴 | 决策 |
|---|---|
| 功能深度 | **全量**：结构分析 + 探针 + ROM 树 + 大视图 + 轻量编辑 + AI 分析 |
| ROM | **Foray 侧独立 ROM**，不改写 Hurray 104 个转换器；Hurray 继续文件流 |
| 编辑保存 | **默认另存副本**；高级选项可 **原地覆盖**（二次确认） |
| AI 提供方 | **OpenAI 兼容**（自定义 `baseURL` + API Key + 模型名） |
| 隶属关系 | 不隶属 Hurray；平行独立模块；默认关闭 |

### 2.1 架构位置

```
2-Pyramid
├── Hurray Engine（转换核心，Rust）          — 不改写
├── Foray（分析工作台，独立模块）            — 本特性
│   ├── ROM（Resource-pack Object Model）  — 仅 Foray 内存树
│   ├── Probe 系统
│   ├── 恶意 / 安全分析
│   ├── 轻量编辑（像素级）
│   └── AI 分析（OpenAI 兼容，用户自备 Key）
├── Desktop UI（Tauri + Vue）              — 新增 Foray 路由 / 面板
├── Android / TUI / Enzyme                 — 2.5.0 不接
└── Ifaso（外部商业软件）                   — 仅互链入口，不共用代码
```

**Foray ↔ Hurray 契约**：

- 2.5.0 **单向可选**：Foray 可把「已分析路径 / 元数据」交给转换页做二次转换，但转换不依赖 ROM。
- Hurray 仍以「解压 → 文件变换 → 打包」为主路径；**禁止**在本特性内把转换器改成对象操作。
- 未来 Master Enzyme 实时编辑走 ROM 时，另开迁移特性（Out of Scope）。

### 2.2 工作流（主路径）

```
EM=on → 拖 zip 到主页面
  → 安全预检（Zip Slip / Bomb / Symlink）
  → 解析 pack.mcmeta / pack.png
  → 解析 assets/ 目录树 → 构建 ROM
  → 并行跑探针（文件 / 权限 / 加密 / 解析 / 恶意）
  → ROM 大视图（树 + 概览卡 + 异常高亮）
  → [可选] AI 分析（按档位打包摘要）
  → [可选] 轻量编辑 → 另存 / 原地覆盖
  → [可选] 发送到转换页（Hurray）
```

失败路径：安全预检失败则 **拒绝进入 ROM** 并展示原因；单探针失败不阻断树构建，记为「探针错误」徽章。

### 2.3 模块划分（Rust `foray/`）

| 模块 | 职责 | 对外接口 |
|---|---|---|
| `foray::zip_safe` | Zip Slip / Bomb / Symlink / 嵌套深度 / 解压尺寸上限 | `open_pack` / `open_pack_bytes` |
| `foray::mcmeta` | `pack.mcmeta` 严格解析 | `parse_mcmeta` |
| `foray::rom` | 资源包对象树 | `rom::build` |
| `foray::probe` | 探针调度与结果聚合 | `run_probes` |
| `foray::paint` | 涂抹 / 吸管 / HSV / 撤销 | `PaintSession` |
| `foray::ai` | OpenAI 兼容 + 档位打包 | `build_tier_payload` / `chat_completion_blocking` |
| `foray::export` | 另存 / 原地覆盖（原子替换 + 回滚） | `export_pack` |

**安全边界（已实现）**：

- 解压上限：单文件 ≤ 64 MiB，总解压 ≤ 1 GiB，条目数 ≤ 50_000，路径深度 ≤ 32。
- 拒绝绝对路径 / `..` 逃逸 / symlink（unix mode）。
- JSON 嵌套深度 ≤ 64、输入 ≤ 8 MiB；PNG 宽高 ≤ 8192。
- AI：Key 不写日志；请求仅发用户 `baseURL`；档位 0 空载荷；探针 JSON **默认不附带**。
- 提示词：内置默认，可被配置覆盖（`AiConfig.system_prompt`）。

### 2.4 UI（Desktop Vue）

- **ForayPage**：树 / 概览 / issues / 预览 / Paint / 导出 / AI 面板 / Ifaso 占位。
- **设置**：Editor Mode 开关。
- **主页 / 转换页拖放**：事件时读 EM → foray.pendingPath → `foray` 页。

### 2.5 ROM（Foray 侧独立对象模型）

见 `foray/rom.rs`：`Rom { meta, icon, root: RomDir, stats, issues, source_path }`；`RomFile { path, kind, size, sha256, parse, paintable, dirty, data }`。

**约束**：ROM 只服务 Foray；导出时未 `dirty` 条目 **raw_copy 字节保留**。

### 2.6 AI 数据档位

| 档位 | 发送内容 |
|---|---|
| 0 | 无（不调用 API） |
| 1 | 目录树 + 扩展名统计 |
| 2 | + `pack.mcmeta` |
| 3 | + 勾选 JSON 副本（≤20×64KB） |
| 4 | + 勾选着色器（≤10×128KB） |
| 5 | + 贴图概括（尺寸/均色，非像素） |

默认 1；≥3 显式勾选；「将发送」预览在 ForayPage 可查看。

### 2.7 导出 / 保存契约（已强化）

| 模式 | 行为 |
|---|---|
| 另存副本（默认） | `源名_foray_<ts>.zip` |
| 原地覆盖 | 二次确认；先 `.zip.bak`；写 `.foray_tmp` 成功后再替换；失败恢复 bak |

### 2.8 与 Ifaso 关系

仅文案/入口占位；无互通实现。

### 2.9 测试边界

已有单测：zip 逃逸/绝对路径/条目上限、mcmeta、rom 树、探针恶意/熵、paint、export 另存/原地/dirty、AI 档位/脱敏。  
**已知缺口（2.5.0 验收后可补）**：bomb 比率夹具、HTTP mock、UI 组件测试、提示词「还原默认」按钮、Key 落盘 0600、贴图直方图摘要、吸管快捷键。

## [S3] Out of Scope

- 改写 Hurray 104 个转换器 / 全量测试重写
- 图层、蒙版、滤镜、3D
- AI 文生图、账号云、插件市场
- Android / TUI / Enzyme Foray
- Ifaso 协议互通
- CVE 在线库 UI

## Tasks

- [x] T1: `foray::zip_safe` 安全解压门禁 — acceptance: 路径逃逸/zip bomb/symlink 夹具全拒，单测绿（covers: S2.3）
- [x] T2: `foray::mcmeta` + `foray::rom` 构建与统计 — acceptance: 最小 pack fixture 建树，能列目录/类型/体积（covers: S2.5; depends: T1）
- [x] T3: `foray::probe` 五探针 + 恶意/加密启发式 — acceptance: 损坏/可执行/加密夹具产出对应 issue，不阻断树（covers: S2.3; depends: T2）
- [x] T4: EM 开关 + 主页拖入分流 + Foray 路由骨架 — acceptance: EM 开/关拖 zip 行为符合 [S1]，Foray 页可进可退（covers: S2.1, S2.4; depends: T2）
- [x] T5: ROM 大视图（树 + 概览 + 探针列表 + 异常高亮） — acceptance: 打开样例包可见树与探针结果，选中有详情（covers: S2.4; depends: T3, T4）
- [x] T6: 轻量编辑 PaintSession + 撤销 — acceptance: 涂抹/吸管/HSV 可改 PNG 且导出可见差异；撤销可用（covers: S2.4, S2.3; depends: T2）
- [x] T7: 导出（默认另存 / 原地覆盖+bak） — acceptance: 未编辑文件字节保留；原地有确认与备份（covers: S2.7; depends: T6）
- [x] T8: AI 档位打包器 + 提示词 JSON 加载/覆盖 — acceptance: 0–5 档内容符合表，预览列表准确（covers: S2.6, S2.3; depends: T2）
- [x] T9: OpenAI 兼容客户端 + 连接测试 + 分析报告 — acceptance: 真 Key 可选手测；日志无 Key（covers: S2.3, S2.6; depends: T8）
- [x] T10: AI/编辑侧栏与档位隐私 UI + 免责声明 — acceptance: 确认前可见「将发送」与风险说明（covers: S2.4, S2.6; depends: T5, T8）
- [x] T11: Ifaso 入口占位（不实现互通） — acceptance: Foray 出现互链文案，无网络副作用（covers: S2.8; depends: T5）
- [x] T12: 安全与回归测试收口 — acceptance: `cargo test --lib` + `vue-tsc` 通过（covers: S2.3, S2.9; depends: T1–T11）
