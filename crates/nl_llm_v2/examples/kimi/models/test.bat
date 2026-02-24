@echo off
setlocal

cd /d "%~dp0\..\..\.."
echo ========================================
echo   Kimi Models Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example kimi_models

echo ========================================
echo   Test Complete
echo ========================================
pause
