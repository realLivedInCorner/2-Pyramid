# Microsoft Store 提交文案 · 2.200.0

> 字段：**此版本的新增功能 / What's new in this version**
> 规则：**纯文本**（不支持 Markdown / HTML / 图片），上限 **1500 字符**（含换行）。
> 建议直接整段复制，不要在商店后台加粗或加链接。

---

## 中文（zh-CN）

2.200.0 换掉了转换内核，并由此明显更快——结果与上一版完全一致。

· 更快：同一个 3600 文件的材质包，内部转换耗时的可比口径由约 6.0 秒降到约 3.6 秒；日常实际等待为约 0.3 秒的内部转换加上约 1.2 秒的读写。界面操作（GUI 贴图处理）单项由约 1.4 秒降到约 0.5 秒。
· 结果不变：本次提速不改变任何输出内容——4018 个文件逐条目比对（路径、大小、内容哈希）全部一致，并额外校验了打包与目录两种输出方式完全等价。
· 批量转换更快：同时转换多个包时，线程会按你的性能档位设置使用 CPU，而不是被并发包数限制。
· 法律信息更完整：安装包内附第三方开源组件的许可证正文，可在应用内的法律声明中查阅。
· 提示更清楚：时间日志新增各阶段耗时明细，反馈问题时更容易定位。

遇到问题欢迎在仓库 Issues 反馈，并附上「设置 → 开发者模式 → 导出日志」（默认脱敏，可放心分享）。

---

## English (en-US)

2.200.0 replaces the conversion engine, and is noticeably faster because of it -- with identical results.

· Faster: on the same 3,600-file pack, comparable internal conversion time dropped from about 6.0s to about 3.6s; day-to-day waiting is roughly 0.3s of conversion plus about 1.2s of file I/O. The GUI texture pass alone went from about 1.4s to about 0.5s.
· Same output: nothing about the result changed. All 4,018 files match entry by entry (path, size, content hash), and zipped and unpacked output were verified to be equivalent.
· Faster batch conversion: with several packs at once, threads now follow your performance setting instead of being capped by the number of packs.
· More complete legal info: the installer now bundles the license texts of its third-party components, readable in the in-app legal notices.
· Clearer reporting: timing logs now break the conversion into stages, making problem reports easier to place.

Found a problem? Please open an issue and attach a log exported from Settings > Developer mode > Export logs (redacted by default, safe to share).

---

## 可选：简短说明（Short description，若本次一并更新）

> 与 2.7.0 相比**无实质变化**——本版是内核重写，不改变产品定位。若商店要求随包更新该字段，可沿用 2.7.0 的文本，无需改动。

**中文**：无损提速的 Minecraft 材质包版本转换器：多版本分层解析、Bedrock 实验性转换、日志脱敏。

**English**: A losslessly faster Minecraft resource pack converter: multi-version overlay analysis, experimental Bedrock conversion, redacted logs.

---

## 各字段字数（含换行）

| 字段 | 语言 | 字符数 | 上限 | 余量 |
|---|---|---|---|---|
| 此版本的新增功能 | zh-CN | 413 | 1500 | 1087 |
| 此版本的新增功能 | en-US | 1085 | 1500 | 415 |
| 简短说明（可选） | zh-CN | 52 | 1000 | 948 |
| 简短说明（可选） | en-US | 134 | 1000 | 866 |

（字符数为**实测值**，含换行与空格；测量方式是取标题行与 `---` 之间的整段正文，`len("\n".join(正文行))`。
正文不含 Markdown/HTML/链接记号，可直接粘贴。）

## 与 2.7.0 文案的取舍差异（本次刻意不写的）

| 没写的内容 | 原因 |
|---|---|
| 内部结构（对象模型、模块搬迁、删掉旧引擎） | 商店读者不关心实现，只关心体验与结果。 |
| 「4018 条目 / 指纹」等闸门术语 | 2.7.0 用「4018 个文件全部一致」已足够传达，术语留给仓库文档。 |
| release 口径的倍数（如「快 N 倍」） | 该口径没有与上一版可比的历史记录，给倍数会不可复核；因此只报绝对值，而把「约 6.0s → 约 3.6s」放在**可比口径**里。 |
| 「性能档位」作为新增功能 | 它是 2.7.0 的新功能，本版只是让它在批量场景下真正生效——按「修正」而非「新增」表述。 |
| 「进度更新不再让每个任务白等」 | **这是 2.7.0 的修复**（`progress-ticker` 的 `join()` 与 `sleep(200ms)`）。本版初稿误把它当成新改善，核对 CHANGELOG 后删除——用户在 2.7.0 就已经拿到了这个提速，重复计入等于虚报。 |
