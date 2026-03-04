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
REM     {"ok":true,"accounts":[{"file_name":"codex-<account_id>.json","download_url":"https://..."}], ...}

set "SCRIPT_DIR=%~dp0"
REM 自动检测：全平台版本（在 windows/ 子目录）vs 分平台版本（在根目录）
set "ROOT_DIR=%SCRIPT_DIR%"
for %%I in ("%SCRIPT_DIR:~0,-1%") do (
  if /I "%%~nxI"=="windows" set "ROOT_DIR=%SCRIPT_DIR%..\"
)
for %%I in ("%ROOT_DIR%.") do set "ROOT_DIR=%%~fI\"
set "CFG_ENV=%ROOT_DIR%无限续杯配置.env"

REM 某些调用链下 %0 可能被错误覆盖为参数（如 --sync-all），这里强制回退到同目录脚本路径
set "SELF_BAT=%~f0"
if /I "%~nx0"=="--sync-all" set "SELF_BAT=%SCRIPT_DIR%单次续杯.bat"
if /I "%~nx0"=="--from-task" set "SELF_BAT=%SCRIPT_DIR%单次续杯.bat"
if not exist "%SELF_BAT%" set "SELF_BAT=%SCRIPT_DIR%单次续杯.bat"

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
  goto :ARGS_DONE
)
:ARGS_DONE

set "SERVER_URL="
set "USER_KEY="
set "ACCOUNTS_DIR=%ROOT_DIR%accounts"
set "TARGET_POOL_SIZE=10"
set "TOTAL_HOLD_LIMIT=50"
set "SYNC_TARGET_DIR="
set "WHAM_PROXY_MODE=auto"
set "WHAM_CONNECT_TIMEOUT=5"
set "WHAM_MAX_TIME=15"
set "TOPUP_CONNECT_TIMEOUT=10"
set "TOPUP_MAX_TIME=180"
set "TOPUP_RETRY=3"
set "TOPUP_RETRY_DELAY=3"
set "RUN_OUTPUT_MODE=compact"
set "PROBE_PARALLEL=6"
set "FINAL_REPORT=%ROOT_DIR%out\最终续杯报告.json"
set "REPORT_REPLAY_PERCENT="
set "REPORT_ISSUED_REPLAY="
set "REPORT_AUTO_DISABLED="
set "REPORT_ABUSE_AUTO_BANNED="
set "REFILL_ITER_MAX=6"
set "REFILL_ITER=0"

if exist "%CFG_ENV%" (
  for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%CFG_ENV%") do (
    if /I "%%A"=="SERVER_URL" set "SERVER_URL=%%B"
    if /I "%%A"=="USER_KEY" set "USER_KEY=%%B"
    if /I "%%A"=="ACCOUNTS_DIR" set "ACCOUNTS_DIR=%%B"
    if /I "%%A"=="TARGET_POOL_SIZE" set "TARGET_POOL_SIZE=%%B"
    if /I "%%A"=="TOTAL_HOLD_LIMIT" set "TOTAL_HOLD_LIMIT=%%B"
    if /I "%%A"=="SYNC_TARGET_DIR" set "SYNC_TARGET_DIR=%%B"
    if /I "%%A"=="WHAM_PROXY_MODE" set "WHAM_PROXY_MODE=%%B"
    if /I "%%A"=="WHAM_CONNECT_TIMEOUT" set "WHAM_CONNECT_TIMEOUT=%%B"
    if /I "%%A"=="WHAM_MAX_TIME" set "WHAM_MAX_TIME=%%B"
    if /I "%%A"=="TOPUP_CONNECT_TIMEOUT" set "TOPUP_CONNECT_TIMEOUT=%%B"
    if /I "%%A"=="TOPUP_MAX_TIME" set "TOPUP_MAX_TIME=%%B"
    if /I "%%A"=="TOPUP_RETRY" set "TOPUP_RETRY=%%B"
    if /I "%%A"=="TOPUP_RETRY_DELAY" set "TOPUP_RETRY_DELAY=%%B"
    if /I "%%A"=="RUN_OUTPUT_MODE" set "RUN_OUTPUT_MODE=%%B"
    if /I "%%A"=="PROBE_PARALLEL" set "PROBE_PARALLEL=%%B"
    if /I "%%A"=="REFILL_ITER_MAX" set "REFILL_ITER_MAX=%%B"
  )
)
REM 已经统一使用 CFG_ENV 读取配置，无需重复读取

REM 允许命令行覆盖
if not "%~1"=="" set "SERVER_URL=%~1"
if not "%~2"=="" set "USER_KEY=%~2"

if "%SERVER_URL%"=="" (
  echo [ERROR] 未配置 SERVER_URL。请先运行“无限续杯”设置配置：
  echo         "%SCRIPT_DIR%无限续杯.bat"
  set "_MAIN_EC=2"
  goto :EXIT_MAIN
)
if "%USER_KEY%"=="" (
  echo [ERROR] 未配置 USER_KEY。
  set "_MAIN_EC=2"
  goto :EXIT_MAIN
)

if "%ACCOUNTS_DIR%"=="" set "ACCOUNTS_DIR=%ROOT_DIR%accounts"
if not exist "%ACCOUNTS_DIR%" mkdir "%ACCOUNTS_DIR%" >nul 2>nul
if not exist "%ACCOUNTS_DIR%" (
  echo [ERROR] 账户目录不存在且创建失败："%ACCOUNTS_DIR%"
  set "_MAIN_EC=3"
  goto :EXIT_MAIN
)

where powershell >nul 2>nul
if errorlevel 1 (
  echo [ERROR] 当前系统缺少 PowerShell，无法运行客户端。
  echo [ERROR] 请在 Windows 功能中启用 PowerShell 后重试。
  set "_MAIN_EC=6"
  goto :EXIT_MAIN
)

set "HAS_CURL=1"
where curl >nul 2>nul
if errorlevel 1 (
  set "HAS_CURL=0"
  echo [WARN] 未检测到 curl，已自动回退到 PowerShell 网络请求模式。
)

echo %TOPUP_CONNECT_TIMEOUT%| findstr /R "^[0-9][0-9]*$" >nul 2>nul
if errorlevel 1 set "TOPUP_CONNECT_TIMEOUT=10"
if %TOPUP_CONNECT_TIMEOUT% LSS 1 set "TOPUP_CONNECT_TIMEOUT=10"
echo %TOPUP_MAX_TIME%| findstr /R "^[0-9][0-9]*$" >nul 2>nul
if errorlevel 1 set "TOPUP_MAX_TIME=180"
if %TOPUP_MAX_TIME% LSS 30 set "TOPUP_MAX_TIME=180"
echo %TOPUP_RETRY%| findstr /R "^[0-9][0-9]*$" >nul 2>nul
if errorlevel 1 set "TOPUP_RETRY=3"
if %TOPUP_RETRY% LSS 0 set "TOPUP_RETRY=3"
echo %TOPUP_RETRY_DELAY%| findstr /R "^[0-9][0-9]*$" >nul 2>nul
if errorlevel 1 set "TOPUP_RETRY_DELAY=3"
if %TOPUP_RETRY_DELAY% LSS 1 set "TOPUP_RETRY_DELAY=3"
echo %REFILL_ITER_MAX%| findstr /R "^[0-9][0-9]*$" >nul 2>nul
if errorlevel 1 set "REFILL_ITER_MAX=6"
if %REFILL_ITER_MAX% LSS 1 set "REFILL_ITER_MAX=1"
if %REFILL_ITER_MAX% GTR 20 set "REFILL_ITER_MAX=20"

REM 注意：不要清空全局代理环境变量。
REM - 探测 OpenAI(wham) 可能依赖代理；
REM - 仅对本服务端请求使用 --noproxy "*" 强制直连，避免命中失效本地代理。

if "%MODE_FROM_TASK%"=="0" echo [INFO] 服务器地址=%SERVER_URL%
if "%MODE_FROM_TASK%"=="0" echo [INFO] accounts-dir=%ACCOUNTS_DIR%
if "%MODE_FROM_TASK%"=="0" echo [INFO] 目标账户数=%TARGET_POOL_SIZE% 总持有上限=%TOTAL_HOLD_LIMIT% 触发规则=存在失效账号即续杯
if "%MODE_FROM_TASK%"=="0" echo [DIAG] script=%SELF_BAT%
if "%MODE_FROM_TASK%"=="0" echo [DIAG] script_dir=%SCRIPT_DIR%
if "%MODE_FROM_TASK%"=="0" if not exist "%SELF_BAT%" echo [DIAG] script_missing=%SELF_BAT%

if "%MODE_SYNC_ALL%"=="1" goto :SYNC_ALL_PREP

:TOPUP_FLOW_BEGIN
set /a REFILL_ITER+=1
if "%MODE_FROM_TASK%"=="0" echo [INFO] 续杯闭环轮次：%REFILL_ITER%/%REFILL_ITER_MAX%

REM 输出目录
for /f "usebackq delims=" %%T in (`powershell -NoProfile -Command "Get-Date -Format 'yyyyMMdd-HHmmss'"`) do set "TS=%%T"
set "OUT_DIR=%ROOT_DIR%out\单次续杯-%TS%"
set "REPORT_JSONL=%OUT_DIR%\reports.jsonl"
set "RESP_JSON=%OUT_DIR%\topup_response.json"
set "BODY_JSON=%OUT_DIR%\topup_body.json"
set "BACKUP_DIR=%OUT_DIR%\backup"
set "NETFAIL_LOG=%OUT_DIR%\probe_netfail.log"
set "TOPUP_CURL_ERR=%OUT_DIR%\topup_curl_stderr.log"
set "RESPONSE_SCOPE_FILE=%OUT_DIR%\response_scope.txt"
set "TOPUP_WRITE_FAIL_LOG=%OUT_DIR%\topup_write_failures.log"

if /I "%RUN_OUTPUT_MODE%"=="compact" (
  set "OUT_DIR=%ROOT_DIR%out\latest"
  set "REPORT_JSONL=!OUT_DIR!\reports.jsonl"
  set "RESP_JSON=!OUT_DIR!\topup_response.json"
  set "BODY_JSON=!OUT_DIR!\topup_body.json"
  set "BACKUP_DIR=!OUT_DIR!\backup"
  set "NETFAIL_LOG=!OUT_DIR!\probe_netfail.log"
  set "TOPUP_CURL_ERR=!OUT_DIR!\topup_curl_stderr.log"
  set "RESPONSE_SCOPE_FILE=!OUT_DIR!\response_scope.txt"
  set "TOPUP_WRITE_FAIL_LOG=!OUT_DIR!\topup_write_failures.log"
  if exist "!OUT_DIR!" rmdir /s /q "!OUT_DIR!" >nul 2>nul
)

if not exist "%ROOT_DIR%out" mkdir "%ROOT_DIR%out" >nul 2>nul
if not exist "%OUT_DIR%" mkdir "%OUT_DIR%" >nul 2>nul
set "REPLAY_QUEUE_FILE=%ROOT_DIR%out\replay_feedback_queue.txt"
set "ITER_SCOPE_FILE=%ROOT_DIR%out\refill_iter_scope.txt"
set "NEXT_SCOPE_FILE=%ROOT_DIR%out\refill_iter_scope_next.txt"
if not exist "%REPLAY_QUEUE_FILE%" type nul >"%REPLAY_QUEUE_FILE%"
if %REFILL_ITER% EQU 1 (
  if exist "%ITER_SCOPE_FILE%" del /q "%ITER_SCOPE_FILE%" >nul 2>nul
  if exist "%NEXT_SCOPE_FILE%" del /q "%NEXT_SCOPE_FILE%" >nul 2>nul
)

