# 交接：2-Pyramid A-ROM / M3 收口（最后一步）

## 一句话

A-ROM 的 M3 只剩**最后一步**：把 `invoke_conversion.rs` 里 88 个任务的**元数据**与**闭包体**拆开，
让生产二进制不再包含 1.6 万行旧转换器代码，同时保留测试用的基线重生成能力。

---

## 环境

| 项 | 值 |
|---|---|
| 工作树 | `D:\develop\2pyr\2pyr-desktop\code\.worktrees\astray` |
| 分支 | `feat/astray`（远端 `origin/feat/astray`，`git@github.com:realLivedInCorner/2-Pyramid.git`） |
| **当前 HEAD** | **`88e2f0e`** —— 已推送，与远端 0/0 同步，工作树干净 |
| 会话起点 | `f587ad3`（本次工作共 64 个提交） |
| `CARGO_TARGET_DIR` | `D:\develop\2pyr\2pyr-desktop\code\src-tauri\target` |
| manifest | `...\.worktrees\astray\src-tauri\Cargo.toml` |
| 真实测试包 | `D:\develop\2pyr\2pyr-desktop\code\tools\TapL 16x.zip`（env `AROM_REAL_PACK`，`AROM_TARGET=97`） |

### 每次改动后必须跑的验证

```powershell
$wt='D:\develop\2pyr\2pyr-desktop\code\.worktrees\astray'
$env:CARGO_TARGET_DIR='D:\develop\2pyr\2pyr-desktop\code\src-tauri\target'
$env:AROM_REAL_PACK='D:\develop\2pyr\2pyr-desktop\code\tools\TapL 16x.zip'
$env:AROM_TARGET='97'
Set-Location $wt

cargo test --lib --manifest-path "$wt\src-tauri\Cargo.toml"                      # 单测（~20s）
cargo check --bins --manifest-path "$wt\src-tauri\Cargo.toml"                   # 二进制
cargo test --lib --manifest-path "$wt\src-tauri\Cargo.toml" -- --ignored        # 全部对照（~90-100s）
```

**基线（必须逐字不变，除非有意改动并给出理由）**：

```powershell
cargo test --lib --manifest-path "$wt\src-tauri\Cargo.toml" real_pack_content_baseline_is_frozen -- --ignored --nocapture
# 期望：内容基线：4018 个条目，聚合指纹 = 0x75bb3260e7f578a6
```

**当前绿色状态**：309 passed / 22 ignored；警告 91。

---

## 背景：M3 要解决什么

把转换内核从「解压到临时目录 + 逐文件读写」改成 **A-ROM 单一对象模型**
（`Pack` / `Tx` / `Layer` / `PackView`），对用户可见行为与产物字节**零变化**。

- **M2「全部任务原生化」已达成** —— 生产配置 `native_tasks: 46, legacy_tasks: 0`；
  46 个任务的实现都在 `src-tauri/src/pilots/`（`mod.rs` 约 9 千行）。
- **M3 = 收口**：删掉适配层与旧引擎。

### 已完成（M3）

| 步 | 内容 | 提交 |
|---|---|---|
| ① | `GuiSurgeon` 本地化到 `Tx`（8 步全移植，237 文件逐条对照）并**上生产** | `de5c544`…`fdde143` |
| ②-a | **冻结内容基线**（4018 条目聚合指纹）—— 使闸门不再只依赖旧实现 | `c750ede` |
| ②-b | 生产入口改道 `native_run::run_native` + `Output::Dir` | `388e7e6` |
| ②-c | 删旧入口（`invoke_conversion` / `_ex` / `run_direct_steps`）+ 死模块 | `3fb0c2e` 等 |
| — | `run_mixed` → **`run_native`**（名字已名副其实） | `726e6d3` |
| — | 旧转换器**移出版本控制** + `archive/legacy-converters/` 存档 | `375c427` / `db80807` |
| — | **`pilots/` 生产段对旧转换器零依赖**（提取 `chest_region` + `color`） | `c7c0ef1` / `4aa215a` |

