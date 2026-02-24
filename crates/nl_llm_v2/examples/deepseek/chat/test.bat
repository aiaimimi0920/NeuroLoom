@echo off
cd /d "%~dp0"
echo ========================================
echo   DeepSeek Chat Test
echo ========================================

REM 从 .env.local 读取密钥（examples 目录下，上两级）
if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="DEEPSEEK_API_KEY" set "DEEPSEEK_API_KEY=%%b"
    )
)

if "%DEEPSEEK_API_KEY%"=="" (
    echo 请设置 DEEPSEEK_API_KEY 环境变量
    echo 或在 examples\.env.local 中配置: DEEPSEEK_API_KEY=sk-xxx
    pause
    exit /b 1
)

set "PROMPT=%~1"
if "%PROMPT%"=="" set "PROMPT=Hello!"
cargo run -p nl_llm_v2 --example deepseek_chat -- "%PROMPT%"
echo ========================================
echo   Test Complete
echo ========================================
pause
