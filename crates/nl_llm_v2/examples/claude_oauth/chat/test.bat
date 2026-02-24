@echo off
setlocal

cd /d "%~dp0\..\.."
echo ========================================
echo   Claude OAuth Chat Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example claude_oauth_chat -- "Hello! Please introduce yourself briefly."

echo.
echo ========================================
echo   Test Complete
echo ========================================
pause
