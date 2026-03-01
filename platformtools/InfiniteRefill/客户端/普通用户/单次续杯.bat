@echo off
setlocal EnableExtensions EnableDelayedExpansion
chcp 65001 >nul

REM 单次续杯（探测 -> 上报状态 -> 触发 topup -> 写入新账号 -> 删除失效账号）
REM
REM 依赖：curl + PowerShell
REM
REM 服务端契约（你将自行实现）：
REM - POST /v1/refill/topup
REM   Header: X-User-Key: <USER_KEY or UPLOAD_KEY>
REM   Body:
REM     {"target_pool_size":10,"reports":[{"file_name":"x.json","email_hash":"...","account_id":"...","status_code":401,"probed_at":"2026-..Z"}]}
REM   Resp:
REM     {"ok":true,"accounts":[{"file_name":"无限续杯-001.json","auth_json":{...}}, ...]}

set "SCRIPT_DIR=%~dp0"
set "ROOT_DIR=%SCRIPT_DIR%..\.."
set "CFG_YAML=%ROOT_DIR%\config.yaml"
set "CFG_ENV=%SCRIPT_DIR%状态\无限续杯配置.env"

REM 读取配置（SERVER_URL/USER_KEY/TARGET_POOL_SIZE/TRIGGER_REMAINING）
set "SERVER_URL="
set "USER_KEY="
set "TARGET_POOL_SIZE=10"
set "TRIGGER_REMAINING=2"

if exist "%CFG_ENV%" (
  for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%CFG_ENV%") do (
    if /I "%%A"=="SERVER_URL" set "SERVER_URL=%%B"
    if /I "%%A"=="USER_KEY" set "USER_KEY=%%B"
    if /I "%%A"=="TARGET_POOL_SIZE" set "TARGET_POOL_SIZE=%%B"
    if /I "%%A"=="TRIGGER_REMAINING" set "TRIGGER_REMAINING=%%B"
  )
)

REM 允许命令行覆盖
if not "%~1"=="" set "SERVER_URL=%~1"
if not "%~2"=="" set "USER_KEY=%~2"

if "%SERVER_URL%"=="" (
  echo [ERROR] 未配置 SERVER_URL。请先运行“无限续杯”设置配置：
  echo         "%SCRIPT_DIR%无限续杯.bat"
  exit /b 2
)
if "%USER_KEY%"=="" (
  echo [ERROR] 未配置 USER_KEY。
  exit /b 2
)

REM 解析 auth-dir
set "ACCOUNTS_DIR="
if exist "%CFG_YAML%" (
  for /f "usebackq delims=" %%A in (`powershell -NoProfile -Command "$c=Get-Content -Raw -LiteralPath '%CFG_YAML%'; if($c -match '(?m)^\s*auth-dir\s*:\s*(?:\"([^\"]+)\"|([^#\r\n]+))\s*$'){ $v=($Matches[1] + $Matches[2]).Trim().Replace('\\\\','\\'); Write-Output $v }" 2^>nul`) do (
    if not "%%A"=="" set "ACCOUNTS_DIR=%%A"
  )
)

if "%ACCOUNTS_DIR%"=="" (
  echo [ERROR] 无法从 "%CFG_YAML%" 解析 auth-dir。
  exit /b 3
)

echo [INFO] 服务器地址=%SERVER_URL%
echo [INFO] auth-dir=%ACCOUNTS_DIR%
echo [INFO] 目标账户数=%TARGET_POOL_SIZE% 触发阈值=失效后剩余<=%TRIGGER_REMAINING%

REM 输出目录
for /f "usebackq delims=" %%T in (`powershell -NoProfile -Command "Get-Date -Format 'yyyyMMdd-HHmmss'"`) do set "TS=%%T"
set "OUT_DIR=%SCRIPT_DIR%状态\out\单次续杯-%TS%"
set "REPORT_JSONL=%OUT_DIR%\reports.jsonl"
set "RESP_JSON=%OUT_DIR%\topup_response.json"
set "BODY_JSON=%OUT_DIR%\topup_body.json"
set "BACKUP_DIR=%OUT_DIR%\backup"

