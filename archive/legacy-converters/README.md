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

## 当前仓库状态的已知限制（§9.125 已更新）

**默认构建（无 feature）在全新 clone 上可以正常构建与测试** —— 实测
`git archive HEAD | tar -x`（只有 tracked 文件）后：

```
cargo check --bins  → Finished
cargo test  --lib   → 234 passed / 6 ignored
```

原因是 §9.125 把旧转换器树整体按 `legacy-oracle` feature 门控了：
`invoke_conversion.rs` 的 88 个旧闭包、`pilots/mod.rs` 的对照测试、`arom/pathview.rs` 的对照用例，
以及（更早的 §9.124）`pilots/mod.rs` 生产段对 `mirror_region` / `swap_and_mirror` / `color::utils`
的复用，都已经不再存在于默认构建里。

**`--features legacy-oracle` 仍需先取回本存档的源码**（它本来就是这个 feature 的含义）：

```powershell
pwsh tools/legacy-oracle/restore.ps1
cargo test --lib --features legacy-oracle
```

未取回时该构建会报一批 `file not found for module` —— **预期**行为，不是故障。

**因此本存档不能删**：它现在是 `legacy-oracle` 的**唯一源码来源**
（全新 clone 里旧树根本不存在，`restore.ps1` 的 `REF_COMMIT` 是移出时那个提交，
新 clone 里也未必可达，`archive/` 才是可靠来源）。

§9.124 收尾清单里「第 ③ 步做完后存档可删」的前提**尚未成立**：
feature 保留的是**开关与门控**，源码仍来自这里。

## 已知陷阱

`swap_and_mirror` 在仓库里有**三个语义不同的版本**：

| 版本 | 翻转用哪张图 |
|---|---|
| `image_utils::swap_and_mirror` | 翻转后 `copy_from` |
| `converters/reverse/chest_folder.rs` | **重裁后**的图 |
| `converters/ui/process_chest_folder.rs` | **最初裁下的** `r1`/`r2` |

**两个区域重叠时三者结果不同** —— 不要当作重复代码合并，除非逐像素验证。
