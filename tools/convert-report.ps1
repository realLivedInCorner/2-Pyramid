#Requires -Version 5.1
<#
  2-Pyramid 拖放转换 + 报告（PowerShell 版）

  用法：
    1) 把一个资源包（.zip / .mcpack）或含资源包的文件夹拖到 convert-report.bat 上；
    2) 或在本目录执行：
         .\convert-report.bat "D:\packs\my.zip" -Target 1.21.4
         .\convert-report.bat "D:\packs" -Target 26.3 -Yes -NoOpen
         .\convert-report.bat "D:\packs\my.zip" -MenuChoice 7 -Yes

  行为：
    * 自动查找支持 --convert 的 2-pyramid.exe（仓库构建优先 → 已安装版本）；
    * 跑完整转换管线（与 GUI 同一条），打印两个耗时口径与慢任务；
    * 生成结构化 JSON 报告（结构分析 / 耗时 / 逐任务画像 / 体积变化）；
    * 默认打开报告所在文件夹。

  参数：
    -Paths      拖入的路径（可多个；位置参数）
    -Target     目标版本：1.21.4 / 1.21 / 26.3 / bedrock / 直接 pack_format（默认 26.3）
    -MenuChoice 菜单序号（1-16），等价于交互时输入序号；用于脚本化调用
    -Yes        跳过交互确认（Bedrock 警告、结尾等待）
    -NoOpen     不自动打开报告所在文件夹
#>
[CmdletBinding()]
param(
    # 所有参数都收进这里再手动解析：用 `-File` 调用时，声明式的多个位置参数会
    # 抢着绑定第一个路径（实测第一个位置参数会绑给 $Target），手动解析才能保证
    # 「路径永远被当成路径」，也允许开关出现在任意位置。
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$RawArgs
)

$ErrorActionPreference = 'Stop'

# ── 手动参数解析 ────────────────────────────────────────────
$Target = ''
$MenuChoice = ''
$Yes = $false
$NoOpen = $false
$Paths = @()

for ($i = 0; $i -lt @($RawArgs).Count; $i++) {
    $a = $RawArgs[$i]
    if ([string]::IsNullOrWhiteSpace($a)) { continue }
    switch -Regex ($a) {
        '^-Target$' { if ($i + 1 -lt $RawArgs.Count) { $Target = $RawArgs[++$i] }; continue }
        '^-MenuChoice$' { if ($i + 1 -lt $RawArgs.Count) { $MenuChoice = $RawArgs[++$i] }; continue }
        '^-Yes$' { $Yes = $true; continue }
        '^-NoOpen$' { $NoOpen = $true; continue }
        '^-Target=(.+)$' { $Target = $Matches[1]; continue }
        '^-MenuChoice=(.+)$' { $MenuChoice = $Matches[1]; continue }
        default { $Paths += $a.Trim('"') }
    }
}

# 控制台按 UTF-8 输出，避免中文乱码
try {
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    $OutputEncoding = [System.Text.Encoding]::UTF8
} catch { }

function Write-Head([string]$text) {
    Write-Host ''
    Write-Host ('=' * 62) -ForegroundColor DarkGray
    Write-Host "  $text" -ForegroundColor Cyan
    Write-Host ('=' * 62) -ForegroundColor DarkGray
}

# 统一的问答入口：交互控制台用 Read-Host；stdin 被重定向（自动化/管道）时
# 改从 Console.In 读一行，避免 Read-Host 在该场景下阻塞。
function Read-Answer([string]$prompt) {
    if ([Console]::IsInputRedirected) {
        Write-Host "$prompt " -NoNewline
        $line = [Console]::In.ReadLine()
        Write-Host ''
        if ($null -eq $line) { return '' }
        return $line.Trim()
    }
    return (Read-Host $prompt)
}

# 探测某个 exe 是否支持 --convert。旧版本不认识该参数会直接打开 GUI，
# 所以必须带超时并主动结束进程，避免脚本被"卡住"。
function Test-ConvertSupport([string]$exe) {
    try {
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = $exe
        $psi.Arguments = '--convert'
        $psi.UseShellExecute = $false
        $psi.RedirectStandardOutput = $true
        $psi.RedirectStandardError = $true
        $psi.CreateNoWindow = $true
        $proc = [System.Diagnostics.Process]::Start($psi)
        if (-not $proc.WaitForExit(6000)) {
            try { $proc.Kill() } catch { }
            return $false
        }
        $err = $proc.StandardError.ReadToEnd()
        return ($err -match '用法' -or $err -match '--convert')
    } catch {
        return $false
    }
}

