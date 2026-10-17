# 贡献指南 / Contributing

> 本文件是 **GitHub 发现入口**。完整正文在 [legal/CONTRIBUTING.md](../legal/CONTRIBUTING.md)
> （English: [legal/en/CONTRIBUTING.md](../legal/en/CONTRIBUTING.md)）——请以那里为准。

## 提交 PR 前请先跑 / Before opening a PR

```bash
npm run test:offline                                # Rust 单测（离线）
npm run build                                        # 前端构建
cargo check --bins --manifest-path src-tauri/Cargo.toml
```

改到转换逻辑或引擎时，**另外**要跑真实包对照（需要 `tools/TapL 16x.zip`）：

```powershell
$env:AROM_REAL_PACK='tools\TapL 16x.zip'; $env:AROM_TARGET='97'
cargo test --lib --manifest-path src-tauri/Cargo.toml -- --ignored
```

**内容基线只报告、不判定**（2.203.0 起）：

```
内容基线：4018 个条目，聚合指纹 = 0x75bb3260e7f578a6
绝对产物：files=4018 bytes=19294735
```

> 上面这两个数字是 **2.202.0 时期的实测快照**，仅作参考 —— 用例会打印当前值，但**不会因为不等于它们而失败**。
>
> 为什么不冻结了：那个常量后来挡住的不是回归，而是**有意的改动**（修按钮九宫格边厚、加物品模型定义），每次都要先改常量再提交。**一个改代码时就要同步改的数字，不是哨兵。**
>
> 需要判断「产物有没有变」时，用 `AROM_BASELINE_DUMP=<路径>` 导出逐条目清单，与上一版清单**逐行 diff** —— 那才回答「这版和上版差在哪」，比一个总数信息量大得多。仓库内 `tools/arom-baseline.txt` 是 2.201.0 时期的快照，不再被断言。


## 代码放哪里

| 内容 | 位置 |
|---|---|
| 转换任务实现（按角色分组） | `src-tauri/src/natives/{eraser,architect,surgeon,reverse}/` |
| 任务元数据（名字 / 类型 / 阶段） | `src-tauri/src/task_registry.rs`（改后跑 `cargo test --lib registry_shape_is_pinned --manifest-path src-tauri/Cargo.toml` 校验） |
| 对象模型与引擎 | `src-tauri/src/arom/`（调度在 `arom/engine/scheduler.rs`） |
| 资源包 I/O 与分析 | `src-tauri/src/pack/` |
| 基岩互转 | `src-tauri/src/bedrock_convert/` |

**不要**把业务写进 `invoke_conversion.rs`（该模块现在不做任何事，元数据在 `task_registry.rs`）。

提交信息用英文祈使句 + conventional commit（如 `fix(overlay): correct share code prefix`）。

其余规范（前端、文案、测试要求）见 [legal/CONTRIBUTING.md](../legal/CONTRIBUTING.md)。
