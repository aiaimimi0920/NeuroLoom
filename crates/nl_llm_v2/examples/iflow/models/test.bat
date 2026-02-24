@echo off
REM iflow 模型列表查询测试
REM Cookie 从 examples/iflow/iflow_config.txt 自动读取

cd /d "%~dp0\..\..\.."

echo ========================================
echo   iFlow Models Query Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example iflow_models

echo.
echo ========================================
echo   Test Complete
echo ========================================
