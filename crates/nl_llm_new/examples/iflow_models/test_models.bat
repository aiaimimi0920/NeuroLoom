@echo off
chcp 65001 >nul
cd /d "%~dp0..\.."

set CARGO=C:\Users\Administrator\.cargo\bin\cargo.exe

echo.
echo ========================================
echo  iFlow Models Query (nl_llm_new)
echo ========================================
echo.

%CARGO% run --example iflow_models -p nl_llm_new

echo.
pause
