@echo off
chcp 65001 >nul
REM vertex 平台测试 - models
REM 用法: test.bat [sa_json_path]

cd /d "%~dp0"

set "SA_PATH=%~dp0..\vertex_sa.json"
if "%~1" NEQ "" set "SA_PATH=%~1"

echo ========================================
echo   vertex models Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example vertex_models -- "%SA_PATH%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
