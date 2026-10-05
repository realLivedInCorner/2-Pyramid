# 一次性结构整理：pilots/ → natives/，并把 48 个文件按职责分入子文件夹
#
# 做法（**定义位置不动，只动文件位置 + 声明方式**）：
#   * `natives/mod.rs` 保留「共享前导」（Outcome / helper / `all()` / `read_dimensions()` /
#     各 `macro_rules!` / 宏调用），与改造前的 `pilots/mod.rs` **逐字相同**；
#   * 每个模块体搬到 `natives/<组>/<名字>.rs`，在原声明处换成
#     `#[path = "<组>/<名字>.rs"] pub mod <名字>;` —— 因此 `crate::natives::<名字>`
#     路径**完全不变**（外部 89 处引用无需改动），且子模块里 `use super::*` 仍然解析到 `mod.rs`；
#   * 分组文件夹各加一个 `mod.rs` 写明本组职责（纯文档 + 目录锚点）。
#
# 为什么**不**按组嵌套模块（`natives::surgeon::X`）：
#   组与 tier 不是一一对应（`surgeon_mid` 实际是 Eraser、`reverse_*` 跨三个 tier），
#   嵌套会给出错误的结构暗示；而 `#[path]` 方案既进了文件夹、又不动路径。

param(
    [string]$Root = (Split-Path -Parent $PSScriptRoot),
    [switch]$AlsoWrite
)

$srcDir = Join-Path $Root 'src-tauri\src\pilots'
$dstDir = Join-Path $Root 'src-tauri\src\natives'
$modPath = Join-Path $srcDir 'mod.rs'

if (-not (Test-Path $modPath)) { throw "找不到 $modPath（本脚本应在拆分之后运行）" }

# ── 分组表 ──
$groups = [ordered]@{
    'native'    = @('rename_blocks_tables')
    'eraser'    = @('drop_font', 'drop_horse', 'drop_shaders', 'drop_glint', 'drop_blockstates',
                    'rename_blocks', 'chest', 'old_paths', 'animated', 'rename_blocks_reverse')
    'architect' = @('arch_gen', 'arch_gen2', 'arch_gen3', 'arch_gen_metal', 'arch_gen_planks',
                    'arch_gen_breeze', 'potion_lingering_gen', 'shulker_box_gen', 'mcpatcher_optifine')
    'surgeon'   = @('shader_adapt', 'surgeon_early', 'surgeon_early2', 'surgeon_mid', 'surgeon_mid2',
                    'surgeon_mid3', 'surgeon_mid4', 'surgeon_late', 'surgeon_ui', 'surgeon_machinery',
                    'surgeon_survival', 'surgeon_smithing2', 'surgeon_cut_gui', 'gui_surgeon_tx')
    'reverse'   = @('chest_reverse', 'reverse_trivial', 'mcpatcher_optifine_reverse', 'reverse_defer',
                    'reverse_defer_extra', 'reverse_defer_metal', 'reverse_defer_ui', 'reverse_pixels',
                    'reverse_compose', 'reverse_survival', 'reverse_armor')
    'tests'     = @('tests', 'legacy_armor_semantics_tests')
}

$groupOf = @{}
foreach ($g in $groups.Keys) { foreach ($m in $groups[$g]) { $groupOf[$m] = $g } }

$lines = Get-Content $modPath -Encoding UTF8
Write-Host "源 mod.rs：$($lines.Count) 行"

# ── 切出所有顶层块（与 split-pilots.ps1 同一套规则）──
$blocks = New-Object System.Collections.Generic.List[object]
for ($i = 0; $i -lt $lines.Count; $i++) {
    $L = $lines[$i]
    if ($L -notmatch '^((pub )?mod\s+([a-z_0-9]+)|#\[cfg\(test\)\]\s*mod\s+([a-z_0-9]+))\s*\{\s*$') { continue }
    if ($L -match '^macro_rules!') { continue }
    $name = if ($Matches[3]) { $Matches[3] } else { $Matches[4] }
    $depth = 0; $end = -1
    for ($j = $i; $j -lt $lines.Count; $j++) {
        foreach ($ch in $lines[$j].ToCharArray()) {
            if ($ch -eq '{') { $depth++ } elseif ($ch -eq '}') { $depth-- }
        }
        if ($depth -le 0) { $end = $j; break }
    }
    if ($end -lt 0) { throw "模块 $name 括号不配平" }
    $blocks.Add([pscustomobject]@{ Name = $name; Start = $i; End = $end })
    $i = $end
}

