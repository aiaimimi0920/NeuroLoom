@echo off
cd /d "%~dp0"
echo ========================================
echo   AIGoCode Stream Test
echo ========================================

if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="AIGOCODE_API_KEY" set "AIGOCODE_API_KEY=%%b"
    )
)

if "%AIGOCODE_API_KEY%"=="" (
    set AIGOCODE_API_KEY=YOUR_API_KEY_HERE
)

cargo run -p nl_llm_v2 --example aigocode_stream -- "%AIGOCODE_API_KEY%"
echo ========================================
echo   Test Complete
echo ========================================
pause
