@echo off
setlocal enabledelayedexpansion
chcp 65001 >nul

for /f "tokens=1,* delims==" %%a in ('type ..\..\..\..\..\..\.env.local 2^>nul ^| findstr /B "LONGCAT_API_KEY="') do set LONGCAT_API_KEY=%%b

if "%LONGCAT_API_KEY%"=="" (
    echo [INFO] LONGCAT_API_KEY not found in .env.local, using hardcoded key for testing.
    set LONGCAT_API_KEY=YOUR_API_KEY_HERE
)

if "%~1"=="" (
    cargo run -p nl_llm_v2 --example longcat_chat -- "%LONGCAT_API_KEY%" "用一句话介绍一下长猫。"
) else (
    cargo run -p nl_llm_v2 --example longcat_chat -- "%LONGCAT_API_KEY%" %*
)
