@echo off
cd /d "%~dp0"
echo ========================================
echo   Azure OpenAI Models
echo ========================================
cargo run -p nl_llm_v2 --example azure_openai_models
echo ========================================
echo   Test Complete
echo ========================================
pause