---

## 下一步（唯一剩余）

### 目标

让 `cargo build` **不含任何旧转换器代码**，同时保留「重生成基线」的能力。

### 为什么不能直接删 `invoke_conversion.rs`

它（512 行）现在带 **71 个 `use crate::converters`** 与 **88 个闭包**。两条**实测**约束：

1. **元数据是生产必需的** —— 驱动 `native_run::native_placements()` 用
   `scheduler.task_tier(name)` 决定每个任务的**放置侧**（Early/Late 阶段），
   而阶段来自这 88 个注册项。删掉注册表 ⇒ 放置计算失效。
2. **闭包体同时是基线的实现**（§9.123 实测）—— 删掉它们，`off` 配置失去实现，
   **冻结基线从 4018 条目掉到 3792**（指纹 `0x53bda27dbeea7469`）。

### 做法（四步）

1. **新建 `src-tauri/src/task_registry.rs`**：纯元数据表
   ```rust
   pub const REGISTRY: &[(&str, TaskType, TaskTier)] = &[ /* 88 项，顺序与现注册表逐字一致 */ ];
   ```
   **顺序即语义**（阶段内顺序、`task_tier` 查询都依赖它）。**不依赖 `converters`**。

   88 项可从现有 `invoke_conversion.rs` 提取：
   ```powershell
   [regex]::Matches($t, 'scheduler\.register_task\("([a-z_0-9]+)",\s*TaskType::(\w+),\s*TaskTier::(\w+)')
   ```

2. **`Scheduler` 从 `task_registry` 取名字与阶段**（生产路径），
   `invoke_conversion.rs` 的闭包版改为**复用同一份元数据**（避免两份漂移）。

3. **`invoke_conversion.rs` 的闭包版本用 Cargo feature 门控**：
   - `src-tauri/Cargo.toml` 加 `[features] legacy-oracle = []`（**默认关闭**）；
   - 闭包体与那 71 个 `use` 一起放进 `#[cfg(feature = "legacy-oracle")]`；
   - `NativeSwitches::none()`（`off`/`legacy` 基线专用）同样门控。

4. **测试改动**：`off`/`legacy` 两个对照配置在 `#[cfg(feature = "legacy-oracle")]` 下；
   默认（无 feature）时，`on` 配置改与**冻结基线**对照。

### 验收标准

| 判据 | 期望 |
|---|---|
| `cargo check --lib`（无 feature） | 0 error，且 `invoke_conversion.rs` 不再 `use converters` |
| `cargo test --lib`（无 feature） | 全绿（忽略用例数可能下降，需在 §9.NN 说明） |
| `cargo test --lib --features legacy-oracle` | 全绿，含 `off`/`legacy` 对照 |
| **冻结指纹** | 仍为 `0x75bb3260e7f578a6`（4018 条目） |
| 相对闸门 | `on` vs 冻结基线一致 |

### 风险与纪律

- **动的是驱动取阶段的数据源**，一旦顺序或阶段错，放置会静默改变 ⇒ 产物分叉。
  必须**两道闸门同时守住**（相对对照 + 冻结指纹）。
- 本会话已因**判断失误返工 5 次**，根因都是**用局部证据推断全局**。教训：
  1. **删除前必须做全库检查**，且覆盖「别名」与「全路径」两种引用形态；
  2. **「某文件不再引用」≠「没人引用」**；
  3. **最终以编译器 + 测试为准**，不要只靠 grep；
  4. **不要脚本化重写大型代码块**（`native_run.rs` 曾有两次失败）。

---

## 关键文件

