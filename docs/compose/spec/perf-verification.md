# 性能验证指南（§9.142–§9.147 之后的管线）

本文给出一套**可复现的**测量与验收步骤。写作动机是一次真实的教训：本轮的优化过程中，
我自己的外部计时方法**每次虚增 2.4–2.7 秒**，并据此做过一次错误的取舍。所以先讲怎么量。

---

## 1. 计时方法（重要：先看这一节）

### 三种口径，只有一种可信

| 方法 | 结果 | 可信度 |
|---|---|---|
| 程序打印的**进程内**时间 | `纯转换 / 含 IO 总时间 / 墙钟时间` | ✅ **以它为准** |
| `cmd /C` 同步执行（PowerShell 阻塞到进程树结束） | 与进程内基本相等 | ✅ 可用于独立复核 |
| PowerShell `Start-Process -Wait` | **每次虚增 2.4–2.7s** | ❌ **不要用** |

实测同一次运行的三个数：`Start-Process -Wait` 报 **3.93s**、进程内 **1.49s**、
`cmd /C` 同步 **1.52s**。真实进程启动开销只有约 **0.05s**。

### 推荐的计时写法

```powershell
$bin = "src-tauri\target\release\2-pyramid.exe"
$out = "$env:TEMP\perf"; New-Item -ItemType Directory $out -Force | Out-Null
Copy-Item "tools\TapL 16x.zip" "$out\pack.zip" -Force   # 文件名不要带空格

foreach ($i in 1..5) {
    Get-ChildItem $out -Filter *.zip | Where-Object Name -ne 'pack.zip' | Remove-Item -Force
    $sw = [Diagnostics.Stopwatch]::StartNew()
    & cmd /C "`"$bin`" --convert `"$out\pack.zip`" --to 97 > `"$env:TEMP\r$i.log`" 2>&1"
    $sw.Stop()
    "第 $i 次 {0:N2}s" -f $sw.Elapsed.TotalSeconds
    Get-Content "$env:TEMP\r$i.log" | Select-String '纯转换时间合计|管线相位'
}
```

**注意**：CLI 的位置参数只认路径与 `--to/--out/--report/--fast`。
`--convert <包> 97` 里的 `97` **会被忽略**，必须写 `--to 97`。

---

## 2. 当前读数（release，真实包 `TapL 16x.zip`）

```
同步墙钟 1.55–1.93s | 纯转换 0.26–0.27s | 含 IO 总 1.35–1.62s | IO 0.89–1.15s

管线相位: open 0.03 · materialize 0.00 · tasks 0.10 · direct 0.02 · harvest 0.00 · tail 0.00 · output 0.09
```

同一口径下旧引擎（`master`，commit `503a085`）是 **纯 2.09s / 含 IO 3.83s**。

### 相位含义

| 相位 | 含义 | 优化它意味着 |
|---|---|---|
| `open` | 构造 `Pack`（读容器 + 建索引） | 换来源类型 |
| `materialize` | 把视图落到 workdir / 建基线 | Zip 路径已降到 0（§9.147） |
| `tasks` | 46 个任务（含并行批落盘与提交） | 任务本身 |
| `direct` | `tasks` 之后的直接步骤（`cut_gui_direct`）与延迟删除 | 已从 0.99s(debug) 降到 0.50s（§9.154） |
| `harvest` | workdir 差异收成一层 | **仅 `Output::Dir` 非零**；Zip 路径应为 0.00 |
| `tail` | 调用方收尾（`pack.mcmeta` 改写） | 已改为 `Tx` |
| `output` | 序列化产物 | 直写 zip 已从 2.30s 降到 0.11s |
| `other` | 总时长减去以上之和 | **偏大即说明相位划分漏了一段** |

### 自洽性检查（建议每次改动后都做）

**各相位之和应约等于 `纯转换`。** 实测示例：

```
0.03 + 0.00 + 0.18 + 0.08 + 0.00 + 0.00 + 0.11 = 0.40s   ≈  纯转换 0.40–0.42s  ✓
```

若两者对不上，或某个相位出现"按代码路径不该发生"的非零值（例如 Zip 路径上的
`harvest`），**那就是计时 bug，不是性能问题**——§9.148 正是这样发现并修掉的：
`harvest_s` 的赋值错位导致 Zip 路径报出 `harvest 0.07s`（那其实是直接步骤的时间），
**相位名撒谎比没有相位更糟，它会把下一步优化引到错误的目标上**。

