@echo off
setlocal EnableExtensions EnableDelayedExpansion
chcp 65001 >nul

set "SCRIPT_DIR=%~dp0"
REM 自动检测：全平台版本（在 windows/ 子目录）vs 分平台版本（在根目录）
set "ROOT_DIR=%SCRIPT_DIR%"
for %%I in ("%SCRIPT_DIR:~0,-1%") do (
  if /I "%%~nxI"=="windows" set "ROOT_DIR=%SCRIPT_DIR%..\"
)
for %%I in ("%ROOT_DIR%.") do set "ROOT_DIR=%%~fI\"
set "CFG=%ROOT_DIR%无限续杯配置.env"

REM 读取 USER_KEY 并计算 hash 前缀（用于定时任务名称隔离）
set "_UK_HASH="
if exist "%CFG%" (
  for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%CFG%") do (
    if /I "%%A"=="USER_KEY" set "_UK_RAW=%%B"
  )
)
if not "!_UK_RAW!"=="" (
  for /f "usebackq delims=" %%H in (`powershell -NoProfile -Command "$s='!_UK_RAW!'; $sha=[System.Security.Cryptography.SHA256]::Create(); $b=[Text.Encoding]::UTF8.GetBytes($s); (($sha.ComputeHash($b)|ForEach-Object{$_.ToString('x2')}) -join '').Substring(0,6)"`) do set "_UK_HASH=%%H"
)
if "!_UK_HASH!"=="" set "_UK_HASH=000000"

set "REFILL_TASK=无限续杯_定时任务_!_UK_HASH!"
set "CLEAN_TASK=自动清理_定时任务_!_UK_HASH!"

set "ACTIVE_CFG="
if exist "%CFG%" set "ACTIVE_CFG=%CFG%"
if "%ACTIVE_CFG%"=="" (
  >"%CFG%" (
    echo # 无限续杯配置（简化模板）
    echo # 注意：请勿分享/上传此文件（含密钥）。
    echo SERVER_URL=
    echo USER_KEY=
    echo ACCOUNTS_DIR=%ROOT_DIR%accounts
    echo TARGET_POOL_SIZE=10
    echo TOTAL_HOLD_LIMIT=50
    echo INTERVAL_MINUTES=30
    echo SYNC_TARGET_DIR=
  )
  set "ACTIVE_CFG=%CFG%"
)

where powershell >nul 2>nul
if errorlevel 1 (
  echo [ERROR] 当前系统缺少 PowerShell，无法运行客户端。
  pause
  exit /b 6
)

REM 兼容：无限续杯.bat 服务器地址 用户密钥 -> 直接单次续杯
set "_CLI_URL=%~1"
set "_CLI_KEY=%~2"
if not "%_CLI_URL%"=="" (
  if not "%_CLI_KEY%"=="" (
    call "%SCRIPT_DIR%单次续杯.bat" "%_CLI_URL%" "%_CLI_KEY%"
    exit /b !ERRORLEVEL!
  )
)

:MENU
echo.
echo ====== 无限续杯（配置入口 / Windows）======
echo 配置文件："%ACTIVE_CFG%"
echo.
echo 1) 立即执行一次【单次续杯】（使用已保存配置）
echo 2) 设置/更新【无限续杯配置】（服务器地址/用户密钥/间隔）
echo 3) 开启/更新【定时续杯】计划任务（单任务串行：先清理后续杯）
echo 4) 关闭【定时续杯】计划任务
echo.
echo 5) 同步所有账号（谨慎：高频调用会触发风控）
echo 6) 退出
echo.
set /p _CHOICE=请选择 (1-6，默认 3)：
if "!_CHOICE!"=="" set "_CHOICE=3"

