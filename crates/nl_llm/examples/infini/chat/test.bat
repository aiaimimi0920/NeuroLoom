@echo off
setlocal enabledelayedexpansion
chcp 65001 > nul
cd /d "%~dp0\..\..\.."

set LOCAL_KEY_FILE=infini_key.txt
set API_KEY=
if exist "%LOCAL_KEY_FILE%" ( set /p API_KEY=<"%LOCAL_KEY_FILE%" )
if "%API_KEY%"=="" ( if "%1"=="" ( set API_KEY=YOUR_INFINI_API_KEY_HERE ) else ( set API_KEY=%1 ) )

set PROMPT=你好，请介绍一下你自己。
if not "%2"=="" set PROMPT=%2

echo [1/2] 编译无问芯穹 Chat 示例...
cargo build --example infini_chat
if %errorlevel% neq 0 ( echo [错误] 编译失败！ & pause & exit /b %errorlevel% )

echo.
echo [2/2] 执行无问芯穹 Chat 示例...
echo =======================================================
cargo run --example infini_chat -- "%API_KEY%" "%PROMPT%"
echo =======================================================
pause
