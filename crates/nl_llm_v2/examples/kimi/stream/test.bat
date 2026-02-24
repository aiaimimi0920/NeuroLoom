@echo off
setlocal

cd /d "%~dp0\..\..\.."
echo ========================================
echo   Kimi OAuth Stream Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example kimi_stream -- "Hello! Tell me a short story."

echo ========================================
echo   Test Complete
echo ========================================
pause