function Get-TargetMenu {
    return @(
        @{ n = '1';  label = '1.20-1.20.1';      v = '1.20-1.20.1' },
        @{ n = '2';  label = '1.20.2';           v = '1.20.2' },
        @{ n = '3';  label = '1.20.3-1.20.4';    v = '1.20.3-1.20.4' },
        @{ n = '4';  label = '1.20.5-1.20.6';    v = '1.20.5-1.20.6' },
        @{ n = '5';  label = '1.21-1.21.1';      v = '1.21-1.21.1' },
        @{ n = '6';  label = '1.21.2-1.21.3';    v = '1.21.2-1.21.3' },
        @{ n = '7';  label = '1.21.4';           v = '1.21.4' },
        @{ n = '8';  label = '1.21.5';           v = '1.21.5' },
        @{ n = '9';  label = '1.21.6';           v = '1.21.6' },
        @{ n = '10'; label = '1.21.7-1.21.8';    v = '1.21.7-1.21.8' },
        @{ n = '11'; label = '1.21.9-1.21.10';   v = '1.21.9-1.21.10' },
        @{ n = '12'; label = '1.21.11';          v = '1.21.11' },
        @{ n = '13'; label = '26.1-26.1.2';      v = '26.1-26.1.2' },
        @{ n = '14'; label = '26.2';             v = '26.2' },
        @{ n = '15'; label = '26.3（最新，默认）'; v = '26.3' },
        @{ n = '16'; label = 'bedrock（实验性，仅测试）'; v = 'bedrock' }
    )
}

function Select-Target([string]$current) {
    if ($current) { return $current }
    $menu = Get-TargetMenu

    Write-Host ''
    Write-Host '选择目标版本（输入序号，或直接输入版本号 / pack_format）：' -ForegroundColor Yellow
    foreach ($m in $menu) {
        Write-Host ("  {0,2}) {1}" -f $m.n, $m.label)
    }

    # -MenuChoice 用于脚本化调用（等价于在菜单里输入序号）
    if (-not [string]::IsNullOrWhiteSpace($MenuChoice)) {
        $hit = $menu | Where-Object { $_.n -eq $MenuChoice.Trim() }
        if ($hit) {
            Write-Host ("  → 已按 -MenuChoice {0} 选择：{1}" -f $MenuChoice, $hit.label) -ForegroundColor DarkGray
            return $hit.v
        }
        Write-Host "[警告] -MenuChoice $MenuChoice 不在 1-16 范围内，已忽略。" -ForegroundColor Yellow
    }

    $choice = Read-Answer '选择 [15]'
    if ([string]::IsNullOrWhiteSpace($choice)) { $choice = '15' }
    $hit = $menu | Where-Object { $_.n -eq $choice.Trim() }
    if ($hit) { return $hit.v }
    return $choice.Trim()
}

function Show-KeyLogLines([string[]]$lines) {
    # CLI 自身的结构化输出已含结构/耗时/慢任务；这里丢掉落盘日志的逐条
    # 时间戳行，只在 WARN/ERROR 时保留，避免刷屏。
    foreach ($line in $lines) {
        $text = $line.ToString()
        if ($text -match '^\[\d{4}-\d{2}-\d{2} ') {
            if ($text -match '\[(WARN|ERROR)\]') { Write-Host $text -ForegroundColor Yellow }
            continue
        }
        Write-Host $text
    }
}

# ── 1) 输入路径 ─────────────────────────────────────────────
if (-not $Paths -or $Paths.Count -eq 0) {
    Write-Host ''
    Write-Host '请把资源包（.zip / .mcpack）或文件夹拖到 convert-report.bat 上，' -ForegroundColor Yellow
    Write-Host '或在此粘贴路径后回车：'
    $typed = Read-Answer '路径'
    if ([string]::IsNullOrWhiteSpace($typed)) {
        Write-Host '[错误] 没有提供路径。' -ForegroundColor Red
        exit 2
    }
    $Paths = @($typed)
}

# 拖放时 Windows 会给带空格的路径加引号，这里统一去引号；若路径被拆成多个
# 参数（某些外壳/自动化调用不加引号），尝试用空格重组回一条完整路径。
$rawParts = @($Paths | ForEach-Object { $_.Trim('"') })
$hasExisting = $false
foreach ($p in $rawParts) { if (Test-Path -LiteralPath $p) { $hasExisting = $true } }

