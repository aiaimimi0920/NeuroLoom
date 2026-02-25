@echo off
setlocal
cd /d "%~dp0"

for /f "tokens=1,* delims==" %%a in ('type ..\..\..\..\..\..\.env.local 2^>nul ^| findstr /B "KAT_CODER_API_KEY="') do set KAT_CODER_API_KEY=%%b

if "%KAT_CODER_API_KEY%"=="" (
    echo [INFO] KAT_CODER_API_KEY not found in .env.local, using hardcoded key for testing.
    set KAT_CODER_API_KEY=YOUR_API_KEY_HERE
) else (
    echo [INFO] KAT_CODER_API_KEY loaded
)

cargo run -p nl_llm_v2 --example kat_coder_auth -- "%KAT_CODER_API_KEY%"

endlocal
