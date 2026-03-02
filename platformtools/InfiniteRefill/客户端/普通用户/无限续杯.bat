@echo off
setlocal EnableExtensions EnableDelayedExpansion
chcp 65001 >nul

set "SCRIPT_DIR=%~dp0"
set "CFG_DIR=%SCRIPT_DIR%"
set "CFG=%CFG_DIR%无限续杯配置.env"
set "ROOT_DIR=%SCRIPT_DIR%..\.."
set "ROOT_CFG=%ROOT_DIR%\无限续杯配置.env"

set "REFILL_TASK=无限续杯_定时任务"
set "CLEAN_TASK=自动清理_定时任务"

if not exist "%CFG_DIR%" mkdir "%CFG_DIR%" >nul 2>nul
set "ACTIVE_CFG="
if exist "%CFG%" set "ACTIVE_CFG=%CFG%"
if "%ACTIVE_CFG%"=="" if exist "%ROOT_CFG%" set "ACTIVE_CFG=%ROOT_CFG%"
if "%ACTIVE_CFG%"=="" (
  >"%CFG%" (
    echo # 无限续杯配置（本地文件）
    echo # 注意：请勿分享/上传此文件。
    echo SERVER_URL=
    echo USER_KEY=
    echo ACCOUNTS_DIR=%ROOT_DIR%\accounts
    echo TARGET_POOL_SIZE=10
    echo TRIGGER_REMAINING=2
    echo INTERVAL_MINUTES=30
    echo AUTO_REFILL_AFTER_CLEAN=1
    echo AUTO_CLEAN_INTERVAL_MINUTES=30
    echo AUTO_CLEAN_APPLY=1
    echo CLEAN_DELETE_STATUSES=401,429
    echo CLEAN_EXPIRED_DAYS=30
    echo SYNC_MODE=none
    echo SYNC_TARGET_DIR=
  )
  set "ACTIVE_CFG=%CFG%"
)
 
REM 兼容：无限续杯.bat 服务器地址 用户密钥 -> 直接单次续杯
set "服务器地址=%~1"
set "用户密钥=%~2"
if not "%服务器地址%"=="" (
  if not "%用户密钥%"=="" (
    call "%SCRIPT_DIR%单次续杯.bat" "%服务器地址%" "%用户密钥%"
    exit /b %ERRORLEVEL%
  )
)

:MENU
echo.
echo ====== 无限续杯（配置入口 / Windows）======
echo 配置文件："%ACTIVE_CFG%"
echo.
echo 1) 立即执行一次【单次续杯】（使用已保存配置）
echo 2) 设置/更新【无限续杯配置】（服务器地址/用户密钥/间隔）
echo 3) 开启/更新【定时续杯】计划任务（从配置读取，不在任务里保存密钥）
echo 4) 关闭【定时续杯】计划任务
echo.
echo 5) 同步所有账号（谨慎：高频调用会触发风控）
echo 6) 退出
echo.
set /p 选择=请选择 (1-6，默认 3)：
if "!选择!"=="" set "选择=3"

if "!选择!"=="1" goto :RUN_ONCE
if "!选择!"=="2" goto :CONFIG
if "!选择!"=="3" goto :ENABLE_TASK
if "!选择!"=="4" goto :DISABLE_TASK
if "!选择!"=="5" goto :SYNC_ALL
if "!选择!"=="6" goto :EOF

echo [WARN] 无效选择：!选择!
goto :MENU

:RUN_ONCE
call "%SCRIPT_DIR%单次续杯.bat"
set "RUN_EC=%ERRORLEVEL%"
if "%RUN_EC%"=="0" (
  call :RESET_TASK_AFTER_MANUAL
)
pause
goto :MENU

:SYNC_ALL
call "%SCRIPT_DIR%单次续杯.bat" --sync-all
pause
goto :MENU

:CONFIG
echo.
echo ====== 设置无限续杯配置 ======
set "默认服务器地址="
set "默认用户密钥="
set "默认账户目录=%SCRIPT_DIR%accounts"
for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%ACTIVE_CFG%") do (
  if /I "%%A"=="SERVER_URL" set "默认服务器地址=%%B"
  if /I "%%A"=="USER_KEY" set "默认用户密钥=%%B"
)
if not exist "!默认账户目录!" (
  for /d %%D in ("%SCRIPT_DIR%*") do (
    if exist "%%~fD\accounts" (
      set "默认账户目录=%%~fD\accounts"
      goto :ACCOUNTS_DIR_DETECTED
    )
  )
)
:ACCOUNTS_DIR_DETECTED
if not exist "!默认账户目录!" mkdir "!默认账户目录!" >nul 2>nul
set /p 服务器地址=请输入服务器地址（填空则使用默认值：!默认服务器地址!）:
if "!服务器地址!"=="" set "服务器地址=!默认服务器地址!"
set /p 用户密钥=请输入用户密钥（填空则使用默认值：!默认用户密钥!）:
if "!用户密钥!"=="" set "用户密钥=!默认用户密钥!"

