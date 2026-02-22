@echo off
chcp 65001 >nul
cd /d "%~dp0..\..\.."

echo.
echo ========================================
echo Antigravity Chat Test (nl_llm_new) - Non-Streaming
echo ========================================
echo.

set "PROMPT=%~1"
if "%PROMPT%"=="" (
    set "PROMPT=Hello! Please introduce yourself in Chinese and explain what you can do."
)

cargo run --example antigravity_chat -p nl_llm_new -- "%PROMPT%"

echo.
pause
