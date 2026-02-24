@echo off
cd /d "%~dp0"
echo ========================================
echo   Z.AI (智谱GLM海外版) Auth Test
echo ========================================

REM 从 .env.local 读取密钥
if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="ZAI_API_KEY" set "ZAI_API_KEY=%%b"
    )
)

if "%ZAI_API_KEY%"=="" (
    echo 请设置 ZAI_API_KEY 环境变量
    pause
    exit /b 1
)

cargo run -p nl_llm_v2 --example zai_auth
echo ========================================
echo   Test Complete
echo ========================================
pause
