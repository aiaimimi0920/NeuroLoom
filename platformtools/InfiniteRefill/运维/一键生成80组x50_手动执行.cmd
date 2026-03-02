@echo off
setlocal EnableExtensions EnableDelayedExpansion
chcp 65001 >nul

REM 可直接双击运行（内置默认密钥）；也支持命令行覆盖。
REM 用法（可选覆盖）：
REM   一键生成80组x50_手动执行.cmd [SERVER_URL] [ADMIN_TOKEN] [ADMIN_GUARD]

set "SERVER_URL=%~1"
set "ADMIN_TOKEN=%~2"
set "ADMIN_GUARD=%~3"

if "%SERVER_URL%"=="" set "SERVER_URL=https://refill.aiaimimi.com"
if "%ADMIN_TOKEN%"=="" set "ADMIN_TOKEN=adm_PTAtWdJk8lLr2SZQEoqHHSuDUhGMfWUdZHvzSqT84AwAyblddnrAcrkeMlktUNyL"
if "%ADMIN_GUARD%"=="" set "ADMIN_GUARD=CDIQyfens6fVp0jSm4X1FqJgnDKpz72s4bT9k4SSTDU"

set "OUT_JSON=issue_80x50_response.json"
set "OUT_ZIP=80x50-packages.bundle.zip"

echo [INFO] 请求创建：80 组用户，每组 50 账号...
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$ErrorActionPreference='Stop';" ^
  "$body=@{type='user';count=80;label='manual-80x50';server_url='%SERVER_URL%';max_accounts_per_user=50;min_accounts_required=50;ttl_minutes=120;return_bundle_zip=$true}|ConvertTo-Json -Compress;" ^
  "$headers=@{Authorization='Bearer %ADMIN_TOKEN%';'X-Admin-Guard'='%ADMIN_GUARD%'};" ^
  "$resp=Invoke-RestMethod -Method Post -Uri ('%SERVER_URL%/admin/packages/issue') -Headers $headers -ContentType 'application/json' -Body $body;" ^
  "$resp | ConvertTo-Json -Depth 12 | Set-Content -Encoding UTF8 './%OUT_JSON%';" ^
  "if($resp.bundle -and $resp.bundle.download_url){ Invoke-WebRequest -Uri $resp.bundle.download_url -OutFile './%OUT_ZIP%' };" ^
  "Write-Output ('OK=' + $resp.ok);" ^
  "Write-Output ('BATCH_ID=' + $resp.batch_id);" ^
  "Write-Output ('PACKAGES=' + (@($resp.packages).Count));" ^
  "if($resp.bundle){ Write-Output ('BUNDLE=' + $resp.bundle.name) }"

if errorlevel 1 (
  echo [ERROR] 创建失败，请检查白名单/令牌/库存。
  exit /b 1
)

echo [OK] 已输出响应：%OUT_JSON%
echo [OK] 已下载总包：%OUT_ZIP%
exit /b 0

:USAGE
echo 用法：%~nx0 [服务器地址] [管理员令牌] [管理员护卫码]
echo 示例：%~nx0 https://refill.aiaimimi.com adm_xxx guard_xxx
exit /b 1
