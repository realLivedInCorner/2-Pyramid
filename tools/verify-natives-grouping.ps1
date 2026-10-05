# 校验：natives/ 的**改名 + 分组**是纯搬移（内容零改动）。
#
# 判据（两层）：
#   ① 取 `git show <REF>:src-tauri/src/pilots/mod.rs`（改造前的原始文件），按与
#      `group-natives.ps1` 同一套规则切出每个模块体；
#   ② 与磁盘上 `natives/<组>/<名字>.rs` **逐行比对**（大小写敏感）。
# 另比 `natives/mod.rs` 的共享前导与 `all()`/`read_dimensions()`。
#
# 用法：pwsh tools/verify-natives-grouping.ps1 [-Ref HEAD] [-Root <仓库根>]

param(
    # 必须是**拆分之前**的提交（那时 mod.rs 里还有模块体）；HEAD 里只剩声明了。
    [string]$Ref = 'f40aab2~1',
    [string]$Root = (Split-Path -Parent $PSScriptRoot)
)

$origPath = Join-Path $env:TEMP 'verify-natives-orig.rs'
Push-Location $Root
git show "${Ref}:src-tauri/src/pilots/mod.rs" | Set-Content -Path $origPath -Encoding UTF8
Pop-Location
if (-not (Test-Path $origPath)) { throw "取不到 ${Ref}:src-tauri/src/pilots/mod.rs" }

$lines = Get-Content $origPath
Write-Host "原始 ${Ref}:src-tauri/src/pilots/mod.rs：$($lines.Count) 行"
if ($lines.Count -lt 1000) {
    throw "参照版本不对：该版本的 mod.rs 只有 $($lines.Count) 行（拆分后只剩声明）。请传 -Ref <拆分之前的提交>。"
}

# ── 切出原始模块体 ──
$orig = @{}
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
    $orig[$name] = $lines[($i + 1)..($end - 1)]   # 模块**体**
    $i = $end
}
Write-Host "原始文件里共 $($orig.Count) 个模块体"
if ($orig.Count -lt 30) { throw "参照版本里切不出模块体（$($orig.Count) 个）：Ref 不是拆分前的提交" }

# ── 按新布局逐个比对 ──
$dir = Join-Path $Root 'src-tauri\src\natives'
$modPath = Join-Path $dir 'mod.rs'
$modLines = Get-Content $modPath
$bad = @(); $checked = 0; $intentional = 0; $skippedNested = 0

for ($i = 0; $i -lt $modLines.Count; $i++) {
    if ($modLines[$i] -notmatch '^#\[path = "([a-z_0-9]+)/([a-z_0-9]+)\.rs"\]') { continue }
    $group = $Matches[1]; $name = $Matches[2]
    $file = Join-Path (Join-Path $dir $group) "$name.rs"
    if (-not (Test-Path $file)) { $bad += "$group/$name.rs 不存在"; continue }
    if (-not $orig.ContainsKey($name)) {
        # `rename_blocks_tables` 原本是 `mod.rs` 里的**嵌套**声明（在 rename_blocks 内），
        # 不是顶层模块，无法按顶层规则比对 —— 计入"跳过"而非失败。
        if ($name -eq 'rename_blocks_tables') { $skippedNested++; continue }
        $bad += "$name 在原始 mod.rs 里找不到对应模块"; continue
    }

    $expected = $orig[$name]
    $actual = Get-Content $file
    $checked++
    # **有意为之的例外**（§9.127）：`pilots` → `natives` 的改名会同时改到
    # ① 测试名（`pilots_match_*` → `natives_match_*`）、② 文档里的命令、③ 模块路径。
    # 只允许这三类差异；规范化后必须**完全一致**，否则仍算失败。
    if ($name -eq 'tests') {
        $a = ($actual -join "`n") -replace 'natives_match_the_old_implementations', 'pilots_match_the_old_implementations'
        $a = $a -replace 'cargo test --lib natives', 'cargo test --lib pilots'
        $a = $a -replace 'crate::natives', 'crate::pilots'
        $e = $expected -join "`n"
        if ($a -eq $e) {
            $intentional++
            continue
        }
    }
    if ($actual.Count -ne $expected.Count) {
        $bad += "$group/$name.rs 行数不同：新 $($actual.Count) vs 原 $($expected.Count)"
        continue
    }
    for ($k = 0; $k -lt $expected.Count; $k++) {
        if ($actual[$k] -cne $expected[$k]) {
            $bad += "$group/$name.rs 第 $($k + 1) 行不同：`n      新=<$($actual[$k])>`n      原=<$($expected[$k])>"
            break
        }
    }
}

# ── 反向：每个原模块都要在磁盘上找得到 ──
foreach ($name in $orig.Keys) {
    $found = Get-ChildItem $dir -Recurse -File -Filter "$name.rs" -ErrorAction SilentlyContinue
    if (-not $found) { $bad += "原模块 $name 在 natives/ 下找不到文件" }
}

# ── 共享前导（改造前后必须逐字相同）──
# 原始 mod.rs 的第 1..第一个模块声明行 与 新 mod.rs 的对应区间一致即可；
# 这里只校验"共享项没丢"这一可机检的部分。
foreach ($needle in @('pub struct Outcome', 'fn defer_remove_if_present', 'fn remove_if_present',
                      'pub fn all()', 'pub type PilotFn', 'pub fn read_dimensions')) {
    $inOrig = ($lines | Select-String -Pattern ([regex]::Escape($needle)) | Measure-Object).Count
    $inNew = ($modLines | Select-String -Pattern ([regex]::Escape($needle)) | Measure-Object).Count
    if ($inNew -lt 1) { $bad += "新 mod.rs 缺少共享项：$needle" }
    if ($inOrig -ge 1 -and $inNew -lt 1) { $bad += "共享项丢了：$needle" }
}

if ($bad.Count -gt 0) {
    Write-Host "`n**校验失败**（$($bad.Count) 处）：" -ForegroundColor Red
    $bad | Select-Object -First 20 | ForEach-Object { Write-Host "  $_" }
    exit 1
}
Write-Host "`n✅ 校验通过：$checked 个模块体与 $Ref 原文逐行一致（零内容改动）" -ForegroundColor Green
Write-Host "   反向检查：$($orig.Count) 个原模块在新布局下都能找到文件"
Write-Host "   共享项（Outcome / 两个 helper / all() / PilotFn / read_dimensions）均在"
if ($intentional -gt 0) { Write-Host "   已知且有意为之的例外 $intentional 处：测试名 pilots_match_* → natives_match_*（§9.127）" }
if ($skippedNested -gt 0) { Write-Host "   跳过 $skippedNested 个非顶层声明：rename_blocks_tables（原为 rename_blocks 内的嵌套声明）" }