if not exist "%OUT_DIR%" mkdir "%OUT_DIR%" >nul 2>nul
if not exist "%BACKUP_DIR%" mkdir "%BACKUP_DIR%" >nul 2>nul

>"%REPORT_JSONL%" echo.

REM 选择要管理的文件集合：优先无限续杯-*.json，否则退化到全部 codex json
set "USE_PREFIX=0"
for %%P in ("%ACCOUNTS_DIR%\无限续杯-*.json") do (
  set "USE_PREFIX=1"
  goto :prefix_done
)
:prefix_done

set /a TOTAL=0
set /a INVALID=0

if "%USE_PREFIX%"=="1" (
  echo [INFO] 检测到前缀“无限续杯-*.json”，仅管理这些文件
  for %%F in ("%ACCOUNTS_DIR%\无限续杯-*.json") do (
    set /a TOTAL+=1
    call :probe_one "%%~fF" "%%~nxF"
  )
) else (
  echo [WARN] 未检测到“无限续杯-*.json”，将退化为管理 auth-dir 下所有 codex 文件（可能包含你的其它文件）
  for %%F in ("%ACCOUNTS_DIR%\*.json") do (
    set /a TOTAL+=1
    call :probe_one "%%~fF" "%%~nxF"
  )
)

set /a THRESH=%TARGET_POOL_SIZE% - %TRIGGER_REMAINING%
if %THRESH% LSS 1 set /a THRESH=1

echo.
echo [INFO] 统计：total=%TOTAL% invalid(401/429)=%INVALID% trigger_invalid>=%THRESH%

REM 如果不足目标数量，也触发（bootstrap）
set "NEED_TRIGGER=0"
if %TOTAL% LSS %TARGET_POOL_SIZE% set "NEED_TRIGGER=1"
if %INVALID% GEQ %THRESH% set "NEED_TRIGGER=1"

if "%NEED_TRIGGER%"=="0" (
  echo [OK] 未达到续杯条件：无需 topup
  exit /b 0
)

REM 构造 topup body：读取 jsonl 为数组
powershell -NoProfile -Command ^
  "$items=@(); foreach($l in Get-Content -LiteralPath '%REPORT_JSONL%' -ErrorAction SilentlyContinue){ if($l -and $l.Trim()){ try{$items += ($l | ConvertFrom-Json)}catch{}} }; $body=@{target_pool_size=[int]('%TARGET_POOL_SIZE%'); reports=$items}; $body | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath '%BODY_JSON%' -Encoding UTF8" >nul 2>nul

echo [INFO] 触发 topup：POST %SERVER_URL%/v1/refill/topup
curl -sS -X POST "%SERVER_URL%/v1/refill/topup" ^
  -H "X-User-Key: %USER_KEY%" ^
  -H "Content-Type: application/json" ^
  --data-binary "@%BODY_JSON%" >"%RESP_JSON%"

REM 解析并写入 accounts
powershell -NoProfile -Command ^
  "try{$r=Get-Content -Raw -LiteralPath '%RESP_JSON%'|ConvertFrom-Json}catch{Write-Output '[ERROR] bad response json'; exit 2}; if(-not $r.ok){Write-Output ('[ERROR] topup failed: ' + ($r.error|Out-String)); exit 2}; $accs=@($r.accounts); if($accs.Count -le 0){Write-Output '[WARN] no accounts returned'; exit 0}; foreach($a in $accs){ $fn=$a.file_name; if(-not $fn){$fn=('无限续杯-' + [Guid]::NewGuid().ToString('N').Substring(0,8) + '.json')}; $dst=Join-Path '%ACCOUNTS_DIR%' $fn; ($a.auth_json | ConvertTo-Json -Depth 10) | Set-Content -LiteralPath $dst -Encoding UTF8 }" 
set "EC=%ERRORLEVEL%"
if not "%EC%"=="0" exit /b %EC%

