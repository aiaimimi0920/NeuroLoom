@echo off
cd /d "%~dp0"
echo ========================================
echo   DouBaoSeed Stream Test
echo ========================================

REM 从 .env.local 读取密钥（examples 目录下，上两级）
if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="DOUBAOSEED_API_KEY" set "DOUBAOSEED_API_KEY=%%b"
    )
)

if "%DOUBAOSEED_API_KEY%"=="" (
    echo 请设置 DOUBAOSEED_API_KEY 环境变量
    pause
    exit /b 1
)

cargo run -p nl_llm_v2 --example doubaoseed_stream
echo ========================================
echo   Test Complete
echo ========================================
pause
