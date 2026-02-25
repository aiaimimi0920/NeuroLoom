@echo off
setlocal enabledelayedexpansion
chcp 65001 >nul

for /f "tokens=1,* delims==" %%a in ('type ..\..\..\..\..\..\..\.env.local 2^>nul ^| findstr /B "SPARK_API_KEY="') do set SPARK_API_KEY=%%b

if "%SPARK_API_KEY%"=="" (
    echo [INFO] SPARK_API_KEY not found in .env.local, using hardcoded APIPassword.
    set SPARK_API_KEY=YOUR_API_PASSWORD_HERE
)

if "%~1"=="" (
    cargo run -p nl_llm_v2 --example spark_stream -- "%SPARK_API_KEY%" "写一首关于人工智能的五言绝句。"
) else (
    cargo run -p nl_llm_v2 --example spark_stream -- "%SPARK_API_KEY%" %*
)

endlocal
