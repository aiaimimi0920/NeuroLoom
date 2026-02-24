@echo off
setlocal

cd /d "%~dp0\..\.."
echo ========================================
echo   Claude OAuth Auth Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example claude_oauth_auth

echo.
echo ========================================
echo   Test Complete
echo ========================================
pause
