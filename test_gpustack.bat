@echo off
setlocal

echo ==============================================
echo Running GPUStack White-Box Test
echo ==============================================

:: Default configuration (adjust to your local GPUStack server address)
set GPUSTACK_BASE_URL=http://127.0.0.1:8080/v1

:: Request user implicitly provided API key
if "%~1"=="" (
    echo Usage: test_gpustack.bat ^<your_api_key^>
    echo Please provide your GPUStack key as the first argument.
    exit /b 1
)

set NL_API_KEY=%~1

echo Testing with Base URL: %GPUSTACK_BASE_URL%
echo.

cd /d "%~dp0\crates\nl_llm_v2"
cargo run --example gpustack_test

endlocal
