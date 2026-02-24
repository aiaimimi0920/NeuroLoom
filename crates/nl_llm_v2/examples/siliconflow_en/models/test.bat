@echo off
cd /d "%~dp0"
echo ========================================
echo   SiliconFlow EN Models Test
echo ========================================

if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="SILICONFLOW_EN_API_KEY" set "SILICONFLOW_EN_API_KEY=%%b"
    )
)

if "%SILICONFLOW_EN_API_KEY%"=="" (
    echo [INFO] SILICONFLOW_EN_API_KEY not found in .env.local
    echo [INFO] Please set SILICONFLOW_EN_API_KEY in .env.local or pass as argument
    set SILICONFLOW_EN_API_KEY=YOUR_API_KEY_HERE
)

cargo run -p nl_llm_v2 --example siliconflow_en_models -- "%SILICONFLOW_EN_API_KEY%"
echo ========================================
echo   Test Complete
echo ========================================
pause