| 文件 | 说明 |
|---|---|
| `src-tauri/src/native_run.rs` | **驱动**（原 `mixed_run.rs`）：materialize → 原生派发 → harvest → 序列化；`Output::{Zip,Dir}`；`native_for` 派发表 |
| `src-tauri/src/invoke_conversion.rs` | **待处理**：88 任务注册表（元数据 + 旧闭包） |
| `src-tauri/src/pilots/mod.rs` | 46 个原生实现 + 大量对照 oracle（`#[cfg(test)]` 起于约 1940 行） |
| `src-tauri/src/chest_region.rs` | 箱子区域变换（原生与旧转换器共用） |
| `src-tauri/src/color/` | 颜色工具（同上） |
| `src-tauri/src/arom/` | A-ROM 内核（`Pack`/`Tx`/`Layer`/`PackView`） |
| `src-tauri/src/converters/` | **仅剩 21 个 tracked**（`bedrock/`、`pack_analysis`、`pack_diff`、`scale_factor`、`version_converter`、`zip`、`ui/process_chest_folder`、三个 `mod.rs`） |
| `archive/legacy-converters/` | 旧转换器完整快照（106 文件 / 16,580 行）**不参与编译** |
| `tools/legacy-oracle/` | `restore.ps1` + 清单 + README（如何取回旧转换器） |
| `tools/arom-baseline.txt` | 冻结基线的逐条目清单（4018 行，用于 diff 定位） |
| `docs/compose/spec/astray-arom-model.md` | **设计全文（§9.1–§9.124）**，已入库 |
| `docs/compose/spec/astray.md` | 路线图与 T 项，已入库 |

---

## 必读文档（按顺序）

1. **`docs/compose/spec/astray-arom-model.md` §9.124** —— 上一步的进展与本步方案（**先读这个**）
2. **§9.123** —— 为什么闭包不能删（两次实测：3792 vs 4018）
3. **§9.118 / §9.122** —— 生产入口改道的形态与理由
4. **§9.115** —— 冻结基线的意义（比计数契约强在哪）
5. **`archive/legacy-converters/README.md`** —— 存档的定位与已知陷阱

---

## 已入库的「负结果」（别重复踩）

- **§9.90/§9.91**：「`cut_gui` 与 `run_direct_steps` 冗余」是**错的** —— 两者各自必需，删任一个都丢 sprite。
- **§9.98/§9.99**：「放置模型阻塞 `cut_gui` 原生」是**错的** —— 位置不变、实现换 `Tx` 即可。
- **§9.100**：`cut_gui` **不能**离开计划槽位 `(15,18)` —— 挪出批次就少 3 个 sprite。
- **§9.104/§9.112**：`temp_dir` 的阻塞是**验证依赖**（`off` 是闸门的基线），**不是**功能依赖。
- **§9.123**：旧闭包**不是**死代码 —— 它们是 `off`/`legacy` 基线的实现。
- **`swap_and_mirror` 有三个语义不同的版本**（`image_utils` / `reverse/chest_folder` / 已提取的 `chest_region`），**区域重叠时结果不同，不要合并**。

---

## 报告要求（沿用本会话约定）

- **中文**回答；
- 每次交付说明：**改了什么、为什么、验证结果**（全数字：单测/忽略用例/警告/内容指纹）；
- **失败与返工如实报告**，不隐瞒、不夸大；
- 每个批次：**读实现 → 改 → 全量测试 + `--ignored` 对照 → 更新 `docs/compose/spec/astray-arom-model.md` 新增 §9.NN → 提交并推送 `feat/astray`**；
- 不可观测的部分**必须补验证手段或明确标注为未验证**。

---

## 完成后的收尾（M3 真正结束）

1. `archive/legacy-converters/` 与 `tools/legacy-oracle/` 可删除（若第 ③ 步做完，基线的重生成能力已由 feature 保留）；
2. `archive/legacy-converters/README.md` 里记录的「全新 clone 无法构建」限制随之解除 —— **务必实测验证一次**（`git clone` 到临时目录 + `cargo check --bins`）；
3. `src-tauri/src/converters/` 目录可整体删除或保留（取决于是否还有非闭包用途）；
4. 更新 `docs/compose/spec/astray.md` 的 T 项为完成，并在 `astray-arom-model.md` 写收口总结。
