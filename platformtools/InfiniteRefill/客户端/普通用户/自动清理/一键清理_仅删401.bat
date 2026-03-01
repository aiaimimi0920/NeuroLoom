@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM 一键直连清理（仅删 401，无需 Python）
REM - 从 ..\..\..\config.yaml 读取 auth-dir
REM - 直连探测 https://chatgpt.com/backend-api/wham/usage
REM - 仅当 HTTP 401 才会删除
REM - 默认：DryRun（不删除，只生成计划与报告）
REM
REM 用法：
REM   一键清理_仅删401.bat
REM   一键清理_仅删401.bat apply

chcp 65001 >nul

set "SCRIPT_DIR=%~dp0"
set "ROOT_DIR=%SCRIPT_DIR%..\..\.."
set "CFG=%ROOT_DIR%\config.yaml"
set "USER_CFG=%SCRIPT_DIR%..\..\状态\无限续杯配置.env"

set "APPLY=0"
set "NOPAUSE=0"

if /I "%~1"=="apply" set "APPLY=1"
if /I "%~2"=="apply" set "APPLY=1"
if /I "%~3"=="apply" set "APPLY=1"

if /I "%~1"=="nopause" set "NOPAUSE=1"
if /I "%~2"=="nopause" set "NOPAUSE=1"
if /I "%~3"=="nopause" set "NOPAUSE=1"

REM 读取 auth-dir
set "ACCOUNTS_DIR="
if exist "%CFG%" (
  for /f "usebackq delims=" %%A in (`powershell -NoProfile -Command "$c=Get-Content -Raw -LiteralPath '%CFG%'; if($c -match '(?m)^\s*auth-dir\s*:\s*(?:\"([^\"]+)\"|([^#\r\n]+))\s*$'){ $v=($Matches[1] + $Matches[2]).Trim().Replace('\\\\','\\'); Write-Output $v }" 2^>nul`) do (
    if not "%%A"=="" set "ACCOUNTS_DIR=%%A"
  )
)

if "%ACCOUNTS_DIR%"=="" (
  echo [ERROR] 无法从 "%CFG%" 解析 auth-dir。
  echo        请确认 config.yaml 中存在：auth-dir: "C:\\path\\to\\.cli-proxy-api"
  if "%NOPAUSE%"=="1" exit /b 3
  pause
  exit /b 3
)

echo Config:      "%CFG%"
echo AccountsDir: "%ACCOUNTS_DIR%"
if "%APPLY%"=="1" (
  echo Apply:      true  ^(会删除 401 文件；会先备份^)
) else (
  echo Apply:      false ^(DryRun：只生成计划，不删除^)
)
echo.

REM 输出目录
for /f "usebackq delims=" %%T in (`powershell -NoProfile -Command "Get-Date -Format 'yyyyMMdd-HHmmss'"`) do set "TS=%%T"
set "OUT_DIR=%SCRIPT_DIR%out\清理-401-%TS%"
set "PLAN=%OUT_DIR%\计划删除-401.txt"
set "REPORT=%OUT_DIR%\报告.txt"
set "BACKUP=%OUT_DIR%\backup"

if not exist "%OUT_DIR%" mkdir "%OUT_DIR%" >nul 2>nul
if not exist "%BACKUP%" mkdir "%BACKUP%" >nul 2>nul

echo [INFO] 输出目录："%OUT_DIR%"
echo.>%PLAN%
echo.>%REPORT%

set /a TOTAL=0
set /a CAND=0
set /a DELETED=0

for %%F in ("%ACCOUNTS_DIR%\*.json") do (
  set /a TOTAL+=1
  call :处理单个文件 "%%~fF" "%%~nxF"
)

echo.
echo [DONE] total=%TOTAL% candidates_401=%CAND% deleted=%DELETED%
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
set "OUT_REFILL=%SCRIPT_DIR%..\..\状态\清理后补齐结果.jsonl"

REM 组装 body（reports 留空；你也可以按需在这里加上清理报告）
set "BODY=%SCRIPT_DIR%..\..\状态\_topup_body.json"
>"%BODY%" (
  echo {"target_pool_size":%目标数量%,"reports":[]}
)

echo [INFO] 开始补齐：请求一次 topup（目标=%目标数量%），结果写入："%OUT_REFILL%"
for /f "usebackq delims=" %%R in (`curl -sS -X POST "%服务器地址%/v1/refill/topup" -H "X-User-Key: %用户密钥%" -H "Content-Type: application/json" --data-binary "@%BODY%"`) do (
  >>"%OUT_REFILL%" echo %%R
)

echo [OK] 已触发补齐请求
exit /b 0

:处理单个文件
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
if "%AID%"=="" (
  for /f "usebackq delims=" %%S in (`curl -sS -o NUL -w "%%{http_code}" -H "Authorization: Bearer %TOKEN%" -H "Accept: application/json, text/plain, */*" -H "User-Agent: codex_cli_rs/0.76.0 (Windows 11; x86_64)" "https://chatgpt.com/backend-api/wham/usage"`) do set "STATUS=%%S"
) else (
  for /f "usebackq delims=" %%S in (`curl -sS -o NUL -w "%%{http_code}" -H "Authorization: Bearer %TOKEN%" -H "Chatgpt-Account-Id: %AID%" -H "Accept: application/json, text/plain, */*" -H "User-Agent: codex_cli_rs/0.76.0 (Windows 11; x86_64)" "https://chatgpt.com/backend-api/wham/usage"`) do set "STATUS=%%S"
)

>>"%REPORT%" echo [PROBE] %BASE% status=%STATUS%

if "%STATUS%"=="401" (
  set /a CAND+=1
  >>"%PLAN%" echo %FILE%

  if "%APPLY%"=="1" (
    copy /Y "%FILE%" "%BACKUP%\%BASE%" >nul 2>nul
    del /Q "%FILE%" >nul 2>nul
    if not exist "%FILE%" (
      set /a DELETED+=1
      >>"%REPORT%" echo [DEL]  %BASE%
    ) else (
      >>"%REPORT%" echo [FAIL] %BASE%
    )
  )
)

exit /b 0
