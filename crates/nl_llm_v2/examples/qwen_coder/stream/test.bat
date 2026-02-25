@echo off
setlocal
cd /d "%~dp0"

for /f "tokens=1,* delims==" %%a in ('type ..\..\..\..\..\.env.local 2^>nul ^| findstr /B "QWEN_API_KEY="') do set QWEN_API_KEY=%%b

if "%QWEN_API_KEY%"=="" (
    echo [ERROR] QWEN_API_KEY not found in .env.local
    goto :eof
)

if "%~1"=="" (
    cargo run -p nl_llm_v2 --example qwen_coder_stream -- %QWEN_API_KEY%
) else (
    cargo run -p nl_llm_v2 --example qwen_coder_stream -- %QWEN_API_KEY% "%~1"
)

endlocal
