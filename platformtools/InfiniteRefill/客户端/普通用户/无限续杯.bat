@echo off
setlocal EnableExtensions EnableDelayedExpansion
chcp 65001 >nul

set "SCRIPT_DIR=%~dp0"
set "CFG_DIR=%SCRIPT_DIR%状态"
set "CFG=%CFG_DIR%\无限续杯配置.env"

set "REFILL_TASK=无限续杯_定时任务"
set "CLEAN_TASK=自动清理_定时任务"

if not exist "%CFG_DIR%" mkdir "%CFG_DIR%" >nul 2>nul
if not exist "%CFG%" (
  >"%CFG%" (
    echo # 无限续杯配置（本地文件）
    echo # 注意：请勿分享/上传此文件。
    echo SERVER_URL=
    echo USER_KEY=
    echo INTERVAL_MINUTES=30
    echo AUTO_CLEAN_INTERVAL_MINUTES=30
    echo AUTO_CLEAN_APPLY=0
  )
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

:菜单
echo.
echo ====== 无限续杯（配置入口 / Windows）======
echo 配置文件："%CFG%"
echo.
echo 1) 立即执行一次【单次续杯】（使用已保存配置）
echo 2) 设置/更新【无限续杯配置】（服务器地址/用户密钥/间隔）
echo 3) 开启/更新【定时续杯】计划任务（从配置读取，不在任务里保存密钥）
echo 4) 关闭【定时续杯】计划任务
echo.
echo 5) 立即执行一次【自动清理】(默认 DryRun)
echo 6) 开启/更新【自动清理】计划任务（可选）
echo 7) 关闭【自动清理】计划任务
echo.
echo 8) 退出
echo.
set /p 选择=请选择 (1-8，默认 3)：
if "!选择!"=="" set "选择=3"

if "!选择!"=="1" goto :单次
if "!选择!"=="2" goto :配置
if "!选择!"=="3" goto :开启定时
if "!选择!"=="4" goto :关闭定时
if "!选择!"=="5" goto :清理一次
if "!选择!"=="6" goto :开启清理定时
if "!选择!"=="7" goto :关闭清理定时
if "!选择!"=="8" goto :EOF

echo [WARN] 无效选择：!选择!
goto :菜单

:单次
call "%SCRIPT_DIR%单次续杯.bat"
pause
goto :菜单

:配置
echo.
echo ====== 设置无限续杯配置 ======
set /p 服务器地址=请输入服务器地址（例如 https://127.0.0.1:8787 ）: 
set /p 用户密钥=请输入用户密钥（USER_KEY 或 UPLOAD_KEY）: 
set /p 间隔分钟=请输入续杯间隔（分钟，默认 30）: 
if "!间隔分钟!"=="" set "间隔分钟=30"

if "!服务器地址!"=="" (
  echo [ERROR] 服务器地址不能为空
  pause
  goto :菜单
)
if "!用户密钥!"=="" (
  echo [ERROR] 用户密钥不能为空
  pause
  goto :菜单
)

>"%CFG%" (
  echo # 无限续杯配置（本地文件）
  echo # 注意：请勿分享/上传此文件。
  echo SERVER_URL=!服务器地址!
  echo USER_KEY=!用户密钥!
  echo INTERVAL_MINUTES=!间隔分钟!
  echo AUTO_CLEAN_INTERVAL_MINUTES=30
  echo AUTO_CLEAN_APPLY=0
)

echo [OK] 已保存："%CFG%"
pause
goto :菜单

:开启定时
REM 从配置中读取间隔
set "间隔分钟="
for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%CFG%") do (
  if /I "%%A"=="INTERVAL_MINUTES" set "间隔分钟=%%B"
)
if "!间隔分钟!"=="" set "间隔分钟=30"

set "TR=\"%SCRIPT_DIR%单次续杯.bat\""

echo.
echo [INFO] 正在创建/更新计划任务：%REFILL_TASK%
schtasks /Create /F /TN "%REFILL_TASK%" /SC MINUTE /MO !间隔分钟! /TR "!TR!" /RL HIGHEST >nul 2>nul
if errorlevel 1 (
  echo [WARN] 创建失败（可能需要管理员权限）。
  pause
  goto :菜单
)

echo [OK] 已创建/更新：%REFILL_TASK%（每 !间隔分钟! 分钟执行一次）
pause
goto :菜单

:关闭定时
echo.
echo [INFO] 正在关闭计划任务：%REFILL_TASK%
schtasks /Delete /F /TN "%REFILL_TASK%" >nul 2>nul
if errorlevel 1 (
  echo [WARN] 关闭失败（可能任务不存在或需要管理员权限）。
) else (
  echo [OK] 已关闭：%REFILL_TASK%
)
pause
goto :菜单

:清理一次
echo.
echo ====== 自动清理（仅删 401）======
echo 1) DryRun（不删除，只生成计划/报告）
echo 2) Apply（执行删除，仅删 401，会先备份）
echo.
set /p 清理模式=请选择 (1/2，默认 1)：
if "!清理模式!"=="" set "清理模式=1"
if "!清理模式!"=="2" (
  call "%SCRIPT_DIR%自动清理\一键清理_仅删401.bat" apply
) else (
  call "%SCRIPT_DIR%自动清理\一键清理_仅删401.bat"
)
pause
goto :菜单

:开启清理定时
set "清理间隔分钟=30"
for /f "usebackq eol=# tokens=1,* delims==" %%A in ("%CFG%") do (
  if /I "%%A"=="AUTO_CLEAN_INTERVAL_MINUTES" set "清理间隔分钟=%%B"
)
if "!清理间隔分钟!"=="" set "清理间隔分钟=30"

echo.
echo 1) DryRun（不删除）
echo 2) Apply（执行删除，仅删 401，会先备份）
echo.
set /p 清理定时模式=请选择 (1/2，默认 1)：
if "!清理定时模式!"=="" set "清理定时模式=1"

set "CLEAN_ARGS=nopause"
if "!清理定时模式!"=="2" set "CLEAN_ARGS=apply nopause"

set "CLEAN_TR=\"%SCRIPT_DIR%自动清理\一键清理_仅删401.bat\" !CLEAN_ARGS!"

echo.
echo [INFO] 正在创建/更新计划任务：%CLEAN_TASK%
schtasks /Create /F /TN "%CLEAN_TASK%" /SC MINUTE /MO !清理间隔分钟! /TR "!CLEAN_TR!" /RL HIGHEST >nul 2>nul
if errorlevel 1 (
  echo [WARN] 创建失败（可能需要管理员权限）。
  pause
  goto :菜单
)

echo [OK] 已创建/更新：%CLEAN_TASK%（每 !清理间隔分钟! 分钟执行一次）
pause
goto :菜单

:关闭清理定时
echo.
echo [INFO] 正在关闭计划任务：%CLEAN_TASK%
schtasks /Delete /F /TN "%CLEAN_TASK%" >nul 2>nul
if errorlevel 1 (
  echo [WARN] 关闭失败（可能任务不存在或需要管理员权限）。
) else (
  echo [OK] 已关闭：%CLEAN_TASK%
)
pause
goto :菜单
