@echo off
setlocal

cd /d "%~dp0\..\..\.."
echo ========================================
echo   gemini_cli stream Test
echo ========================================
echo.
echo   Prompt: Hello! Tell me a long story.
echo.

cargo run -p nl_llm_v2 --example gemini_cli_stream -- "" "Hello! Tell me a long story."

echo ========================================
echo   Test Complete
echo ========================================
pause
