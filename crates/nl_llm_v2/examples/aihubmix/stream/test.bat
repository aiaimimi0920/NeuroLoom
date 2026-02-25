@echo off
cd /d "%~dp0"
echo ========================================
echo   AiHubMix Stream Test
echo ========================================

if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="AIHUBMIX_API_KEY" set "AIHUBMIX_API_KEY=%%b"
    )
)

if "%AIHUBMIX_API_KEY%"=="" (
    echo [INFO] AIHUBMIX_API_KEY not found in .env.local
    echo [INFO] Please set AIHUBMIX_API_KEY in .env.local or pass as argument
    set AIHUBMIX_API_KEY=YOUR_API_KEY_HERE
)

cargo run -p nl_llm_v2 --example aihubmix_stream -- "%AIHUBMIX_API_KEY%"
echo ========================================
echo   Test Complete
echo ========================================
pause
