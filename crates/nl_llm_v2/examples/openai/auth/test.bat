@echo off
chcp 65001 >nul
cd /d "%~dp0"
echo ========================================
echo   OpenAI API Auth Test
echo ========================================
echo 请设置 OPENAI_API_KEY 环境变量
echo Usage: set OPENAI_API_KEY=your_key && test.bat
echo ========================================
cargo run -p nl_llm_v2 --example openai_auth
echo ========================================
echo   Test Complete
echo ========================================
pause
