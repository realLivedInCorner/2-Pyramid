# 隐私说明 / Privacy Policy

**生效日期：** 2026-10-06  
**适用产品：** 2-Pyramid（Windows 桌面应用及自研安装器）

2-Pyramid 的设计原则是：**完全本地处理、无账号、无遥测**。下文说明我们实际会与不会收集什么。

---

## 一句话结论

转换资源包时，你的文件**不会离开本机**。我们不上传任何个人身份信息，也不做使用行为追踪。

---

## 1. 我们不收集的内容

- 不上传你导入的资源包、贴图、着色器或覆盖包配置（**Foray AI 分析除外**，见第 4 节）
- 不采集崩溃堆栈自动上报、不埋点、不统计 DAU/功能使用率
- 不要求注册账号，不存储密码、邮箱、手机号
- 不读取与功能无关的浏览器历史、通讯录或其它应用数据

你填写的「显示名称 / 用户名」仅保存在本机配置中，用于主页问候语，可随时在设置中修改或清空；**不会上传**。

## 2. 本机存储（多个目录，用途如下）

| 内容 | 实际位置 | 说明 |
|------|----------|------|
| 用户设置、历史、背景图、备份、Foray AI 配置 | 用户主目录下的 `.2pyr`（如 `~/.2pyr` / `%USERPROFILE%\.2pyr`） | `configs/settings.json`、`history.json`、`background/`、`backups/`、`foray-ai.json` |
| 覆盖包项目 | 系统「文档」目录下的 `2-Pyramid` | 项目元数据与你编辑的覆盖包工作区 |
| 运行日志 | 系统「本地应用数据」下的 `2-Pyramid\logs` | 日期滚动日志，可经「设置 → 开发者模式 → 导出日志」导出 |
| 更新标记 | 系统「配置」目录下的 `2-Pyramid` | 应用内更新状态 |
| 转换临时文件 | 系统临时目录（`.2pyr-work-*`） | 解压与中间产物；正常结束后由后台线程或独立子进程清理，异常退出残留会在下次转换启动时清除 |

日志可能包含**文件路径**与错误信息，便于排障。**两种导出（会话日志 / 磁盘日志）默认都会脱敏**：导出的文件会自动隐藏 Windows 用户名、机器名、IP、邮箱、API Key/token，并把包路径的目录部分压成 `<path>\文件名`（仅导出文件脱敏，磁盘日志与界面保持原文，便于你自己排查）。可在「设置 → 开发者选项」关闭脱敏；开发者模式下还可导出未脱敏原文（需二次确认，请勿公开分享）。

## 3. 网络访问（在你主动触发时，或「自动检查更新」开启时）

| 场景 | 目的地 | 数据 |
|------|--------|------|
| 检查更新 | GitHub Releases API（`api.github.com`） | 仅拉取 release 列表；不上传你的文件或设备指纹 |
| 下载更新 | `github.com` / `objects.githubusercontent.com` | 下载官方安装包与 `.sha256` 校验文件 |
| **Foray AI 分析（可选，默认关闭）** | **你配置的 OpenAI 兼容 `baseURL`** | **依数据档位而定，详见第 4 节** |

> 更新源已固定为 GitHub 官方：设置里不再有「更新源」选项、测速或切换入口。国内镜像
> `cdn.5eggpack.top` 曾作为可选更新源，**已于 2026-10 彻底移除**（镜像作者停止维护），
> 现不再有任何请求发往该域名。

**自动检查更新默认开启**：应用启动时会请求一次 release 列表（即上表第一行）。在「设置 → 版本与更新」关闭「自动检查更新」后，只有你主动点击检查/下载时才会联网。除此之外，未启用 Foray AI 时应用不会发起上述请求。

## 4. 第三方处理者

除更新检查/下载（仅 GitHub）外，本软件默认**不向第三方发送数据**。

**唯一例外：Foray AI 分析**（仅当你主动启用并配置服务时）：请求会发往你填写的 **OpenAI 兼容** `baseURL`。可能包含：**目录树与扩展名统计（数据档位 ≥1 即包含，这是默认档位）**、`pack.mcmeta`、你勾选的 JSON（≤64 KB）/ 着色器（≤128 KB）**副本**、贴图**概括**（尺寸/均色/直方图，**非像素**），依你选择的数据档位而定；**档位 0 不发送任何内容、也不调用外部服务**。API Key 仅保存在本机配置文件（`~/.2pyr/foray-ai.json`）用于向该服务鉴权；**Windows 下该文件按当前用户可读的明文保存，请勿在共享账户中留存密钥**。该服务的隐私政策由服务商制定，与 2-Pyramid 作者无关。

## 5. 分享码

覆盖包「分享码」（`2PYR-…`）由本机导出、通过你自己的渠道粘贴传输。应用本身**不代传、不托管**分享码。若你使用第三方平台分享，请同时阅读该平台的用户协议与隐私说明。

## 6. 未成年人

本软件不面向 13 岁以下儿童设计，也不会在知情情况下收集儿童个人信息。

## 7. 你的权利

- 可随时删除第 2 节所列各目录中的数据以清除本地记录（主要为 `~/.2pyr`、文档与本地应用数据下的 `2-Pyramid`）
- 可卸载应用；卸载器默认保留用户数据，删除数据需你手动操作
- 对隐私有疑问，请在仓库 Issues 提问

## 8. 变更

政策更新会修改文首日期并在仓库说明。重大变更将通过 Release Notes 提示。

## 8.1 仓库层面的数据流（与应用无关）

本节说明的是 **GitHub 仓库**，不是安装后的应用：

- **Issue / PR / 提交元数据 → 飞书（Lark）**：本仓库的 GitHub Actions 工作流（`.github/workflows/feishu-notify.yml`）会把 **Issue 与 PR 的标题、作者用户名、链接，以及提交元数据**发送到项目维护的第三方飞书（Lark）webhook，用于通知维护者。这些信息在 GitHub 上本就是**公开内容**，不包含你本机的任何数据或资源包内容；若你不希望被转发，请不要提交 Issue / PR，或要求维护者移除该 webhook。
- 应用本身**从不访问飞书**，除你自行配置的 Foray AI endpoint 外，也不会把资源包内容发往任何地方。

## 9. 安装与卸载（EXE / MSIX / 静默）

静默安装（`--silent`）不会额外上传数据。卸载删除程序文件；`~/.2pyr` 等用户数据默认保留。GitHub Releases 的 exe 安装器与 Microsoft Store 的 MSIX 包写入的是**同样的逻辑路径**，但 MSIX（打包应用）下 Windows 会把 `%APPDATA%` / `%LOCALAPPDATA%` / `%TEMP%` 写入**重定向到包专属的虚拟化位置**，因此实际落盘位置与 exe 版不同（卸载行为由商店条款约束）。

---

**English summary:** 2-Pyramid processes resource packs entirely on your device. No account, no telemetry, no upload of your packs (except optional Foray AI analysis you enable yourself). Network is used for update checks — **automatic update check is on by default at startup** and can be disabled in Settings — for downloading updates (GitHub Releases only), or when Foray AI is enabled. Share codes are exported locally; the app does not host them.

**联系方式：** GitHub Issues — https://github.com/realLivedInCorner/2-Pyramid/issues
