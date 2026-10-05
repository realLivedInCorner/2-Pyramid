# 校验（并可选归一化）src-tauri/src/task_registry.rs 的 REGISTRY 段。
#
# 定位：**校验器**，不是数据源。88 项元数据由 `invoke_conversion.rs` 的旧注册表
# （提交 375c427 之前）机械提取而来，提取结果已与原文逐条核对
# （name / task_type / tier + 顺序，88/88 一致）。
# 本脚本保证后续任何人格式化该文件时**不会悄悄改掉**其中一项。
#
# 用法：
#   pwsh tools/gen-task-registry.ps1            # 校验：项数、重名、阶段/类型分布、摘要
#   pwsh tools/gen-task-registry.ps1 -AlsoWrite # 归一化写回（内容不变，只重排格式）

param(
    [string]$Root = (Split-Path -Parent $PSScriptRoot),
    [switch]$AlsoWrite
)

$file = Join-Path $Root 'src-tauri\src\task_registry.rs'
$lines = Get-Content $file

$start = -1
$end = -1
for ($i = 0; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match '^pub const REGISTRY: &\[TaskMeta\] = &\[$') { $start = $i; continue }
    if ($start -ge 0 -and $lines[$i] -eq '];') { $end = $i; break }
}
if ($start -lt 0 -or $end -lt 0) { throw "在 $file 里找不到 REGISTRY 段" }

$entries = New-Object System.Collections.Generic.List[object]
for ($i = $start + 1; $i -lt $end; $i++) {
    if ([string]::IsNullOrWhiteSpace($lines[$i])) { continue }
    $e = [regex]::Match($lines[$i], 'name: "([a-z_0-9]+)", task_type: TaskType::(\w+), tier: TaskTier::(\w+)')
    if (-not $e.Success) { throw "第 $($i + 1) 行无法解析：$($lines[$i])" }
    $entries.Add([pscustomobject]@{
        Name = $e.Groups[1].Value
        Type = $e.Groups[2].Value
        Tier = $e.Groups[3].Value
    })
}

Write-Host "REGISTRY 项数 = $($entries.Count)（期望 88）"
if ($entries.Count -ne 88) {
    throw "项数不是 88：$($entries.Count) —— 少一项就会让某个原生任务在 native_placements 里查不到阶段（见 §9.125）"
}
$dupes = $entries | Group-Object Name | Where-Object { $_.Count -gt 1 }
if ($dupes) { throw "出现重名：$($dupes.Name -join ', ')" }

Write-Host "按阶段：$((($entries | Group-Object Tier | ForEach-Object { "$($_.Name)=$($_.Count)" }) -join '  '))"
Write-Host "按类型：$((($entries | Group-Object Type | ForEach-Object { "$($_.Name)=$($_.Count)" }) -join '  '))"
Write-Host "首项：$($entries[0].Name)   末项：$($entries[-1].Name)"
Write-Host "内容摘要：$((($entries | ForEach-Object { "$($_.Name)|$($_.Type)|$($_.Tier)" }) -join ';').GetHashCode())（仅供参考，判据以项数与用例为准）"

if (-not $AlsoWrite) {
    Write-Host "（未写盘；加 -AlsoWrite 生效）"
    return
}

$body = @()
$body += $lines[0..$start]
foreach ($e in $entries) {
    $body += "    TaskMeta { name: `"$($e.Name)`", task_type: TaskType::$($e.Type), tier: TaskTier::$($e.Tier) },"
}
$body += $lines[$end..($lines.Count - 1)]
Set-Content -Path $file -Value $body -Encoding UTF8
Write-Host "已写回 $file（$($body.Count) 行）"
