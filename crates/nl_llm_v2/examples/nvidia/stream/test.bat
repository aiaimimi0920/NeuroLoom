@echo off
cd /d "%~dp0"
echo ========================================
echo   Nvidia NIM Stream Test
echo ========================================

if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="NVIDIA_API_KEY" set "NVIDIA_API_KEY=%%b"
    )
)

if "%NVIDIA_API_KEY%"=="" (
    echo [INFO] NVIDIA_API_KEY not found in .env.local
    echo [INFO] Please set NVIDIA_API_KEY in .env.local or pass as argument
    set NVIDIA_API_KEY=YOUR_API_KEY_HERE
)

cargo run -p nl_llm_v2 --example nvidia_stream -- "%NVIDIA_API_KEY%"
echo ========================================
echo   Test Complete
echo ========================================
pause
