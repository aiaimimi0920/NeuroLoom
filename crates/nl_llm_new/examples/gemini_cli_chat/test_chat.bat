@echo off
chcp 65001 >nul
setlocal

echo ========================================
echo   Gemini CLI Chat Test (nl_llm_new)
echo ========================================
echo.

set "PROMPT=%~1"
if "%PROMPT%"=="" (
    set "PROMPT=Hello! Please introduce yourself in Chinese."
)

set "PROJECT_ROOT=%~dp0..\..\..\.."
set "EXE=%PROJECT_ROOT%\target\debug\examples\gemini_cli_chat.exe"

echo [1/2] Building...
cargo build --example gemini_cli_chat -p nl_llm_new
if %errorlevel% neq 0 (
    echo Build FAILED!
    pause
    exit /b 1
)
echo.

echo [2/2] Non-streaming request...
echo.
"%EXE%" "%PROMPT%"
echo.

echo ========================================
echo   Streaming mode test:
echo ========================================
echo.
"%EXE%" "%PROMPT%" --stream
echo.

pause
