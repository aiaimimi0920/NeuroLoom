@echo off
setlocal
cd /d "%~dp0\..\.."
echo ========================================
echo   Codex OAuth Stream Test
echo ========================================
echo.
cargo run -p nl_llm_v2 --example codex_oauth_stream
echo.
echo ========================================
echo   Test Complete
echo ========================================
pause
