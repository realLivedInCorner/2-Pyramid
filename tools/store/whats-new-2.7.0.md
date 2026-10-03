# Microsoft Store 提交文案 · 2.7.0

> 字段：**此版本的新增功能 / What's new in this version**
> 规则：**纯文本**（不支持 Markdown / HTML / 图片），上限 **1500 字符**（含换行）。
> 建议直接整段复制，不要在商店后台加粗或加链接。

---

## 中文（zh-CN）

2.7.0 让转换明显更快，同时保证结果完全不变。

· 更快：用 3600 个文件的大包实测，从点击转换到完成的等待时间由约 8 秒降到约 3.5 秒（约 2.2 倍）；其中解压环节快 54%。
· 无损：本次提速不改变任何输出内容——同一个包在优化前后各转一次，4018 个文件全部一致，并通过逐字节校验。
· 新增性能档位：设置里可选「平衡」（用一半核心，电脑照常使用）或「性能」（吃满核心）。界面会显示本机核数与同时转换的包数，内存不足时自动降低并发。
· 完成提示更即时：转换结束后不再等待临时文件清理，清理在后台继续完成。
· 新增「分析资源包结构」（开发者选项）：转换前查看多版本包的分层、适用版本区间与目录结构，便于判断转换范围。
· 耗时日志更清楚：日志分别给出「纯转换时间」与「总时间（含读写）」，反馈问题时更容易定位。

遇到问题欢迎在仓库 Issues 反馈，并附上「设置 → 开发者模式 → 导出日志」（默认脱敏，可放心分享）。

---

## English (en-US)

2.7.0 makes conversion noticeably faster while keeping the output identical.

· Faster: on a large pack with 3,600 files, end-to-end waiting dropped from about 8s to about 3.5s (roughly 2.2x); extraction alone is 54% faster.
· Lossless: none of the output changed. The same pack converted before and after the optimization produced 4,018 identical files, verified byte for byte.
· New performance mode: pick Balanced (half the cores, your PC stays responsive) or Performance (all cores). The app shows your core count and how many packs run at once, and lowers concurrency automatically when memory is low.
· Immediate completion: the "conversion complete" notice no longer waits for temporary files to be deleted; cleanup continues in the background.
· New "Analyze pack structure" (Developer options): inspect overlay layers, supported version ranges and directory layout of multi-version packs before converting.
· Clearer timing logs: logs now separate pure conversion time from total time including file I/O, which makes reports easier to diagnose.

Found a problem? Please open an issue and attach a log exported from Settings > Developer mode > Export logs (redacted by default, safe to share).

---

## 可选：简短说明（Short description，若本次一并更新）

**中文**：无损提速的 Minecraft 材质包版本转换器：多版本分层解析、Bedrock 实验性转换、日志脱敏。

**English**: A losslessly faster Minecraft resource pack converter: multi-version overlay analysis, experimental Bedrock conversion, redacted logs.

---

## 各字段字数（含换行）

| 字段 | 语言 | 字符数 | 上限 | 余量 |
|---|---|---|---|---|
| 此版本的新增功能 | zh-CN | 427 | 1500 | 1073 |
| 此版本的新增功能 | en-US | 1201 | 1500 | 299 |
| 简短说明（可选） | zh-CN | 52 | 1000 | 948 |
| 简短说明（可选） | en-US | 134 | 1000 | 866 |

（字符数为实测值，含换行与空格；正文不含 Markdown/HTML 记号，可直接粘贴。）
