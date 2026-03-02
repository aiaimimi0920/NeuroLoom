@echo off
setlocal EnableExtensions EnableDelayedExpansion
chcp 65001 >nul

if /I "%~1"=="--probe-one-worker" goto :PROBE_ONE_WORKER

REM 单次续杯（探测 -> 上报状态 -> 触发 topup -> 写入新账号 -> 删除失效账号）
REM
REM 依赖：curl + PowerShell
REM
REM 服务端契约（你将自行实现）：
REM - POST /v1/refill/topup
REM   Header: X-User-Key: <USER_KEY>
REM   Body:
REM     {"target_pool_size":10,"reports":[{"file_name":"x.json","email_hash":"...","account_id":"...","status_code":401,"probed_at":"2026-..Z"}]}
REM   Resp:
REM     {"ok":true,"accounts":[{"file_name":"无限续杯-001.json","download_url":"https://..."}], ...}

set "SCRIPT_DIR=%~dp0"
set "ROOT_DIR=%SCRIPT_DIR%..\.."
set "CFG_YAML=%ROOT_DIR%\config.yaml"
set "CFG_ENV=%SCRIPT_DIR%无限续杯配置.env"
set "ROOT_CFG_ENV=%ROOT_DIR%\无限续杯配置.env"

REM 读取配置（SERVER_URL/USER_KEY/ACCOUNTS_DIR/TARGET_POOL_SIZE/TRIGGER_REMAINING）
set "MODE_SYNC_ALL=0"
set "MODE_FROM_TASK=0"
if /I "%~1"=="--sync-all" (
  set "MODE_SYNC_ALL=1"
  shift
)
if /I "%~1"=="--from-task" (
  set "MODE_FROM_TASK=1"
  shift
)

set "SERVER_URL="
set "USER_KEY="
set "ACCOUNTS_DIR=%ROOT_DIR%\accounts"
set "TARGET_POOL_SIZE=10"
set "TRIGGER_REMAINING=2"
set "SYNC_MODE=none"
set "SYNC_TARGET_DIR="
set "WHAM_PROXY_MODE=auto"
set "WHAM_CONNECT_TIMEOUT=5"
set "WHAM_MAX_TIME=15"
set "RUN_OUTPUT_MODE=compact"
set "PROBE_PARALLEL=6"
set "FINAL_REPORT=%SCRIPT_DIR%out\最终续杯报告.json"

