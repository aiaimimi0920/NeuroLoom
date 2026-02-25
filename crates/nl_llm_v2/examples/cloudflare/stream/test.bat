@echo off
cd /d "%~dp0"
echo ========================================
echo   Cloudflare Stream Test
echo ========================================

if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="CLOUDFLARE_API_TOKEN" set "CLOUDFLARE_API_TOKEN=%%b"
    )
)

if "%CLOUDFLARE_API_TOKEN%"=="" (
    echo 请设置 CLOUDFLARE_API_TOKEN 环境变量或在 .env.local 中配置
    pause
    exit /b 1
)

cargo run -p nl_llm_v2 --example cloudflare_stream -- "%CLOUDFLARE_API_TOKEN%"
echo ========================================
echo   Test Complete
echo ========================================
pause
