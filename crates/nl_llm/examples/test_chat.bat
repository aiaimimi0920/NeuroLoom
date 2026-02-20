@echo off
chcp 65001 >nul
cd /d "%~dp0"

set CARGO=C:\Users\Administrator\.cargo\bin\cargo.exe

echo.
echo ========================================
echo iFlow Chat Test
echo ========================================
echo Models: qwen3-max, deepseek-v3.2, glm-4.6
echo         kimi-k2, qwen3-coder-plus, deepseek-r1
echo ========================================
echo.

if not exist "..\..\..\target\debug\examples\test_iflow_chat.exe" (
    echo Building...
    %CARGO% build --example test_iflow_chat
)

..\..\..\target\debug\examples\test_iflow_chat.exe

echo.
pause
