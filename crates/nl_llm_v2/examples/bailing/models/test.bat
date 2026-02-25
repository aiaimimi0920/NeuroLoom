@echo off
cd /d "%~dp0"
echo ========================================
echo   BaiLing Models Test
echo ========================================

if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="BAILING_API_KEY" set "BAILING_API_KEY=%%b"
    )
)

if "%BAILING_API_KEY%"=="" (
    set BAILING_API_KEY=YOUR_API_KEY_HERE
)

cargo run -p nl_llm_v2 --example bailing_models -- "%BAILING_API_KEY%"
echo ========================================
echo   Test Complete
echo ========================================
pause
