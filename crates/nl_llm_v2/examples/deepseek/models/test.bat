@echo off
cd /d "%~dp0"
echo ========================================
echo   DeepSeek Models Test
echo ========================================
cargo run -p nl_llm_v2 --example deepseek_models
echo ========================================
echo   Test Complete
echo ========================================
pause
