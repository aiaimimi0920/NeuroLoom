@echo off
cd /d "%~dp0"
echo ========================================
echo   OpenAI Models Test
echo ========================================
echo 本测试不需要 API Key，仅演示 ModelResolver 功能
echo ========================================
cargo run -p nl_llm_v2 --example openai_models
echo ========================================
echo   Test Complete
echo ========================================
pause
