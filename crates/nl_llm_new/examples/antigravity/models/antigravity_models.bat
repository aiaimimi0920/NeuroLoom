@echo off
chcp 65001 >nul
cd /d "%~dp0..\..\.."

echo.
echo ========================================
echo Antigravity Models Test (nl_llm_new)
echo ========================================
echo.

cargo run --example antigravity_models -p nl_llm_new

echo.
pause
