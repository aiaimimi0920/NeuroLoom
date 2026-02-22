@echo off
chcp 65001 >nul
cd /d "%~dp0..\..\.."


echo.
echo ========================================
echo  iFlow Models Query (nl_llm_new)
echo ========================================
echo.

cargo run --example iflow_models -p nl_llm_new

echo.
pause
