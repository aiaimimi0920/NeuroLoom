@echo off
chcp 65001 > nul
setlocal

echo ========================================
echo   Antigravity Chat Test
echo ========================================
echo.

if not exist "%USERPROFILE%\.nl_llm\antigravity_token.json" (
    echo [!] Not logged in. Run test_antigravity.bat first.
    pause
    exit /b 1
)

set "PROMPT=%~1"
if "%PROMPT%"=="" (
    set "PROMPT=Hello! Please introduce yourself in Chinese and explain what you can do."
)

set "EXE=%~dp0..\..\..\target\debug\examples\test_antigravity_chat.exe"

echo [1/2] Building...
cargo build --example test_antigravity_chat -p nl_llm
if %errorlevel% neq 0 (
    echo Build failed!
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