>"%REPORT_JSONL%" echo.
>"%NETFAIL_LOG%" echo.

REM 仅管理 accounts-dir 下所有 .json 文件（并行探测）
set /a TOTAL=0
set /a PROBED_OK=0
set /a NET_FAIL=0
set /a INVALID=0
set /a INVALID_401=0
set /a INVALID_429=0
set "PROBE_DIR=%OUT_DIR%\probe_jobs"
if exist "%PROBE_DIR%" rmdir /s /q "%PROBE_DIR%" >nul 2>nul
mkdir "%PROBE_DIR%" >nul 2>nul

set /a LAUNCHED=0
set "PROBE_USE_SCOPE=0"
set "SCOPE_COUNT=0"
set /a SCOPE_READ_COUNT=0
set /a SCOPE_SKIP_MISSING=0
if %REFILL_ITER% GTR 1 (
  for /f "usebackq delims=" %%C in (`powershell -NoProfile -Command "$q='%REPLAY_QUEUE_FILE%'; $acc='%ACCOUNTS_DIR%'; $scope='%ITER_SCOPE_FILE%'; $list=New-Object 'System.Collections.Generic.List[string]'; if(Test-Path -LiteralPath $scope){ foreach($ln in (Get-Content -LiteralPath $scope -ErrorAction SilentlyContinue)){ $name=(''+$ln).Trim(); if(-not $name){ continue }; if(Test-Path -LiteralPath (Join-Path $acc $name)){ $list.Add($name) | Out-Null } } }; try{ if(Test-Path -LiteralPath $q){ foreach($ln in (Get-Content -LiteralPath $q -ErrorAction SilentlyContinue)){ $name=(''+$ln).Trim(); if(-not $name){ continue }; if(Test-Path -LiteralPath (Join-Path $acc $name)){ $list.Add($name) | Out-Null } } } }catch{}; $uniq=@($list | Select-Object -Unique); if($uniq.Count -gt 0){ Set-Content -LiteralPath $scope -Value $uniq -Encoding UTF8; Write-Output $uniq.Count } else { if(Test-Path -LiteralPath $scope){ Remove-Item -LiteralPath $scope -Force -ErrorAction SilentlyContinue }; Write-Output 0 }"`) do set "SCOPE_COUNT=%%C"
  if "!SCOPE_COUNT!"=="" set "SCOPE_COUNT=0"
  if !SCOPE_COUNT! GTR 0 set "PROBE_USE_SCOPE=1"
)
if "%MODE_FROM_TASK%"=="0" (
  if "%PROBE_USE_SCOPE%"=="1" (
    if %REFILL_ITER% GTR 1 (
      echo [INFO] 开始探测账号状态（增量模式，仅本轮新增/待确认；并行=%PROBE_PARALLEL%）...
    ) else (
      echo [INFO] 开始探测账号状态（增量模式，仅本轮新增；并行=%PROBE_PARALLEL%）...
    )
  ) else (
    if %REFILL_ITER% GTR 1 (
      echo [INFO] 后续轮次无可探测增量，准备结束闭环。
    ) else (
      echo [INFO] 开始探测账号状态（全量模式；并行=%PROBE_PARALLEL%）...
    )
  )
)
if "%PROBE_USE_SCOPE%"=="1" (
  if "%MODE_FROM_TASK%"=="0" (
    copy /Y "%ITER_SCOPE_FILE%" "%OUT_DIR%\iter_scope_before_probe.txt" >nul 2>nul
    for /f "usebackq delims=" %%L in (`powershell -NoProfile -Command "$p='%ITER_SCOPE_FILE%'; if(-not (Test-Path -LiteralPath $p)){ Write-Output '[DIAG] iter_scope.file_missing'; exit 0 }; $arr=@(Get-Content -LiteralPath $p -ErrorAction SilentlyContinue | ForEach-Object{ $s=(''+$_).Trim(); $s=$s.TrimStart([char]0xFEFF,[char]0xEF,[char]0xBB,[char]0xBF); if($s.StartsWith('ï»¿')){ $s=$s.Substring(3) }; $s } | Where-Object{ $_ -ne '' }); Write-Output ('[DIAG] iter_scope.lines=' + $arr.Count); $i=0; foreach($n in $arr){ if($i -ge 20){ break }; $i++; Write-Output ('[DIAG] iter_scope.item['+$i+'] ' + $n) }"`) do echo %%L
  )
  for /f "usebackq delims=" %%F in (`powershell -NoProfile -Command "$p='%ITER_SCOPE_FILE%'; if(Test-Path -LiteralPath $p){ Get-Content -LiteralPath $p -ErrorAction SilentlyContinue | ForEach-Object { $s=(''+$_).Trim(); $s=$s.TrimStart([char]0xFEFF,[char]0xEF,[char]0xBB,[char]0xBF); if($s.StartsWith('ï»¿')){ $s=$s.Substring(3) }; $s } | Where-Object { $_ -ne '' } }"`) do (
    if not "%%F"=="" (
      set /a SCOPE_READ_COUNT+=1
      if exist "%ACCOUNTS_DIR%\%%F" (
        set /a TOTAL+=1
        set /a LAUNCHED+=1
        if "%MODE_FROM_TASK%"=="0" echo [PROBE] 启动 !TOTAL!: %%F
        if "%MODE_FROM_TASK%"=="0" echo [DIAG] probe_cmd=call "%SELF_BAT%" --probe-one-worker "%ACCOUNTS_DIR%\%%F" "%%F" "%PROBE_DIR%\!TOTAL!" "%WHAM_PROXY_MODE%" "%WHAM_CONNECT_TIMEOUT%" "%WHAM_MAX_TIME%"
        start "" /b "%ComSpec%" /v:off /c call "%SELF_BAT%" --probe-one-worker "%ACCOUNTS_DIR%\%%F" "%%F" "%PROBE_DIR%\!TOTAL!" "%WHAM_PROXY_MODE%" "%WHAM_CONNECT_TIMEOUT%" "%WHAM_MAX_TIME%" ^>nul 2^>nul
        call :WAIT_FOR_PROBE_SLOT "%PROBE_DIR%" "!LAUNCHED!" "%PROBE_PARALLEL%"
      ) else (
        set /a SCOPE_SKIP_MISSING+=1
        if "%MODE_FROM_TASK%"=="0" echo [DIAG] iter_scope.skip_missing %%F
      )
    )
  )
  if "%MODE_FROM_TASK%"=="0" echo [DIAG] iter_scope.launch_summary read=!SCOPE_READ_COUNT! launched=!LAUNCHED! skip_missing=!SCOPE_SKIP_MISSING!
) else (
  if %REFILL_ITER% LEQ 1 (
    for /f "usebackq delims=" %%F in (`dir /b /a-d "%ACCOUNTS_DIR%\*.json" 2^>nul`) do (
      set /a TOTAL+=1
      set /a LAUNCHED+=1
      if "%MODE_FROM_TASK%"=="0" echo [PROBE] 启动 !TOTAL!: %%F
      if "%MODE_FROM_TASK%"=="0" echo [DIAG] probe_cmd=call "%SELF_BAT%" --probe-one-worker "%ACCOUNTS_DIR%\%%F" "%%F" "%PROBE_DIR%\!TOTAL!" "%WHAM_PROXY_MODE%" "%WHAM_CONNECT_TIMEOUT%" "%WHAM_MAX_TIME%"
      start "" /b "%ComSpec%" /v:off /c call "%SELF_BAT%" --probe-one-worker "%ACCOUNTS_DIR%\%%F" "%%F" "%PROBE_DIR%\!TOTAL!" "%WHAM_PROXY_MODE%" "%WHAM_CONNECT_TIMEOUT%" "%WHAM_MAX_TIME%" ^>nul 2^>nul
      call :WAIT_FOR_PROBE_SLOT "%PROBE_DIR%" "!LAUNCHED!" "%PROBE_PARALLEL%"
    )
  )
)

if %LAUNCHED% GTR 0 (
  call :WAIT_FOR_PROBE_ALL "%PROBE_DIR%" "%LAUNCHED%"

  >"%REPORT_JSONL%" (
    for %%R in ("%PROBE_DIR%\*.rep") do if exist "%%~fR" type "%%~fR"
  )

  >"%NETFAIL_LOG%" (
    for %%N in ("%PROBE_DIR%\*.net") do if exist "%%~fN" type "%%~fN"
  )

  set /a PROBED_OK=0
  set /a NET_FAIL=0
  set /a INVALID=0
  set /a INVALID_401=0
  set /a INVALID_429=0

  for %%M in ("%PROBE_DIR%\*.meta") do if exist "%%~fM" (
    for /f "tokens=1,2,3 delims=|" %%a in ('type "%%~fM"') do (
      set /a PROBED_OK+=%%a
      set /a NET_FAIL+=%%b
      set /a INVALID+=%%c
    )
  )

  for %%R in ("%PROBE_DIR%\*.rep") do if exist "%%~fR" (
    findstr /C:"\"status_code\":401" "%%~fR" >nul 2>nul
    if not errorlevel 1 set /a INVALID_401+=1
    findstr /C:"\"status_code\":429" "%%~fR" >nul 2>nul
    if not errorlevel 1 set /a INVALID_429+=1
  )

  set /a INVALID=!INVALID_401! + !INVALID_429!
)

set /a AVAILABLE_EST=%TOTAL% - %INVALID%

set "REPLAY_PENDING_COUNT=0"
for /f "usebackq delims=" %%C in (`powershell -NoProfile -Command "$q='%REPLAY_QUEUE_FILE%'; $acc='%ACCOUNTS_DIR%'; $list=New-Object 'System.Collections.Generic.List[string]'; try{ if(Test-Path -LiteralPath $q){ foreach($ln in (Get-Content -LiteralPath $q -ErrorAction SilentlyContinue)){ $name=(''+$ln).Trim(); if(-not $name){ continue }; if(Test-Path -LiteralPath (Join-Path $acc $name)){ $list.Add($name) | Out-Null } } } }catch{}; $uniq=@($list | Select-Object -Unique); if($uniq.Count -gt 0){ Set-Content -LiteralPath $q -Value $uniq -Encoding UTF8; Write-Output $uniq.Count } else { if(Test-Path -LiteralPath $q){ Set-Content -LiteralPath $q -Value @() -Encoding UTF8 }; Write-Output 0 }"`) do set "REPLAY_PENDING_COUNT=%%C"
if "%REPLAY_PENDING_COUNT%"=="" set "REPLAY_PENDING_COUNT=0"

REM 计算 HOLD_LIMIT（用 findstr 安全验证数字，避免路径报错）
set /a HOLD_LIMIT=0
echo %TOTAL_HOLD_LIMIT%| findstr /R "^[0-9][0-9]*$" >nul 2>nul
if not errorlevel 1 ( set /a HOLD_LIMIT=%TOTAL_HOLD_LIMIT% ) else ( set /a HOLD_LIMIT=50 )
if %HOLD_LIMIT% LSS 1 set /a HOLD_LIMIT=50

REM REQUEST_TARGET = hold_limit - available_est（精确补差，不超发）
set /a REQUEST_TARGET=%HOLD_LIMIT% - %AVAILABLE_EST%
if %REQUEST_TARGET% LSS 0 set /a REQUEST_TARGET=0

