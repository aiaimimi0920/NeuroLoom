@echo off
cd /d "%~dp0"
echo ========================================
echo   ModelScope Auth Test
echo ========================================

if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="MODELSCOPE_API_KEY" set "MODELSCOPE_API_KEY=%%b"
    )
)

if "%MODELSCOPE_API_KEY%"=="" (
    set MODELSCOPE_API_KEY=YOUR_API_KEY_HERE
)

cargo run -p nl_llm_v2 --example modelscope_auth -- "%MODELSCOPE_API_KEY%"
echo ========================================
echo   Test Complete
echo ========================================
pause
