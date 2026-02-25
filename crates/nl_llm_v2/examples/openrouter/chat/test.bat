@echo off
cd /d "%~dp0"
echo ========================================
echo   OpenRouter Chat Test
echo ========================================

if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="OPENROUTER_API_KEY" set "OPENROUTER_API_KEY=%%b"
    )
)

if "%OPENROUTER_API_KEY%"=="" (
    echo [INFO] OPENROUTER_API_KEY not found in .env.local
    echo [INFO] Please set OPENROUTER_API_KEY in .env.local or pass as argument
    set OPENROUTER_API_KEY=YOUR_API_KEY_HERE
)

cargo run -p nl_llm_v2 --example openrouter_chat -- "%OPENROUTER_API_KEY%"
echo ========================================
echo   Test Complete
echo ========================================
pause
