@echo off
cd /d "%~dp0"
echo ========================================
echo   Cohere Models
echo ========================================
cargo run -p nl_llm_v2 --example cohere_models
echo ========================================
echo   Test Complete
echo ========================================
pause
