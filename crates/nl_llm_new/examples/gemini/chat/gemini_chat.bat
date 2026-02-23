@echo off
chcp 65001 >nul
cd /d "%~dp0..\..\.."

echo.
echo ========================================
echo Gemini Chat Test (nl_llm_new) - Non-Streaming
echo ========================================
echo.

set "PROMPT=%~1"
if "%PROMPT%"=="" (
    set "PROMPT=Hello! Please introduce yourself in Chinese and explain what you can do."
)

set "API_KEY=AIzaSyAnI9Z_nCB2j4g0wt7DB2rB7kaXY47FIRc"

if "%GEMINI_API_KEY%" NEQ "" (
    set "API_KEY=%GEMINI_API_KEY%"
)

if "%API_KEY%"=="AIzaSyAnI9Z_nCB2j4g0wt7DB2rB7kaXY47FIRc" (
    echo [WARNING] Using default embedded API_KEY. Consider setting GEMINI_API_KEY environment variable.
)

cargo run --example gemini_chat -p nl_llm_new -- "%PROMPT%" --key "%API_KEY%"

echo.
pause
