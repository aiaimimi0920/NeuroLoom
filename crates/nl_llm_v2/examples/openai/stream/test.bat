@echo off
cd /d "%~dp0"
echo ========================================
echo   OpenAI Stream Test
echo ========================================
echo 请设置 OPENAI_API_KEY 环境变量
echo Usage: set OPENAI_API_KEY=your_key && test.bat "Hello!"
echo ========================================
set "PROMPT=%~1"
if "%PROMPT%"=="" set "PROMPT=Hello!"
cargo run -p nl_llm_v2 --example openai_stream -- "%PROMPT%"
echo ========================================
echo   Test Complete
echo ========================================
pause