# 现在这些模块体都已在各自的 $name.rs 里（上一步拆分产物），因此这里**不需要**再切块，
# 只需知道"声明行"的位置。重新扫一遍声明行。
$decls = New-Object System.Collections.Generic.List[object]
for ($i = 0; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match '^(pub )?mod ([a-z_0-9]+);\s*$') {
        $decls.Add([pscustomobject]@{ Name = $Matches[2]; Line = $i; Pub = ($Matches[1] -eq 'pub ') })
    }
}
Write-Host "找到 $($decls.Count) 条模块声明"

$missing = $decls | Where-Object { -not $groupOf.ContainsKey($_.Name) }
if ($missing) { throw "分组表缺少这些模块：$($missing.Name -join ', ')" }
# 分组表里允许有"不是 `pub mod` 声明"的模块（如私有的 `mod rename_blocks_tables;`），
# 它们由单独的私有声明分支处理，这里只提示、不报错。
$extra = $groupOf.Keys | Where-Object { $decls.Name -notcontains $_ }
if ($extra) { Write-Host "  提示：以下项不是 `pub mod` 声明，将按私有声明处理：$($extra -join ', ')" }

if (-not $AlsoWrite) {
    foreach ($g in $groups.Keys) {
        Write-Host ("  {0,-10} {1,2} 个：{2}" -f $g, $groups[$g].Count, ($groups[$g] -join ', '))
    }
    Write-Host "（未写盘；加 -AlsoWrite 生效）"
    return
}

# ── 建目录、搬文件 ──
foreach ($g in $groups.Keys) {
    $d = Join-Path $dstDir $g
    New-Item -ItemType Directory -Path $d -Force | Out-Null
}

# `all` 是**新增**模块（原先 `PilotFn` + `all()` 直接写在 `pilots/mod.rs` 里）：
# 把它也归到 native/ 组，natives/mod.rs 只留「共享前导 + 声明」。
$groups['native'] = @('all') + $groups['native']
$groupOf['all'] = 'native'

$groupDoc = @{
    'native'    = '原生实现共用的基础设施：延迟删除/条件删除两个 helper，以及 `rename_blocks` 的映射表。'
    'eraser'    = 'Eraser 级任务：删除、改名、路径迁移（含正向/反向两套改名与路径表）。'
    'architect' = 'Architect 级任务：由已有贴图**生成**新资源（按像素/坐标逐条照抄旧实现）。'
    'surgeon'   = 'Surgeon 级任务：就地修改已有贴图与 UI（含 GUI 切片 `gui_surgeon_tx`）。'
    'reverse'   = '反向（撤销）任务：把正向变换倒回去。'
    'tests'     = '对照 oracle 测试：拿旧转换器当尺子逐项比对（随 `legacy-oracle` 门控）。'
}

$movedCount = 0
foreach ($decl in $decls) {
    $name = $decl.Name
    $g = $groupOf[$name]
    $from = Join-Path $srcDir "$name.rs"
    $to = Join-Path (Join-Path $dstDir $g) "$name.rs"
    if (-not (Test-Path $from)) { throw "缺源文件 $from" }
    Move-Item -Path $from -Destination $to -Force -ErrorAction Stop
    $movedCount++
}
Write-Host "已搬 $movedCount 个文件到 $dstDir"

# ── 生成组文件夹的 mod.rs（**不是**纯文档：它负责把「祖父模块」的共享项转出去）──
#
# 关键：搬进子文件夹后，模块体里的 `use super::*` 解析到的是**组模块**（如 `natives::surgeon`），
# 而不是 `natives`。共享项（`Outcome` / `TaskDecl` / `Tx` / `read_dimensions` / 各 `macro_rules!`）
# 都在 `natives` 里，因此组模块必须 `pub use crate::natives::*;` 把它们**透传**给下一层。
foreach ($g in $groups.Keys) {
    $body = @(
        "//! ``natives/$g``：$($groupDoc[$g])",
        '//!',
        '//! 模块**声明**统一留在 `natives/mod.rs`（用 `#[path]` 指向本目录的文件），',
        '//! 因此 `crate::natives::<名字>` 路径保持不变。',
        '//!',
        '//! 下面这行不是装饰：本目录下的模块体用 `use super::*;` 引用 `natives` 的共享项',
        '//! （`Outcome` / `TaskDecl` / `Tx` / `read_dimensions` / 各 `macro_rules!`），',
        '//! 而 `super` 在这里指的是**本模块**——必须显式透传，否则它们全部解析不到。'
    )
    if ($g -ne 'native') {
        $body += ''
        $body += '#![allow(unused_imports)]'
        $body += 'pub use crate::natives::*;'
    } else {
        $body += ''
        $body += '#![allow(unused_imports)]'
        $body += 'pub use crate::natives::*;'
        $body += ''
        $body += '// 本组还承载 `all()`（全量试点清单）——**注意**：它定义在本文件而非 `natives/mod.rs`，'
        $body += '// 因为 `natives/mod.rs` 在测试模块声明之后，而 `all()` 原先就在那个位置。'
    }
    Set-Content -Path (Join-Path (Join-Path $dstDir $g) 'mod.rs') -Value (($body -join "`n") + "`n") -Encoding UTF8
}

