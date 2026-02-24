@echo off
setlocal

cd /d "%~dp0\..\..\.."
echo ========================================
echo   vertex_api models Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example vertex_api_models

echo ========================================
echo   Test Complete
echo ========================================
pause
