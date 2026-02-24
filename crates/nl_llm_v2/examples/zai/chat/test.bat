@echo off
cd /d "%~dp0"
echo ========================================
echo   Z.AI (智谱GLM海外版) Chat Test
echo ========================================

REM 从 .env.local 读取密钥（上两级 examples 目录）
if exist "%~dp0..\..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\..\.env.local") do (
        if "%%a"=="ZAI_API_KEY" set "ZAI_API_KEY=%%b"
    )
)
REM 也检查 examples 目录下
if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="ZAI_API_KEY" set "ZAI_API_KEY=%%b"
    )
)

if "%ZAI_API_KEY%"=="" (
    echo 请设置 ZAI_API_KEY 环境变量
    echo 或在 examples\.env.local 中配置: ZAI_API_KEY=your_key
    pause
    exit /b 1
)

set "PROMPT=%~1"
if "%PROMPT%"=="" set "PROMPT=Hello!"
cargo run -p nl_llm_v2 --example zai_chat -- "%PROMPT%"
echo ========================================
echo   Test Complete
echo ========================================
pause
