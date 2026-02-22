@echo off
setlocal

cd /d "%~dp0\..\..\.."
echo ========================================
echo Gemini CLI Chat Test (nl_llm_new) - Non-Streaming
echo ========================================

cargo run --example gemini_cli_chat -p nl_llm_new -- "Hello! Please introduce yourself in Chinese and explain what you can do."

pause
