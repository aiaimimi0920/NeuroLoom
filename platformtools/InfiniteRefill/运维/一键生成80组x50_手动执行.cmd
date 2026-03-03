@echo off
setlocal EnableExtensions EnableDelayedExpansion
chcp 65001 >nul

REM 可直接双击一键运行；默认从 platformtools/.dev.vars 读取。
REM 也支持命令行覆盖：
REM   一键生成80组x50_手动执行.cmd [SERVER_URL] [ADMIN_TOKEN] [ADMIN_GUARD]
REM .dev.vars 建议配置以下键：
REM   INFINITE_REFILL_SERVER_URL
REM   INFINITE_REFILL_ADMIN_TOKEN
REM   INFINITE_REFILL_ADMIN_GUARD
REM 兼容读取：SERVER_URL / REFILL_SERVER_URL / ADMIN_TOKEN / REFILL_ADMIN_TOKEN / ADMIN_GUARD / REFILL_ADMIN_GUARD

set "SERVER_URL=%~1"
set "ADMIN_TOKEN=%~2"
set "ADMIN_GUARD=%~3"

set "DEV_VARS=%~dp0..\..\.dev.vars"
if exist "%DEV_VARS%" (
  for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%DEV_VARS%") do (
    if /I "%%A"=="SERVER_URL" if "!SERVER_URL!"=="" set "SERVER_URL=%%B"
    if /I "%%A"=="INFINITE_REFILL_SERVER_URL" if "!SERVER_URL!"=="" set "SERVER_URL=%%B"
    if /I "%%A"=="REFILL_SERVER_URL" if "!SERVER_URL!"=="" set "SERVER_URL=%%B"

    if /I "%%A"=="ADMIN_TOKEN" if "!ADMIN_TOKEN!"=="" set "ADMIN_TOKEN=%%B"
    if /I "%%A"=="INFINITE_REFILL_ADMIN_TOKEN" if "!ADMIN_TOKEN!"=="" set "ADMIN_TOKEN=%%B"
    if /I "%%A"=="REFILL_ADMIN_TOKEN" if "!ADMIN_TOKEN!"=="" set "ADMIN_TOKEN=%%B"

    if /I "%%A"=="ADMIN_GUARD" if "!ADMIN_GUARD!"=="" set "ADMIN_GUARD=%%B"
    if /I "%%A"=="INFINITE_REFILL_ADMIN_GUARD" if "!ADMIN_GUARD!"=="" set "ADMIN_GUARD=%%B"
    if /I "%%A"=="REFILL_ADMIN_GUARD" if "!ADMIN_GUARD!"=="" set "ADMIN_GUARD=%%B"
  )
)

if "%SERVER_URL%"=="" set "SERVER_URL=https://refill.aiaimimi.com"
if "%ADMIN_TOKEN%"=="" goto :USAGE
if "%ADMIN_GUARD%"=="" goto :USAGE

echo [INFO] SERVER_URL=%SERVER_URL%
echo [INFO] 已从命令行/.dev.vars 加载管理员密钥

set "OUT_JSON=issue_33x30_response.json"

echo [INFO] 请求创建：33 组用户，每组 30 账号（不下载总包）...
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$ErrorActionPreference='Stop';" ^
  "$body=@{type='user';count=33;label='manual-33x30';server_url='%SERVER_URL%';max_accounts_per_user=30;min_accounts_required=30;ttl_minutes=120;return_bundle_zip=$false}|ConvertTo-Json -Compress;" ^
  "$headers=@{Authorization='Bearer %ADMIN_TOKEN%';'X-Admin-Guard'='%ADMIN_GUARD%'};" ^
  "$resp=Invoke-RestMethod -Method Post -Uri ('%SERVER_URL%/admin/packages/issue') -Headers $headers -ContentType 'application/json' -Body $body;" ^
  "$resp | ConvertTo-Json -Depth 12 | Set-Content -Encoding UTF8 './%OUT_JSON%';" ^
  "if($resp.bundle -and $resp.bundle.download_url){ Write-Output ('BUNDLE_URL=' + $resp.bundle.download_url) };" ^
  "Write-Output ('OK=' + $resp.ok);" ^
  "Write-Output ('BATCH_ID=' + $resp.batch_id);" ^
  "Write-Output ('PACKAGES=' + (@($resp.packages).Count));" ^
  "if($resp.bundle){ Write-Output ('BUNDLE=' + $resp.bundle.name) }"

if errorlevel 1 (
  echo [ERROR] 创建失败，请检查白名单/令牌/库存。
  exit /b 1
)

echo [OK] 已输出响应：%OUT_JSON%
exit /b 0

:USAGE
echo [ERROR] 缺少管理员密钥，请在 platformtools/.dev.vars 配置：
echo         INFINITE_REFILL_ADMIN_TOKEN=...
echo         INFINITE_REFILL_ADMIN_GUARD=...
echo 用法：%~nx0 [服务器地址] [管理员令牌] [管理员护卫码]
exit /b 1

