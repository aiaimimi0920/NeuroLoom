@echo off
chcp 65001 >nul
REM gemini 平台测试 - stream
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

set "API_KEY=AIzaSyBcsmzMVtViYn_pFYweNt_l7aWOsD5xjqM"

if "%GEMINI_API_KEY%" NEQ "" set "API_KEY=%GEMINI_API_KEY%"
if "%~1" NEQ "" set "API_KEY=%~1"

set "PROMPT=你好！请简单介绍一下你自己。"
if "%~2" NEQ "" set "PROMPT=%~2"

echo ========================================
echo   gemini stream Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example gemini_stream -- "%API_KEY%" "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
