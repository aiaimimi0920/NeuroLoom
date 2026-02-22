@echo off
setlocal

cd /d "%~dp0\..\..\.."

echo ========================================
echo   Gemini CLI Models (nl_llm_new)
echo ========================================

cargo run --example gemini_cli_models -p nl_llm_new

pause
