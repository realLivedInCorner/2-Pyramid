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

## 重要：当前仓库状态

**移出后，全新 clone 无法构建** —— 因为 `invoke_conversion.rs`（旧闭包注册表）
与 `pilots/mod.rs` 里的若干原生实现仍引用这些文件。

**这是 M3 进行期间的过渡状态，是刻意的**：

| 场景 | 是否可构建 |
|---|---|
| 本地开发（文件仍在磁盘上） | ✅ 可构建、可测试 |
| 全新 clone（无这些文件） | ❌ **不可构建，需先跑 `restore.ps1`** |

**M3 收口时**应当二选一：

1. **完成迁移**：把剩余引用（`invoke_conversion.rs` 的旧闭包注册表、
   `pilots/mod.rs` 里对 `mirror_region` / `swap_and_mirror` 的复用）一并原生化，
   然后删除旧转换器目录；
2. **转为归档**：把旧转换器移入独立目录或独立仓库，仅测试时引用。

在此之前，**不要把它当成死代码删除**。

## 已知陷阱

`swap_and_mirror` 在仓库里有**三个语义不同的版本**：

| 版本 | 翻转用哪张图 |
|---|---|
| `image_utils::swap_and_mirror` | 翻转后 `copy_from` |
| `converters/reverse/chest_folder.rs` | **重裁后**的图 |
| `converters/ui/process_chest_folder.rs` | **最初裁下的** `r1`/`r2` |

**两个区域重叠时三者结果不同** —— 不要当作重复代码合并，除非逐像素验证。
