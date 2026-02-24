@echo off
setlocal

cd /d "%~dp0\..\..\.."
echo ========================================
echo   gemini_cli models Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example gemini_cli_models

echo ========================================
echo   Test Complete
echo ========================================
pause
