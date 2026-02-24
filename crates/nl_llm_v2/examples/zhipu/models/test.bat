@echo off
cd /d "%~dp0"
echo ========================================
echo   Zhipu BigModel (GLM) Models Test
echo ========================================
cargo run -p nl_llm_v2 --example zhipu_models
echo ========================================
echo   Test Complete
echo ========================================
pause
