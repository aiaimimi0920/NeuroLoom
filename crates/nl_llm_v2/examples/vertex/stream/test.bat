@echo off
chcp 65001 >nul
REM vertex 平台测试 - stream
REM 用法: test.bat [sa_json_path] [prompt]

cd /d "%~dp0"

set "SA_PATH=%~dp0..\vertex_sa.json"
if "%~1" NEQ "" set "SA_PATH=%~1"

set "PROMPT=你好！请简单介绍一下你自己。"
if "%~2" NEQ "" set "PROMPT=%~2"

echo ========================================
echo   vertex stream Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example vertex_stream -- "%SA_PATH%" "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