echo.
echo [INFO] 统计：total=%TOTAL% available_est=%AVAILABLE_EST% probed_ok=%PROBED_OK% net_fail=%NET_FAIL% invalid_401=%INVALID_401% invalid_429=%INVALID_429% invalid(401/429)=%INVALID% replay_pending=%REPLAY_PENDING_COUNT% hold_limit=%HOLD_LIMIT% request_target=%REQUEST_TARGET%
if %REFILL_ITER% GTR 1 if "%PROBE_USE_SCOPE%"=="1" if %INVALID% LEQ 0 if %REPLAY_PENDING_COUNT% LEQ 0 (
  echo [OK] 增量范围内账号均可用且无回放待确认，闭环结束。
  if exist "%ITER_SCOPE_FILE%" del /q "%ITER_SCOPE_FILE%" >nul 2>nul
  call :WRITE_FINAL_REPORT "topup" "incremental_all_healthy" "%TOTAL%" "%PROBED_OK%" "%NET_FAIL%" "%INVALID%" "%OUT_DIR%"
  set "_MAIN_EC=0"
  goto :EXIT_MAIN
)
if %REFILL_ITER% GTR 1 if "%PROBE_USE_SCOPE%"=="0" (
  if %REPLAY_PENDING_COUNT% LEQ 0 (
    echo [OK] 增量范围为空：本轮无需继续探测，闭环结束。
    call :WRITE_FINAL_REPORT "topup" "incremental_done" "0" "0" "0" "0" "%OUT_DIR%"
    set "_MAIN_EC=0"
    goto :EXIT_MAIN
  )
)
if %TOTAL% EQU 0 (
  if %REFILL_ITER% GTR 1 (
    if %REPLAY_PENDING_COUNT% GTR 0 (
      echo [WARN] 后续轮次仍有 replay_pending=%REPLAY_PENDING_COUNT%，但未命中本地文件；结束闭环并保留队列待下次处理。
      call :WRITE_FINAL_REPORT "topup" "incremental_pending_without_local" "0" "0" "0" "0" "%OUT_DIR%"
      set "_MAIN_EC=0"
      goto :EXIT_MAIN
    )
    echo [WARN] 后续轮次未命中可探测文件，跳过本轮且不触发 sync-all。
    call :WRITE_FINAL_REPORT "topup" "incremental_no_probe" "0" "0" "0" "0" "%OUT_DIR%"
    set "_MAIN_EC=0"
    goto :EXIT_MAIN
  )
  echo [WARN] accounts-dir 下未发现 .json 文件：%ACCOUNTS_DIR%
  echo [INFO] 本地账号为0，自动切换为全量同步。
  goto :SYNC_ALL_PREP
)

REM 触发规则：
REM 1) 只要发现失效账号(401/429)就触发续杯；
REM 2) 若总量不足目标池，也触发（bootstrap）；
REM 3) 若总量低于总持有上限，也触发补齐。
set "NEED_TRIGGER=0"
if %INVALID% GTR 0 set "NEED_TRIGGER=1"
if %INVALID_401% GTR 0 set "NEED_TRIGGER=1"
if %INVALID_429% GTR 0 set "NEED_TRIGGER=1"
if %TOTAL% LSS %TARGET_POOL_SIZE% set "NEED_TRIGGER=1"
if %HOLD_LIMIT% GTR 0 if %AVAILABLE_EST% LSS %HOLD_LIMIT% set "NEED_TRIGGER=1"
if %REPLAY_PENDING_COUNT% GTR 0 set "NEED_TRIGGER=1"

if "%NEED_TRIGGER%"=="0" (
  echo [OK] 未达到续杯条件：无需 topup
  call :WRITE_FINAL_REPORT "topup" "not_triggered" "%TOTAL%" "%PROBED_OK%" "%NET_FAIL%" "%INVALID%" "%OUT_DIR%"
  set "_MAIN_EC=0"
  goto :EXIT_MAIN
)

REM 持有量已达上限且无 replay 待确认时提前退出
if %REQUEST_TARGET% EQU 0 if %REPLAY_PENDING_COUNT% EQU 0 (
  echo [OK] 无需续杯：持有量已达上限且无回放待确认（available_est=%AVAILABLE_EST% hold_limit=%HOLD_LIMIT%）
  call :WRITE_FINAL_REPORT "topup" "at_limit" "%TOTAL%" "%PROBED_OK%" "%NET_FAIL%" "%INVALID%" "%OUT_DIR%"
  set "_MAIN_EC=0"
  goto :EXIT_MAIN
)

REM 构造 topup body：读取 jsonl 为数组（增强容错+错误日志）
powershell -NoProfile -Command ^
  "$probe='%PROBE_DIR%'; $bodyPath='%BODY_JSON%'; $target=[int]('%REQUEST_TARGET%'); $accDir='%ACCOUNTS_DIR%'; $items=@(); if(Test-Path -LiteralPath $probe){ Get-ChildItem -LiteralPath $probe -Filter '*.rep' -File -ErrorAction SilentlyContinue | Sort-Object Name | ForEach-Object { try { $txt=Get-Content -Raw -LiteralPath $_.FullName -ErrorAction Stop; if($txt -and $txt.Trim()){ $o=($txt | ConvertFrom-Json); $sc=[int]($o.status_code); $replay=($o.replay_from_confidence -eq $true); if($sc -eq 401 -or $sc -eq 429 -or $replay){ $items += $o } } } catch {} } }; $ids=@(); if(Test-Path -LiteralPath $accDir){ Get-ChildItem -LiteralPath $accDir -Filter 'codex-*.json' -File -ErrorAction SilentlyContinue | ForEach-Object { if($_.BaseName -match '^codex-(.+)$'){ $id=(''+$Matches[1]).Trim(); if($id){ $ids += $id } } } }; if($ids.Count -gt 0){ $ids=@($ids | Select-Object -Unique | Select-Object -First 500) }; $body=[ordered]@{target_pool_size=$target; reports=$items; account_ids=$ids}; $dir=Split-Path -Parent $bodyPath; if($dir -and -not (Test-Path -LiteralPath $dir)){ New-Item -ItemType Directory -Path $dir -Force | Out-Null }; $json=($body | ConvertTo-Json -Depth 6); $utf8NoBom=New-Object System.Text.UTF8Encoding($false); [System.IO.File]::WriteAllText($bodyPath,$json,$utf8NoBom)" 1>"%OUT_DIR%\topup_body_build.log" 2>"%OUT_DIR%\topup_body_error.log"
if not exist "%BODY_JSON%" (
  echo [WARN] topup body 生成失败，尝试使用兜底 body：%BODY_JSON%
  if exist "%OUT_DIR%\topup_body_error.log" type "%OUT_DIR%\topup_body_error.log"
  >"%BODY_JSON%" echo {"target_pool_size":%REQUEST_TARGET%,"reports":[]}
)
if exist "%TOPUP_WRITE_FAIL_LOG%" del /q "%TOPUP_WRITE_FAIL_LOG%" >nul 2>nul
if not exist "%BODY_JSON%" (
  echo [ERROR] 兜底 body 仍生成失败：%BODY_JSON%
  set "_MAIN_EC=2"
  goto :EXIT_MAIN
)

for /f "usebackq tokens=1,2 delims=|" %%a in (`powershell -NoProfile -Command "$n=0; $bad=0; try{$o=Get-Content -Raw -LiteralPath '%BODY_JSON%'|ConvertFrom-Json; $arr=@($o.reports); $n=$arr.Count; foreach($r in $arr){ $s=[int]$r.status_code; if($s -eq 401 -or $s -eq 429){$bad++} }}catch{}; Write-Output ($n.ToString() + '|' + $bad.ToString())"`) do (
  set "BODY_REPORTS=%%a"
  set "BODY_INVALID=%%b"
)
if "%BODY_REPORTS%"=="" set "BODY_REPORTS=0"
if "%BODY_INVALID%"=="" set "BODY_INVALID=0"
echo [INFO] 上报报告条数：%BODY_REPORTS%（失效401/429=%BODY_INVALID%）

echo [INFO] 触发 topup：POST %SERVER_URL%/v1/refill/topup
if "%HAS_CURL%"=="1" (
  if exist "%TOPUP_CURL_ERR%" del /q "%TOPUP_CURL_ERR%" >nul 2>nul
  curl -sS --connect-timeout %TOPUP_CONNECT_TIMEOUT% --max-time %TOPUP_MAX_TIME% --retry %TOPUP_RETRY% --retry-all-errors --retry-delay %TOPUP_RETRY_DELAY% --noproxy "*" -X POST "%SERVER_URL%/v1/refill/topup" ^
    -H "X-User-Key: %USER_KEY%" ^
    -H "Content-Type: application/json" ^
    --data-binary "@%BODY_JSON%" >"%RESP_JSON%" 2>"%TOPUP_CURL_ERR%"
  set "CURL_EC=!ERRORLEVEL!"
) else (
  powershell -NoProfile -Command ^
    "$ErrorActionPreference='Stop'; $uri='%SERVER_URL%/v1/refill/topup'; $headers=@{'X-User-Key'='%USER_KEY%'}; $body=[System.IO.File]::ReadAllText('%BODY_JSON%',[System.Text.Encoding]::UTF8); try{ $r=Invoke-WebRequest -UseBasicParsing -Method POST -Uri $uri -Headers $headers -ContentType 'application/json' -Body $body -TimeoutSec %TOPUP_MAX_TIME%; [System.IO.File]::WriteAllText('%RESP_JSON%', [string]$r.Content, [System.Text.Encoding]::UTF8); exit 0 } catch { $code=1; try{ $resp=$_.Exception.Response; if($resp){ $sr=New-Object System.IO.StreamReader($resp.GetResponseStream()); $txt=$sr.ReadToEnd(); [System.IO.File]::WriteAllText('%RESP_JSON%', [string]$txt, [System.Text.Encoding]::UTF8) } } catch {}; exit $code }"
  set "CURL_EC=!ERRORLEVEL!"
)
if not "%CURL_EC%"=="0" (
  echo [ERROR] topup 请求失败（网络异常或服务端超时）
  if exist "%TOPUP_CURL_ERR%" type "%TOPUP_CURL_ERR%"
  set "_MAIN_EC=2"
  goto :EXIT_MAIN
)
if not exist "%RESP_JSON%" (
  echo [ERROR] topup 响应文件缺失：%RESP_JSON%
  set "_MAIN_EC=2"
  goto :EXIT_MAIN
)
for %%S in ("%RESP_JSON%") do if %%~zS LSS 2 (
  echo [ERROR] topup 响应为空：%RESP_JSON%
  set "_MAIN_EC=2"
  goto :EXIT_MAIN
)

REM 统一清洗 topup 响应中的 file_name/download_url/account_id 两端控制字符/BOM/EOF()
powershell -NoProfile -Command "$ErrorActionPreference='SilentlyContinue'; $p='%RESP_JSON%'; try{ $r=Get-Content -Raw -LiteralPath $p | ConvertFrom-Json }catch{ exit 0 }; $changed=0; foreach($a in @($r.accounts)){ foreach($k in @('file_name','download_url','account_id')){ $v=(''+$a.$k); if($null -eq $a.$k){ continue }; $n=$v.Trim(); $n=$n.TrimStart([char]0xFEFF,[char]0xEF,[char]0xBB,[char]0xBF); if($n.StartsWith('ï»¿')){ $n=$n.Substring(3) }; $n=($n -replace '^[\u0000-\u001F\u007F\uFEFF]+','' -replace '[\u0000-\u001F\u007F\uFEFF]+$',''); if($n -ne $v){ $a.$k=$n; $changed++ } } }; if($changed -gt 0){ $json=($r | ConvertTo-Json -Depth 20); $utf8NoBom=New-Object System.Text.UTF8Encoding($false); [System.IO.File]::WriteAllText($p,$json,$utf8NoBom); Write-Output ('[INFO] topup_response 清洗字段次数=' + $changed) }"