---

## 3. 三个不可动摇的判据

任何改动都必须同时满足这三条，缺一不可：

```powershell
$env:CARGO_TARGET_DIR = "src-tauri\target"
$env:AROM_REAL_PACK   = "tools\TapL 16x.zip"
$env:AROM_TARGET      = "97"

# ① 冻结内容指纹：4018 条目 / 0x75bb3260e7f578a6
cargo test --lib real_pack_content_baseline_is_frozen -- --ignored --nocapture

# ② 绝对契约：files=4018 bytes=19294735
cargo test --lib real_pack_absolute_output_is_pinned -- --ignored --nocapture

# ③ 全量 + 忽略名单（含 Output::Zip ↔ Output::Dir 逐项一致）
cargo test --lib
cargo test --lib -- --ignored --nocapture
cargo check --bins
```

**为什么指纹比对大小更强**：它逐条目比对（路径、长度、内容哈希）再聚合，能发现
"条目数对但内容错"这类分叉。而大小/条目数相同**不能**证明内容相同。

**`Output::Zip` ↔ `Output::Dir` 一致性用例抓过真 bug**：`harvest` 与 `tail` 顺序放反时，
Zip 产物有 `max_format:[97,1]` 而 Dir 产物是 `pack_format:1`——指纹用例当时是绿的
（它只跑 Zip 路径），是这个一致性用例把它揪出来的。

---

## 4. 产物核对（独立于测试）

测试是进程内的；对**磁盘上的实际产物**再核一遍：

```powershell
Add-Type -AssemblyName System.IO.Compression.FileSystem
$f = Get-ChildItem "$env:TEMP\perf" -Filter *.zip | Where-Object Name -ne 'pack.zip' | Select-Object -First 1
$z = [IO.Compression.ZipFile]::OpenRead($f.FullName)
$files = @($z.Entries | Where-Object { -not $_.FullName.EndsWith('/') })
"文件 {0} / 全部 {1} / 解压字节 {2}" -f $files.Count, $z.Entries.Count,
    ($files | Measure-Object Length -Sum).Sum          # 期望 4018 / 4138 / 19294735
$mc = $z.Entries | Where-Object FullName -eq 'pack.mcmeta'
$sr = New-Object IO.StreamReader($mc.Open()); $sr.ReadToEnd(); $sr.Close()
$z.Dispose()
```

期望 `pack.mcmeta` 携带 `min_format`/`max_format` = 97（≥69 的包不写旧字段 `pack_format`）。

---

## 5. 本轮已试过并**证明无效**的方向（勿重走）

记录这些是为了省下重复的探索成本；每条都有实测数字。

| 方向 | 实测 | 处置 |
|---|---|---|
| 范围化 `harvest`（用目录 mtime 证明未变） | 13.96s vs 全量 14.11s，**在噪声内** | 已回退（§9.137） |
| 任务级并行（21 个 `Parallel` 任务） | 收益≈0——它们全是 0.000–0.003s 的 `generate_*` 微任务；最贵的 `rename_mcpatcher_to_optifine` 是 `Exclusive` | 保留（符合注册表语义），但非杠杆 |
| `DirSource` 作为管线输入 | 纯 0.45→**0.63s**，三次全部更慢（`walkdir` 建索引 + 逐条目读文件系统） | 已回退（§9.145） |
| 给 `ZipSource` 加字字节缓存 | 重复读 712 次共 **0.021s**（单次 30µs，比首读的 61µs 还快——zip 本身在 page cache 里） | 未做，不值得 |
| `apply_layer_to_workdir` 并行化 | **受益面已消失**：Zip 路径不再调用它 | 未做 |
| 用 `Start-Process -Wait` 做外部计时 | 每次虚增 2.4–2.7s，据此做过一次错误取舍 | 已弃用并写入 CLI 提示（§9.146） |
| **函数级并行 PNG 编码**（`save_slices` 整批 `put_images`） | debug 纯 4.69–4.74s vs 4.78–4.80s，**在噪声内**。原因：`par_iter` 固定开销吃掉了毫秒级批次 | 已回退 |
| **事务级并行 PNG 编码**（把编码推迟到 `into_layer` 前） | 实测批次太小：609 张分散在 **22 个事务**，最大批次 210 张 = **0.073s**，即每批 73ms。并行开销与 73ms 同量级 | **未做**（否定了"调大并行度"这一整条路） |
| 去掉 `put_image` 的整图克隆（`DynamicImage::ImageRgba8(img.clone())`） | release 微基准 200 次 256×256：含克隆 0.358ms/张、不克隆 0.357ms/张、**仅克隆 0.027ms/张（7.5%）**。折算 609 张约 0.02s | **未做**（等价性测试保留为回归护栏，§9.155） |

