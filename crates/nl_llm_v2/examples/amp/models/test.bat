@echo off
cd /d "%~dp0"
echo ========================================
echo   Sourcegraph Amp Models Test
echo ========================================
echo 请设置 AMP_API_KEY 环境变量
echo Usage: set AMP_API_KEY=your_key ^&^& test.bat
echo ========================================
cargo run -p nl_llm_v2 --example amp_models
echo ========================================
echo   Test Complete
echo ========================================
pause
