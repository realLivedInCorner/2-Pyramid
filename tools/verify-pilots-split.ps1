# 校验：pilots/ 拆分是**逐个模块的纯搬移**（内容零改动）。
#
# 做法：取 git HEAD 里的原始 mod.rs，按与 split-pilots.ps1 **同一套规则**切出每个模块体，
# 再与磁盘上对应的 X.rs 逐行比对。任何一行不同即报红并中止。

param([string]$Root = (Split-Path -Parent $PSScriptRoot))

$origPath = Join-Path $env:TEMP 'pilots-mod-orig.rs'
Push-Location $Root
git show HEAD:src-tauri/src/pilots/mod.rs | Set-Content -Path $origPath -Encoding UTF8
Pop-Location
$lines = Get-Content $origPath
Write-Host "git HEAD 的 mod.rs：$($lines.Count) 行"

$dir = Join-Path $Root 'src-tauri\src\pilots'
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
    $blocks.Add([pscustomobject]@{ Name = $name; BodyStart = $i; End = $end })
    $i = $end
}
Write-Host "原文件里共 $($blocks.Count) 个模块体待比对"

$bad = @()
foreach ($b in $blocks) {
    $expected = $lines[($b.BodyStart + 1)..($b.End - 1)]
    $file = Join-Path $dir "$($b.Name).rs"
    if (-not (Test-Path $file)) { $bad += "$($b.Name).rs 不存在"; continue }
    $actual = Get-Content $file
    if ($actual.Count -ne $expected.Count) {
        $bad += "$($b.Name).rs 行数不同：文件 $($actual.Count) vs 原文 $($expected.Count)"
        continue
    }
    for ($k = 0; $k -lt $expected.Count; $k++) {
        if ($actual[$k] -cne $expected[$k]) {
            $bad += "$($b.Name).rs 第 $($k + 1) 行不同：`n    文件=<$($actual[$k])>`n    原文=<$($expected[$k])>"
            break
        }
    }
}

# 反向：pilots 下不该有"原文里没有"的模块文件（rename_blocks_tables.rs 是原有子模块）
$expectedFiles = $blocks | ForEach-Object { "$($_.Name).rs" }
$extra = Get-ChildItem $dir -File -Filter *.rs |
    Where-Object { $expectedFiles -notcontains $_.Name -and $_.Name -ne 'rename_blocks_tables.rs' -and $_.Name -ne 'mod.rs' } |
    ForEach-Object { $_.Name }

if ($extra) { $bad += "多出未预期的文件：$($extra -join ', ')" }

if ($bad.Count -gt 0) {
    Write-Host "`n**校验失败**（$($bad.Count) 处）：" -ForegroundColor Red
    $bad | Select-Object -First 20 | ForEach-Object { Write-Host "  $_" }
    exit 1
}
Write-Host "`n✅ 校验通过：$($blocks.Count) 个模块体与 git HEAD 原文逐行一致（零内容改动）" -ForegroundColor Green
Write-Host "   pilots/ 下文件清单也与原文模块一一对应，无多余文件"