### 真正有效并被保留的

| 改动 | 收益 |
|---|---|
| 整目录改名走一次 `fs::rename`（原为逐文件 1600 次移动） | `rename_mcpatcher_to_optifine` 3.62s → **0.013s** |
| Java 目标直接写 zip（原为"物化目录 + 重打包"双份写盘） | `output` 2.30 → **0.11s**；`pack` 0.41 → 0.10s |
| 不物化 workdir（全内存管线） | 纯转换 3.11 → **0.45s** |
| Zip 路径跳过无人读的内容基线 | `materialize` 0.06 → **0.00s** |
| 修正 `harvest` 相位的错误归属（拆分出 `direct_s`） | 无性能收益，但**避免了把优化引向错误目标**（§9.148） |
| **`Tx` 内的图片解码缓存**（§9.149） | 真实包上一次转换 `tx.image` 被调 262 次 / 不同路径仅 75 个，重复解码累计 **1.353s**（端到端 5.866s 的 23%） |
| Zip 路径跳过无人读的基线同步（§9.150） | −0.28s（debug）；**注意 direct 步骤曾漏掉，§9.154 补齐** |
| 池大小按 CPU 预算而非包数（§9.151） | 原先单包只有 6 线程（24 核机器上 18 核闲置）；并行编码的前提 |
| 已提交视图的条目表按版本缓存（§9.153） | `entries()` 19 次调用占全流程 **57%**；`output` 相位 0.80 → 0.55s |
| **`Tx` 内条目表按写入数缓存**（§9.154） | `gui_surgeon` 一次 run 里 20 次 `has_prefix` 各重建整表（143ms）⇒ 该步约 **0.7s** 纯浪费；debug 纯转换 4.47 → **3.48s**，`direct` 0.99 → **0.50s** |

### 一个反复出现的模式：**缓存被 `pending` 条件挡住**

`PackView` 的类型化缓存（mcmeta / images / entries）都以 `let cacheable = !self.is_pending();`
为条件，而 **`Tx::view()` 恒为 pending**（它必须看到本事务的在途写入）。
于是**每一个"包级缓存"对任务内部都是失效的**——§9.149（图片解码）与 §9.154（条目表）
都是这个模式的两个实例，各自占全流程 20%+。

排查这类问题的通用手法：**在疑似热点函数里按"输入"计数**（路径、调用数、不同键数），
跑一次真实转换，看 `总调用 / 不同键数 / 单键最高次数`。两次都靠这个手法定位。


### 一个必须记住的陷阱：**`Tx` 绕过包级图片缓存**

`PackView::image` 带缓存，但条件是 `let cacheable = !self.is_pending();`，
而 `Tx::view()` **恒为 pending**（要看到本事务的在途写入）。因此**所有任务都绕过了它**。
`gui_surgeon_tx` 的 `process_icons`/`process_widgets`/`process_tabs` 对同一张图做几十次裁切，
每次都重新解码整图——实测 `enchanting_table.png` 在一次转换里被解码 **18 次**。

诊断方法（可复用）：在 `Tx::image` 里按路径计数，跑一次真实转换，看
`总调用次数 / 不同路径数 / 单路径最高次数`。

| `apply_layer_to_workdir` 目录缓存 | 0.43s（较小但真实） |

---

## 6. 剩余空间

```
1.52s 端到端
├─ 预检解压 3631 条目   ~1.2s   ← 只为 Bedrock 探测与结构分析能看目录
├─ 重新打包 zip         ~0.8s   ← 只为让 Pack 读
├─ 管线（全内存）        0.40s
└─ 其余                 ~0.1s
```

这两项 I/O **只在 Java 目标上是纯浪费**：`Pack` 本来就能直接读输入 zip。
要让 `is_bedrock_resource_pack` 与 `analyze_dir` 直接读 zip（而不是目录），
才能取消预检解压——**这是下一步，且是当前最大的剩余空间（约占端到端 60%）**。

**不要**试图用 `DirSource` 绕过：那条路已实测更慢（见第 5 节）。
