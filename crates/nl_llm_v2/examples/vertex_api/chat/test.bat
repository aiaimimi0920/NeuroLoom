@echo off
setlocal

cd /d "%~dp0\..\..\.."
echo ========================================
echo   vertex_api chat Test
echo ========================================
echo.
echo   Prompt: Hello! Please introduce yourself briefly.
echo.

cargo run -p nl_llm_v2 --example vertex_api_chat -- "" "Hello! Please introduce yourself briefly."

echo ========================================
echo   Test Complete
echo ========================================
pause
