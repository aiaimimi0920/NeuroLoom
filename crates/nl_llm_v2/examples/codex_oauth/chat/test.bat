@echo off
setlocal
cd /d "%~dp0\..\.."
echo ========================================
echo   Codex OAuth Chat Test
echo ========================================
echo.
cargo run -p nl_llm_v2 --example codex_oauth_chat
echo.
echo ========================================
echo   Test Complete
echo ========================================
pause
