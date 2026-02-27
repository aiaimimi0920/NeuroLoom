@echo off
setlocal

echo ==============================================
echo Running Qiniu AI White-Box Configuration Test
echo ==============================================

:: Request user implicitly provided API key
if "%~1"=="" (
    echo Usage: test_qiniu.bat ^<your_api_key^>
    echo Please provide your Qiniu AI key as the first argument.
    exit /b 1
)

set NL_API_KEY=%~1

cd /d "%~dp0\crates\nl_llm_v2"
cargo run --example qiniu_test

endlocal
