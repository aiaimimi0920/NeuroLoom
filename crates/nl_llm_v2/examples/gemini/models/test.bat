@echo off
chcp 65001 >nul
REM gemini 平台测试 - models
REM 用法: test.bat [api_key]

cd /d "%~dp0"

set "API_KEY=AIzaSyBcsmzMVtViYn_pFYweNt_l7aWOsD5xjqM"

if "%GEMINI_API_KEY%" NEQ "" (
    set "API_KEY=%GEMINI_API_KEY%"
)

if "%1" NEQ "" (
    set "API_KEY=%1"
)

if "%API_KEY%"=="AIzaSyBcsmzMVtViYn_pFYweNt_l7aWOsD5xjqM" (
    echo [WARNING] Using default embedded API_KEY. Consider setting GEMINI_API_KEY environment variable.
)

echo ========================================
echo   gemini models Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example gemini_models -- %API_KEY%

echo.
echo ========================================
echo   Test Complete
echo ========================================