REM 删除失效文件（401/429）并备份
powershell -NoProfile -Command ^
  "$items=@(); foreach($l in Get-Content -LiteralPath '%REPORT_JSONL%' -ErrorAction SilentlyContinue){ if($l -and $l.Trim()){ try{$items += ($l | ConvertFrom-Json)}catch{}} }; foreach($it in $items){ $sc=[int]$it.status_code; if($sc -eq 401 -or $sc -eq 429){ $fn=$it.file_name; if($fn){ $src=Join-Path '%ACCOUNTS_DIR%' $fn; if(Test-Path -LiteralPath $src){ Copy-Item -LiteralPath $src -Destination (Join-Path '%BACKUP_DIR%' $fn) -Force -ErrorAction SilentlyContinue; Remove-Item -LiteralPath $src -Force -ErrorAction SilentlyContinue } } } }" >nul 2>nul

echo [OK] 已完成单次续杯：新账号已写入 auth-dir；失效(401/429)文件已备份并删除。
echo      输出：%OUT_DIR%
exit /b 0

:probe_one
set "FILE=%~1"
set "BASE=%~2"

set "TYPE="
set "TOKEN="
set "AID="
set "EMAIL="

for /f "usebackq tokens=1,2,3,4 delims=|" %%A in (`powershell -NoProfile -Command "try{$o=Get-Content -Raw -LiteralPath '%FILE%'|ConvertFrom-Json; $ty=($o.type|ForEach-Object{$_.ToString()}); $tok=($o.access_token|ForEach-Object{$_.ToString()}); $id=($o.account_id|ForEach-Object{$_.ToString()}); $em=($o.email|ForEach-Object{$_.ToString()}); Write-Output ($ty + '|' + $tok + '|' + $id + '|' + $em)}catch{Write-Output '|||'}"`) do (
  set "TYPE=%%A"
  set "TOKEN=%%B"
  set "AID=%%C"
  set "EMAIL=%%D"
)

if /I not "%TYPE%"=="codex" exit /b 0
if "%TOKEN%"=="" exit /b 0

REM probe status
set "STATUS="
if "%AID%"=="" (
  for /f "usebackq delims=" %%S in (`curl -sS -o NUL -w "%%{http_code}" -H "Authorization: Bearer %TOKEN%" -H "Accept: application/json, text/plain, */*" -H "User-Agent: codex_cli_rs/0.76.0 (Windows 11; x86_64)" "https://chatgpt.com/backend-api/wham/usage"`) do set "STATUS=%%S"
) else (
  for /f "usebackq delims=" %%S in (`curl -sS -o NUL -w "%%{http_code}" -H "Authorization: Bearer %TOKEN%" -H "Chatgpt-Account-Id: %AID%" -H "Accept: application/json, text/plain, */*" -H "User-Agent: codex_cli_rs/0.76.0 (Windows 11; x86_64)" "https://chatgpt.com/backend-api/wham/usage"`) do set "STATUS=%%S"
)

for /f "usebackq delims=" %%Z in (`powershell -NoProfile -Command "(Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')"`) do set "NOW=%%Z"

REM email_hash = sha256('email:<lower>') or sha256('account_id:<id>')
set "IDENT="
for /f "usebackq delims=" %%I in (`powershell -NoProfile -Command "if('%EMAIL%'){ $e='%EMAIL%'.Trim().ToLower(); if($e){'email:'+$e}else{''} } else { '' }"`) do set "IDENT=%%I"
if "%IDENT%"=="" set "IDENT=account_id:%AID%"

for /f "usebackq delims=" %%H in (`powershell -NoProfile -Command "$s='%IDENT%'; $sha=[System.Security.Cryptography.SHA256]::Create(); $b=[Text.Encoding]::UTF8.GetBytes($s); ($sha.ComputeHash($b) | ForEach-Object { $_.ToString('x2') }) -join ''"`) do set "EH=%%H"

>>"%REPORT_JSONL%" echo {"file_name":"%BASE%","email_hash":"%EH%","account_id":"%AID%","status_code":%STATUS%,"probed_at":"%NOW%"}

if "%STATUS%"=="401" set /a INVALID+=1
if "%STATUS%"=="429" set /a INVALID+=1

exit /b 0
