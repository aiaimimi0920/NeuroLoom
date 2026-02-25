@echo off
cd /d "%~dp0"
echo ========================================
echo   Cohere Auth Test
echo ========================================

if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="COHERE_API_KEY" set "COHERE_API_KEY=%%b"
    )
)

if "%COHERE_API_KEY%"=="" (
    echo 请设置 COHERE_API_KEY 环境变量或在 .env.local 中配置
    pause
    exit /b 1
)

cargo run -p nl_llm_v2 --example cohere_auth -- "%COHERE_API_KEY%"
echo ========================================
echo   Test Complete
echo ========================================
pause