if "!_CHOICE!"=="1" goto :RUN_ONCE
if "!_CHOICE!"=="2" goto :CONFIG
if "!_CHOICE!"=="3" goto :ENABLE_TASK
if "!_CHOICE!"=="4" goto :DISABLE_TASK
if "!_CHOICE!"=="5" goto :SYNC_ALL
if "!_CHOICE!"=="6" goto :EOF

echo [WARN] 无效选择：!_CHOICE!
goto :MENU

:RUN_ONCE
call "%SCRIPT_DIR%单次续杯.bat"
set "RUN_EC=!ERRORLEVEL!"
if "!RUN_EC!"=="0" (
  schtasks /Query /TN "%REFILL_TASK%" >nul 2>nul
  if not errorlevel 1 (
    call :RESET_TASK_AFTER_MANUAL
  )
) else (
  if "!RUN_EC!"=="4" (
    echo [WARN] 服务端已自动禁用当前用户（超日限）。已自动关闭本机定时续杯任务。
    call :DISABLE_TASK_SILENT
  )
  if "!RUN_EC!"=="5" (
    echo [WARN] 服务端判定滥用并自动封禁。已自动关闭本机定时续杯任务。
    call :DISABLE_TASK_SILENT
  )
)
goto :MENU

:SYNC_ALL
call "%SCRIPT_DIR%单次续杯.bat" --sync-all
goto :MENU

:CONFIG
echo.
echo ====== 设置无限续杯配置 ======
set "_DEF_URL="
set "_DEF_KEY="
set "_DEF_HOLD=50"
set "_IS_FULL_PACKAGE=0"
for %%I in ("%SCRIPT_DIR:~0,-1%") do if /I "%%~nxI"=="windows" set "_IS_FULL_PACKAGE=1"
if "!_IS_FULL_PACKAGE!"=="1" (
  set "_DEF_ACCOUNTS=%ROOT_DIR%accounts"
) else (
  set "_DEF_ACCOUNTS=%SCRIPT_DIR%accounts"
)
for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%ACTIVE_CFG%") do (
  if /I "%%A"=="SERVER_URL" set "_DEF_URL=%%B"
  if /I "%%A"=="USER_KEY" set "_DEF_KEY=%%B"
  if /I "%%A"=="ACCOUNTS_DIR" if not "%%B"=="" set "_DEF_ACCOUNTS=%%B"
  if /I "%%A"=="TOTAL_HOLD_LIMIT" set "_DEF_HOLD=%%B"
)
if "!_DEF_ACCOUNTS!"=="" (
  if "!_IS_FULL_PACKAGE!"=="1" (
    set "_DEF_ACCOUNTS=%ROOT_DIR%accounts"
  ) else (
    set "_DEF_ACCOUNTS=%SCRIPT_DIR%accounts"
  )
)
if not exist "!_DEF_ACCOUNTS!" mkdir "!_DEF_ACCOUNTS!" >nul 2>nul
for %%I in ("!_DEF_ACCOUNTS!") do set "_DEF_ACCOUNTS=%%~fI"
set /p _IN_URL=请输入服务器地址（填空则使用默认值：!_DEF_URL!）:
if "!_IN_URL!"=="" set "_IN_URL=!_DEF_URL!"
set /p _IN_KEY=请输入用户密钥（填空则使用默认值：!_DEF_KEY!）:
if "!_IN_KEY!"=="" set "_IN_KEY=!_DEF_KEY!"
set /p _IN_ACCOUNTS=请输入账号文件保存路径（ACCOUNTS_DIR，填空则使用默认值：!_DEF_ACCOUNTS!）:
if "!_IN_ACCOUNTS!"=="" set "_IN_ACCOUNTS=!_DEF_ACCOUNTS!"
if not exist "!_IN_ACCOUNTS!" mkdir "!_IN_ACCOUNTS!" >nul 2>nul
for %%I in ("!_IN_ACCOUNTS!") do set "_IN_ACCOUNTS=%%~fI"

