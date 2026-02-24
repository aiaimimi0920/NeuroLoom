@echo off
setlocal

cd /d "%~dp0\..\..\.."
echo ========================================
echo   Kimi OAuth Auth Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example kimi_auth

echo ========================================
echo   Test Complete
echo ========================================
pause
