@echo off
cd /d "%~dp0"
echo ========================================
echo   Sourcegraph Amp Chat Test
echo ========================================
echo 请设置 AMP_API_KEY 环境变量
echo Usage: set AMP_API_KEY=your_key ^&^& test.bat "Hello!"
echo ========================================
set "PROMPT=%~1"
if "%PROMPT%"=="" set "PROMPT=Hello!"
cargo run -p nl_llm_v2 --example amp_chat -- "%PROMPT%"
echo ========================================
echo   Test Complete
echo ========================================
pause
