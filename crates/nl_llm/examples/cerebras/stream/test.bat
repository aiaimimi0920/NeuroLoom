@echo off
setlocal enabledelayedexpansion
chcp 65001 > nul

cd /d "%~dp0\..\..\.."

set LOCAL_KEY_FILE=cerebras_key.txt
set API_KEY=
if exist "%LOCAL_KEY_FILE%" (
    set /p API_KEY=<"%LOCAL_KEY_FILE%"
)

if "%API_KEY%"=="" (
    if "%1"=="" (
        set API_KEY=YOUR_CEREBRAS_API_KEY_HERE
    ) else (
        set API_KEY=%1
    )
)

set PROMPT=Write a short poem about AI.
if not "%2"=="" set PROMPT=%2

echo [1/2] 编译 CereBras Stream 示例...
cargo build --example cerebras_stream
if %errorlevel% neq 0 (
    echo [错误] 编译失败！
    pause
    exit /b %errorlevel%
)

echo.
echo [2/2] 执行 CereBras Stream 示例...
echo =======================================================
cargo run --example cerebras_stream -- "%API_KEY%" "%PROMPT%"
echo =======================================================
pause
