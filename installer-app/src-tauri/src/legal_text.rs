// 安装器展示用 EULA / 免责摘要（与仓库 legal/EULA.md、DISCLAIMER.md 同步维护）
// 简短版：安装向导勾选用；完整文本随 payload 释放到安装目录 legal/。

pub const EULA_TITLE: &str = "最终用户协议（摘要）";

pub const EULA_BODY: &str = r#"2-Pyramid 以 MIT 许可证开源。本协议不缩减你在 MIT 下的权利（使用、修改、再分发等）；与 MIT 冲突时以 MIT 为准。

安装或继续使用即表示你已阅读并知悉：

1. 转换 / 分发资源包时，你须自行遵守 Minecraft EULA 以及资源包原作者的授权。
2. 本软件按「现状」提供，转换结果不作担保；请先备份重要资源包。
3. 本软件与 Mojang Studios / Microsoft 无隶属、无赞助、未获认可。
4. 完整法律文本见安装目录 legal/ 文件夹，或应用内「设置 → 法律信息」。

完整协议：legal/EULA.md · 免责：legal/DISCLAIMER.md · 隐私：legal/PRIVACY.md"#;