if exist "%ROOT_CFG_ENV%" (
  for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%ROOT_CFG_ENV%") do (
    if /I "%%A"=="SERVER_URL" set "SERVER_URL=%%B"
    if /I "%%A"=="USER_KEY" set "USER_KEY=%%B"
    if /I "%%A"=="ACCOUNTS_DIR" set "ACCOUNTS_DIR=%%B"
    if /I "%%A"=="TARGET_POOL_SIZE" set "TARGET_POOL_SIZE=%%B"
    if /I "%%A"=="TRIGGER_REMAINING" set "TRIGGER_REMAINING=%%B"
    if /I "%%A"=="SYNC_MODE" set "SYNC_MODE=%%B"
    if /I "%%A"=="SYNC_TARGET_DIR" set "SYNC_TARGET_DIR=%%B"
    if /I "%%A"=="WHAM_PROXY_MODE" set "WHAM_PROXY_MODE=%%B"
    if /I "%%A"=="WHAM_CONNECT_TIMEOUT" set "WHAM_CONNECT_TIMEOUT=%%B"
    if /I "%%A"=="WHAM_MAX_TIME" set "WHAM_MAX_TIME=%%B"
    if /I "%%A"=="RUN_OUTPUT_MODE" set "RUN_OUTPUT_MODE=%%B"
    if /I "%%A"=="PROBE_PARALLEL" set "PROBE_PARALLEL=%%B"
  )
)
if exist "%CFG_ENV%" (
  for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%CFG_ENV%") do (
    if /I "%%A"=="SERVER_URL" set "SERVER_URL=%%B"
    if /I "%%A"=="USER_KEY" set "USER_KEY=%%B"
    if /I "%%A"=="ACCOUNTS_DIR" set "ACCOUNTS_DIR=%%B"
    if /I "%%A"=="TARGET_POOL_SIZE" set "TARGET_POOL_SIZE=%%B"
    if /I "%%A"=="TRIGGER_REMAINING" set "TRIGGER_REMAINING=%%B"
    if /I "%%A"=="SYNC_MODE" set "SYNC_MODE=%%B"
    if /I "%%A"=="SYNC_TARGET_DIR" set "SYNC_TARGET_DIR=%%B"
    if /I "%%A"=="WHAM_PROXY_MODE" set "WHAM_PROXY_MODE=%%B"
    if /I "%%A"=="WHAM_CONNECT_TIMEOUT" set "WHAM_CONNECT_TIMEOUT=%%B"
    if /I "%%A"=="WHAM_MAX_TIME" set "WHAM_MAX_TIME=%%B"
    if /I "%%A"=="PROBE_PARALLEL" set "PROBE_PARALLEL=%%B"
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

if "%ACCOUNTS_DIR%"=="" set "ACCOUNTS_DIR=%ROOT_DIR%\accounts"
if not exist "%ACCOUNTS_DIR%" mkdir "%ACCOUNTS_DIR%" >nul 2>nul
if not exist "%ACCOUNTS_DIR%" (
  echo [ERROR] 账户目录不存在且创建失败："%ACCOUNTS_DIR%"
  exit /b 3
)

REM 注意：不要清空全局代理环境变量。
REM - 探测 OpenAI(wham) 可能依赖代理；
REM - 仅对本服务端请求使用 --noproxy "*" 强制直连，避免命中失效本地代理。

if "%MODE_FROM_TASK%"=="0" echo [INFO] 服务器地址=%SERVER_URL%
if "%MODE_FROM_TASK%"=="0" echo [INFO] accounts-dir=%ACCOUNTS_DIR%
if "%MODE_FROM_TASK%"=="0" echo [INFO] 目标账户数=%TARGET_POOL_SIZE% 触发阈值=失效后剩余<=%TRIGGER_REMAINING%

if "%MODE_SYNC_ALL%"=="1" goto :SYNC_ALL_PREP

REM 输出目录
for /f "usebackq delims=" %%T in (`powershell -NoProfile -Command "Get-Date -Format 'yyyyMMdd-HHmmss'"`) do set "TS=%%T"
set "OUT_DIR=%SCRIPT_DIR%out\单次续杯-%TS%"
set "REPORT_JSONL=%OUT_DIR%\reports.jsonl"
set "RESP_JSON=%OUT_DIR%\topup_response.json"
set "BODY_JSON=%OUT_DIR%\topup_body.json"
set "BACKUP_DIR=%OUT_DIR%\backup"
set "NETFAIL_LOG=%OUT_DIR%\probe_netfail.log"

if /I "%RUN_OUTPUT_MODE%"=="compact" (
  set "OUT_DIR=%SCRIPT_DIR%out\latest"
  set "REPORT_JSONL=%OUT_DIR%\reports.jsonl"
  set "RESP_JSON=%OUT_DIR%\topup_response.json"
  set "BODY_JSON=%OUT_DIR%\topup_body.json"
  set "BACKUP_DIR=%OUT_DIR%\backup"
  set "NETFAIL_LOG=%OUT_DIR%\probe_netfail.log"
  if exist "%OUT_DIR%" rmdir /s /q "%OUT_DIR%" >nul 2>nul
)

if not exist "%OUT_DIR%" mkdir "%OUT_DIR%" >nul 2>nul
if not exist "%SCRIPT_DIR%out" mkdir "%SCRIPT_DIR%out" >nul 2>nul

>"%REPORT_JSONL%" echo.
>"%NETFAIL_LOG%" echo.

REM 仅管理 accounts-dir 下所有 .json 文件（并行探测）
set /a TOTAL=0
set /a PROBED_OK=0
set /a NET_FAIL=0
set /a INVALID=0
set "PROBE_DIR=%OUT_DIR%\probe_jobs"
if exist "%PROBE_DIR%" rmdir /s /q "%PROBE_DIR%" >nul 2>nul
mkdir "%PROBE_DIR%" >nul 2>nul

set /a LAUNCHED=0
for /f "usebackq delims=" %%F in (`dir /b /a-d "%ACCOUNTS_DIR%\*.json" 2^>nul`) do (
  set /a TOTAL+=1
  set /a LAUNCHED+=1
  start "" /b cmd /c ""%~f0" --probe-one-worker "%ACCOUNTS_DIR%\%%F" "%%F" "%PROBE_DIR%\!TOTAL!" "%WHAM_PROXY_MODE%" "%WHAM_CONNECT_TIMEOUT%" "%WHAM_MAX_TIME%""
  call :WAIT_FOR_PROBE_SLOT "%PROBE_DIR%" "!LAUNCHED!" "%PROBE_PARALLEL%"
)

if %LAUNCHED% GTR 0 (
  call :WAIT_FOR_PROBE_ALL "%PROBE_DIR%" "%LAUNCHED%"

  >"%REPORT_JSONL%" (
    for /f "usebackq delims=" %%M in (`dir /b /a-d "%PROBE_DIR%\*.meta" 2^>nul`) do (
      for /f "usebackq tokens=1,2,3 delims=|" %%a in ("%PROBE_DIR%\%%M") do (
        set /a PROBED_OK+=%%a
        set /a NET_FAIL+=%%b
        set /a INVALID+=%%c
      )
    )
    for /f "usebackq delims=" %%R in (`dir /b /a-d "%PROBE_DIR%\*.rep" 2^>nul`) do type "%PROBE_DIR%\%%R"
  )

  >"%NETFAIL_LOG%" (
    for /f "usebackq delims=" %%N in (`dir /b /a-d "%PROBE_DIR%\*.net" 2^>nul`) do type "%PROBE_DIR%\%%N"
  )
)

set /a THRESH=%TARGET_POOL_SIZE% - %TRIGGER_REMAINING%
if %THRESH% LSS 1 set /a THRESH=1

set /a AVAILABLE_EST=%TOTAL% - %INVALID%

echo.
echo [INFO] 统计：total=%TOTAL% available_est=%AVAILABLE_EST% probed_ok=%PROBED_OK% net_fail=%NET_FAIL% invalid(401/429)=%INVALID% trigger_invalid^>=%THRESH%
if %TOTAL% EQU 0 (
  echo [WARN] accounts-dir 下未发现 .json 文件：%ACCOUNTS_DIR%
)

REM 如果不足目标数量，也触发（bootstrap）
REM 规则：网络失败(net_fail)默认按“可用”计入 available_est（即不算 invalid）
set "NEED_TRIGGER=0"
if %TOTAL% LSS %TARGET_POOL_SIZE% set "NEED_TRIGGER=1"
if %AVAILABLE_EST% LEQ %TRIGGER_REMAINING% set "NEED_TRIGGER=1"

if "%NEED_TRIGGER%"=="0" (
  echo [OK] 未达到续杯条件：无需 topup
  call :WRITE_FINAL_REPORT "topup" "not_triggered" "%TOTAL%" "%PROBED_OK%" "%NET_FAIL%" "%INVALID%" "%OUT_DIR%"
  exit /b 0
)

REM 构造 topup body：读取 jsonl 为数组
powershell -NoProfile -Command ^
  "$items=@(); foreach($l in Get-Content -LiteralPath '%REPORT_JSONL%' -ErrorAction SilentlyContinue){ if($l -and $l.Trim()){ try{$items += ($l | ConvertFrom-Json)}catch{}} }; $body=@{target_pool_size=[int]('%TARGET_POOL_SIZE%'); reports=$items}; $body | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath '%BODY_JSON%' -Encoding UTF8" >nul 2>nul

echo [INFO] 触发 topup：POST %SERVER_URL%/v1/refill/topup
curl -sS --connect-timeout 8 --max-time 30 --noproxy "*" -X POST "%SERVER_URL%/v1/refill/topup" ^
  -H "X-User-Key: %USER_KEY%" ^
  -H "Content-Type: application/json" ^
  --data-binary "@%BODY_JSON%" >"%RESP_JSON%"

REM 解析并写入 accounts（兼容 auth_json / download_url）
powershell -NoProfile -Command ^
  "$ErrorActionPreference='Stop'; $ProgressPreference='SilentlyContinue'; $utf8NoBom=New-Object System.Text.UTF8Encoding($false); try{$r=Get-Content -Raw -LiteralPath '%RESP_JSON%'|ConvertFrom-Json}catch{Write-Output '[ERROR] bad response json'; exit 2}; if(-not $r.ok){Write-Output ('[ERROR] topup failed: ' + ($r.error|Out-String)); exit 2}; $accs=@($r.accounts); if($accs.Count -le 0){Write-Output '[WARN] no accounts returned'; exit 0}; $written=0; foreach($a in $accs){ $fn=$a.file_name; if(-not $fn){$fn=('无限续杯-' + [Guid]::NewGuid().ToString('N').Substring(0,8) + '.json')}; $dst=Join-Path '%ACCOUNTS_DIR%' $fn; if($null -ne $a.auth_json){ $canon=($a.auth_json | ConvertTo-Json -Depth 20 -Compress); [System.IO.File]::WriteAllText($dst, ($canon + [Environment]::NewLine), $utf8NoBom); $written++; continue }; $dl=($a.download_url|ForEach-Object{$_.ToString().Trim()}); if($dl){ try{ $raw=(Invoke-WebRequest -UseBasicParsing -Uri $dl -Method GET -TimeoutSec 30).Content; $obj=$raw | ConvertFrom-Json; $canon=($obj | ConvertTo-Json -Depth 20 -Compress); [System.IO.File]::WriteAllText($dst, ($canon + [Environment]::NewLine), $utf8NoBom); $written++; }catch{} } }; Write-Output ('[INFO] 写入新账号：' + $written)"
set "EC=%ERRORLEVEL%"
if not "%EC%"=="0" exit /b %EC%

REM 删除失效文件（401/429）并备份
powershell -NoProfile -Command ^
  "$items=@(); foreach($l in Get-Content -LiteralPath '%REPORT_JSONL%' -ErrorAction SilentlyContinue){ if($l -and $l.Trim()){ try{$items += ($l | ConvertFrom-Json)}catch{}} }; foreach($it in $items){ $sc=[int]$it.status_code; if($sc -eq 401 -or $sc -eq 429){ $fn=$it.file_name; if($fn){ $src=Join-Path '%ACCOUNTS_DIR%' $fn; if(Test-Path -LiteralPath $src){ if(-not (Test-Path -LiteralPath '%BACKUP_DIR%')){ New-Item -ItemType Directory -Path '%BACKUP_DIR%' -Force | Out-Null }; Copy-Item -LiteralPath $src -Destination (Join-Path '%BACKUP_DIR%' $fn) -Force -ErrorAction SilentlyContinue; Remove-Item -LiteralPath $src -Force -ErrorAction SilentlyContinue } } } }" >nul 2>nul

if /I not "%SYNC_MODE%"=="none" if not "%SYNC_TARGET_DIR%"=="" (
  if not exist "%SYNC_TARGET_DIR%" mkdir "%SYNC_TARGET_DIR%" >nul 2>nul
  for /f "usebackq delims=" %%L in (`powershell -NoProfile -Command ^
    "$ErrorActionPreference='Stop'; $accounts='%ACCOUNTS_DIR%'; $targetRaw='%SYNC_TARGET_DIR%'; $fallback=Join-Path $env:USERPROFILE '.cli-proxy-api';" ^
    "$canWrite={ param($p) try{ if(-not (Test-Path -LiteralPath $p)){ New-Item -ItemType Directory -Path $p -Force | Out-Null }; $t=Join-Path $p '.write_test.tmp'; Set-Content -LiteralPath $t -Value 'ok' -Encoding ASCII; Remove-Item -LiteralPath $t -Force -ErrorAction SilentlyContinue; $true } catch { $false } };" ^
    "$target=$targetRaw; if(-not (& $canWrite $target)){ if(& $canWrite $fallback){ Write-Output ('[WARN] sync target 不可写，已回退到: ' + $fallback); $target=$fallback } else { Write-Output ('[WARN] sync target 不可写，已跳过同步: ' + $targetRaw); exit 0 } };" ^
    "$manifest=Join-Path $target '.infinite_refill_sync_manifest.txt';" ^
    "$src=@(Get-ChildItem -LiteralPath $accounts -Filter '无限续杯-*.json' -File -ErrorAction SilentlyContinue); if($src.Count -eq 0){$src=@(Get-ChildItem -LiteralPath $accounts -Filter '*.json' -File -ErrorAction SilentlyContinue)};" ^
    "$names=@(); foreach($f in $src){ $names += $f.Name };" ^
    "$old=@(); if(Test-Path -LiteralPath $manifest){ $old=@(Get-Content -LiteralPath $manifest -ErrorAction SilentlyContinue | Where-Object { $_ -and $_.Trim() -ne '' }) };" ^
    "$removed=0; foreach($n in $old){ if($names -notcontains $n){ $tp=Join-Path $target $n; if(Test-Path -LiteralPath $tp){ $it=Get-Item -LiteralPath $tp -Force -ErrorAction SilentlyContinue; if($it -and (($it.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)){ Remove-Item -LiteralPath $tp -Force -ErrorAction SilentlyContinue; $removed++ } } } };" ^
    "$linked=0; foreach($f in $src){ $tp=Join-Path $target $f.Name; if(Test-Path -LiteralPath $tp){ $it=Get-Item -LiteralPath $tp -Force -ErrorAction SilentlyContinue; if($it -and (($it.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)){ Remove-Item -LiteralPath $tp -Force -ErrorAction SilentlyContinue }; if(Test-Path -LiteralPath $tp){ continue } }; New-Item -ItemType SymbolicLink -Path $tp -Target $f.FullName -Force -ErrorAction SilentlyContinue | Out-Null; if(Test-Path -LiteralPath $tp){ $linked++ } };" ^
    "try{ Set-Content -LiteralPath $manifest -Value $names -Encoding UTF8 } catch { Write-Output ('[WARN] manifest 写入失败: ' + $manifest) };" ^
    "'INFO: linked=' + $linked + '; removed=' + $removed + '; target=' + $target"`) do echo %%L
  echo [OK] 已同步 managed json 到：%SYNC_TARGET_DIR%（mode=%SYNC_MODE%）
)

echo [OK] 已完成单次续杯：新账号已写入 accounts-dir；失效(401/429)文件已备份并删除。
echo      输出：%OUT_DIR%
call :WRITE_FINAL_REPORT "topup" "triggered" "%TOTAL%" "%PROBED_OK%" "%NET_FAIL%" "%INVALID%" "%OUT_DIR%"
exit /b 0

:SYNC_ALL_PREP
for /f "usebackq delims=" %%T in (`powershell -NoProfile -Command "Get-Date -Format 'yyyyMMdd-HHmmss'"`) do set "TS=%%T"
set "TMP_BASE=%TEMP%"
if "%TMP_BASE%"=="" set "TMP_BASE=%SCRIPT_DIR%out"
if not exist "%TMP_BASE%" mkdir "%TMP_BASE%" >nul 2>nul
if /I "%RUN_OUTPUT_MODE%"=="compact" (
  set "OUT_DIR=%SCRIPT_DIR%out\latest-syncall"
) else (
  set "OUT_DIR=%TMP_BASE%\InfiniteRefill-syncall-%TS%"
)
set "RESP_JSON=%OUT_DIR%\sync_all_response.json"
if /I "%RUN_OUTPUT_MODE%"=="compact" if exist "%OUT_DIR%" rmdir /s /q "%OUT_DIR%" >nul 2>nul
if not exist "%OUT_DIR%" mkdir "%OUT_DIR%" >nul 2>nul
goto :SYNC_ALL

:SYNC_ALL
echo [INFO] 全量同步：POST %SERVER_URL%/v1/refill/sync-all
curl -sS --connect-timeout 8 --max-time 30 --noproxy "*" -X POST "%SERVER_URL%/v1/refill/sync-all" ^
  -H "X-User-Key: %USER_KEY%" ^
  -H "Content-Type: application/json" ^
  --data-binary "{}" >"%RESP_JSON%"
set "CURL_EC=%ERRORLEVEL%"
if not "%CURL_EC%"=="0" (
  echo [ERROR] 请求失败（curl exit=%CURL_EC%），可能是本机 TLS/代理网络问题。
  exit /b 2
)

powershell -NoProfile -Command ^
  "$ErrorActionPreference='Stop'; $ProgressPreference='SilentlyContinue'; $utf8NoBom=New-Object System.Text.UTF8Encoding($false); try{$r=Get-Content -Raw -LiteralPath '%RESP_JSON%'|ConvertFrom-Json}catch{Write-Output '[ERROR] bad response json'; exit 2}; if(-not $r.ok){ $errObj=$r.error; if($null -eq $errObj){ $err='' } elseif($errObj -is [System.Array]){ $err=(($errObj|ForEach-Object{''+$_}) -join '; ') } else { $err=(''+$errObj) }; if($err -match 'not[-_]?found'){ Write-Output '[ERROR] sync-all 接口不存在：请先部署最新版服务端（包含 /v1/refill/sync-all）'; exit 2 }; if($err -match 'invalid user key|missing X-User-Key'){ Write-Output '[ERROR] 用户密钥无效：请在【设置/更新无限续杯配置】里重新填写正确 USER_KEY'; exit 2 }; Write-Output ('[ERROR] sync-all failed: ' + $err); exit 2}; $accs=@($r.accounts); if($accs.Count -le 0){Write-Output '[WARN] no accounts returned'; exit 0}; $written=0; foreach($a in $accs){ $fn=$a.file_name; if(-not $fn){$fn=('无限续杯-' + [Guid]::NewGuid().ToString('N').Substring(0,8) + '.json')}; $dst=Join-Path '%ACCOUNTS_DIR%' $fn; $dl=($a.download_url|ForEach-Object{$_.ToString().Trim()}); if($dl){ try{ $raw=(Invoke-WebRequest -UseBasicParsing -Uri $dl -Method GET -TimeoutSec 30).Content; $obj=$raw | ConvertFrom-Json; $canon=($obj | ConvertTo-Json -Depth 20 -Compress); [System.IO.File]::WriteAllText($dst, ($canon + [Environment]::NewLine), $utf8NoBom); $written++; }catch{} } }; Write-Output ('[INFO] 已同步账号：' + $written)"
set "EC=%ERRORLEVEL%"
if not "%EC%"=="0" exit /b %EC%

if /I not "%SYNC_MODE%"=="none" if not "%SYNC_TARGET_DIR%"=="" (
  if not exist "%SYNC_TARGET_DIR%" mkdir "%SYNC_TARGET_DIR%" >nul 2>nul
  for /f "usebackq delims=" %%L in (`powershell -NoProfile -Command ^
    "$ErrorActionPreference='Stop'; $accounts='%ACCOUNTS_DIR%'; $targetRaw='%SYNC_TARGET_DIR%'; $fallback=Join-Path $env:USERPROFILE '.cli-proxy-api';" ^
    "$canWrite={ param($p) try{ if(-not (Test-Path -LiteralPath $p)){ New-Item -ItemType Directory -Path $p -Force | Out-Null }; $t=Join-Path $p '.write_test.tmp'; Set-Content -LiteralPath $t -Value 'ok' -Encoding ASCII; Remove-Item -LiteralPath $t -Force -ErrorAction SilentlyContinue; $true } catch { $false } };" ^
    "$target=$targetRaw; if(-not (& $canWrite $target)){ if(& $canWrite $fallback){ Write-Output ('[WARN] sync target 不可写，已回退到: ' + $fallback); $target=$fallback } else { Write-Output ('[WARN] sync target 不可写，已跳过同步: ' + $targetRaw); exit 0 } };" ^
    "$manifest=Join-Path $target '.infinite_refill_sync_manifest.txt';" ^
    "$src=@(Get-ChildItem -LiteralPath $accounts -Filter '无限续杯-*.json' -File -ErrorAction SilentlyContinue); if($src.Count -eq 0){$src=@(Get-ChildItem -LiteralPath $accounts -Filter '*.json' -File -ErrorAction SilentlyContinue)};" ^
    "$names=@(); foreach($f in $src){ $names += $f.Name };" ^
    "$old=@(); if(Test-Path -LiteralPath $manifest){ $old=@(Get-Content -LiteralPath $manifest -ErrorAction SilentlyContinue | Where-Object { $_ -and $_.Trim() -ne '' }) };" ^
    "$removed=0; foreach($n in $old){ if($names -notcontains $n){ $tp=Join-Path $target $n; if(Test-Path -LiteralPath $tp){ $it=Get-Item -LiteralPath $tp -Force -ErrorAction SilentlyContinue; if($it -and (($it.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)){ Remove-Item -LiteralPath $tp -Force -ErrorAction SilentlyContinue; $removed++ } } } };" ^
    "$linked=0; foreach($f in $src){ $tp=Join-Path $target $f.Name; if(Test-Path -LiteralPath $tp){ $it=Get-Item -LiteralPath $tp -Force -ErrorAction SilentlyContinue; if($it -and (($it.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)){ Remove-Item -LiteralPath $tp -Force -ErrorAction SilentlyContinue }; if(Test-Path -LiteralPath $tp){ continue } }; New-Item -ItemType SymbolicLink -Path $tp -Target $f.FullName -Force -ErrorAction SilentlyContinue | Out-Null; if(Test-Path -LiteralPath $tp){ $linked++ } };" ^
    "try{ Set-Content -LiteralPath $manifest -Value $names -Encoding UTF8 } catch { Write-Output ('[WARN] manifest 写入失败: ' + $manifest) };" ^
    "'INFO: linked=' + $linked + '; removed=' + $removed + '; target=' + $target"`) do echo %%L
)

echo [OK] 全量同步完成
call :WRITE_FINAL_REPORT "sync-all" "ok" "0" "0" "0" "0" "%OUT_DIR%"
exit /b 0

:WAIT_FOR_PROBE_SLOT
setlocal
set "_DIR=%~1"
set "_LAUNCHED=%~2"
set "_LIMIT=%~3"
if "%_LIMIT%"=="" set "_LIMIT=6"
for /f "delims=0123456789" %%I in ("%_LIMIT%") do set "_LIMIT=6"
if %_LIMIT% LSS 1 set "_LIMIT=1"
:WAIT_SLOT_LOOP
for /f %%C in ('dir /b /a-d "%_DIR%\*.meta" 2^>nul ^| find /c /v ""') do set "_DONE=%%C"
if "%_DONE%"=="" set "_DONE=0"
set /a _ACTIVE=%_LAUNCHED% - %_DONE%
if %_ACTIVE% GEQ %_LIMIT% (
  timeout /t 1 >nul
  goto :WAIT_SLOT_LOOP
)
endlocal & exit /b 0

:WAIT_FOR_PROBE_ALL
setlocal
set "_DIR=%~1"
set "_TOTAL=%~2"
:WAIT_ALL_LOOP
for /f %%C in ('dir /b /a-d "%_DIR%\*.meta" 2^>nul ^| find /c /v ""') do set "_DONE=%%C"
if "%_DONE%"=="" set "_DONE=0"
if not "%_DONE%"=="%_TOTAL%" (
  timeout /t 1 >nul
  goto :WAIT_ALL_LOOP
)
endlocal & exit /b 0

:PROBE_ONE_WORKER
setlocal EnableDelayedExpansion
set "FILE=%~2"
set "BASE=%~3"
set "PREFIX=%~4"
set "WHAM_PROXY_MODE=%~5"
set "WHAM_CONNECT_TIMEOUT=%~6"
set "WHAM_MAX_TIME=%~7"

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

if /I not "!TYPE!"=="codex" (
  >"!PREFIX!.meta" echo 0^|0^|0
  >"!PREFIX!.done" echo.
  exit /b 0
)
if "!TOKEN!"=="" (
  >"!PREFIX!.meta" echo 0^|0^|0
  >"!PREFIX!.done" echo.
  exit /b 0
)

set "STATUS="
set "WHAM_PROXY_ARG="
set "WHAM_BODY=!PREFIX!.wham"
if /I "!WHAM_PROXY_MODE!"=="direct" set "WHAM_PROXY_ARG=--noproxy *"
if "!AID!"=="" (
  for /f "usebackq delims=" %%S in (`curl -s !WHAM_PROXY_ARG! --connect-timeout !WHAM_CONNECT_TIMEOUT! --max-time !WHAM_MAX_TIME! -o "!WHAM_BODY!" -w "%%{http_code}" -H "Authorization: Bearer !TOKEN!" -H "Accept: application/json, text/plain, */*" -H "User-Agent: codex_cli_rs/0.76.0 (Windows 11; x86_64)" "https://chatgpt.com/backend-api/wham/usage" 2^>nul`) do set "STATUS=%%S"
) else (
  for /f "usebackq delims=" %%S in (`curl -s !WHAM_PROXY_ARG! --connect-timeout !WHAM_CONNECT_TIMEOUT! --max-time !WHAM_MAX_TIME! -o "!WHAM_BODY!" -w "%%{http_code}" -H "Authorization: Bearer !TOKEN!" -H "Chatgpt-Account-Id: !AID!" -H "Accept: application/json, text/plain, */*" -H "User-Agent: codex_cli_rs/0.76.0 (Windows 11; x86_64)" "https://chatgpt.com/backend-api/wham/usage" 2^>nul`) do set "STATUS=%%S"
)
if "!STATUS!"=="" set "STATUS=000"
echo !STATUS!| findstr /R "^[0-9][0-9][0-9]$" >nul 2>nul
if errorlevel 1 set "STATUS=000"
if "!STATUS!"=="000" (
  if "!AID!"=="" (
    for /f "usebackq delims=" %%S in (`curl -s --noproxy "*" --connect-timeout !WHAM_CONNECT_TIMEOUT! --max-time !WHAM_MAX_TIME! -o "!WHAM_BODY!" -w "%%{http_code}" -H "Authorization: Bearer !TOKEN!" -H "Accept: application/json, text/plain, */*" -H "User-Agent: codex_cli_rs/0.76.0 (Windows 11; x86_64)" "https://chatgpt.com/backend-api/wham/usage" 2^>nul`) do set "STATUS=%%S"
  ) else (
    for /f "usebackq delims=" %%S in (`curl -s --noproxy "*" --connect-timeout !WHAM_CONNECT_TIMEOUT! --max-time !WHAM_MAX_TIME! -o "!WHAM_BODY!" -w "%%{http_code}" -H "Authorization: Bearer !TOKEN!" -H "Chatgpt-Account-Id: !AID!" -H "Accept: application/json, text/plain, */*" -H "User-Agent: codex_cli_rs/0.76.0 (Windows 11; x86_64)" "https://chatgpt.com/backend-api/wham/usage" 2^>nul`) do set "STATUS=%%S"
  )
  if "!STATUS!"=="" set "STATUS=000"
  echo !STATUS!| findstr /R "^[0-9][0-9][0-9]$" >nul 2>nul
  if errorlevel 1 set "STATUS=000"
)

call :NORMALIZE_INVALID_STATUS "!STATUS!" "!WHAM_BODY!" STATUS

if "!STATUS!"=="000" (
  >"!PREFIX!.meta" echo 0^|1^|0
  >"!PREFIX!.net" echo !BASE!
  >"!PREFIX!.done" echo.
  exit /b 0
)

for /f "usebackq delims=" %%Z in (`powershell -NoProfile -Command "(Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')"`) do set "NOW=%%Z"
set "IDENT="
for /f "usebackq delims=" %%I in (`powershell -NoProfile -Command "if('%EMAIL%'){ $e='%EMAIL%'.Trim().ToLower(); if($e){'email:'+$e}else{''} } else { '' }"`) do set "IDENT=%%I"
if "!IDENT!"=="" set "IDENT=account_id:!AID!"
for /f "usebackq delims=" %%H in (`powershell -NoProfile -Command "$s='%IDENT%'; $sha=[System.Security.Cryptography.SHA256]::Create(); $b=[Text.Encoding]::UTF8.GetBytes($s); ($sha.ComputeHash($b) | ForEach-Object { $_.ToString('x2') }) -join ''"`) do set "EH=%%H"

>"!PREFIX!.rep" echo {"file_name":"!BASE!","email_hash":"!EH!","account_id":"!AID!","status_code":!STATUS!,"probed_at":"!NOW!"}
set "INV=0"
if "!STATUS!"=="401" set "INV=1"
if "!STATUS!"=="429" set "INV=1"
>"!PREFIX!.meta" echo 1^|0^|!INV!
>"!PREFIX!.done" echo.
exit /b 0

:NORMALIZE_INVALID_STATUS
setlocal EnableDelayedExpansion
set "_RAW=%~1"
set "_BODY=%~2"
set "_N=!_RAW!"
if "!_N!"=="" set "_N=000"
echo !_N!| findstr /R "^[0-9][0-9][0-9]$" >nul 2>nul
if errorlevel 1 set "_N=000"
if "!_N!"=="000" goto :NORMALIZE_END
if "!_N!"=="401" goto :NORMALIZE_END
if "!_N!"=="429" goto :NORMALIZE_END
if not "!_N:~0,1!"=="2" goto :NORMALIZE_END
set "Q0=0"
for /f "usebackq delims=" %%Q in (`powershell -NoProfile -Command "$q=0; try{$o=Get-Content -Raw -LiteralPath '%_BODY%'|ConvertFrom-Json; $rl=$o.rate_limit; if($null -ne $rl){ if($rl.allowed -eq $false){$q=1}; if($rl.limit_reached -eq $true){$q=1}; $up=[double]($rl.primary_window.used_percent); if($up -ge 100){$q=1} }; foreach($k in @('allowed','limit_reached','is_available')){ $v=$o.$k; if($v -eq $false -or $v -eq 0){$q=1} }} catch{}; Write-Output $q"`) do set "Q0=%%Q"
if "!Q0!"=="1" set "_N=429"

:NORMALIZE_END
endlocal & set "%~3=%_N%" & exit /b 0

:WRITE_FINAL_REPORT
setlocal
set "R_MODE=%~1"
set "R_STATUS=%~2"
set "R_TOTAL=%~3"
set "R_PROBED=%~4"
set "R_NET=%~5"
set "R_INVALID=%~6"
set "R_OUT=%~7"
if not exist "%SCRIPT_DIR%out" mkdir "%SCRIPT_DIR%out" >nul 2>nul
powershell -NoProfile -Command "$o=[ordered]@{ generated_at=(Get-Date).ToString('yyyy-MM-ddTHH:mm:ssK'); mode='%R_MODE%'; status='%R_STATUS%'; total=[int]('%R_TOTAL%'); probed_ok=[int]('%R_PROBED%'); net_fail=[int]('%R_NET%'); invalid_401_429=[int]('%R_INVALID%'); out_dir='%R_OUT%'; final_report='%FINAL_REPORT%' }; $o|ConvertTo-Json -Depth 4 | Set-Content -LiteralPath '%FINAL_REPORT%' -Encoding UTF8" >nul 2>nul
endlocal & exit /b 0
