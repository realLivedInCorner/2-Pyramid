✨ Beta 2.4.2 更新（BUILD 20047）

这是 2-Pyramid 的测试版（tag: Beta-2.4.2）。本版加入 **Foray（Editor Mode）分析工作台**（实验），并修复桌面 Toast / 系统通知与导出安全问题。

可通过应用内「更新通道 → 测试版 / 全部」检查到，或直接在 Releases 下载 2-Pyramid-Installer-2.4.2-beta.20047.exe。

✨ 新增

Foray 分析工作台（Editor Mode，默认关闭）：在设置中开启 EM 后，将 zip 拖入主页面即可进入 Foray，而不再走普通转换。功能包括资源包 ROM 树与类型统计、五类探针（文件 / 权限 / 加密 / 解析 / 恶意）、轻量像素编辑（涂抹、吸管、HSV），以及可选的 AI 分析（OpenAI 兼容接口，自备 API Key）。AI 支持数据档位 0–5：贴图只发送尺寸、均色与粗直方图等概括，不发送像素；档位 0 不会访问外部 API。默认导出为另存副本；原地覆盖需二次确认并写 .bak 备份。Foray 与转换引擎平行，默认关闭，不影响普通转换流程。

系统通知改走 Rust 插件路径（notify-rust），桌面 Toast 作为可靠主通道。

🐛 修复

通知无法弹出：桌面 Toast 窗口权限与透明窗口修复；系统通知不再依赖 WebView 的 window.Notification（在 Windows 上经常静默失败）。仅选「系统」时若系统通知失败，会自动回退桌面 Toast。

Foray 导出：未修改文件保持压缩字节原样；原地覆盖失败时从备份回滚，避免损坏源包。

Editor Mode 拖放：切换开关后立刻生效，不再误入普通转换。

🔐 安全与法律

legal/ 补充 Foray AI 外部 API 免责声明与隐私说明：API Key 只存本机（~/.2pyr/foray-ai.json），请求只发往你配置的服务地址；外部服务行为与作者无关。

zip 安全门禁：拒绝路径穿越、条目/体积/压缩比异常；PNG 超大辅助块告警。

发版说明：本 Release 需同时上传 2-Pyramid-Installer-2.4.2-beta.20047.exe 与
2-Pyramid-Installer-2.4.2-beta.20047.exe.sha256 两个资产（更新器 SHA-256 校验依赖后者）。

校验（本次构建）：
- 2-Pyramid-Installer-2.4.2-beta.20047.exe.sha256: a2e6265986ceb8e16eec9750b3a6ed0b519e3dbcaf17e09014f288997928a036

如何反馈问题：在仓库 Issues 提交即可，最好附上「设置 → 开发者模式 → 导出日志」的日志文件，
以及原资源包版本与目标版本（Java / Bedrock）。
