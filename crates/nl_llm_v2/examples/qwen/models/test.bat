@echo off
setlocal
cd /d "%~dp0\..\..\.."
echo ========================================
echo   Qwen Models Test
echo ========================================
echo.
cargo run -p nl_llm_v2 --example qwen_models
echo.
echo ========================================
echo   Test Complete
echo ========================================
pause