set "_SYNC_DETECT="
if exist "%USERPROFILE%\.cli-proxy-api\" (
  set "_SYNC_DETECT=%USERPROFILE%\.cli-proxy-api"
) else if exist "%USERPROFILE%\cli-proxy-api\" (
  set "_SYNC_DETECT=%USERPROFILE%\cli-proxy-api"
) else (
  set "_SYNC_DETECT=%USERPROFILE%\.cli-proxy-api"
)
for %%I in ("!_SYNC_DETECT!") do set "_SYNC_DETECT=%%~fI"
set /p _IN_SYNC=请输入同步目录（SYNC_TARGET_DIR，默认：!_SYNC_DETECT!，回车直接使用默认）:
if "!_IN_SYNC!"=="" set "_IN_SYNC=!_SYNC_DETECT!"
for %%I in ("!_IN_SYNC!") do set "_IN_SYNC=%%~fI"
set /p _IN_INTERVAL=请输入续杯间隔（分钟，最低 10，默认 30）:
if "!_IN_INTERVAL!"=="" set "_IN_INTERVAL=30"
for /f "delims=0123456789" %%I in ("!_IN_INTERVAL!") do set "_IN_INTERVAL=30"
if !_IN_INTERVAL! LSS 10 (
  echo [WARN] 续杯间隔过低，已强制调整为 10 分钟。
  set "_IN_INTERVAL=10"
)
set "_HOLD=!_DEF_HOLD!"
for /f "delims=0123456789" %%I in ("!_HOLD!") do set "_HOLD=50"
if !_HOLD! LSS 1 set "_HOLD=50"
if "!_IN_URL!"=="" (
  echo [ERROR] 服务器地址不能为空
  pause
  goto :MENU
)
if "!_IN_KEY!"=="" (
  echo [ERROR] 用户密钥不能为空
  pause
  goto :MENU
)

if "%ACTIVE_CFG%"=="" set "ACTIVE_CFG=%CFG%"
>"%ACTIVE_CFG%" (
  echo # 无限续杯配置（简化模板）
  echo # 注意：请勿分享/上传此文件（含密钥）。
  echo SERVER_URL=!_IN_URL!
  echo USER_KEY=!_IN_KEY!
  echo ACCOUNTS_DIR=!_IN_ACCOUNTS!
  echo TARGET_POOL_SIZE=10
  echo TOTAL_HOLD_LIMIT=!_HOLD!
  echo INTERVAL_MINUTES=!_IN_INTERVAL!
  echo SYNC_TARGET_DIR=!_IN_SYNC!
)

echo [OK] 已保存："%ACTIVE_CFG%"
call :DO_SYNC "!_IN_SYNC!" "!_IN_ACCOUNTS!"
pause
goto :MENU

:ENABLE_TASK
echo [INFO] 清理历史/遗漏定时任务（仅保留当前任务名）...
call :CLEANUP_OLD_TASKS
REM 从配置中读取间隔
set "_INTERVAL="
set "_SYNC_DIR="
set "_ACC_DIR=%ROOT_DIR%accounts"
for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%ACTIVE_CFG%") do (
  if /I "%%A"=="INTERVAL_MINUTES" set "_INTERVAL=%%B"
  if /I "%%A"=="SYNC_TARGET_DIR" set "_SYNC_DIR=%%B"
  if /I "%%A"=="ACCOUNTS_DIR" set "_ACC_DIR=%%B"
)
if "!_INTERVAL!"=="" set "_INTERVAL=30"
for /f "delims=0123456789" %%I in ("!_INTERVAL!") do set "_INTERVAL=30"
if !_INTERVAL! LSS 10 (
  echo [WARN] 配置中的续杯间隔过低，已强制调整为 10 分钟。
  set "_INTERVAL=10"
)