set "检测同步目录="
if exist "%USERPROFILE%\.cli-proxy-api" set "检测同步目录=%USERPROFILE%\.cli-proxy-api"
if "!检测同步目录!"=="" if exist "%USERPROFILE%\cli-proxy-api" set "检测同步目录=%USERPROFILE%\cli-proxy-api"
if "!检测同步目录!"=="" set "检测同步目录=%USERPROFILE%\.cli-proxy-api"
echo [INFO] 检测到默认同步目录：!检测同步目录!
set /p 是否同步=是否同步到CLI目录（y/N）:
if /I "!是否同步!"=="y" (
  set "同步模式=symlink"
  set /p 同步目录=请选择同步目录（填空则使用默认值：!检测同步目录!）:
  if "!同步目录!"=="" set "同步目录=!检测同步目录!"
) else (
  set "同步模式=none"
  set "同步目录="
)
set /p 间隔分钟=请输入续杯间隔（分钟，最低 10，默认 30）:
if "!间隔分钟!"=="" set "间隔分钟=30"
for /f "delims=0123456789" %%I in ("!间隔分钟!") do set "间隔分钟=30"
if !间隔分钟! LSS 10 (
  REM 过频繁的探测额度，容易被封号
  echo [WARN] 续杯间隔过低，已强制调整为 10 分钟。
  set "间隔分钟=10"
)
set "清理间隔分钟=!间隔分钟!"

if "!服务器地址!"=="" (
  echo [ERROR] 服务器地址不能为空
  pause
  goto :MENU
)
if "!用户密钥!"=="" (
  echo [ERROR] 用户密钥不能为空
  pause
  goto :MENU
)

if "%ACTIVE_CFG%"=="" set "ACTIVE_CFG=%CFG%"
>"%ACTIVE_CFG%" (
  echo # 无限续杯配置（本地文件）
  echo # 注意：请勿分享/上传此文件。
  echo SERVER_URL=!服务器地址!
  echo USER_KEY=!用户密钥!
  echo ACCOUNTS_DIR=!默认账户目录!
  echo TARGET_POOL_SIZE=10
  echo TRIGGER_REMAINING=2
  echo INTERVAL_MINUTES=!间隔分钟!
  echo AUTO_REFILL_AFTER_CLEAN=1
  echo AUTO_CLEAN_INTERVAL_MINUTES=!清理间隔分钟!
  echo AUTO_CLEAN_APPLY=1
  echo CLEAN_DELETE_STATUSES=401,429
  echo CLEAN_EXPIRED_DAYS=30
  echo SYNC_MODE=!同步模式!
  echo SYNC_TARGET_DIR=!同步目录!
)

echo [OK] 已保存："%ACTIVE_CFG%"
call :ENSURE_SYNC_LINKS "!同步模式!" "!同步目录!" "!默认账户目录!"
pause
goto :MENU

:ENABLE_TASK
echo [INFO] 清理历史/遗漏定时任务（仅保留当前任务名）...
call :CLEANUP_OLD_TASKS
REM 从配置中读取间隔与清理策略（无限续杯依赖自动清理，固定 apply 删除）
set "间隔分钟="
set "同步模式=none"
set "同步目录="
set "账户目录=%SCRIPT_DIR%accounts"
for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%ACTIVE_CFG%") do (
  if /I "%%A"=="INTERVAL_MINUTES" set "间隔分钟=%%B"
  if /I "%%A"=="SYNC_MODE" set "同步模式=%%B"
  if /I "%%A"=="SYNC_TARGET_DIR" set "同步目录=%%B"
  if /I "%%A"=="ACCOUNTS_DIR" set "账户目录=%%B"
)
if "!间隔分钟!"=="" set "间隔分钟=30"
for /f "delims=0123456789" %%I in ("!间隔分钟!") do set "间隔分钟=30"
if !间隔分钟! LSS 10 (
  REM 过频繁的探测额度，容易被封号
  echo [WARN] 配置中的续杯间隔过低，已强制调整为 10 分钟。
  set "间隔分钟=10"
)
set "清理间隔分钟=!间隔分钟!"

call :CALC_START_TIME !间隔分钟!
set "TR=powershell -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -Command ""& '%SCRIPT_DIR%单次续杯.bat' --from-task"""

