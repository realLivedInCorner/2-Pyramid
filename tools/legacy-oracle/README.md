# 旧转换器：移出版本控制的参考实现

## 这是什么

`src-tauri/src/converters/` 下的**旧转换器**（A-ROM 迁移前的实现，约 1.6 万行 / 106 个文件）
已被**移出版本控制**，但**本地保留**，作为 A-ROM 原生实现的**验证对照**。

## 为什么移出

M3 完成后，生产路径**不再使用**这些旧转换器：`version_converter::process_zip_timed`
改为调用 `native_run::run_native`（§9.118），逐任务派发到 `src-tauri/src/pilots/` 下的
原生实现。实测生产配置为 `native_tasks: 46, legacy_tasks: 0`。

## 为什么不能直接删

它们还是**唯一的参考实现**：

- `native_switch` 闸门的 `legacy` / `off` 两个对照配置，**实现体**就是这些旧转换器；
- **冻结内容基线**（`tools/arom-baseline.txt`，4018 条目的哈希清单）也是由 `off` 配置生成的。

冻结基线只能回答「**和上次比变了没有**」，**不能回答「变了之后对不对」**。
保留旧转换器，才有能力在**有意变更**时判断新产物是否正确（重新生成基线并与旧实现逐项对照）。

## 怎么恢复

```powershell
pwsh tools/legacy-oracle/restore.ps1          # 只补缺失的文件（幂等）
pwsh tools/legacy-oracle/restore.ps1 -Force   # 覆盖已存在的文件
```

恢复源是 `restore.ps1` 里的 `REF_COMMIT`（移出时所在的提交），
待恢复清单是同目录的 `untracked-files.txt`。

## 重要：当前仓库状态（§9.125 已更新）

**默认构建在全新 clone 上可构建、可测试**（实测，见 `archive/legacy-converters/README.md`）。
旧转换器树已整体按 `legacy-oracle` feature 门控，**默认关闭**：

| 场景 | 是否可构建 |
|---|---|
| 本地开发（文件仍在磁盘上） | ✅ 可构建、可测试 |
| **全新 clone + 默认构建** | ✅ **可构建、可测试**（234 passed / 6 ignored） |
| 全新 clone + `--features legacy-oracle` | ❌ 需先跑 `restore.ps1`（预期行为） |

打开 feature 后即可使用 `legacy` / `off` 两个对照配置与**基线的重生成能力**：

```powershell
pwsh tools/legacy-oracle/restore.ps1
cargo test --lib --features legacy-oracle          # 314 passed / 22 ignored
cargo test --lib --features legacy-oracle -- --ignored
```

**本目录与 `archive/legacy-converters/` 不能删**：它们是该 feature 的**唯一源码来源**
（全新 clone 里旧树不存在；`REF_COMMIT` 在新 clone 里也未必可达，`archive/` 才是可靠来源）。

## 已知陷阱

`swap_and_mirror` 在仓库里有**三个语义不同的版本**：

| 版本 | 翻转用哪张图 |
|---|---|
| `image_utils::swap_and_mirror` | 翻转后 `copy_from` |
| `converters/reverse/chest_folder.rs` | **重裁后**的图 |
| `converters/ui/process_chest_folder.rs` | **最初裁下的** `r1`/`r2` |

**两个区域重叠时三者结果不同** —— 不要当作重复代码合并，除非逐像素验证。
