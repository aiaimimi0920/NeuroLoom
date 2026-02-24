@echo off
cd /d "%~dp0"
echo ========================================
echo   Zhipu BigModel (GLM) Auth Test
echo ========================================

REM 从 .env.local 读取密钥
if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="ZHIPU_API_KEY" set "ZHIPU_API_KEY=%%b"
    )
)

if "%ZHIPU_API_KEY%"=="" (
    echo 请设置 ZHIPU_API_KEY 环境变量
    pause
    exit /b 1
)

cargo run -p nl_llm_v2 --example zhipu_auth
echo ========================================
echo   Test Complete
echo ========================================
pause
