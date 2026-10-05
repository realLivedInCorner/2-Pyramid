# 安全策略 / Security Policy

> 本文件是 **GitHub 发现入口**。完整正文在 [legal/SECURITY.md](../legal/SECURITY.md)
> （English: [legal/en/SECURITY.md](../legal/en/SECURITY.md)）——请以那里为准，本文件不重复内容。

## 报告漏洞 / Reporting a vulnerability

**请勿用公开 Issue 报告安全问题。** 使用 GitHub 的私密报告通道：

**Do not open a public issue for a security problem.** Use GitHub's private reporting:

<https://github.com/realLivedInCorner/2-Pyramid/security/advisories/new>

其它联系与支持走 [Issues](https://github.com/realLivedInCorner/2-Pyramid/issues)。

---

## 快速摘要 / At a glance

2-Pyramid 是**本地优先**的桌面工具：资源包转换全程离线，不上传你的文件。会联网的只有两处，
且都可关闭：**更新检查**（仅官方 GitHub）与你**主动启用**的 Foray AI 分析（发往你自己配置的
endpoint）。详见 [legal/PRIVACY.md](../legal/PRIVACY.md)。

发布产物的完整性是**强制校验**的：Release 必须附带同名 `.sha256`，更新器校验不过即拒绝安装。

支持的版本线与完整边界说明见 [legal/SECURITY.md](../legal/SECURITY.md)。
