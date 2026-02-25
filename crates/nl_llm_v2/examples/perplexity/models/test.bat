@echo off
cd /d "%~dp0"
echo ========================================
echo   Perplexity AI Models
echo ========================================
cargo run -p nl_llm_v2 --example perplexity_models
echo ========================================
echo   Test Complete
echo ========================================
pause
