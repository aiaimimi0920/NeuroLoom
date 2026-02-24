@echo off
REM antigravity auth test
REM Usage: test.bat [prompt]
REM OAuth cache stored in examples/antigravity/.cache/

cd /d "%~dp0\..\..\.."

if "%1"=="" (
    set "PROMPT=Hello! Please introduce yourself."
) else (
    set "PROMPT=%1"
)

echo ========================================
echo   antigravity auth Test
echo ========================================
echo.
echo   Prompt: %PROMPT%
echo.

cargo run -p nl_llm_v2 --example antigravity_auth -- "" "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
