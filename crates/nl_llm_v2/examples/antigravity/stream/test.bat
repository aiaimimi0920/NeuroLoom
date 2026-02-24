@echo off
REM antigravity stream test
REM Usage: test.bat [prompt]
REM OAuth cache stored in examples/antigravity/.cache/

cd /d "%~dp0\..\..\.."

if "%1"=="" (
    set "PROMPT=Hello! Tell me a long story."
) else (
    set "PROMPT=%1"
)

echo ========================================
echo   antigravity stream Test
echo ========================================
echo.
echo   Prompt: %PROMPT%
echo.

cargo run -p nl_llm_v2 --example antigravity_stream -- "" "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
