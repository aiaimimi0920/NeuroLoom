@echo off
cd /d "%~dp0"
echo ========================================
echo   百度千帆 Models
echo ========================================
cargo run -p nl_llm_v2 --example qianfan_models
echo ========================================
echo   Test Complete
echo ========================================
pause
