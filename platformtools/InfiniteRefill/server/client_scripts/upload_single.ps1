#requires -Version 5.1
<#[
普通管理员（UPLOAD_KEY）客户端脚本：上传单个认证文件（合规版：不上传 token）。

模式：
- register: 仅注册身份（email_hash/account_id/seen_at）
- probe:    本地 probe wham/usage 后上报（email_hash/account_id/status_code/probed_at）

用法示例：
  pwsh ./upload_single.ps1 -Server http://127.0.0.1:8789 -UploadKey test_upload_key -File "C:\Users\Administrator\.cli-proxy-api\xxx@email.json" -Mode register
  pwsh ./upload_single.ps1 -Server http://127.0.0.1:8789 -UploadKey test_upload_key -File "C:\Users\Administrator\.cli-proxy-api\xxx@email.json" -Mode probe
#>

param(
  [Parameter(Mandatory=$true)][string]$Server,
  [Parameter(Mandatory=$true)][string]$UploadKey,
  [Parameter(Mandatory=$true)][string]$File,
  [ValidateSet('register','probe')][string]$Mode = 'probe',
  [int]$TimeoutSeconds = 15
)

$ErrorActionPreference = 'Stop'

function UtcNowIso() {
  return (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
}

function Sha256Hex([string]$s) {
  $sha = [System.Security.Cryptography.SHA256]::Create()
  try {
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($s)
    $hash = $sha.ComputeHash($bytes)
    return ($hash | ForEach-Object { $_.ToString('x2') }) -join ''
  } finally {
    $sha.Dispose()
  }
}

function EmailHash([string]$email, [string]$accountId) {
  if ($email -and $email.Trim()) {
    return Sha256Hex("email:$($email.Trim().ToLower())")
  }
  return Sha256Hex("account_id:$($accountId.Trim())")
}

function InferEmailFromFilename([string]$path) {
  $name = [System.IO.Path]::GetFileName($path)
  if ($name.ToLower().EndsWith('.json')) { $name = $name.Substring(0, $name.Length - 5) }
  if ($name.Contains('@')) { return $name }
  return ''
}

function LoadAuth([string]$path) {
  $raw = Get-Content -LiteralPath $path -Raw -Encoding UTF8
  return $raw | ConvertFrom-Json
}

function InvokePostJson([string]$url, [object]$bodyObj) {
  $headers = @{ 'X-Upload-Key' = $UploadKey }
  $json = $bodyObj | ConvertTo-Json -Depth 10
  return Invoke-WebRequest -Method Post -Uri $url -Headers $headers -ContentType 'application/json' -Body $json -TimeoutSec $TimeoutSeconds
}

function ProbeWham([string]$accessToken, [string]$chatgptAccountId) {
  $headers = @{ 
    'Accept' = 'application/json, text/plain, */*'
    'Authorization' = "Bearer $accessToken"
    'User-Agent' = 'codex_cli_rs/0.76.0 (PowerShell)'
  }
  if ($chatgptAccountId -and $chatgptAccountId.Trim()) {
    $headers['Chatgpt-Account-Id'] = $chatgptAccountId.Trim()
  }

  try {
    $resp = Invoke-WebRequest -Method Get -Uri 'https://chatgpt.com/backend-api/wham/usage' -Headers $headers -TimeoutSec $TimeoutSeconds
    return [int]$resp.StatusCode
  } catch {
    # 在 PowerShell 中，非 2xx 常会抛异常；尽量取出 StatusCode
    if ($_.Exception.Response -and $_.Exception.Response.StatusCode) {
      return [int]$_.Exception.Response.StatusCode
    }
    return $null
  }
}

$auth = LoadAuth -path $File
$email = ''
if ($auth.PSObject.Properties.Name -contains 'email') { $email = [string]$auth.email }
if (-not $email.Trim()) { $email = InferEmailFromFilename -path $File }

$accountId = ''
if ($auth.PSObject.Properties.Name -contains 'account_id') { $accountId = [string]$auth.account_id }

$emailHash = EmailHash -email $email -accountId $accountId

if ($Mode -eq 'register') {
  $payload = @{ accounts = @(@{ email_hash = $emailHash; account_id = $accountId; seen_at = (UtcNowIso) }) }
  $url = ($Server.TrimEnd('/') + '/v1/accounts/register')
  $resp = InvokePostJson -url $url -bodyObj $payload
  Write-Host "HTTP $($resp.StatusCode)";
  Write-Host $resp.Content
  exit 0
}

# probe
$accessToken = ''
if ($auth.PSObject.Properties.Name -contains 'access_token') { $accessToken = [string]$auth.access_token }

$statusCode = ProbeWham -accessToken $accessToken -chatgptAccountId $accountId
$payload = @{ reports = @(@{ email_hash = $emailHash; account_id = $accountId; status_code = $statusCode; probed_at = (UtcNowIso) }) }
$url = ($Server.TrimEnd('/') + '/v1/probe-report')
$resp = InvokePostJson -url $url -bodyObj $payload
Write-Host "probe_result status_code=$statusCode";
Write-Host "HTTP $($resp.StatusCode)";
Write-Host $resp.Content
exit 0
