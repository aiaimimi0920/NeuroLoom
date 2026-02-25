@echo off
setlocal
cd /d "%~dp0"

for /f "tokens=1,* delims==" %%a in ('type ..\..\..\..\..\.env.local 2^>nul ^| findstr /B "KIMI_API_KEY="') do set KIMI_API_KEY=%%b

if "%KIMI_API_KEY%"=="" (
    echo [INFO] KIMI_API_KEY not found in .env.local, using a blank fallback to trigger auth diagnostic test.
    set KIMI_API_KEY=blank
) else (
    echo [INFO] KIMI_API_KEY loaded
)

if "%~1"=="" (
    cargo run -p nl_llm_v2 --example kimi_stream -- %KIMI_API_KEY%
) else (
    cargo run -p nl_llm_v2 --example kimi_stream -- %KIMI_API_KEY% "%~1"
)

endlocal
