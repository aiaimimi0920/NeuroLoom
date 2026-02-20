@echo off
chcp 65001 >nul 2>&1
echo ========================================
echo   Gemini CLI Chat Test
echo ========================================
echo.

echo [1/2] Building...
cargo build --example test_gemini_cli_chat -p nl_llm
if %ERRORLEVEL% neq 0 (
    echo Build FAILED!
    pause
    exit /b 1
)
echo.

echo [2/2] Non-streaming request...
echo.
cargo run --example test_gemini_cli_chat -p nl_llm

echo.
pause
