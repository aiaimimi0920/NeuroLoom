@echo off
setlocal enabledelayedexpansion
chcp 65001 > nul

cd /d "%~dp0\..\..\.."

:: LM Studio 本地服务通常无需 API Key
set API_KEY=lm-studio
set PROMPT=Write a haiku about coding.
if not "%1"=="" set PROMPT=%1

echo [注意] 请确保 LM Studio 本地服务已启动 (默认 http://localhost:1234)
echo.
echo [1/2] 编译 LM Studio Stream 示例...
cargo build --example lmstudio_stream
if %errorlevel% neq 0 (
    echo [错误] 编译失败！
    pause
    exit /b %errorlevel%
)

echo.
echo [2/2] 执行 LM Studio Stream 示例...
echo =======================================================
cargo run --example lmstudio_stream -- "%API_KEY%" "%PROMPT%"
echo =======================================================
pause
