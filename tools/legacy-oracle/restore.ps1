<#
.SYNOPSIS
    把「旧转换器参考实现」从 git 恢复到工作树。

.DESCRIPTION
    M3 期间，`src-tauri/src/converters/` 下的旧转换器被**移出版本控制**（本地保留），
    原因是它们在生产路径上已被 A-ROM 原生实现取代（§9.118），
    但仍作为**验证对照的参考实现**在使用（冻结基线 §9.115 由它们生成）。

    本脚本从 `REF_COMMIT` 恢复全部被移出的文件到**工作树**（不改变 git 跟踪状态）。
    **幂等**：已存在的文件默认不覆盖，除非加 `-Force`。

.PARAMETER Force
    覆盖已存在的文件（会丢失本地对参考实现的临时修改）。

.EXAMPLE
    pwsh tools/legacy-oracle/restore.ps1
    pwsh tools/legacy-oracle/restore.ps1 -Force
#>
[CmdletBinding()]
param(
    [switch]$Force
)

$ErrorActionPreference = 'Stop'

# 移出时所在的提交 —— 所有被移出的文件都能从这里取回
$REF_COMMIT = '460f05235cae3da803dc7aa03c53aa7ac606f3a5'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$manifest = Join-Path $PSScriptRoot 'untracked-files.txt'

if (-not (Test-Path $manifest)) { throw "清单不存在：$manifest" }

Write-Host "仓库：$repoRoot"
Write-Host "恢复源：$REF_COMMIT"
Write-Host ""

# 确认该提交可达
& git -C $repoRoot cat-file -e "${REF_COMMIT}^{commit}" 2>$null
if ($LASTEXITCODE -ne 0) {
    throw "提交 $REF_COMMIT 在当前仓库里不可达 —— 无法恢复。请检查仓库历史。"
}

$files = Get-Content $manifest | Where-Object { $_.Trim() -ne '' }

# 只挑出"当前工作树里缺失"的文件交给 git checkout（避免覆盖本地修改）
$missing = @()
$present = 0
foreach ($rel in $files) {
    $target = Join-Path $repoRoot ($rel -replace '/', '\')
    if ((Test-Path $target) -and -not $Force) { $present++ } else { $missing += $rel }
}

Write-Host "清单 $($files.Count) 个：缺失/待覆盖 $($missing.Count) 个，已存在跳过 $present 个。"

if ($missing.Count -eq 0) {
    Write-Host "无需恢复（全部已存在）。"
    exit 0
}

# 交给 git 做恢复 —— 它按字节写文件，不会改变行尾/编码
$args = @('-C', $repoRoot, 'checkout', $REF_COMMIT, '--') + $missing
& git @args
if ($LASTEXITCODE -ne 0) { throw "git checkout 失败（退出码 $LASTEXITCODE）" }

# git checkout <commit> -- <paths> 会把这些文件放进**索引**；
# 我们要的是"只恢复工作树、不改跟踪状态"，因此把它们从索引撤出。
& git -C $repoRoot reset --quiet HEAD -- @missing 2>$null | Out-Null

Write-Host ""
Write-Host "已恢复 $($missing.Count) 个工作树文件（跟踪状态未改变）。"
Write-Host "注意：恢复后请重新运行完整验证："
Write-Host "  cargo test --lib"
Write-Host "  cargo test --lib -- --ignored        # 含真实包对照"
