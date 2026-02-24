@echo off
setlocal

cd /d "%~dp0\..\..\.."
echo ========================================
echo   Kimi OAuth Chat Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example kimi_chat -- "Hello! Please introduce yourself briefly."

echo ========================================
echo   Test Complete
echo ========================================
pause
