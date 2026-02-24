@echo off
cd /d "%~dp0"
echo ========================================
echo   BaiLing Auth Test
echo ========================================

REM 从 .env.local 读取密钥
if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="BAILING_API_KEY" set "BAILING_API_KEY=%%b"
    )
)

if "%BAILING_API_KEY%"=="" (
    set BAILING_API_KEY=sk-studio-14bd9faa0a174be39adde5c133de0c9a
)

cargo run -p nl_llm_v2 --example bailing_auth -- "%BAILING_API_KEY%"
echo ========================================
echo   Test Complete
echo ========================================
pause
