@echo off
REM antigravity models test
REM Requires OAuth login first (run auth or chat test first)

cd /d "%~dp0\..\..\..\.."

echo ========================================
echo   antigravity models Test
echo ========================================
echo.
echo   NOTE: Run auth or chat test first to complete OAuth login
echo.

cargo run -p nl_llm_v2 --example antigravity_models

echo.
echo ========================================
echo   Test Complete
echo ========================================