for /f "usebackq delims=" %%L in (`powershell -NoProfile -Command "$ErrorActionPreference='SilentlyContinue'; try{$r=Get-Content -Raw -LiteralPath '%RESP_JSON%'|ConvertFrom-Json; $i=0; foreach($a in @($r.accounts)){ if($i -ge 5){ break }; $i++; $n=(''+$a.file_name); if(-not $n){$n='(null)'}; Write-Output ('[DIAG] server account['+$i+'] file_name=' + $n) }}catch{}"`) do echo %%L

set "SERVER_ACCOUNTS_COUNT="
for /f "usebackq delims=" %%C in (`powershell -NoProfile -Command "$n=0; try{$r=Get-Content -Raw -LiteralPath '%RESP_JSON%'|ConvertFrom-Json; $n=@($r.accounts).Count}catch{}; Write-Output $n"`) do set "SERVER_ACCOUNTS_COUNT=%%C"
if "%SERVER_ACCOUNTS_COUNT%"=="" set "SERVER_ACCOUNTS_COUNT=0"
echo [INFO] 服务端返回账号条数：%SERVER_ACCOUNTS_COUNT%

REM 解析并写入 accounts（兼容 auth_json / download_url），并把 replay 标记写入 out 队列文件
set "WRITTEN_COUNT="
set "SERVER_HOLD_LIMIT="
set "PARSE_FAILED=0"
set "PARSE_ERR_MSG="
for /f "usebackq tokens=1,* delims==" %%A in (`powershell -NoProfile -Command "$ErrorActionPreference='Stop'; $ProgressPreference='SilentlyContinue'; $utf8NoBom=New-Object System.Text.UTF8Encoding($false); $queuePath='%ROOT_DIR%out\replay_feedback_queue.txt'; $replayNames=New-Object 'System.Collections.Generic.List[string]'; try{$r=Get-Content -Raw -LiteralPath '%RESP_JSON%'|ConvertFrom-Json}catch{Write-Output 'ERROR=bad response json'; exit 2}; if(-not $r.ok){Write-Output ('ERROR=' + ($r.error|Out-String)); exit 2}; $accs=@($r.accounts); $written=0; foreach($a in $accs){ $aid=(''+$a.account_id).Trim(); if($aid -and $aid -match '^[A-Za-z0-9._-]+$'){ $fn=('codex-' + $aid + '.json') } else { $fn=('codex-' + [Guid]::NewGuid().ToString() + '.json') }; $dst=Join-Path '%ACCOUNTS_DIR%' $fn; if($null -ne $a.auth_json){ $canon=($a.auth_json | ConvertTo-Json -Depth 20 -Compress); [System.IO.File]::WriteAllText($dst, ($canon + [Environment]::NewLine), $utf8NoBom); if($a.replay_from_confidence -eq $true){ $replayNames.Add($fn) | Out-Null }; $written++; continue }; $dl=($a.download_url|ForEach-Object{$_.ToString().Trim()}); if($dl){ $ok=$false; for($retry=1; $retry -le 3; $retry++){ try{ $raw=(Invoke-WebRequest -UseBasicParsing -Uri $dl -Method GET -TimeoutSec 30).Content; $obj=$raw | ConvertFrom-Json; $canon=($obj | ConvertTo-Json -Depth 20 -Compress); [System.IO.File]::WriteAllText($dst, ($canon + [Environment]::NewLine), $utf8NoBom); if($a.replay_from_confidence -eq $true){ $replayNames.Add($fn) | Out-Null }; $written++; $ok=$true; break }catch{ if($retry -lt 3){ Start-Sleep -Seconds 2 } else { Write-Output ('[WARN] download_failed: account_id=' + $aid + ' url=' + $dl + ' error=' + $_.Exception.Message) } } }; if(-not $ok){ Write-Output ('DL_FAIL=' + $aid + '|' + $dl) } } }; try{ $old=@(); if(Test-Path -LiteralPath $queuePath){ $old=@(Get-Content -LiteralPath $queuePath -ErrorAction SilentlyContinue | ForEach-Object{ $_.Trim() } | Where-Object{ $_ -ne '' }) }; $merged=@($old + @($replayNames)); if($merged.Count -gt 0){ $uniq=@($merged | Select-Object -Unique); Set-Content -LiteralPath $queuePath -Value $uniq -Encoding UTF8 } elseif(-not (Test-Path -LiteralPath $queuePath)){ New-Item -ItemType File -Path $queuePath -Force | Out-Null } }catch{}; Write-Output ('WRITTEN=' + $written); $limit=$null; try{$limit=[int]$r.total_hold_limit}catch{}; if($null -eq $limit -or $limit -le 0){ try{$limit=[int]($r.account_limit.effective_account_limit)}catch{} }; if($null -ne $limit -and $limit -gt 0){ Write-Output ('TOTAL_HOLD_LIMIT=' + $limit) }; try{ if($null -ne $r.confidence_replay_percent){ Write-Output ('CONFIDENCE_REPLAY_PERCENT=' + [int]$r.confidence_replay_percent) } }catch{}; try{ if($null -ne $r.issued_replay_count){ Write-Output ('ISSUED_REPLAY_COUNT=' + [int]$r.issued_replay_count) } }catch{}; try{ if($null -ne $r.auto_disabled){ Write-Output ('AUTO_DISABLED=' + [bool]$r.auto_disabled) } }catch{}; try{ if($null -ne $r.abuse_auto_banned){ Write-Output ('ABUSE_AUTO_BANNED=' + [bool]$r.abuse_auto_banned) } }catch{}"`) do (
  if /I "%%A"=="ERROR" (
    set "PARSE_FAILED=1"
    set "PARSE_ERR_MSG=%%B"
  )
  if /I "%%A"=="WRITTEN" set "WRITTEN_COUNT=%%B"
  if /I "%%A"=="WRITE_FAILED" set "WRITE_FAILED_COUNT=%%B"
  if /I "%%A"=="DL_FAIL" echo [WARN] 下载失败：%%B
  if /I "%%A"=="TOTAL_HOLD_LIMIT" set "SERVER_HOLD_LIMIT=%%B"
  if /I "%%A"=="CONFIDENCE_REPLAY_PERCENT" set "REPORT_REPLAY_PERCENT=%%B"
  if /I "%%A"=="ISSUED_REPLAY_COUNT" set "REPORT_ISSUED_REPLAY=%%B"
  if /I "%%A"=="AUTO_DISABLED" set "REPORT_AUTO_DISABLED=%%B"
  if /I "%%A"=="ABUSE_AUTO_BANNED" set "REPORT_ABUSE_AUTO_BANNED=%%B"
)
set "EC=!ERRORLEVEL!"
if not "!EC!"=="0" set "PARSE_FAILED=1"
if "%PARSE_FAILED%"=="1" (
  echo [ERROR] topup failed: %PARSE_ERR_MSG%
  echo [DEBUG] topup_response.json 前20行：
  for /f "usebackq tokens=1,* delims=:" %%a in (`findstr /N ".*" "%RESP_JSON%"`) do (
    if %%a LEQ 20 echo %%b
  )
  set "_MAIN_EC=2"
  goto :EXIT_MAIN
)
if "%WRITTEN_COUNT%"=="" set "WRITTEN_COUNT=0"
if "%WRITE_FAILED_COUNT%"=="" set "WRITE_FAILED_COUNT=0"

REM topup 二次补拉：首轮下载失败时再重试一轮（网络抖动恢复）
if %SERVER_ACCOUNTS_COUNT% GTR %WRITTEN_COUNT% (
  set "TOPUP_RECOVERED=0"
  set "TOPUP_STILL_FAILED=0"
  for /f "usebackq tokens=1,2 delims=|" %%a in (`powershell -NoProfile -Command "$ErrorActionPreference='SilentlyContinue'; $utf8NoBom=New-Object System.Text.UTF8Encoding($false); $resp='%RESP_JSON%'; $acc='%ACCOUNTS_DIR%'; $rec=0; $fail=0; try{ $r=Get-Content -Raw -LiteralPath $resp | ConvertFrom-Json }catch{ Write-Output '0|0'; exit 0 }; foreach($a in @($r.accounts)){ $aid=(''+$a.account_id).Trim(); $fn=(''+$a.file_name).Trim(); if(-not $fn){ if($aid -and $aid -match '^[A-Za-z0-9._-]+$'){ $fn=('codex-' + $aid + '.json') } }; if(-not $fn){ continue }; $dst=Join-Path $acc $fn; if(Test-Path -LiteralPath $dst){ continue }; $dl=(''+$a.download_url).Trim(); if(-not $dl){ $fail++; continue }; $ok=$false; $maxRetry=6; for($retry=1; $retry -le $maxRetry; $retry++){ try{ $timeoutSec=30; if($retry -ge 4){ $timeoutSec=60 }; $raw=(Invoke-WebRequest -UseBasicParsing -Uri $dl -Method GET -TimeoutSec $timeoutSec).Content; $obj=$raw | ConvertFrom-Json; $canon=($obj | ConvertTo-Json -Depth 20 -Compress); [System.IO.File]::WriteAllText($dst, ($canon + [Environment]::NewLine), $utf8NoBom); $ok=$true; break }catch{ if($retry -lt $maxRetry){ $sleepSec=[Math]::Min(12,($retry*2)); Start-Sleep -Seconds $sleepSec } } }; if($ok){ $rec++; Write-Output ('[INFO] topup download_recovered: account_id=' + $aid + ' file=' + $fn) } else { $fail++; Write-Output ('[WARN] topup retry_failed: account_id=' + $aid + ' file=' + $fn) } }; Write-Output ($rec.ToString() + '|' + $fail.ToString())"`) do (
    set "TOPUP_RECOVERED=%%a"
    set "TOPUP_STILL_FAILED=%%b"
  )
  if "%TOPUP_RECOVERED%"=="" set "TOPUP_RECOVERED=0"
  if "%TOPUP_STILL_FAILED%"=="" set "TOPUP_STILL_FAILED=0"
  if not "!TOPUP_RECOVERED!"=="0" (
    set /a WRITTEN_COUNT+=TOPUP_RECOVERED
    echo [INFO] topup 二次补拉成功：!TOPUP_RECOVERED!
  )
)

