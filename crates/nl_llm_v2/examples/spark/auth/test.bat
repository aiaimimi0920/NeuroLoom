@echo off
setlocal enabledelayedexpansion
chcp 65001 >nul

REM 讯飞星火 Bearer Token 推荐使用 APIPassword（兼容 APIKey:APISecret）
for /f "tokens=1,* delims==" %%a in ('type ..\..\..\..\..\..\..\.env.local 2^>nul ^| findstr /B "SPARK_API_KEY="') do set SPARK_API_KEY=%%b

if "%SPARK_API_KEY%"=="" (
    echo [INFO] SPARK_API_KEY not found in .env.local, using hardcoded key.
    set SPARK_API_KEY=YOUR_API_PASSWORD_HERE
)

cargo run -p nl_llm_v2 --example spark_auth -- "%SPARK_API_KEY%"

endlocal
