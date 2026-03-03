@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM 一键直连清理（可配置删 401/429 + 过期文件，无需 Python）
REM - 仅读取本地 accounts 目录
REM - 直连探测 https://chatgpt.com/backend-api/wham/usage
REM - 可按 CLEAN_DELETE_STATUSES 删除状态码命中项（默认 401,429）
REM - 可按 CLEAN_EXPIRED_DAYS 删除“过期文件”（按文件修改时间）
REM - 默认：DryRun（不删除，只生成计划与报告）
REM
REM 用法：
REM   一键清理失效.bat
REM   一键清理失效.bat apply

chcp 65001 >nul

set "SCRIPT_DIR=%~dp0"
set "ROOT_DIR=%SCRIPT_DIR%..\..\.."
set "CFG=%ROOT_DIR%\config.yaml"
set "USER_CFG=%SCRIPT_DIR%无限续杯配置.env"
set "ROOT_CFG=%ROOT_DIR%\无限续杯配置.env"

set "APPLY=0"
set "NOPAUSE=0"
set "ACCOUNTS_DIR=%SCRIPT_DIR%accounts"
set "CLEAN_DELETE_STATUSES=401,429"
set "CLEAN_EXPIRED_DAYS=30"

if /I "%~1"=="apply" set "APPLY=1"
if /I "%~2"=="apply" set "APPLY=1"
if /I "%~3"=="apply" set "APPLY=1"

if /I "%~1"=="nopause" set "NOPAUSE=1"
if /I "%~2"=="nopause" set "NOPAUSE=1"
if /I "%~3"=="nopause" set "NOPAUSE=1"

if exist "%ROOT_CFG%" (
  for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%ROOT_CFG%") do (
    if /I "%%A"=="ACCOUNTS_DIR" set "ACCOUNTS_DIR=%%B"
    if /I "%%A"=="CLEAN_DELETE_STATUSES" set "CLEAN_DELETE_STATUSES=%%B"
    if /I "%%A"=="CLEAN_EXPIRED_DAYS" set "CLEAN_EXPIRED_DAYS=%%B"
  )
)
if exist "%USER_CFG%" (
  for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%USER_CFG%") do (
    if /I "%%A"=="ACCOUNTS_DIR" set "ACCOUNTS_DIR=%%B"
    if /I "%%A"=="CLEAN_DELETE_STATUSES" set "CLEAN_DELETE_STATUSES=%%B"
    if /I "%%A"=="CLEAN_EXPIRED_DAYS" set "CLEAN_EXPIRED_DAYS=%%B"
  )
)

if "%ACCOUNTS_DIR%"=="" set "ACCOUNTS_DIR=%SCRIPT_DIR%accounts"
if not exist "%ACCOUNTS_DIR%" mkdir "%ACCOUNTS_DIR%" >nul 2>nul
if not exist "%ACCOUNTS_DIR%" (
  echo [ERROR] 账户目录不存在且创建失败："%ACCOUNTS_DIR%"
  if "%NOPAUSE%"=="1" exit /b 3
  pause
  exit /b 3
)

echo Config:      "%USER_CFG%"
echo AccountsDir: "%ACCOUNTS_DIR%"
echo DeleteStatus:"%CLEAN_DELETE_STATUSES%"
echo ExpiredDays:"%CLEAN_EXPIRED_DAYS%"
if "%APPLY%"=="1" (
  echo Apply:      true  ^(会删除命中状态/过期文件；会先备份^)
) else (
  echo Apply:      false ^(DryRun：只生成计划，不删除^)
)
echo.

REM 输出目录
for /f "usebackq delims=" %%T in (`powershell -NoProfile -Command "Get-Date -Format 'yyyyMMdd-HHmmss'"`) do set "TS=%%T"
set "OUT_DIR=%SCRIPT_DIR%out\清理-失效-%TS%"
set "PLAN=%OUT_DIR%\计划删除-失效.txt"
set "REPORT=%OUT_DIR%\报告.txt"
set "BACKUP=%OUT_DIR%\backup"

if not exist "%OUT_DIR%" mkdir "%OUT_DIR%" >nul 2>nul
if not exist "%BACKUP%" mkdir "%BACKUP%" >nul 2>nul

echo [INFO] 输出目录："%OUT_DIR%"
echo.>%PLAN%
echo.>%REPORT%

set /a TOTAL=0
set /a PROBED_OK=0
set /a NET_FAIL=0
set /a CAND=0
set /a DELETED=0