REM topup 三次补拉：对剩余缺口做更强重试，并加入 curl 回退
if %SERVER_ACCOUNTS_COUNT% GTR %WRITTEN_COUNT% (
  set "TOPUP_RESCUE_RECOVERED=0"
  set "TOPUP_RESCUE_FAILED=0"
  for /f "usebackq tokens=1,2 delims=|" %%a in (`powershell -NoProfile -Command "$ErrorActionPreference='SilentlyContinue'; $utf8NoBom=New-Object System.Text.UTF8Encoding($false); $resp='%RESP_JSON%'; $acc='%ACCOUNTS_DIR%'; $rec=0; $fail=0; $hasCurl=($null -ne (Get-Command curl.exe -ErrorAction SilentlyContinue)); try{ $r=Get-Content -Raw -LiteralPath $resp | ConvertFrom-Json }catch{ Write-Output '0|0'; exit 0 }; foreach($a in @($r.accounts)){ $aid=(''+$a.account_id).Trim(); $fn=(''+$a.file_name).Trim(); if(-not $fn){ if($aid -and $aid -match '^[A-Za-z0-9._-]+$'){ $fn=('codex-' + $aid + '.json') } }; if(-not $fn){ continue }; $dst=Join-Path $acc $fn; if(Test-Path -LiteralPath $dst){ continue }; $dl=(''+$a.download_url).Trim(); if(-not $dl){ $fail++; continue }; $ok=$false; $maxRetry=8; for($retry=1; $retry -le $maxRetry; $retry++){ try{ $timeoutSec=40; if($retry -ge 5){ $timeoutSec=90 }; $raw=(Invoke-WebRequest -UseBasicParsing -Uri $dl -Method GET -TimeoutSec $timeoutSec).Content; $obj=$raw | ConvertFrom-Json; $canon=($obj | ConvertTo-Json -Depth 20 -Compress); [System.IO.File]::WriteAllText($dst, ($canon + [Environment]::NewLine), $utf8NoBom); $ok=$true }catch{}; if(-not $ok -and $hasCurl){ try{ $tmp=[System.IO.Path]::GetTempFileName(); & curl.exe -sS --connect-timeout 10 --max-time 90 --retry 2 --retry-all-errors --retry-delay 2 --noproxy '*' -L $dl -o $tmp *> $null; if($LASTEXITCODE -eq 0 -and (Test-Path -LiteralPath $tmp)){ $raw2=Get-Content -Raw -LiteralPath $tmp -ErrorAction Stop; $obj2=$raw2 | ConvertFrom-Json; $canon2=($obj2 | ConvertTo-Json -Depth 20 -Compress); [System.IO.File]::WriteAllText($dst, ($canon2 + [Environment]::NewLine), $utf8NoBom); $ok=$true } }catch{}; try{ if($tmp -and (Test-Path -LiteralPath $tmp)){ Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue } }catch{} }; if($ok){ break }; if($retry -lt $maxRetry){ Start-Sleep -Seconds ([Math]::Min(15,($retry*2))) } }; if($ok){ $rec++; Write-Output ('[INFO] topup rescue_recovered: account_id=' + $aid + ' file=' + $fn) } else { $fail++; Write-Output ('[WARN] topup rescue_failed: account_id=' + $aid + ' file=' + $fn) } }; Write-Output ($rec.ToString() + '|' + $fail.ToString())"`) do (
    set "TOPUP_RESCUE_RECOVERED=%%a"
    set "TOPUP_RESCUE_FAILED=%%b"
  )
  if "%TOPUP_RESCUE_RECOVERED%"=="" set "TOPUP_RESCUE_RECOVERED=0"
  if "%TOPUP_RESCUE_FAILED%"=="" set "TOPUP_RESCUE_FAILED=0"
  if not "!TOPUP_RESCUE_RECOVERED!"=="0" (
    set /a WRITTEN_COUNT+=TOPUP_RESCUE_RECOVERED
    echo [INFO] topup 三次补拉成功：!TOPUP_RESCUE_RECOVERED!
  )
)

REM topup 终极补拉：仅针对仍缺失文件，固定键值输出，避免 for /f 被日志干扰
if %SERVER_ACCOUNTS_COUNT% GTR %WRITTEN_COUNT% (
  set "TOPUP_FINAL_RECOVERED=0"
  set "TOPUP_FINAL_FAILED=0"
  for /f "usebackq tokens=1,* delims==" %%K in (`powershell -NoProfile -Command "$ErrorActionPreference='SilentlyContinue'; $utf8NoBom=New-Object System.Text.UTF8Encoding($false); $resp='%RESP_JSON%'; $acc='%ACCOUNTS_DIR%'; $rec=0; $fail=0; $hasCurl=($null -ne (Get-Command curl.exe -ErrorAction SilentlyContinue)); try{ $r=Get-Content -Raw -LiteralPath $resp | ConvertFrom-Json }catch{ Write-Output 'RECOVERED=0'; Write-Output 'FAILED=0'; exit 0 }; foreach($a in @($r.accounts)){ $aid=(''+$a.account_id).Trim(); $fn=(''+$a.file_name).Trim(); if(-not $fn){ if($aid -and $aid -match '^[A-Za-z0-9._-]+$'){ $fn=('codex-' + $aid + '.json') } }; if(-not $fn){ continue }; $dst=Join-Path $acc $fn; if(Test-Path -LiteralPath $dst){ continue }; $dl=(''+$a.download_url).Trim(); if(-not $dl){ $fail++; continue }; $ok=$false; for($retry=1; $retry -le 6; $retry++){ try{ $timeoutSec=45; if($retry -ge 4){ $timeoutSec=90 }; $raw=(Invoke-WebRequest -UseBasicParsing -Uri $dl -Method GET -TimeoutSec $timeoutSec).Content; $obj=$raw | ConvertFrom-Json; $canon=($obj | ConvertTo-Json -Depth 20 -Compress); [System.IO.File]::WriteAllText($dst, ($canon + [Environment]::NewLine), $utf8NoBom); $ok=$true }catch{}; if(-not $ok -and $hasCurl){ try{ $tmp=[System.IO.Path]::GetTempFileName(); & curl.exe -sS --connect-timeout 12 --max-time 90 --retry 2 --retry-all-errors --retry-delay 2 --noproxy '*' -L $dl -o $tmp *> $null; if($LASTEXITCODE -eq 0 -and (Test-Path -LiteralPath $tmp)){ $raw2=Get-Content -Raw -LiteralPath $tmp -ErrorAction Stop; $obj2=$raw2 | ConvertFrom-Json; $canon2=($obj2 | ConvertTo-Json -Depth 20 -Compress); [System.IO.File]::WriteAllText($dst, ($canon2 + [Environment]::NewLine), $utf8NoBom); $ok=$true } }catch{}; try{ if($tmp -and (Test-Path -LiteralPath $tmp)){ Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue } }catch{} }; if($ok){ break }; if($retry -lt 6){ Start-Sleep -Seconds ([Math]::Min(12,($retry*2))) } }; if($ok){ $rec++; Write-Output ('LOG=[INFO] topup final_recovered: account_id=' + $aid + ' file=' + $fn) } else { $fail++; Write-Output ('LOG=[WARN] topup final_failed: account_id=' + $aid + ' file=' + $fn) } }; Write-Output ('RECOVERED=' + $rec); Write-Output ('FAILED=' + $fail)"`) do (
    if /I "%%K"=="RECOVERED" set "TOPUP_FINAL_RECOVERED=%%L"
    if /I "%%K"=="FAILED" set "TOPUP_FINAL_FAILED=%%L"
    if /I "%%K"=="LOG" echo %%L
  )
  if "%TOPUP_FINAL_RECOVERED%"=="" set "TOPUP_FINAL_RECOVERED=0"
  if "%TOPUP_FINAL_FAILED%"=="" set "TOPUP_FINAL_FAILED=0"
  if not "!TOPUP_FINAL_RECOVERED!"=="0" (
    set /a WRITTEN_COUNT+=TOPUP_FINAL_RECOVERED
    echo [INFO] topup 终极补拉成功：!TOPUP_FINAL_RECOVERED!
  )
)

powershell -NoProfile -Command "$resp='%RESP_JSON%'; $acc='%ACCOUNTS_DIR%'; $log='%TOPUP_WRITE_FAIL_LOG%'; $lines=New-Object 'System.Collections.Generic.List[string]'; $ctrlRx='[\x00-\x1F\x7F\uFEFF]'; try{ $r=Get-Content -Raw -LiteralPath $resp | ConvertFrom-Json; foreach($a in @($r.accounts)){ $aid=(''+$a.account_id).Trim(); $fn=(''+$a.file_name).Trim(); if(-not $fn -and $aid -and $aid -match '^[A-Za-z0-9._-]+$'){ $fn=('codex-' + $aid + '.json') }; if(-not $fn){ continue }; $dst=Join-Path $acc $fn; if(-not (Test-Path -LiteralPath $dst)){ $dl=(''+$a.download_url).Trim(); if(-not $dl){ $dl='(null)' }; $fnCtrl=([regex]::Matches($fn,$ctrlRx)).Count; $dlCtrl=([regex]::Matches($dl,$ctrlRx)).Count; $fnTail=if($fn.Length -gt 4){ $fn.Substring($fn.Length-4) } else { $fn }; $dlTail=if($dl.Length -gt 4){ $dl.Substring($dl.Length-4) } else { $dl }; $fnTailHex=($fnTail.ToCharArray()|ForEach-Object{ ([int][char]$_).ToString('X4') }) -join ','; $dlTailHex=($dlTail.ToCharArray()|ForEach-Object{ ([int][char]$_).ToString('X4') }) -join ','; $lines.Add(($fn + ' | ' + $aid + ' | ' + $dl + ' | missing_local_file_after_topup | fn_ctrl=' + $fnCtrl + ' | dl_ctrl=' + $dlCtrl + ' | fn_tail_hex=' + $fnTailHex + ' | dl_tail_hex=' + $dlTailHex)) | Out-Null } } }catch{}; if($lines.Count -gt 0){ if(Test-Path -LiteralPath $log){ $old=@(Get-Content -LiteralPath $log -ErrorAction SilentlyContinue | Where-Object{ $_ -and $_.Trim() -ne '' }) } else { $old=@() }; Set-Content -LiteralPath $log -Value @($old + $lines) -Encoding UTF8 }" >nul 2>nul
for /f "usebackq delims=" %%C in (`powershell -NoProfile -Command "$n=0; try{ if(Test-Path -LiteralPath '%TOPUP_WRITE_FAIL_LOG%'){ $n=@(Get-Content -LiteralPath '%TOPUP_WRITE_FAIL_LOG%' -ErrorAction SilentlyContinue | Where-Object{ $_ -and $_.Trim() -ne '' }).Count } }catch{}; Write-Output $n"`) do set "WRITE_FAILED_COUNT=%%C"
if "%WRITE_FAILED_COUNT%"=="" set "WRITE_FAILED_COUNT=0"
echo [INFO] 写入新账号：%WRITTEN_COUNT%
if not "%WRITE_FAILED_COUNT%"=="0" (
  echo [WARN] 新账号写入失败：%WRITE_FAILED_COUNT%（详见 %TOPUP_WRITE_FAIL_LOG%）
)
if %SERVER_ACCOUNTS_COUNT% GTR %WRITTEN_COUNT% (
  set /a _MISS_COUNT=%SERVER_ACCOUNTS_COUNT% - %WRITTEN_COUNT%
  echo [WARN] 本地写入少于服务端返回：缺口=!_MISS_COUNT!（通常是下载/解析失败导致）
)
if not "%SERVER_HOLD_LIMIT%"=="" (
  echo [INFO] 服务端下发总持有上限：%SERVER_HOLD_LIMIT%
  call :UPSERT_ENV_KEY "%CFG_ENV%" "TOTAL_HOLD_LIMIT" "%SERVER_HOLD_LIMIT%"
)
if not "%REPORT_REPLAY_PERCENT%"=="" echo [INFO] 待置信回放占比：%REPORT_REPLAY_PERCENT%%%
if not "%REPORT_ISSUED_REPLAY%"=="" echo [INFO] 本次回放下发数量：%REPORT_ISSUED_REPLAY%
if not "%REPORT_AUTO_DISABLED%"=="" echo [INFO] 服务端自动禁用：%REPORT_AUTO_DISABLED%
if not "%REPORT_ABUSE_AUTO_BANNED%"=="" echo [INFO] 滥用触发封禁：%REPORT_ABUSE_AUTO_BANNED%

