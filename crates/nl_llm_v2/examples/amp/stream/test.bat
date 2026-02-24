@echo off
cd /d "%~dp0"
echo ========================================
echo   Sourcegraph Amp Stream Test
echo ========================================
echo 请设置 AMP_API_KEY 环境变量
echo Usage: set AMP_API_KEY=your_key ^&^& test.bat "Hello!"
echo ========================================
set "PROMPT=%~1"
if "%PROMPT%"=="" set "PROMPT=用三句话介绍一下 Rust 语言。"
cargo run -p nl_llm_v2 --example amp_stream -- "%PROMPT%"
echo ========================================
echo   Test Complete
echo ========================================
pause
