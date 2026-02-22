@echo off
setlocal

cd /d "%~dp0\..\..\.."

echo ========================================
echo   Gemini CLI Auth Test (nl_llm_new)
echo ========================================

cargo run --example gemini_cli_auth -p nl_llm_new

pause