# ── 生成新的 natives/mod.rs：声明换成 #[path] 形式；并摘出 PilotFn + all() ──
# 定位 `pub type PilotFn` 与 `pub fn all()`（它们在声明之后，因此必须先摘出来，
# 否则生成的 mod.rs 里会在模块声明**之前**引用 `drop_font::decl()` 等）。
$pilotIdx = -1; $allStart = -1; $allEnd = -1; $readStart = -1; $readEnd = -1
for ($i = 0; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match '^pub type PilotFn') { $pilotIdx = $i }
    if ($lines[$i] -match '^pub fn all\(\)') { $allStart = $i }
    if ($allStart -ge 0 -and $allEnd -lt 0 -and $i -gt $allStart -and $lines[$i] -match '^\}') { $allEnd = $i }
    if ($lines[$i] -match '^pub fn read_dimensions\(') { $readStart = $i }
    if ($readStart -ge 0 -and $readEnd -lt 0 -and $i -gt $readStart -and $lines[$i] -match '^\}') { $readEnd = $i }
}
if ($pilotIdx -lt 0 -or $allStart -lt 0 -or $allEnd -lt 0) { throw "无法定位 PilotFn / all()" }

# `all.rs`：PilotFn 的类型别名 + all()。（`read_dimensions` 是各任务通用的小工具，
# 留在 `natives/mod.rs` 与其它共享项一起。）
$allBody = @(
    '//! 原生实现清单：`(名称, 声明, 执行体)`，按阶段顺序排列（与旧管线一致）。',
    '//!',
    '//! 原先这两个项直接写在 `pilots/mod.rs` 里；§9.127 分组时归入 `natives/native/`。',
    '',
    'use super::*;',
    ''
)
$allBody += $lines[$pilotIdx..($allStart - 1)]
$allBody += $lines[$allStart..$allEnd]
Set-Content -Path (Join-Path (Join-Path $dstDir 'native') 'all.rs') -Value (($allBody -join "`n") + "`n") -Encoding UTF8

$skipRanges = @(@($pilotIdx, $allStart - 1), @($allStart, $allEnd))

$new = New-Object System.Collections.Generic.List[string]
for ($i = 0; $i -lt $lines.Count; $i++) {
    $skip = $false
    foreach ($r in $skipRanges) { if ($i -ge $r[0] -and $i -le $r[1]) { $skip = $true } }
    if ($skip) { continue }

    $d = $decls | Where-Object { $_.Line -eq $i } | Select-Object -First 1
    if ($null -eq $d) { $new.Add($lines[$i]); continue }
    $new.Add("#[path = `"$($groupOf[$d.Name])/$($d.Name).rs`"]")
    $new.Add("$(if ($d.Pub) { 'pub ' } else { '' })mod $($d.Name);")
}
# `all` 的声明插在 `native` 组其它成员旁边（紧跟 `defer_ops` 之后即可，位置不影响语义）
$nativeFirst = $decls | Where-Object { $groupOf[$_.Name] -eq 'native' } | Select-Object -First 1
if ($nativeFirst) {
    $insertAt = $new.IndexOf("#[path = `"native/$($nativeFirst.Name).rs`"]")
    if ($insertAt -ge 0) { $new.Insert($insertAt, "#[path = `"native/all.rs`"]`npub mod all;") }
}
Set-Content -Path (Join-Path $dstDir 'mod.rs') -Value (($new -join "`n") + "`n") -Encoding UTF8
Remove-Item $modPath -Force
Write-Host "已生成 natives/mod.rs（$($new.Count) 行）、native/all.rs，并移除 pilots/mod.rs"


