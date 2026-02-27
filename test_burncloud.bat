@echo off
setlocal

echo ==============================================
echo Running BurnCloud White-Box Test
echo ==============================================

:: Default configuration
set BURNCLOUD_BASE_URL=https://api.burn.hair/v1

:: The user can provide a dummy or real API key
if "%~1" neq "" (
    set NL_API_KEY=%~1
) else (
    echo Note: No API key provided as an argument. The test will fallback to a dummy key to verify the code path to the 401 code.
    set NL_API_KEY=dummy-testing-api-key-without-real-auth
)

echo Testing with Base URL: %BURNCLOUD_BASE_URL%
echo.

cd /d "%~dp0\crates\nl_llm_v2"
cargo run --example burncloud_test

endlocal