REM 生成定时任务入口 .bat（必须用系统原生编码，不能用 UTF-8；否则 CMD 读取时中文乱码）
REM 注意：不再调用 _内部_自动清理.bat（单次续杯.bat 已内含探测+清理+补齐全流程）
set "TASK_ENTRY_BAT=!SCRIPT_DIR!_定时任务_入口.bat"
cmd /c "chcp 936 >nul & (echo @echo off & echo cd /d "%%~dp0" & echo chcp 65001 ^>nul & echo call 单次续杯.bat --from-task) > "!TASK_ENTRY_BAT!""

REM 检测 conhost --headless 支持（Win10 1903+），不支持则回退 PowerShell
call :BUILD_SILENT_TR "!TASK_ENTRY_BAT!"

echo.
echo [INFO] 正在创建/更新计划任务：%REFILL_TASK%
schtasks /Create /F /TN "%REFILL_TASK%" /SC MINUTE /MO !_INTERVAL! /TR "!SILENT_TR!" /RL HIGHEST
if errorlevel 1 (
  echo [WARN] 创建失败（可能需要管理员权限）。
  pause
  goto :MENU
)

REM 清理旧的独立自动清理任务（若存在）
schtasks /Delete /F /TN "%CLEAN_TASK%" >nul 2>nul

REM 清理 out/ 旧目录（保留最近 10 个）
call :CLEANUP_OLD_OUT

echo [OK] 已创建/更新：%REFILL_TASK%（每 !_INTERVAL! 分钟执行一次）
call :DO_SYNC "!_SYNC_DIR!" "!_ACC_DIR!"
echo [INFO] 再次清理历史/遗漏定时任务...
call :CLEANUP_OLD_TASKS
pause
goto :MENU

:DO_SYNC
setlocal EnableDelayedExpansion
set "_TARGET=%~1"
set "_ACCOUNTS=%~2"
if "!_TARGET!"=="" exit /b 0
if "!_ACCOUNTS!"=="" set "_ACCOUNTS=%ROOT_DIR%accounts"
for /f "usebackq delims=" %%L in (`powershell -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%sync_accounts.ps1" -AccountsDir "!_ACCOUNTS!" -TargetDir "!_TARGET!"`) do if not "%%L"=="" echo %%L
endlocal & exit /b 0

:AUTO_SYNC_HEAL_FROM_CFG
setlocal EnableDelayedExpansion
set "_T="
set "_A=%ROOT_DIR%accounts"
if not "%ACTIVE_CFG%"=="" if exist "%ACTIVE_CFG%" (
  for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%ACTIVE_CFG%") do (
    if /I "%%A"=="SYNC_TARGET_DIR" set "_T=%%B"
    if /I "%%A"=="ACCOUNTS_DIR" set "_A=%%B"
  )
)
endlocal & call :DO_SYNC "%_T%" "%_A%" >nul 2>nul
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

echo [INFO] 清理遗留任务：%CLEAN_TASK%
schtasks /Delete /F /TN "%CLEAN_TASK%" >nul 2>nul

echo [INFO] 再次清理历史/遗漏定时任务...
call :CLEANUP_OLD_TASKS
pause
goto :MENU

:DISABLE_TASK_SILENT
schtasks /Delete /F /TN "%REFILL_TASK%" >nul 2>nul
schtasks /Delete /F /TN "%CLEAN_TASK%" >nul 2>nul
exit /b 0

:CLEANUP_OLD_TASKS
setlocal EnableDelayedExpansion
for /f "usebackq delims=" %%L in (`powershell -NoProfile -Command ^
  "$ErrorActionPreference='SilentlyContinue';" ^
  "$keepRef='\' + '%REFILL_TASK%';" ^
  "$cand=Get-ScheduledTask | Where-Object { $_.TaskName -like '*无限续杯*' -or $_.TaskName -like '*自动清理*' -or (($_.Actions | Out-String) -match '单次续杯\.bat|_内部_自动清理\.bat') };" ^
  "$del=0; foreach($t in $cand){ $full=($t.TaskPath + $t.TaskName); if($full -ieq $keepRef){ continue }; try{ Unregister-ScheduledTask -TaskName $t.TaskName -TaskPath $t.TaskPath -Confirm:$false -ErrorAction Stop | Out-Null; $del++ } catch {} };" ^
  "'INFO: cleaned_old_tasks=' + $del"`) do echo %%L
