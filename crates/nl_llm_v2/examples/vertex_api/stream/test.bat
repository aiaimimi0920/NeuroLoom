@echo off
setlocal

cd /d "%~dp0\..\..\.."
echo ========================================
echo   vertex_api stream Test
echo ========================================
echo.
echo   Prompt: Hello! Tell me a short story.
echo.

cargo run -p nl_llm_v2 --example vertex_api_stream -- "" "Hello! Tell me a short story."

echo ========================================
echo   Test Complete
echo ========================================
pause