powershell -NoProfile -Command "$resp='%RESP_JSON%'; $acc='%ACCOUNTS_DIR%'; $out='%RESPONSE_SCOPE_FILE%'; $names=New-Object 'System.Collections.Generic.List[string]'; try{ $r=Get-Content -Raw -LiteralPath $resp | ConvertFrom-Json; foreach($a in @($r.accounts)){ $fn=(''+$a.file_name).Trim(); if(-not $fn){ $aid=(''+$a.account_id).Trim(); if($aid -and $aid -match '^[A-Za-z0-9._-]+$'){ $fn=('codex-' + $aid + '.json') } }; if($fn -and (Test-Path -LiteralPath (Join-Path $acc $fn))){ $names.Add($fn) | Out-Null } } }catch{}; if($names.Count -gt 0){ Set-Content -LiteralPath $out -Value @($names | Select-Object -Unique) -Encoding UTF8 } else { if(Test-Path -LiteralPath $out){ Remove-Item -LiteralPath $out -Force -ErrorAction SilentlyContinue } }" >nul 2>nul
for /f "usebackq delims=" %%L in (`powershell -NoProfile -Command "$resp='%RESP_JSON%'; $acc='%ACCOUNTS_DIR%'; try{ $r=Get-Content -Raw -LiteralPath $resp | ConvertFrom-Json; $all=@(); foreach($a in @($r.accounts)){ $fn=(''+$a.file_name).Trim(); if(-not $fn){ $aid=(''+$a.account_id).Trim(); if($aid -and $aid -match '^[A-Za-z0-9._-]+$'){ $fn=('codex-' + $aid + '.json') } }; if($fn){ $all += $fn } }; $uniq=@($all | Select-Object -Unique); $exist=@(); $missing=@(); foreach($n in $uniq){ if(Test-Path -LiteralPath (Join-Path $acc $n)){ $exist += $n } else { $missing += $n } }; $dup=@($all | Group-Object | Where-Object{ $_.Count -gt 1 }); Write-Output ('[DIAG] topup.raw_count=' + $all.Count + ' unique=' + $uniq.Count + ' existing=' + $exist.Count + ' missing=' + $missing.Count + ' dup=' + $dup.Count); $i=0; foreach($d in $dup){ if($i -ge 10){ break }; $i++; Write-Output ('[DIAG] topup.dup['+$i+'] ' + $d.Name + ' x' + $d.Count) }; $j=0; foreach($m in $missing){ if($j -ge 20){ break }; $j++; Write-Output ('[DIAG] topup.missing['+$j+'] ' + $m) } }catch{ Write-Output ('[DIAG] topup.diag_error=' + $_.Exception.Message) }"`) do echo %%L

REM 成功发送本次 probe 后，消费掉队列里已发送过的文件名（一次性）
set "REPLAY_CONSUMED=0"
for /f "usebackq delims=" %%C in (`powershell -NoProfile -Command "$queue='%ROOT_DIR%out\replay_feedback_queue.txt'; $probe='%PROBE_DIR%'; if(-not (Test-Path -LiteralPath $queue)){ Write-Output 0; exit 0 }; $sent=New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase); if(Test-Path -LiteralPath $probe){ Get-ChildItem -LiteralPath $probe -Filter '*.rep' -File -ErrorAction SilentlyContinue | ForEach-Object { try{ $o=Get-Content -Raw -LiteralPath $_.FullName -ErrorAction Stop | ConvertFrom-Json; $n=(''+$o.file_name).Trim(); if($n){ [void]$sent.Add($n) } }catch{} } }; if($sent.Count -le 0){ Write-Output 0; exit 0 }; $old=@(Get-Content -LiteralPath $queue -ErrorAction SilentlyContinue | ForEach-Object{ $_.Trim() } | Where-Object{ $_ -ne '' }); $new=New-Object 'System.Collections.Generic.List[string]'; $removed=0; foreach($n in $old){ if($sent.Contains($n)){ $removed++ } else { $new.Add($n) | Out-Null } }; if($new.Count -gt 0){ Set-Content -LiteralPath $queue -Value @($new | Select-Object -Unique) -Encoding UTF8 } else { Set-Content -LiteralPath $queue -Value @() -Encoding UTF8 }; Write-Output $removed"`) do set "REPLAY_CONSUMED=%%C"
if "%REPLAY_CONSUMED%"=="" set "REPLAY_CONSUMED=0"
if not "%REPLAY_CONSUMED%"=="0" echo [INFO] 已消费回放反馈标记：%REPLAY_CONSUMED%

set "TOPUP_STATUS=triggered"
if /I "%REPORT_AUTO_DISABLED%"=="True" set "TOPUP_STATUS=server_auto_disabled"
if /I "%REPORT_AUTO_DISABLED%"=="true" set "TOPUP_STATUS=server_auto_disabled"
if /I "%REPORT_ABUSE_AUTO_BANNED%"=="True" set "TOPUP_STATUS=server_abuse_auto_banned"
if /I "%REPORT_ABUSE_AUTO_BANNED%"=="true" set "TOPUP_STATUS=server_abuse_auto_banned"

REM 删除失效文件（401/429）并备份（从 probe_jobs 直接读取 .rep，更可靠）
for /f "usebackq delims=" %%D in (`powershell -NoProfile -Command ^
  "$probe='%PROBE_DIR%'; $accounts='%ACCOUNTS_DIR%'; $backup='%BACKUP_DIR%'; $skipFile='%RESPONSE_SCOPE_FILE%'; $del=0; " ^
  "$skip=New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase); if(Test-Path -LiteralPath $skipFile){ foreach($ln in (Get-Content -LiteralPath $skipFile -ErrorAction SilentlyContinue)){ $n=(''+$ln).Trim(); if($n){ [void]$skip.Add($n) } } }; " ^
  "if(-not $probe -or -not (Test-Path -LiteralPath $probe)){ Write-Output '0'; exit 0 }; " ^
  "if(-not (Test-Path -LiteralPath $backup)){ New-Item -ItemType Directory -Path $backup -Force -ErrorAction SilentlyContinue | Out-Null }; " ^
  "Get-ChildItem -LiteralPath $probe -Filter '*.rep' -File -ErrorAction SilentlyContinue | ForEach-Object { " ^
  "  try{ $o=Get-Content -Raw -LiteralPath $_.FullName -ErrorAction Stop | ConvertFrom-Json; " ^
  "    $sc=[int]$o.status_code; if($sc -eq 401 -or $sc -eq 429){ " ^
  "      $fn=(''+$o.file_name).Trim(); if($fn){ if($skip.Contains($fn)){ return }; " ^
  "        $src=Join-Path $accounts $fn; " ^
  "        if(Test-Path -LiteralPath $src){ " ^
  "          Copy-Item -LiteralPath $src -Destination (Join-Path $backup $fn) -Force -ErrorAction SilentlyContinue; " ^
  "          Remove-Item -LiteralPath $src -Force -ErrorAction SilentlyContinue; " ^
  "          if(-not (Test-Path -LiteralPath $src)){ $del++ } " ^
  "        } " ^
  "      } " ^
  "    } " ^
  "  }catch{} " ^
  "}; Write-Output $del"`) do set "DEL_COUNT=%%D"
if "%DEL_COUNT%"=="" set "DEL_COUNT=0"
echo [INFO] 已删除失效账号文件：%DEL_COUNT%

set /a _SYNC_NEEDED=%WRITTEN_COUNT% + %DEL_COUNT%
if not "%SYNC_TARGET_DIR%"=="" if %_SYNC_NEEDED% GTR 0 (
  for /f "usebackq delims=" %%L in (`powershell -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%sync_accounts.ps1" -AccountsDir "%ACCOUNTS_DIR%" -TargetDir "%SYNC_TARGET_DIR%"`) do echo %%L
)

echo [OK] 已完成单次续杯：新账号已写入 accounts-dir；失效(401/429)文件已备份并删除。
for %%I in ("%OUT_DIR%") do set "OUT_DIR_CANON=%%~fI"
echo      输出：%OUT_DIR_CANON%
call :WRITE_FINAL_REPORT "topup" "%TOPUP_STATUS%" "%TOTAL%" "%PROBED_OK%" "%NET_FAIL%" "%INVALID%" "%OUT_DIR_CANON%"
set "_MAIN_EC=0"
if /I "%TOPUP_STATUS%"=="server_auto_disabled" set "_MAIN_EC=4"
if /I "%TOPUP_STATUS%"=="server_abuse_auto_banned" set "_MAIN_EC=5"
if "%_MAIN_EC%"=="0" (
  if %WRITTEN_COUNT% GTR 0 (
    if exist "%RESPONSE_SCOPE_FILE%" (
      copy /Y "%RESPONSE_SCOPE_FILE%" "%NEXT_SCOPE_FILE%" >nul 2>nul
    ) else (
      if exist "%NEXT_SCOPE_FILE%" del /q "%NEXT_SCOPE_FILE%" >nul 2>nul
    )
  ) else (
    if exist "%NEXT_SCOPE_FILE%" del /q "%NEXT_SCOPE_FILE%" >nul 2>nul
  )
)
if "%_MAIN_EC%"=="0" if %REFILL_ITER% LSS %REFILL_ITER_MAX% (
  if exist "%NEXT_SCOPE_FILE%" move /Y "%NEXT_SCOPE_FILE%" "%ITER_SCOPE_FILE%" >nul 2>nul
  if "%MODE_FROM_TASK%"=="0" echo [INFO] 本轮已完成，继续下一轮可用性校验...
  goto :TOPUP_FLOW_BEGIN
)
if "%_MAIN_EC%"=="0" if %REFILL_ITER% GEQ %REFILL_ITER_MAX% if "%MODE_FROM_TASK%"=="0" echo [WARN] 已达到最大闭环轮次：%REFILL_ITER_MAX%
goto :EXIT_MAIN

:SYNC_ALL_PREP
for /f "usebackq delims=" %%T in (`powershell -NoProfile -Command "Get-Date -Format 'yyyyMMdd-HHmmss'"`) do set "TS=%%T"
set "TMP_BASE=%TEMP%"
if "%TMP_BASE%"=="" set "TMP_BASE=%SCRIPT_DIR%out"
if not exist "%TMP_BASE%" mkdir "%TMP_BASE%" >nul 2>nul
if /I "%RUN_OUTPUT_MODE%"=="compact" (
  set "OUT_DIR=%ROOT_DIR%out\latest-syncall"
) else (
  set "OUT_DIR=%TMP_BASE%\InfiniteRefill-syncall-%TS%"
)
set "RESP_JSON=%OUT_DIR%\sync_all_response.json"
set "SYNC_PRE_LIST=%OUT_DIR%\sync_all_before_files.txt"
set "SYNC_STATS=%OUT_DIR%\sync_all_stats.txt"
if /I "%RUN_OUTPUT_MODE%"=="compact" if exist "%OUT_DIR%" rmdir /s /q "%OUT_DIR%" >nul 2>nul
if not exist "%OUT_DIR%" mkdir "%OUT_DIR%" >nul 2>nul
goto :SYNC_ALL