for /f "usebackq delims=" %%F in (`dir /b /a-d "%ACCOUNTS_DIR%\*.json" 2^>nul`) do (
  set /a TOTAL+=1
  call :PROCESS_ONE_FILE "%ACCOUNTS_DIR%\%%F" "%%F"
)

echo.
echo [DONE] total=%TOTAL% probed_ok=%PROBED_OK% net_fail=%NET_FAIL% candidates_matched=%CAND% deleted=%DELETED%
echo 计划文件："%PLAN%"
echo 报告文件："%REPORT%"
echo.

REM 清理后自动补齐（可选）：仅在 apply 且确实删除了文件时触发
if "%APPLY%"=="1" (
  if not "%DELETED%"=="0" (
    call :清理后自动补齐
  )
)

if "%NOPAUSE%"=="1" exit /b 0
pause
exit /b 0

:清理后自动补齐
REM 读取用户配置（SERVER_URL/USER_KEY/TARGET_POOL_SIZE/AUTO_REFILL_AFTER_CLEAN）
set "服务器地址="
set "用户密钥="
set "目标数量="
set "自动补齐="

if exist "%USER_CFG%" (
  for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%USER_CFG%") do (
    if /I "%%A"=="SERVER_URL" set "服务器地址=%%B"
    if /I "%%A"=="USER_KEY" set "用户密钥=%%B"
    if /I "%%A"=="TARGET_POOL_SIZE" set "目标数量=%%B"
    if /I "%%A"=="AUTO_REFILL_AFTER_CLEAN" set "自动补齐=%%B"
  )
)

if "%目标数量%"=="" set "目标数量=10"
if "%自动补齐%"=="" set "自动补齐=0"

REM 统计当前 codex 文件数量（文件口径=可用账户数量）
set "剩余数量=0"
for /f "usebackq delims=" %%C in (`powershell -NoProfile -Command "try{$n=(Get-ChildItem -LiteralPath '%ACCOUNTS_DIR%' -Filter '*.json' -File | ForEach-Object { try{ (Get-Content -Raw -LiteralPath $_.FullName | ConvertFrom-Json).type }catch{ '' } } | Where-Object { $_ -eq 'codex' } | Measure-Object).Count; Write-Output $n}catch{Write-Output 0}"`) do set "剩余数量=%%C"

set /a 缺口=%目标数量% - %剩余数量%
if %缺口% LEQ 0 (
  echo [INFO] 清理后剩余=%剩余数量% 目标=%目标数量%：无需补齐
  exit /b 0
)

echo [INFO] 清理后剩余=%剩余数量% 目标=%目标数量%：缺口=%缺口%

if not "%自动补齐%"=="1" (
  echo [INFO] 未开启自动补齐（AUTO_REFILL_AFTER_CLEAN=1 才会自动续杯）
  exit /b 0
)

if "%服务器地址%"=="" (
  echo [WARN] 未配置 SERVER_URL，无法自动补齐
  exit /b 0
)
if "%用户密钥%"=="" (
  echo [WARN] 未配置 USER_KEY，无法自动补齐
  exit /b 0
)

REM 触发续杯请求：这里调用 /v1/refill/topup（你将自行实现为“返回账号 JSON”）
set "OUT_REFILL=%SCRIPT_DIR%out\清理后补齐结果.jsonl"

REM 组装 body（reports 留空；你也可以按需在这里加上清理报告）
set "BODY=%SCRIPT_DIR%out\_topup_body.json"
>"%BODY%" (
  echo {"target_pool_size":%目标数量%,"reports":[]}
)

echo [INFO] 开始补齐：请求一次 topup（目标=%目标数量%），结果写入："%OUT_REFILL%"
for /f "usebackq delims=" %%R in (`curl -sS -X POST "%服务器地址%/v1/refill/topup" -H "X-User-Key: %用户密钥%" -H "Content-Type: application/json" --data-binary "@%BODY%"`) do (
  >>"%OUT_REFILL%" echo %%R
)

echo [OK] 已触发补齐请求
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

:PROCESS_ONE_FILE
set "FILE=%~1"
set "BASE=%~2"

REM 用 PowerShell 解析 JSON（稳定、内置）
set "TYPE="
set "TOKEN="
set "AID="

