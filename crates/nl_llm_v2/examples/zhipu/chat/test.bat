@echo off
cd /d "%~dp0"
echo ========================================
echo   Zhipu BigModel (GLM) Chat Test
echo ========================================

REM 从 .env.local 读取密钥（examples 目录下，上两级）
if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="ZHIPU_API_KEY" set "ZHIPU_API_KEY=%%b"
    )
)

if "%ZHIPU_API_KEY%"=="" (
    echo 请设置 ZHIPU_API_KEY 环境变量
    echo 或在 examples\.env.local 中配置: ZHIPU_API_KEY=your_key
    pause
    exit /b 1
)

set "PROMPT=%~1"
if "%PROMPT%"=="" set "PROMPT=Hello!"
cargo run -p nl_llm_v2 --example zhipu_chat -- "%PROMPT%"
echo ========================================
echo   Test Complete
echo ========================================
pause