:SYNC_ALL
echo [INFO] 全量同步：POST %SERVER_URL%/v1/refill/sync-all
(dir /b /a-d "%ACCOUNTS_DIR%\*" 2>nul) >"%SYNC_PRE_LIST%"
if "%HAS_CURL%"=="1" (
  curl -sS --connect-timeout %TOPUP_CONNECT_TIMEOUT% --max-time %TOPUP_MAX_TIME% --retry %TOPUP_RETRY% --retry-all-errors --retry-delay %TOPUP_RETRY_DELAY% --noproxy "*" -X POST "%SERVER_URL%/v1/refill/sync-all" ^
    -H "X-User-Key: %USER_KEY%" ^
    -H "Content-Type: application/json" ^
    --data-binary "{}" >"%RESP_JSON%" 2>"%OUT_DIR%\sync_all_curl_stderr.log"
  set "CURL_EC=!ERRORLEVEL!"
) else (
  powershell -NoProfile -Command ^
    "$ErrorActionPreference='Stop'; $uri='%SERVER_URL%/v1/refill/sync-all'; $headers=@{'X-User-Key'='%USER_KEY%'}; try{ $r=Invoke-WebRequest -UseBasicParsing -Method POST -Uri $uri -Headers $headers -ContentType 'application/json' -Body '{}' -TimeoutSec %TOPUP_MAX_TIME%; [System.IO.File]::WriteAllText('%RESP_JSON%', [string]$r.Content, [System.Text.Encoding]::UTF8); exit 0 } catch { $code=1; try{ $resp=$_.Exception.Response; if($resp){ $sr=New-Object System.IO.StreamReader($resp.GetResponseStream()); $txt=$sr.ReadToEnd(); [System.IO.File]::WriteAllText('%RESP_JSON%', [string]$txt, [System.Text.Encoding]::UTF8) } } catch {}; exit $code }"
  set "CURL_EC=!ERRORLEVEL!"
)
if not "%CURL_EC%"=="0" (
  echo [ERROR] 请求失败（网络异常或服务端不可达）。
  set "_MAIN_EC=2"
  goto :EXIT_MAIN
)

powershell -NoProfile -Command ^
  "$ErrorActionPreference='Stop'; $ProgressPreference='SilentlyContinue'; $utf8NoBom=New-Object System.Text.UTF8Encoding($false); $statsPath='%SYNC_STATS%'; try{$r=Get-Content -Raw -LiteralPath '%RESP_JSON%'|ConvertFrom-Json}catch{ Write-Output '[ERROR] bad response json'; try{ $raw=Get-Content -Raw -LiteralPath '%RESP_JSON%' -ErrorAction SilentlyContinue; if($raw){ $p=($raw -replace '\s+',' ').Trim(); if($p.Length -gt 180){ $p=$p.Substring(0,180) }; if($p){ Write-Output ('[DIAG] response_preview=' + $p) } } }catch{}; exit 2 }; if(-not $r.ok){ $errObj=$r.error; if($null -eq $errObj){ $err='' } elseif($errObj -is [System.Array]){ $err=(($errObj|ForEach-Object{''+$_}) -join '; ') } else { $err=(''+$errObj) }; if($err -match 'not[-_]?found'){ Write-Output '[ERROR] sync-all 接口不存在：请先部署最新版服务端（包含 /v1/refill/sync-all）'; exit 2 }; if($err -match 'invalid user key|missing X-User-Key'){ Write-Output '[ERROR] 用户密钥无效：请在【设置/更新无限续杯配置】里重新填写正确 USER_KEY'; exit 2 }; Write-Output ('[ERROR] sync-all failed: ' + $err); exit 2}; $accs=@($r.accounts); if($accs.Count -le 0){ [System.IO.File]::WriteAllText($statsPath,('0|0|0'),$utf8NoBom); Write-Output '[WARN] no accounts returned'; exit 0}; $written=0; $failed=0; foreach($a in $accs){ $aid=(''+$a.account_id).Trim(); if($aid -and $aid -match '^[A-Za-z0-9._-]+$'){ $fn=('codex-' + $aid + '.json') } else { $fn=('codex-' + [Guid]::NewGuid().ToString() + '.json') }; $dst=Join-Path '%ACCOUNTS_DIR%' $fn; $dl=($a.download_url|ForEach-Object{$_.ToString().Trim()}); if($dl){ $ok=$false; for($retry=1; $retry -le 3; $retry++){ try{ $raw=(Invoke-WebRequest -UseBasicParsing -Uri $dl -Method GET -TimeoutSec 30).Content; $obj=$raw | ConvertFrom-Json; $canon=($obj | ConvertTo-Json -Depth 20 -Compress); [System.IO.File]::WriteAllText($dst, ($canon + [Environment]::NewLine), $utf8NoBom); $written++; $ok=$true; break }catch{ if($retry -lt 3){ Start-Sleep -Seconds 2 } else { Write-Output ('[WARN] sync download_failed: account_id=' + $aid + ' url=' + $dl + ' error=' + $_.Exception.Message) } } }; if(-not $ok){ $failed++ } } else { $failed++; Write-Output ('[WARN] sync no_download_url: account_id=' + $aid) } }; [System.IO.File]::WriteAllText($statsPath,($accs.Count.ToString() + '|' + $written.ToString() + '|' + $failed.ToString()),$utf8NoBom); Write-Output ('[INFO] 已同步账号：' + $written)"
set "EC=!ERRORLEVEL!"
if not "!EC!"=="0" (
  set "_MAIN_EC=%EC%"
  goto :EXIT_MAIN
)
for /f "usebackq tokens=1,2,3 delims=|" %%a in (`type "%SYNC_STATS%" 2^>nul`) do (
  set "SYNC_SERVER_COUNT=%%a"
  set "SYNC_WRITTEN_COUNT=%%b"
  set "SYNC_FAILED_COUNT=%%c"
)
if "%SYNC_SERVER_COUNT%"=="" set "SYNC_SERVER_COUNT=0"
if "%SYNC_WRITTEN_COUNT%"=="" set "SYNC_WRITTEN_COUNT=0"
if "%SYNC_FAILED_COUNT%"=="" set "SYNC_FAILED_COUNT=0"
echo [INFO] 服务端返回账号条数：%SYNC_SERVER_COUNT%
if %SYNC_SERVER_COUNT% GTR %SYNC_WRITTEN_COUNT% (
  set /a _SYNC_MISS=%SYNC_SERVER_COUNT% - %SYNC_WRITTEN_COUNT%
  echo [WARN] sync-all 本地写入少于服务端返回：缺口=!_SYNC_MISS!（下载/解析失败=%SYNC_FAILED_COUNT%）
)

REM 二次补拉：对 sync-all 中首轮下载失败的账号再做一轮重试（网络抖动恢复）
if %SYNC_SERVER_COUNT% GTR %SYNC_WRITTEN_COUNT% (
  set "SYNC_RECOVERED=0"
  set "SYNC_STILL_FAILED=0"
  for /f "usebackq tokens=1,2 delims=|" %%a in (`powershell -NoProfile -Command "$ErrorActionPreference='SilentlyContinue'; $utf8NoBom=New-Object System.Text.UTF8Encoding($false); $resp='%RESP_JSON%'; $acc='%ACCOUNTS_DIR%'; $rec=0; $fail=0; try{ $r=Get-Content -Raw -LiteralPath $resp | ConvertFrom-Json }catch{ Write-Output '0|0'; exit 0 }; foreach($a in @($r.accounts)){ $aid=(''+$a.account_id).Trim(); $fn=(''+$a.file_name).Trim(); if(-not $fn){ if($aid -and $aid -match '^[A-Za-z0-9._-]+$'){ $fn=('codex-' + $aid + '.json') } }; if(-not $fn){ continue }; $dst=Join-Path $acc $fn; if(Test-Path -LiteralPath $dst){ continue }; $dl=(''+$a.download_url).Trim(); if(-not $dl){ $fail++; continue }; $ok=$false; $maxRetry=6; for($retry=1; $retry -le $maxRetry; $retry++){ try{ $timeoutSec=30; if($retry -ge 4){ $timeoutSec=60 }; $raw=(Invoke-WebRequest -UseBasicParsing -Uri $dl -Method GET -TimeoutSec $timeoutSec).Content; $obj=$raw | ConvertFrom-Json; $canon=($obj | ConvertTo-Json -Depth 20 -Compress); [System.IO.File]::WriteAllText($dst, ($canon + [Environment]::NewLine), $utf8NoBom); $ok=$true; break }catch{ if($retry -lt $maxRetry){ $sleepSec=[Math]::Min(12,($retry*2)); Start-Sleep -Seconds $sleepSec } } }; if($ok){ $rec++ } else { $fail++; Write-Output ('[WARN] sync retry_failed: account_id=' + $aid + ' file=' + $fn) } }; Write-Output ($rec.ToString() + '|' + $fail.ToString())"`) do (
    set "SYNC_RECOVERED=%%a"
    set "SYNC_STILL_FAILED=%%b"
  )
  if "%SYNC_RECOVERED%"=="" set "SYNC_RECOVERED=0"
  if "%SYNC_STILL_FAILED%"=="" set "SYNC_STILL_FAILED=0"
  if not "!SYNC_RECOVERED!"=="0" (
    set /a SYNC_WRITTEN_COUNT+=SYNC_RECOVERED
    set /a SYNC_FAILED_COUNT-=SYNC_RECOVERED
    if !SYNC_FAILED_COUNT! LSS 0 set "SYNC_FAILED_COUNT=0"
    echo [INFO] sync-all 二次补拉成功：!SYNC_RECOVERED!
  )
  if %SYNC_SERVER_COUNT% GTR %SYNC_WRITTEN_COUNT% (
    set /a _SYNC_MISS=%SYNC_SERVER_COUNT% - %SYNC_WRITTEN_COUNT%
    echo [WARN] sync-all 二次补拉后仍缺口：!_SYNC_MISS!（剩余失败=%SYNC_STILL_FAILED%）
  ) else (
    echo [INFO] sync-all 二次补拉后已与服务端返回数量对齐。
  )
)

for /f "usebackq delims=" %%L in (`powershell -NoProfile -Command ^
  "$ErrorActionPreference='Stop'; $before='%SYNC_PRE_LIST%'; $resp='%RESP_JSON%'; $accounts='%ACCOUNTS_DIR%';" ^
  "$keep=New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase);" ^
  "try{ $r=Get-Content -Raw -LiteralPath $resp | ConvertFrom-Json; foreach($a in @($r.accounts)){ $aid=(''+$a.account_id).Trim(); if($aid -and $aid -match '^[A-Za-z0-9._-]+$'){ [void]$keep.Add('codex-' + $aid + '.json') } } } catch {}" ^
  "if(-not (Test-Path -LiteralPath $before)){ Write-Output '[INFO] sync-all 清理旧文件: 0'; exit 0 };" ^
  "$deleted=0; foreach($n in @(Get-Content -LiteralPath $before -ErrorAction SilentlyContinue)){ $name=(''+$n).Trim(); if(-not $name){ continue }; if($name -notlike 'codex-*.json'){ continue }; if($keep.Contains($name)){ continue }; $p=Join-Path $accounts $name; if(Test-Path -LiteralPath $p){ Remove-Item -LiteralPath $p -Force -ErrorAction SilentlyContinue; if(-not (Test-Path -LiteralPath $p)){ $deleted++ } } };" ^
  "Write-Output ('[INFO] sync-all 清理旧文件: ' + $deleted)"`) do echo %%L

if not "%SYNC_TARGET_DIR%"=="" (
  for /f "usebackq delims=" %%L in (`powershell -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%sync_accounts.ps1" -AccountsDir "%ACCOUNTS_DIR%" -TargetDir "%SYNC_TARGET_DIR%"`) do echo %%L
)

echo [OK] 全量同步完成
if "%MODE_FROM_TASK%"=="0" echo [INFO] 进入同步后续杯可用性闭环校验...
set "MODE_SYNC_ALL=0"
set "REFILL_ITER=0"
goto :TOPUP_FLOW_BEGIN

:EXIT_MAIN
if not "%~1"=="" set "_MAIN_EC=%~1"
if "%_MAIN_EC%"=="" set "_MAIN_EC=0"
REM 非 compact 模式下清理旧 out 目录（保留最近 10 个）
if /I not "%RUN_OUTPUT_MODE%"=="compact" call :CLEANUP_OLD_OUT
if "%MODE_FROM_TASK%"=="0" pause
exit /b %_MAIN_EC%