endlocal & exit /b 0

:BUILD_SILENT_TR
REM 构建静默执行的 /TR 命令：优先 conhost --headless（零闪烁），回退 PowerShell（微闪烁）
setlocal EnableDelayedExpansion
set "_BAT=%~1"
conhost.exe --headless cmd.exe /c "echo ok" >nul 2>nul
if not errorlevel 1 (
  set "_TR=conhost.exe --headless cmd.exe /c \"!_BAT!\""
  echo [INFO] 静默模式：conhost --headless（零闪烁）
) else (
  set "_TR=powershell.exe -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -Command \"Start-Process cmd.exe -ArgumentList '/c \"\"!_BAT!\"\"' -WindowStyle Hidden -Wait\""
  echo [INFO] 静默模式：PowerShell -WindowStyle Hidden（旧版 Windows 回退）
)
endlocal & set "SILENT_TR=%_TR%" & exit /b 0

:CLEANUP_OLD_OUT
REM 清理 out/ 旧目录，保留最近 10 个
powershell -NoProfile -Command ^
  "$out='%ROOT_DIR%out'; if(-not (Test-Path -LiteralPath $out)){exit 0};" ^
  "$dirs=Get-ChildItem -LiteralPath $out -Directory -ErrorAction SilentlyContinue | Where-Object { $_.Name -ne 'latest' -and $_.Name -ne 'latest-syncall' } | Sort-Object LastWriteTime -Descending;" ^
  "$del=0; if($dirs.Count -gt 10){ $old=$dirs | Select-Object -Skip 10; foreach($d in $old){ try{ Remove-Item -LiteralPath $d.FullName -Recurse -Force -ErrorAction Stop; $del++ }catch{} } };" ^
  "if($del -gt 0){ Write-Output ('[INFO] 已清理 out/ 旧目录: ' + $del) }" >nul 2>nul
exit /b 0

:RESET_TASK_AFTER_MANUAL
setlocal EnableDelayedExpansion
schtasks /Query /TN "%REFILL_TASK%" >nul 2>nul
if errorlevel 1 (
  endlocal & exit /b 0
)
set "_INTERVAL=30"
if not "%ACTIVE_CFG%"=="" if exist "%ACTIVE_CFG%" (
  for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%ACTIVE_CFG%") do (
    if /I "%%A"=="INTERVAL_MINUTES" set "_INTERVAL=%%B"
  )
)
for /f "delims=0123456789" %%I in ("!_INTERVAL!") do set "_INTERVAL=30"
if !_INTERVAL! LSS 10 set "_INTERVAL=10"
REM 复用/重建定时任务入口（不再调用 _内部_自动清理.bat）
set "TASK_ENTRY_BAT=!SCRIPT_DIR!_定时任务_入口.bat"
if not exist "!TASK_ENTRY_BAT!" (
  cmd /c "chcp 936 >nul & (echo @echo off & echo cd /d "%%~dp0" & echo chcp 65001 ^>nul & echo call 单次续杯.bat --from-task) > "!TASK_ENTRY_BAT!""
)
call :BUILD_SILENT_TR "!TASK_ENTRY_BAT!"
schtasks /Create /F /TN "%REFILL_TASK%" /SC MINUTE /MO !_INTERVAL! /TR "!SILENT_TR!" /RL HIGHEST
schtasks /Delete /F /TN "%CLEAN_TASK%" >nul 2>nul
echo [INFO] 已按手动续杯时间重置下次自动续杯时间
call :CLEANUP_OLD_TASKS
endlocal & exit /b 0