for /f "usebackq tokens=1,2,3 delims=|" %%A in (`powershell -NoProfile -Command "try{$j=Get-Content -Raw -LiteralPath '%FILE%'; $o=$j|ConvertFrom-Json; $ty=($o.type|ForEach-Object{$_.ToString()}); $tok=($o.access_token|ForEach-Object{$_.ToString()}); $id=($o.account_id|ForEach-Object{$_.ToString()}); Write-Output ($ty + '|' + $tok + '|' + $id)}catch{Write-Output '||'}"`) do (
  set "TYPE=%%A"
  set "TOKEN=%%B"
  set "AID=%%C"
)

if /I not "%TYPE%"=="codex" (
  >>"%REPORT%" echo [SKIP] %BASE% (type=%TYPE%)
  exit /b 0
)

if "%TOKEN%"=="" (
  >>"%REPORT%" echo [SKIP] %BASE% (missing access_token)
  exit /b 0
)

set "STATUS="
set "WHAM_BODY=%OUT_DIR%\_wham_body.tmp"
if "%AID%"=="" (
  for /f "usebackq delims=" %%S in (`curl -sS -o "%WHAM_BODY%" -w "%%{http_code}" -H "Authorization: Bearer %TOKEN%" -H "Accept: application/json, text/plain, */*" -H "User-Agent: codex_cli_rs/0.76.0 (Windows 11; x86_64)" "https://chatgpt.com/backend-api/wham/usage"`) do set "STATUS=%%S"
) else (
  for /f "usebackq delims=" %%S in (`curl -sS -o "%WHAM_BODY%" -w "%%{http_code}" -H "Authorization: Bearer %TOKEN%" -H "Chatgpt-Account-Id: %AID%" -H "Accept: application/json, text/plain, */*" -H "User-Agent: codex_cli_rs/0.76.0 (Windows 11; x86_64)" "https://chatgpt.com/backend-api/wham/usage"`) do set "STATUS=%%S"
)
if "%STATUS%"=="" set "STATUS=000"
echo %STATUS%| findstr /R "^[0-9][0-9][0-9]$" >nul 2>nul
if errorlevel 1 set "STATUS=000"
if "%STATUS%"=="000" (
  set /a NET_FAIL+=1
  >>"%REPORT%" echo [NETFAIL] %BASE% status=000
  exit /b 0
)

call :NORMALIZE_INVALID_STATUS "%STATUS%" "%WHAM_BODY%" STATUS

set /a PROBED_OK+=1
>>"%REPORT%" echo [PROBE] %BASE% status=%STATUS%

set "DELETE_REASON="

echo ,%CLEAN_DELETE_STATUSES:,=,% | find ",%STATUS%," >nul 2>nul
if not errorlevel 1 set "DELETE_REASON=status=%STATUS%"

set "EXP_DAYS=%CLEAN_EXPIRED_DAYS%"
for /f "usebackq delims=" %%M in (`powershell -NoProfile -Command "$f='%FILE%'; $d=[int]('%EXP_DAYS%'); if($d -gt 0 -and (Test-Path -LiteralPath $f)){ $m=(Get-Item -LiteralPath $f).LastWriteTimeUtc; $n=(Get-Date).ToUniversalTime(); [int][Math]::Floor(($n-$m).TotalDays) } else { -1 }"`) do set "AGE_DAYS=%%M"
if not "%AGE_DAYS%"=="-1" (
  for /f "tokens=* delims= " %%Z in ("%AGE_DAYS%") do set "AGE_DAYS=%%Z"
  if %AGE_DAYS% GEQ %EXP_DAYS% (
    if "%DELETE_REASON%"=="" (
      set "DELETE_REASON=expired=%AGE_DAYS%d"
    ) else (
      set "DELETE_REASON=%DELETE_REASON%,expired=%AGE_DAYS%d"
    )
  )
)

if not "%DELETE_REASON%"=="" (
  set /a CAND+=1
  >>"%PLAN%" echo %FILE% # %DELETE_REASON%

  if "%APPLY%"=="1" (
    copy /Y "%FILE%" "%BACKUP%\%BASE%" >nul 2>nul
    del /Q "%FILE%" >nul 2>nul
    if not exist "%FILE%" (
      set /a DELETED+=1
      >>"%REPORT%" echo [DEL]  %BASE% reason=%DELETE_REASON%
    ) else (
      >>"%REPORT%" echo [FAIL] %BASE% reason=%DELETE_REASON%
    )
  )
)

exit /b 0
