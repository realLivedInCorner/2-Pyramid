# Microsoft Store 上架清单（2-Pyramid）

> 基于 Microsoft Learn（MSIX / Partner Center）整理 · 2026-09 · 供 2.4.x+ 发版用

## 结论先说

| 分发方式 | 包形态 | 更新谁负责 | 需要 `2-Pyramid-Installer.exe` 固定名？ |
|----------|--------|------------|----------------------------------------|
| **A. 正式上 Store** | **MSIX**（`.msix` / `.msixbundle`） | **商店差分更新** | 否（商店不跑你的 exe 安装器） |
| **B. 旁加载 / App Installer** | MSIX + `.appinstaller` 或 exe | 应用/清单拉固定 URL | **是**（或固定名 MSIX） |
| **C. 仅 Store 外 exe 发布** | 当前 exe 安装器 | 自研更新器 | 是（已做） |

2.4.2-beta 的 `--silent` 与固定名 `2-Pyramid-Installer.exe` 主要服务 **B/C** 与脚本部署。若走 **A**，还必须补 MSIX 包。

---

## 走 Store（A）还缺什么

### 1. 账号与名称
- [ ] Partner Center 开发者账号（个人/公司）
- [ ] **提前 reserve** 名称 `2-Pyramid`（保留约 3 个月，过期回收）
- [ ] 确认商标/命名不与他人冲突

### 2. MSIX 包（当前产物是 exe，不是 MSIX）
- [ ] 产出 **MSIX/MSIXBUNDLE**（WinApp CLI / MSIX Packaging Tool / VS）
- [ ] `AppxManifest.xml`：
  - Identity：`Name` / `Publisher`（**必须与签名证书 CN 一致**）/ `Version`
  - `uap:VisualElements`：DisplayName、Description、Logo
  - Desktop bridge：`runFullTrust` 等 capability
  - Application Id、Executable 路径（指向 `2-pyramid.exe` 或安装引导）
- [ ] **图标资源**：Store 要求至少 44×44、150×150、310×310 等尺寸 logo
- [ ] **代码签名**：Store 由微软侧签名上传包时通常用提交流程；旁加载需自签证书

### 3. 功能与合规
- [ ] 隐私声明 URL（`legal/PRIVACY.md` 需可公网访问的链接）
- [ ] 支持 URL / 仓库 Issues
- [ ] 年龄分级问卷（游戏相关工具，一般较低）
- [ ] Store 政策：无恶意、无侵权素材、不伪装官方 Mojang 产品（已有第三方声明）
- [ ] 若含 **Foray AI**：在商店说明中写清「外部 API 自备、数据可出境到用户配置的服务」

### 4. 构建流水线增量
- [ ] `build_release.py` 增加 **msix** 目标（或 `--msix`）：产出 `2-Pyramid-{ver}.msix`
- [ ] 版本号：**MSIX Version 不能降序**；beta 可用四段如 `2.4.2.20047`
- [ ] 上传 **.msixbundle** + 商店配图/截图（1080×1080 等尺寸按 Partner Center 要求）
- [ ] 决定 Store 包内 **是否再嵌入** 你的 exe：通常 **不要**；Store 装的就是解包后的应用

### 5. 与「固定名安装包」的关系
- 你当前要的 `2-Pyramid-Installer.exe` 适合 **更新 URL 固定** 的抓取（B/C）
- 若 Store 商店页更新，则每次仍要 **在 Partner Center 提交新 MSIX**，不依赖 GitHub 固定名 exe
- 若希望「装了 Store 版还能自更」：用 **App Installer**（`.appinstaller`）或应用内检测 + 打开 Store 商品页

---

## 2.4.2-beta 已具备

- `--silent` / `/S` / `--quiet` 等别名，`--dir` / `--install-dir`，`--relaunch`，`--shortcuts`，`--help`
- 固定名 `2-Pyramid-Installer.exe` + `.sha256`（打包线自动复制）
- 法律文本 `legal/`（含 AI 外部服务说明）
- 便携主程序 + 自研安装器（控制面板卸载、静默更新）

## 建议下一步（按优先级）

1. Partner Center **reserve 名称**（可先做，不占开发时间）
2. 决策：Store 只做 **A（MSIX）**，还是 **A+C 双轨**
3. 若做 A：用 **WinApp CLI / MSIX Packaging Tool** 从 `2-pyramid.exe` 便携目录打一版试包，跑一遍 Store 认证检查
4. `build_release.py` 增加 msix 产物与 `msix_version` 字段