$candidates = @()
if (-not $hasExisting -and $rawParts.Count -gt 1) {
    $joined = ($rawParts -join ' ')
    if (Test-Path -LiteralPath $joined) { $candidates = @($joined) }
}
if ($candidates.Count -eq 0) { $candidates = $rawParts }

$valid = @()
foreach ($clean in $candidates) {
    if (Test-Path -LiteralPath $clean) { $valid += (Resolve-Path -LiteralPath $clean).Path }
    else { Write-Host "[警告] 路径不存在，已跳过：$clean" -ForegroundColor Red }
}
if ($valid.Count -eq 0) {
    Write-Host '[错误] 没有可用路径。' -ForegroundColor Red
    exit 2
}

# ── 2) 找程序（并确认它支持 --convert） ─────────────────────
$exe = $null
foreach ($cand in @(
        (Join-Path $PSScriptRoot '..\src-tauri\target\release\2-pyramid.exe'),
        (Join-Path $PSScriptRoot '..\src-tauri\target\debug\2-pyramid.exe'),
        (Join-Path $PSScriptRoot '..\release\staging\2-pyramid.exe'),
        (Join-Path $env:LOCALAPPDATA '2-Pyramid\2-pyramid.exe'),
        (Join-Path $env:LOCALAPPDATA '2-Pyramid-Beta\2-pyramid.exe')
    )) {
    if (-not ($cand -and (Test-Path -LiteralPath $cand -PathType Leaf))) { continue }
    $full = (Resolve-Path -LiteralPath $cand).Path
    Write-Host "检测：$full" -ForegroundColor DarkGray
    if (Test-ConvertSupport $full) { $exe = $full; break }
    Write-Host '      该版本不支持 --convert，跳过。' -ForegroundColor DarkGray
}
if (-not $exe) {
    Write-Host '[错误] 没有找到支持 --convert 的 2-pyramid.exe。' -ForegroundColor Red
    Write-Host '       请先在仓库里构建（npm run buildrelease，或 npm run 2pyr），' -ForegroundColor DarkGray
    Write-Host '       或安装 2.6.1 及以后的版本。' -ForegroundColor DarkGray
    exit 3
}
Write-Host "使用程序：$exe" -ForegroundColor Green

# ── 3) 目标版本 ─────────────────────────────────────────────
$Target = Select-Target $Target
if ($Target -match '^bedrock') {
    Write-Host ''
    Write-Host '[警告] Bedrock 转换尚未完成、存在严重问题，仅用于测试。' -ForegroundColor Yellow
    if (-not $Yes) {
        $ok = Read-Answer '仍要继续？(y/N)'
        if ($ok -notmatch '^[Yy]') { Write-Host '已取消。'; exit 0 }
    }
}

Write-Head "2-Pyramid 转换：目标 $Target"
$exitCode = 0

# ── 4) 逐个转换 ─────────────────────────────────────────────
foreach ($path in $valid) {
    $item = Get-Item -LiteralPath $path
    if ($item.PSIsContainer) { $reportDir = $item.FullName } else { $reportDir = $item.DirectoryName }
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $report = Join-Path $reportDir ("2pyr-report-{0}.json" -f $stamp)

    Write-Host ''
    Write-Host "==> $($item.FullName)" -ForegroundColor White

    $lines = & $exe --convert $item.FullName --to $Target --report $report 2>&1 |
        ForEach-Object { $_.ToString() }
    $rc = $LASTEXITCODE
    if ($rc -ne 0) { $exitCode = $rc }

    Show-KeyLogLines $lines

    if (Test-Path -LiteralPath $report) {
        Write-Host "报告：$report" -ForegroundColor Green
        if (-not $NoOpen) { Start-Process explorer.exe -ArgumentList "/select,`"$report`"" }
    } else {
        Write-Host '[提示] 未生成报告文件。' -ForegroundColor Yellow
    }
}

Write-Head '完成'
if ($exitCode -eq 0) {
    Write-Host '全部资源包转换成功。' -ForegroundColor Green
} else {
    Write-Host "存在失败项（退出码 $exitCode），详见上方输出与报告。" -ForegroundColor Yellow
}

# 仅在真实控制台（拖放/双击）时等待按键，重定向或 -Yes 时不阻塞
if (-not $Yes -and -not [Console]::IsInputRedirected) {
    Write-Host ''
    Read-Host '按回车键关闭' | Out-Null
}
exit $exitCode
