@echo off
chcp 65001 >nul
cd /d "%~dp0"
echo ========================================
echo   Codex API Tools Test
echo ========================================
echo 请设置 OPENAI_API_KEY 环境变量
echo Usage: set OPENAI_API_KEY=your_key && test.bat "Hello!"
echo ========================================
set "PROMPT=%~1"
if "%PROMPT%"=="" set "PROMPT=你好！请简单介绍一下你自己。"
cargo run -p nl_llm_v2 --example codex_api_tools -- "%PROMPT%"
echo ========================================
echo   Test Complete
echo ========================================
pause