echo.
echo [INFO] 正在创建/更新计划任务：%REFILL_TASK%
schtasks /Create /F /TN "%REFILL_TASK%" /SC MINUTE /MO !间隔分钟! /ST !TASK_START! /TR "!TR!" /RL HIGHEST >nul 2>nul
if errorlevel 1 (
  echo [WARN] 创建失败（可能需要管理员权限）。
  pause
  goto :MENU
)

set "CLEAN_TR=powershell -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -Command ""& '%SCRIPT_DIR%_内部_自动清理.bat' apply nopause"""

schtasks /Create /F /TN "%CLEAN_TASK%" /SC MINUTE /MO !清理间隔分钟! /ST !TASK_START! /TR "!CLEAN_TR!" /RL HIGHEST >nul 2>nul
if errorlevel 1 (
  echo [WARN] 自动清理任务创建失败：无限续杯依赖自动清理，请以管理员权限重试。
  pause
  goto :MENU
)

echo [OK] 已创建/更新：%REFILL_TASK%（每 !间隔分钟! 分钟执行一次）
echo [OK] 已创建/更新：%CLEAN_TASK%（每 !清理间隔分钟! 分钟执行一次，固定 apply 删除）
call :ENSURE_SYNC_LINKS "!同步模式!" "!同步目录!" "!账户目录!"
echo [INFO] 再次清理历史/遗漏定时任务...
call :CLEANUP_OLD_TASKS
pause
goto :MENU

:ENSURE_SYNC_LINKS
setlocal EnableDelayedExpansion
set "_MODE=%~1"
set "_TARGET=%~2"
set "_ACCOUNTS=%~3"
if /I not "!_MODE!"=="symlink" exit /b 0
if "!_TARGET!"=="" exit /b 0
if "!_ACCOUNTS!"=="" set "_ACCOUNTS=%SCRIPT_DIR%accounts"
if not exist "!_TARGET!" mkdir "!_TARGET!" >nul 2>nul
if not exist "!_ACCOUNTS!" mkdir "!_ACCOUNTS!" >nul 2>nul

for /f "usebackq delims=" %%L in (`powershell -NoProfile -Command ^
  "$ErrorActionPreference='Stop'; $accounts='%_ACCOUNTS%'; $target='%_TARGET%'; $manifest=Join-Path $target '.infinite_refill_sync_manifest.txt';" ^
  "$src=@(Get-ChildItem -LiteralPath $accounts -Filter '无限续杯-*.json' -File -ErrorAction SilentlyContinue); if($src.Count -eq 0){$src=@(Get-ChildItem -LiteralPath $accounts -Filter '*.json' -File -ErrorAction SilentlyContinue)};" ^
  "$names=@(); foreach($f in $src){ $names += $f.Name };" ^
  "$old=@(); if(Test-Path -LiteralPath $manifest){ $old=@(Get-Content -LiteralPath $manifest -ErrorAction SilentlyContinue | Where-Object { $_ -and $_.Trim() -ne '' }) };" ^
  "$removed=0; foreach($n in $old){ if($names -notcontains $n){ $tp=Join-Path $target $n; if(Test-Path -LiteralPath $tp){ $it=Get-Item -LiteralPath $tp -Force -ErrorAction SilentlyContinue; if($it -and (($it.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)){ Remove-Item -LiteralPath $tp -Force -ErrorAction SilentlyContinue; $removed++ } } } };" ^
  "$linked=0; foreach($f in $src){ $tp=Join-Path $target $f.Name; if(Test-Path -LiteralPath $tp){ $it=Get-Item -LiteralPath $tp -Force -ErrorAction SilentlyContinue; if($it -and (($it.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)){ Remove-Item -LiteralPath $tp -Force -ErrorAction SilentlyContinue }; if(Test-Path -LiteralPath $tp){ continue } }; New-Item -ItemType SymbolicLink -Path $tp -Target $f.FullName -Force -ErrorAction SilentlyContinue | Out-Null; if(Test-Path -LiteralPath $tp){ $linked++ } };" ^
  "Set-Content -LiteralPath $manifest -Value $names -Encoding UTF8;" ^
  "if($linked -gt 0){ 'OK: linked=' + $linked + '; removed=' + $removed } else { 'WARN: linked=0; removed=' + $removed + '; source=' + $accounts }"`) do echo %%L
exit /b 0

