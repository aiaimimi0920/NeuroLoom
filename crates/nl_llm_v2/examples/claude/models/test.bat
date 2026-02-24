@echo off
setlocal

cd /d "%~dp0\..\.."
echo ========================================
echo   Claude Models Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example claude_models

echo.
echo ========================================
echo   Test Complete
echo ========================================
pause
