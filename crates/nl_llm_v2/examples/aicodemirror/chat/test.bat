@echo off
cd /d "%~dp0"
echo ========================================
echo   AICodeMirror Chat Test
echo ========================================

if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="AICODEMIRROR_API_KEY" set "AICODEMIRROR_API_KEY=%%b"
    )
)

if "%AICODEMIRROR_API_KEY%"=="" (
    set AICODEMIRROR_API_KEY=YOUR_API_KEY_HERE
)

cargo run -p nl_llm_v2 --example aicodemirror_chat -- "%AICODEMIRROR_API_KEY%"
echo ========================================
echo   Test Complete
echo ========================================
pause
