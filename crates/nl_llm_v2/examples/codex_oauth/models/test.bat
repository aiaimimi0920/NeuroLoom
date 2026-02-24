@echo off
setlocal
cd /d "%~dp0\..\.."
echo ========================================
echo   Codex OAuth Models Test
echo ========================================
echo.
cargo run -p nl_llm_v2 --example codex_oauth_models
echo.
echo ========================================
echo   Test Complete
echo ========================================
pause
