@echo off
setlocal

echo ==============================================
echo Running NewAPI White-Box Test
echo ==============================================

:: Default configuration (adjust as needed for local testing)
set NEWAPI_BASE_URL=https://api.aiproxy.io/v1

:: Request user implicitly provided API key (xxx is replaced locally)
:: The key is requested at runtime to avoid committing it to the repo
if "%~1"=="" (
    echo Usage: test_newapi.bat ^<your_api_key^>
    echo Please provide your NewAPI key as the first argument.
    exit /b 1
)

set NL_API_KEY=%~1

echo Testing with Base URL: %NEWAPI_BASE_URL%
echo.

cd /d "%~dp0\..\crates\nl_llm_v2"
cargo run --example newapi_test

endlocal