:AUTO_SYNC_HEAL_FROM_CFG
setlocal EnableDelayedExpansion
set "_M=none"
set "_T="
set "_A=%SCRIPT_DIR%accounts"
if not "%ACTIVE_CFG%"=="" if exist "%ACTIVE_CFG%" (
  for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%ACTIVE_CFG%") do (
    if /I "%%A"=="SYNC_MODE" set "_M=%%B"
    if /I "%%A"=="SYNC_TARGET_DIR" set "_T=%%B"
    if /I "%%A"=="ACCOUNTS_DIR" set "_A=%%B"
  )
)
endlocal & call :ENSURE_SYNC_LINKS "%_M%" "%_T%" "%_A%" >nul 2>nul
exit /b 0

:DISABLE_TASK
echo.
echo [INFO] 正在关闭计划任务：%REFILL_TASK%
schtasks /Delete /F /TN "%REFILL_TASK%" >nul 2>nul
if errorlevel 1 (
  echo [WARN] 关闭失败（可能任务不存在或需要管理员权限）。
) else (
  echo [OK] 已关闭：%REFILL_TASK%
)

echo [INFO] 正在关闭计划任务：%CLEAN_TASK%
schtasks /Delete /F /TN "%CLEAN_TASK%" >nul 2>nul
if errorlevel 1 (
  echo [WARN] 关闭失败（可能任务不存在或需要管理员权限）。
) else (
  echo [OK] 已关闭：%CLEAN_TASK%
)

echo [INFO] 再次清理历史/遗漏定时任务...
call :CLEANUP_OLD_TASKS
pause
goto :MENU

:CLEANUP_OLD_TASKS
setlocal EnableDelayedExpansion
for /f "usebackq delims=" %%L in (`powershell -NoProfile -Command ^
  "$ErrorActionPreference='SilentlyContinue';" ^
  "$keepRef='\' + '%REFILL_TASK%'; $keepClean='\' + '%CLEAN_TASK%';" ^
  "$cand=Get-ScheduledTask | Where-Object { $_.TaskName -like '*无限续杯*' -or $_.TaskName -like '*自动清理*' -or (($_.Actions | Out-String) -match '单次续杯\.bat|_内部_自动清理\.bat') };" ^
  "$del=0; foreach($t in $cand){ $full=($t.TaskPath + $t.TaskName); if($full -ieq $keepRef -or $full -ieq $keepClean){ continue }; try{ Unregister-ScheduledTask -TaskName $t.TaskName -TaskPath $t.TaskPath -Confirm:$false -ErrorAction Stop | Out-Null; $del++ } catch {} };" ^
  "'INFO: cleaned_old_tasks=' + $del"`) do echo %%L
endlocal & exit /b 0

:CALC_START_TIME
setlocal
set "_MIN=%~1"
if "%_MIN%"=="" set "_MIN=30"
for /f "delims=0123456789" %%I in ("%_MIN%") do set "_MIN=30"
for /f "usebackq delims=" %%T in (`powershell -NoProfile -Command "(Get-Date).AddMinutes([int]('%_MIN%')).ToString('HH:mm')"`) do set "_TS=%%T"
endlocal & set "TASK_START=%_TS%" & exit /b 0

:RESET_TASK_AFTER_MANUAL
setlocal EnableDelayedExpansion
schtasks /Query /TN "%REFILL_TASK%" >nul 2>nul
if errorlevel 1 (
  endlocal & exit /b 0
)
set "间隔分钟=30"
if not "%ACTIVE_CFG%"=="" if exist "%ACTIVE_CFG%" (
  for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%ACTIVE_CFG%") do (
    if /I "%%A"=="INTERVAL_MINUTES" set "间隔分钟=%%B"
  )
)
for /f "delims=0123456789" %%I in ("!间隔分钟!") do set "间隔分钟=30"
if !间隔分钟! LSS 10 set "间隔分钟=10"
set "清理间隔分钟=!间隔分钟!"
call :CALC_START_TIME !间隔分钟!
set "TR=powershell -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -Command ""& '%SCRIPT_DIR%单次续杯.bat' --from-task"""
set "CLEAN_TR=powershell -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -Command ""& '%SCRIPT_DIR%_内部_自动清理.bat' apply nopause"""
schtasks /Create /F /TN "%REFILL_TASK%" /SC MINUTE /MO !间隔分钟! /ST !TASK_START! /TR "!TR!" /RL HIGHEST >nul 2>nul
schtasks /Create /F /TN "%CLEAN_TASK%" /SC MINUTE /MO !清理间隔分钟! /ST !TASK_START! /TR "!CLEAN_TR!" /RL HIGHEST >nul 2>nul
echo [INFO] 已按手动续杯时间重置下次自动续杯时间：!TASK_START!
call :CLEANUP_OLD_TASKS
endlocal & exit /b 0
