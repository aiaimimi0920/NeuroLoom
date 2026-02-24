@echo off
setlocal

cd /d "%~dp0\..\.."
echo ========================================
echo   Claude OAuth Stream Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example claude_oauth_stream -- "Hello! Tell me a short story."

echo.
echo ========================================
echo   Test Complete
echo ========================================
pause
