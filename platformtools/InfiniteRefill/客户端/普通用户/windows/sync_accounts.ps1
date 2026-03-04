# sync_accounts.ps1
# 统一的账号文件同步脚本（symlink 优先 + copy 回退 + manifest 管理）
#
# 用法：
#   powershell -NoProfile -ExecutionPolicy Bypass -File sync_accounts.ps1 -AccountsDir <path> -TargetDir <path>
#
param(
    [Parameter(Mandatory=$true)]
    [string]$AccountsDir,

    [Parameter(Mandatory=$true)]
    [string]$TargetDir
)

$ErrorActionPreference = 'Stop'

# --- 可写性检测 ---
function Test-CanWrite {
    param([string]$Path)
    try {
        if (-not (Test-Path -LiteralPath $Path)) {
            New-Item -ItemType Directory -Path $Path -Force | Out-Null
        }
        $testFile = Join-Path $Path '.write_test.tmp'
        Set-Content -LiteralPath $testFile -Value 'ok' -Encoding ASCII
        Remove-Item -LiteralPath $testFile -Force -ErrorAction SilentlyContinue
        return $true
    } catch {
        return $false
    }
}

if (-not $TargetDir) { exit 0 }
if (-not $AccountsDir) { exit 0 }

if (-not (Test-Path -LiteralPath $AccountsDir)) {
    New-Item -ItemType Directory -Path $AccountsDir -Force | Out-Null
}
if (-not (Test-Path -LiteralPath $TargetDir)) {
    New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null
}

if (-not (Test-CanWrite $TargetDir)) {
    $esc = [char]27
    Write-Output "${esc}[91m[ERROR] 用户目标目录不可写：${TargetDir}${esc}[0m"
    Write-Output "${esc}[91m[ERROR] 用户目标目录写入需要使用管理员身份运行脚本。${esc}[0m"
    Write-Output "${esc}[91m[ERROR] 请重新配置 SYNC_TARGET_DIR 后重试。${esc}[0m"
    exit 0
}

$manifest = Join-Path $TargetDir '.infinite_refill_sync_manifest.txt'

# --- 列出源文件 ---
$src = @(Get-ChildItem -LiteralPath $AccountsDir -Filter 'codex-*.json' -File -ErrorAction SilentlyContinue)
if ($src.Count -eq 0) {
    $src = @(Get-ChildItem -LiteralPath $AccountsDir -Filter '*.json' -File -ErrorAction SilentlyContinue)
}

$names = @()
foreach ($f in $src) { $names += $f.Name }

# --- 读取旧 manifest ---
$old = @()
if (Test-Path -LiteralPath $manifest) {
    $old = @(Get-Content -LiteralPath $manifest -ErrorAction SilentlyContinue |
             Where-Object { $_ -and $_.Trim() -ne '' })
}

# --- 清理旧条目（仅清理 symlink） ---
$removed = 0
foreach ($n in $old) {
    if ($names -notcontains $n) {
        $tp = Join-Path $TargetDir $n
        if (Test-Path -LiteralPath $tp) {
            $it = Get-Item -LiteralPath $tp -Force -ErrorAction SilentlyContinue
            if ($it -and (($it.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {
                Remove-Item -LiteralPath $tp -Force -ErrorAction SilentlyContinue
                $removed++
            }
        }
    }
}

# --- 尝试 Symlink ---
$linked = 0
foreach ($f in $src) {
    $tp = Join-Path $TargetDir $f.Name
    if (Test-Path -LiteralPath $tp) {
        $it = Get-Item -LiteralPath $tp -Force -ErrorAction SilentlyContinue
        # 如果是旧 symlink，先删除再重建
        if ($it -and (($it.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {
            Remove-Item -LiteralPath $tp -Force -ErrorAction SilentlyContinue
        }
        # 如果文件仍存在（非 symlink 实体文件），跳过
        if (Test-Path -LiteralPath $tp) { continue }
    }
    New-Item -ItemType SymbolicLink -Path $tp -Target $f.FullName -Force -ErrorAction SilentlyContinue | Out-Null
    if (Test-Path -LiteralPath $tp) { $linked++ }
}

# --- Copy 回退（对于 symlink 失败的文件或非管理员环境） ---
$copied = 0
foreach ($f in $src) {
    $tp = Join-Path $TargetDir $f.Name
    if (-not (Test-Path -LiteralPath $tp)) {
        Copy-Item -LiteralPath $f.FullName -Destination $tp -Force -ErrorAction SilentlyContinue
        if (Test-Path -LiteralPath $tp) { $copied++ }
    }
}

# --- 更新 manifest ---
try {
    Set-Content -LiteralPath $manifest -Value $names -Encoding UTF8
} catch {
    Write-Output "[WARN] manifest 写入失败: $manifest"
}

# --- 输出结果 ---
if ($linked -gt 0 -or $copied -gt 0) {
    Write-Output "OK: linked=$linked; copied=$copied; removed=$removed; target=$TargetDir"
} else {
    Write-Output "WARN: linked=0; copied=0; removed=$removed; source=$AccountsDir; target=$TargetDir"
}
