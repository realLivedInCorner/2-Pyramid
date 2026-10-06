# 安全策略 / Security Policy

**生效日期：** 2026-10-06  
**适用范围：** 2-Pyramid 主程序、自研安装器、官方发布渠道的安装包。

---

## 1. 支持版本

| 版本线 | 支持状态 |
|--------|----------|
| 当前版本线（**2.200.x**，最新正式版 Stable tag） | 安全修复优先 |
| 最新 Beta | 尽力修复，不保证 SLA |
| 更早正式版（**2.7.x 及以前**） | 建议升级；一般不再单独出补丁 |

## 2. 如何报告漏洞

**请勿**在公开 Issue 里贴出可直接利用的细节。

推荐路径：

1. 打开仓库 [Security Advisories](https://github.com/realLivedInCorner/2-Pyramid/security/advisories) 提交 **Private vulnerability report**；若该入口不可用，请开带 [SECURITY] 前缀的 Issue，维护者会引导转入私下渠道
2. 或在 Issues 发起标题含 `[SECURITY]` 的讨论，**仅描述影响面与复现步骤概要**，维护者会引导转入私下渠道  
3. 附上：受影响版本 / BUILD 号、复现步骤、预期与实际行为、PoC（可私传）

我们会在收到报告后 **7 个工作日内**初步回应；确认为有效漏洞后会协调修复与披露时间。

## 3. 安全设计要点（便于你评估风险）

- **本地优先**：资源包转换默认不上传文件；详见 `PRIVACY.md`
- **更新完整性（强制）**：Release **必须**附带 `.sha256`，更新器下载后校验哈希：**不匹配则拒绝安装并删除文件；未附带 `.sha256` 或校验文件拉取失败的版本一律拒绝更新**（无法证明完整性的更新不允许安装）
- **下载域名白名单**：安装包与哈希仅允许来自 `github.com` / `objects.githubusercontent.com` **及其子域**（代码为 `host == "github.com" || host.ends_with(".github.com")`）。国内镜像 `cdn.5eggpack.top` 已停止维护并从更新链路移除
- **仅本机监听**：应用只绑定回环地址——单实例锁 `127.0.0.1:24157`（同端口投递）；debug 构建另起动作流端口 `127.0.0.1:24159`。**不监听任何外部网卡**
- **CSP**：WebView 限制脚本与资源来源
- **无管理员权限**：安装走 HKCU，不要求 UAC 提权

## 4. 已知边界（非漏洞，但使用时请注意）

- 转换**任意**用户提供的 zip/资源包会解析大量 PNG / JSON / 文本；异常输入可能导致耗时或失败，一般不会执行包内代码
- 着色器为 GLSL 源码改写，**不会**在你的 GPU 上编译执行不可信二进制
- 若从非官方渠道下载安装包，SHA 校验链路可能被绕过，请只信任本仓库的 GitHub Releases

## 5. 致谢

有效安全报告会在 Release Notes 或本文件中致谢（可要求匿名）。

---

**English summary:** Report vulnerabilities privately via GitHub Security Advisories or a `[SECURITY]` issue without exploit details. We aim to reply within 7 business days. Releases ship SHA-256 checksums; downloads are domain-restricted.
