@echo off
setlocal
cd /d "%~dp0\..\..\.."
echo ========================================
echo   Qwen OAuth Auth Test
echo ========================================
echo.
cargo run -p nl_llm_v2 --example qwen_auth
echo.
echo ========================================
echo   Test Complete
echo ========================================
pause
