#requires -Version 5.1
<#[
普通管理员（UPLOAD_KEY）客户端脚本：批量处理某目录下所有 *.json。

用法示例：
  pwsh ./upload_folder.ps1 -Server http://127.0.0.1:8789 -UploadKey test_upload_key -Dir "C:\Users\Administrator\.cli-proxy-api" -Mode register
  pwsh ./upload_folder.ps1 -Server http://127.0.0.1:8789 -UploadKey test_upload_key -Dir "C:\Users\Administrator\.cli-proxy-api" -Mode probe
#>

param(
  [Parameter(Mandatory=$true)][string]$Server,
  [Parameter(Mandatory=$true)][string]$UploadKey,
  [Parameter(Mandatory=$true)][string]$Dir,
  [ValidateSet('register','probe')][string]$Mode = 'probe',
  [string]$Glob = '*.json',
  [int]$TimeoutSeconds = 15,
  [int]$Limit = 2000
)

$ErrorActionPreference = 'Stop'

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$single = Join-Path $scriptDir 'upload_single.ps1'

$files = Get-ChildItem -LiteralPath $Dir -Filter $Glob -File | Sort-Object FullName
if ($Limit -gt 0) { $files = $files | Select-Object -First $Limit }

$ok = 0
$bad = 0
foreach ($f in $files) {
  try {
    pwsh -NoProfile -ExecutionPolicy Bypass -File $single -Server $Server -UploadKey $UploadKey -File $f.FullName -Mode $Mode -TimeoutSeconds $TimeoutSeconds | Out-Host
    $ok++
  } catch {
    Write-Warning "failed: $($f.FullName) $($_.Exception.Message)"
    $bad++
  }
}

Write-Host "done ok=$ok bad=$bad total=$($ok+$bad)"
if ($bad -gt 0) { exit 2 }
exit 0
