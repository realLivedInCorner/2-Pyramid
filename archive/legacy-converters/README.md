# 旧转换器存档（A-ROM 迁移前的实现）

## 这是什么

`src-tauri/src/converters/` 在 **M3 完成生产路径切换后**的完整快照：
**106 个文件 / 16,580 行**，取自提交 `460f052`。

**它不参与任何构建** —— `src-tauri/src/` 下没有对 `archive/` 的模块引用，
因此这里的代码不会被编译进生产二进制。

## 为什么留着

生产已不再使用这些转换器：入口调用 `native_run::run_native`，逐任务派发到
`src-tauri/src/pilots/` 下的 A-ROM 原生实现（实测 `native_tasks: 46, legacy_tasks: 0`，§9.118）。

**但它们仍是本项目唯一的「参考实现」：**

- `native_switch` 闸门的 `legacy` / `off` 两个对照配置，实现体就是这些转换器；
- **冻结内容基线**（`tools/arom-baseline.txt`，4018 条目的哈希清单）由 `off` 配置生成。

冻结基线只能回答「**和上次比变了没有**」，**不能回答「变了之后对不对」**。
保留这份存档，才有能力在**有意变更**时判断新产物是否正确。

## 怎么恢复到可编译位置

```powershell
pwsh tools/legacy-oracle/restore.ps1
```

该脚本按 `tools/legacy-oracle/untracked-files.txt` 列出的路径，
从提交 `REF_COMMIT` 取回文件到 `src-tauri/src/converters/`（幂等、不改跟踪状态）。

**若 `REF_COMMIT` 已不可达**，也可以直接从本存档复制：

```powershell
Copy-Item -Recurse -Force archive/legacy-converters/* src-tauri/src/converters/
```

## 当前仓库状态的已知限制

**移出后，全新 clone 无法完整构建。** 原因是 `src-tauri/src/` 下仍有引用：

| 位置 | 引用内容 | 性质 |
|---|---|---|
| `invoke_conversion.rs` | 88 个任务的旧闭包注册表 | **仅测试**用（生产走原生派发） |
| `pilots/mod.rs` | `mirror_region` / `swap_and_mirror` / `color::utils` | 前两个是原生实现复用，第三个是合法共享 |
| `pilots/mod.rs`（`#[cfg(test)]` 内） | 约 16 处，作为对照 oracle | 仅测试 |
| `arom/pathview.rs`（`#[cfg(test)]` 内） | 2 处，作为对照 oracle | 仅测试 |
| `version_converter.rs` | `bedrock::{register_tasks, is_bedrock_resource_pack}` | **已 tracked**，不受影响 |

**M3 收口时二选一**：

1. **完成迁移**：把上述引用一并原生化（旧闭包注册表改为「名字+阶段」元数据表、
   两个图像辅助移入 `crate::image_utils`），然后本存档可以删除；
2. **保持存档**：把上述引用用 Cargo feature 门控（`legacy-oracle`，默认关闭），
   使默认构建不含旧代码、测试按需开启。

## 已知陷阱

`swap_and_mirror` 在仓库里有**三个语义不同的版本**：

| 版本 | 翻转用哪张图 |
|---|---|
| `image_utils::swap_and_mirror` | 翻转后 `copy_from` |
| `converters/reverse/chest_folder.rs` | **重裁后**的图 |
| `converters/ui/process_chest_folder.rs` | **最初裁下的** `r1`/`r2` |

**两个区域重叠时三者结果不同** —— 不要当作重复代码合并，除非逐像素验证。
