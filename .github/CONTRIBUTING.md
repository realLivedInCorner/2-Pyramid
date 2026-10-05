# 贡献指南 / Contributing

> 本文件是 **GitHub 发现入口**。完整正文在 [legal/CONTRIBUTING.md](../legal/CONTRIBUTING.md)
> （English: [legal/en/CONTRIBUTING.md](../legal/en/CONTRIBUTING.md)）——请以那里为准。

## 提交 PR 前请先跑 / Before opening a PR

```bash
npm test                                            # Rust 单测（离线）
npm run build                                        # 前端构建
cargo check --bins --manifest-path src-tauri/Cargo.toml
```

改到转换逻辑或引擎时，**另外**要跑真实包对照（需要 `tools/TapL 16x.zip`）：

```powershell
$env:AROM_REAL_PACK='tools\TapL 16x.zip'; $env:AROM_TARGET='97'
cargo test --lib --manifest-path src-tauri/Cargo.toml -- --ignored
```

**冻结基线必须逐字不变**（除非有意改动并说明理由）：

```
内容基线：4018 个条目，聚合指纹 = 0x75bb3260e7f578a6
绝对产物：files=4018 bytes=19294735
```

> 注意：基线的**冻结值**由 `tools/arom-baseline.txt` 与指纹用例守住，但**无法再重新生成**——
> 旧实现已在 M3 中删除。要改基线必须有意为之并在 PR 中说明。

## 代码放哪里

| 内容 | 位置 |
|---|---|
| 转换任务实现（每任务一模块） | `src-tauri/src/natives/{eraser,architect,surgeon,reverse}/` |
| 任务元数据（名字 / 类型 / 阶段） | `src-tauri/src/task_registry.rs`（改后跑 `pwsh tools/gen-task-registry.ps1` 校验） |
| 对象模型与引擎 | `src-tauri/src/arom/`（调度在 `arom/engine/scheduler.rs`） |
| 资源包 I/O 与分析 | `src-tauri/src/pack/` |
| 基岩互转 | `src-tauri/src/bedrock_convert/` |

**不要**把业务写进 `invoke_conversion.rs`（它只做元数据登记的存根）。

提交信息用英文祈使句 + conventional commit（如 `fix(overlay): correct share code prefix`）。

其余规范（前端、文案、测试要求）见 [legal/CONTRIBUTING.md](../legal/CONTRIBUTING.md)。