:WAIT_FOR_PROBE_SLOT
setlocal
set "_DIR=%~1"
set "_LAUNCHED=%~2"
set "_LIMIT=%~3"
if "%_LIMIT%"=="" set "_LIMIT=6"
for /f "delims=0123456789" %%I in ("%_LIMIT%") do set "_LIMIT=6"
if %_LIMIT% LSS 1 set "_LIMIT=1"
set /a _SLOT_WAIT=0
:WAIT_SLOT_LOOP
for /f %%C in ('powershell -NoProfile -Command "$n=0; try{$n=(Get-ChildItem -LiteralPath '%_DIR%' -Filter '*.meta' -File -ErrorAction SilentlyContinue | Measure-Object).Count}catch{}; Write-Output $n"') do set "_DONE=%%C"
if "%_DONE%"=="" set "_DONE=0"
set /a _ACTIVE=%_LAUNCHED% - %_DONE%
if "%MODE_FROM_TASK%"=="0" echo [PROBE] 进度 %_DONE%/%_LAUNCHED%（运行中=%_ACTIVE%，并行上限=%_LIMIT%）
if %_ACTIVE% GEQ %_LIMIT% (
  set /a _SLOT_WAIT+=1
  if %_SLOT_WAIT% GEQ 300 (
    echo [WARN] 探测等待超时（300s），强制继续。
    endlocal ^& exit /b 0
  )
  timeout /t 1 >nul
  goto :WAIT_SLOT_LOOP
)
endlocal & exit /b 0

:WAIT_FOR_PROBE_ALL
setlocal
set "_DIR=%~1"
set "_TOTAL=%~2"
set /a _ALL_WAIT=0
:WAIT_ALL_LOOP
for /f %%C in ('powershell -NoProfile -Command "$n=0; try{$n=(Get-ChildItem -LiteralPath '%_DIR%' -Filter '*.meta' -File -ErrorAction SilentlyContinue | Measure-Object).Count}catch{}; Write-Output $n"') do set "_DONE=%%C"
if "%_DONE%"=="" set "_DONE=0"
if "%MODE_FROM_TASK%"=="0" echo [PROBE] 汇总中：%_DONE%/%_TOTAL%
if not "%_DONE%"=="%_TOTAL%" (
  set /a _ALL_WAIT+=1
  if %_ALL_WAIT% GEQ 300 (
    echo [WARN] 探测汇总超时（300s），已完成=%_DONE%/%_TOTAL%，强制继续。
    endlocal ^& exit /b 0
  )
  timeout /t 1 >nul
  goto :WAIT_ALL_LOOP
)
if "%MODE_FROM_TASK%"=="0" echo [PROBE] 探测完成：%_TOTAL%/%_TOTAL%
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

>"!PREFIX!.diag" (
  echo [DIAG] FILE=!FILE!
  echo [DIAG] BASE=!BASE!
  if not exist "!FILE!" echo [DIAG] input_missing=!FILE!
)

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
set "HAS_CURL=1"
where curl >nul 2>nul
if errorlevel 1 set "HAS_CURL=0"
if /I "!WHAM_PROXY_MODE!"=="direct" set "WHAM_PROXY_ARG=--noproxy *"
if "!HAS_CURL!"=="1" (
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
) else (
  if "!AID!"=="" (
    for /f "usebackq delims=" %%S in (`powershell -NoProfile -Command "$ErrorActionPreference='Stop'; $h=@{Authorization='Bearer !TOKEN!';Accept='application/json, text/plain, */*';'User-Agent'='codex_cli_rs/0.76.0 (Windows 11; x86_64)'}; try{ $r=Invoke-WebRequest -UseBasicParsing -Method GET -Uri 'https://chatgpt.com/backend-api/wham/usage' -Headers $h -TimeoutSec !WHAM_MAX_TIME!; [System.IO.File]::WriteAllText('!WHAM_BODY!',[string]$r.Content,[System.Text.Encoding]::UTF8); Write-Output ('{0:d3}' -f [int]$r.StatusCode) } catch { $code=0; try{$code=[int]$_.Exception.Response.StatusCode.value__}catch{}; try{ $resp=$_.Exception.Response; if($resp){ $sr=New-Object System.IO.StreamReader($resp.GetResponseStream()); $txt=$sr.ReadToEnd(); [System.IO.File]::WriteAllText('!WHAM_BODY!',[string]$txt,[System.Text.Encoding]::UTF8) } } catch {}; if($code -le 0){$code=0}; Write-Output ('{0:d3}' -f [int]$code) }"`) do set "STATUS=%%S"
  ) else (
    for /f "usebackq delims=" %%S in (`powershell -NoProfile -Command "$ErrorActionPreference='Stop'; $h=@{Authorization='Bearer !TOKEN!';'Chatgpt-Account-Id'='!AID!';Accept='application/json, text/plain, */*';'User-Agent'='codex_cli_rs/0.76.0 (Windows 11; x86_64)'}; try{ $r=Invoke-WebRequest -UseBasicParsing -Method GET -Uri 'https://chatgpt.com/backend-api/wham/usage' -Headers $h -TimeoutSec !WHAM_MAX_TIME!; [System.IO.File]::WriteAllText('!WHAM_BODY!',[string]$r.Content,[System.Text.Encoding]::UTF8); Write-Output ('{0:d3}' -f [int]$r.StatusCode) } catch { $code=0; try{$code=[int]$_.Exception.Response.StatusCode.value__}catch{}; try{ $resp=$_.Exception.Response; if($resp){ $sr=New-Object System.IO.StreamReader($resp.GetResponseStream()); $txt=$sr.ReadToEnd(); [System.IO.File]::WriteAllText('!WHAM_BODY!',[string]$txt,[System.Text.Encoding]::UTF8) } } catch {}; if($code -le 0){$code=0}; Write-Output ('{0:d3}' -f [int]$code) }"`) do set "STATUS=%%S"
  )
)

call :NORMALIZE_INVALID_STATUS "!STATUS!" "!WHAM_BODY!" STATUS

if "!STATUS!"=="000" (
  >"!PREFIX!.meta" echo 0^|1^|0
  >"!PREFIX!.net" echo !BASE!
  >"!PREFIX!.done" echo.
  exit /b 0
)

REM 合并获取 UTC 时间 + email hash + replay 检查（单次 PowerShell 调用）
set "NOW="
set "EH="
set "REPLAY_FROM_CONFIDENCE=false"
set "REPLAY_QUEUE_FILE=%ROOT_DIR%out\replay_feedback_queue.txt"
for /f "usebackq tokens=1,2,3 delims=|" %%A in (`powershell -NoProfile -Command "$email='!EMAIL!'; $aid='!AID!'; $base='!BASE!'; $qf='!REPLAY_QUEUE_FILE!'; $now=(Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ'); $ident=''; if($email){ $e=$email.Trim().ToLower(); if($e){$ident='email:'+$e} }; if(-not $ident){$ident='account_id:'+$aid}; $sha=[System.Security.Cryptography.SHA256]::Create(); $b=[Text.Encoding]::UTF8.GetBytes($ident); $eh=($sha.ComputeHash($b)|ForEach-Object{$_.ToString('x2')}) -join ''; $replay='false'; try{ if(Test-Path -LiteralPath $qf){ $hit=@(Get-Content -LiteralPath $qf -ErrorAction SilentlyContinue|ForEach-Object{$_.Trim()}|Where-Object{$_ -ne '' -and $_ -eq $base}).Count -gt 0; if($hit){$replay='true'} } }catch{}; Write-Output ($now+'|'+$eh+'|'+$replay)"`) do (
  set "NOW=%%A"
  set "EH=%%B"
  set "REPLAY_FROM_CONFIDENCE=%%C"
)
>"!PREFIX!.rep" echo {"file_name":"!BASE!","email_hash":"!EH!","account_id":"!AID!","status_code":!STATUS!,"probed_at":"!NOW!","replay_from_confidence":!REPLAY_FROM_CONFIDENCE!}
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

:UPSERT_ENV_KEY
setlocal
set "U_FILE=%~1"
set "U_KEY=%~2"
set "U_VAL=%~3"
if "%U_FILE%"=="" exit /b 0
if "%U_KEY%"=="" exit /b 0
if not exist "%U_FILE%" exit /b 0
powershell -NoProfile -Command ^
  "$f='%U_FILE%'; $k='%U_KEY%'; $v='%U_VAL%'; try{$lines=@(Get-Content -LiteralPath $f -ErrorAction Stop)}catch{exit 0}; $done=$false; $out=@(); foreach($ln in $lines){ if($ln -match ('^\s*' + [regex]::Escape($k) + '=')){ $out += ($k + '=' + $v); $done=$true } else { $out += $ln } }; if(-not $done){ $out += ($k + '=' + $v) }; Set-Content -LiteralPath $f -Value $out -Encoding UTF8" >nul 2>nul
endlocal & exit /b 0

:CLEANUP_OLD_OUT
REM 清理 out/ 旧目录，保留最近 10 个
powershell -NoProfile -Command ^
  "$out='%ROOT_DIR%out'; if(-not (Test-Path -LiteralPath $out)){exit 0};" ^
  "$dirs=Get-ChildItem -LiteralPath $out -Directory -ErrorAction SilentlyContinue | Where-Object { $_.Name -ne 'latest' -and $_.Name -ne 'latest-syncall' } | Sort-Object LastWriteTime -Descending;" ^
  "$del=0; if($dirs.Count -gt 10){ $old=$dirs | Select-Object -Skip 10; foreach($d in $old){ try{ Remove-Item -LiteralPath $d.FullName -Recurse -Force -ErrorAction Stop; $del++ }catch{} } };" ^
  "if($del -gt 0){ Write-Output ('[INFO] 已清理 out/ 旧目录: ' + $del) }" >nul 2>nul
exit /b 0

:WRITE_FINAL_REPORT
setlocal
set "R_MODE=%~1"
set "R_STATUS=%~2"
set "R_TOTAL=%~3"
set "R_PROBED=%~4"
set "R_NET=%~5"
set "R_INVALID=%~6"
set "R_OUT=%~7"
if not exist "%ROOT_DIR%out" mkdir "%ROOT_DIR%out" >nul 2>nul
powershell -NoProfile -Command "$o=[ordered]@{ generated_at=(Get-Date).ToString('yyyy-MM-ddTHH:mm:ssK'); mode='%R_MODE%'; status='%R_STATUS%'; total=[int]('%R_TOTAL%'); probed_ok=[int]('%R_PROBED%'); net_fail=[int]('%R_NET%'); invalid_401_429=[int]('%R_INVALID%'); out_dir='%R_OUT%'; final_report='%FINAL_REPORT%' }; if('%REPORT_REPLAY_PERCENT%' -ne ''){ try{ $o.confidence_replay_percent=[int]('%REPORT_REPLAY_PERCENT%') }catch{} }; if('%REPORT_ISSUED_REPLAY%' -ne ''){ try{ $o.issued_replay_count=[int]('%REPORT_ISSUED_REPLAY%') }catch{} }; if('%REPORT_AUTO_DISABLED%' -ne ''){ $o.auto_disabled = [string]('%REPORT_AUTO_DISABLED%') }; if('%REPORT_ABUSE_AUTO_BANNED%' -ne ''){ $o.abuse_auto_banned = [string]('%REPORT_ABUSE_AUTO_BANNED%') }; $o|ConvertTo-Json -Depth 4 | Set-Content -LiteralPath '%FINAL_REPORT%' -Encoding UTF8" >nul 2>nul
endlocal & exit /b 0
