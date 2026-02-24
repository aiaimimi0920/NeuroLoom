@echo off
setlocal
cd /d "%~dp0\..\..\.."
echo ========================================
echo   Qwen OAuth Chat Test
echo ========================================
echo.
cargo run -p nl_llm_v2 --example qwen_chat -- "Hello! Please introduce yourself briefly."
echo.
echo ========================================
echo   Test Complete
echo ========================================
pause
