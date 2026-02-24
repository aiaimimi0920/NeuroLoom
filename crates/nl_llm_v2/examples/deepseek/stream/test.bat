@echo off
cd /d "%~dp0"
echo ========================================
echo   DeepSeek Stream Test
echo ========================================

REM 从 .env.local 读取密钥（examples 目录下，上两级）
if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="DEEPSEEK_API_KEY" set "DEEPSEEK_API_KEY=%%b"
    )
)

if "%DEEPSEEK_API_KEY%"=="" (
    echo 请设置 DEEPSEEK_API_KEY 环境变量
    pause
    exit /b 1
)

set "PROMPT=%~1"
if "%PROMPT%"=="" set "PROMPT=用三句话介绍一下 Rust 语言。"
cargo run -p nl_llm_v2 --example deepseek_stream -- "%PROMPT%"
echo ========================================
echo   Test Complete
echo ========================================
pause
