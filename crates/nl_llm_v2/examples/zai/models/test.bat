@echo off
cd /d "%~dp0"
echo ========================================
echo   Z.AI (智谱GLM海外版) Models Test
echo ========================================
cargo run -p nl_llm_v2 --example zai_models
echo ========================================
echo   Test Complete
echo ========================================
pause
