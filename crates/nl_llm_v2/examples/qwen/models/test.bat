@echo off
setlocal
cd /d "%~dp0"

for /f "tokens=1,* delims==" %%a in ('type ..\..\..\..\..\.env.local ^| findstr /B "QWEN_API_KEY="') do set QWEN_API_KEY=%%b

if "%QWEN_API_KEY%"=="" (
    echo [ERROR] QWEN_API_KEY not found in .env.local
    exit /b 1
)

echo [INFO] QWEN_API_KEY loaded
cargo run -p nl_llm_v2 --example qwen_models -- %QWEN_API_KEY%

endlocal
