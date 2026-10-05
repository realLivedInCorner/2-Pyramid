# 一次性结构拆分：pilots/mod.rs → pilots/ 下的多个文件
#
# 原则：**只搬不改**
#   - 每个顶层块（`pub mod X { … }` / `#[cfg(test)] mod X { … }`）**逐字节**搬到
#     `pilots/X.rs`，内部一行不动；
#   - 原位留下 `pub mod X;`（测试模块用 `#[path]` 保持模块名不变）；
#   - 共享前导（`Outcome` / 各 helper / 各 `macro_rules!` / 宏调用）**留在 `mod.rs`**，
#     因为被搬走的模块用 `use super::*` 引用它们，路径与可见性都不变。
#
# 稳健性：逐行解析 `{`/`}` 配平，不依赖正则匹配整块；先 DryRun 打印计划，再写盘。

param(
    [string]$Root = (Split-Path -Parent $PSScriptRoot),
    [switch]$AlsoWrite
)

$modPath = Join-Path $Root 'src-tauri\src\pilots\mod.rs'
$dir = Split-Path -Parent $modPath
$lines = Get-Content $modPath -Encoding UTF8
Write-Host "源文件 $($lines.Count) 行"

# ── 找出所有顶层块 ──
$blocks = New-Object System.Collections.Generic.List[object]
for ($i = 0; $i -lt $lines.Count; $i++) {
    $L = $lines[$i]
    # 只认第 0 列开始的块；排除宏调用（以 ; 结尾）与 macro_rules!
    if ($L -notmatch '^((pub )?mod\s+([a-z_0-9]+)|#\[cfg\(test\)\]\s*mod\s+([a-z_0-9]+))\s*\{\s*$') { continue }
    if ($L -match '^macro_rules!') { continue }
    $name = if ($Matches[3]) { $Matches[3] } else { $Matches[4] }
    # `#[cfg(test)]` 可能写在**上一行**（本文件就是这样），因此两处都要认
    $cfgOnPrev = ($i -gt 0) -and ($lines[$i - 1].Trim() -eq '#[cfg(test)]')
    $isCfgTest = ($L -match '#\[cfg\(test\)\]') -or $cfgOnPrev
    $hasPub = $L -match '^pub mod'

    # 配平大括号
    $depth = 0
    $end = -1
    for ($j = $i; $j -lt $lines.Count; $j++) {
        foreach ($ch in $lines[$j].ToCharArray()) {
            if ($ch -eq '{') { $depth++ }
            elseif ($ch -eq '}') { $depth-- }
        }
        if ($depth -le 0) { $end = $j; break }
    }
    if ($end -lt 0) { throw "第 $($i + 1) 行的模块 $name 没有配平的右括号" }
    # 属性在上一行时，把它一起吞掉（输出侧会重新加）
    $attrStart = if ($cfgOnPrev) { $i - 1 } else { $i }
    $blocks.Add([pscustomobject]@{
        Name = $name; Start = $attrStart; BodyStart = $i; End = $end; CfgTest = $isCfgTest; Pub = $hasPub
    })
    $i = $end
}

Write-Host "找到 $($blocks.Count) 个顶层块（含测试模块）"
if ($blocks.Count -lt 30) { throw "块数异常少：$($blocks.Count)" }

# ── 检查重名 ──
$dupes = $blocks | Group-Object Name | Where-Object { $_.Count -gt 1 }
if ($dupes) { throw "模块重名：$($dupes.Name -join ', ')" }

# ── 打印计划 ──
foreach ($b in $blocks) {
    $kind = if ($b.CfgTest) { '测试模块' } else { '生产模块' }
    Write-Host ("  {0,-30} 行 {1,5}-{2,-5} {3}" -f $b.Name, ($b.Start + 1), ($b.End + 1), $kind)
}

if (-not $AlsoWrite) { Write-Host "（未写盘；加 -AlsoWrite 生效）"; return }

# ── 生成新 mod.rs 与各模块文件 ──
# **关键**：写进 `X.rs` 的是模块**体**（去掉外层 `mod X { … }` 包裹），
# 因为 `pub mod X;` 本身就会把文件内容当作模块体。首版把整块（含包裹）写进去了，
# 于是 `crate::pilots::X::foo` 全部解析失败 —— 编译器当场报了一屏 unresolved。
$new = New-Object System.Collections.Generic.List[string]
$i = 0
while ($i -lt $lines.Count) {
    $b = $blocks | Where-Object { $_.Start -eq $i } | Select-Object -First 1
    if ($null -eq $b) { $new.Add($lines[$i]); $i++; continue }

    # 模块体 = 声明行之后 到 块结束前的 '}' 之前（两端各去掉一层包裹）
    $body = $lines[($b.BodyStart + 1)..($b.End - 1)]
    $outFile = Join-Path $dir "$($b.Name).rs"
    Set-Content -Path $outFile -Value $body -Encoding UTF8

    if ($b.CfgTest) {
        # 测试模块：属性（与其它 cfg）在块起始行之前，原样保留；只把 `mod X {` 换成 `mod X;`
        for ($k = $b.Start; $k -lt $b.BodyStart; $k++) { $new.Add($lines[$k]) }
        $new.Add("mod $($b.Name);")
    } else {
        $new.Add("pub mod $($b.Name);")
    }

    $i = $b.End + 1
}
Set-Content -Path $modPath -Value $new -Encoding UTF8

Write-Host ""
Write-Host "已写盘：mod.rs 现在 $($new.Count) 行；产出 $($blocks.Count) 个模块文件"
