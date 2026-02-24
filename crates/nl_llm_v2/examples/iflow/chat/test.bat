@echo off
REM iflow 基础对话测试
REM Cookie 从 examples/iflow/iflow_config.txt 自动读取

cd /d "%~dp0\..\..\.."

echo ========================================
echo   iFlow Chat Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example iflow_chat

echo.
echo ========================================
echo   Test Complete
echo ========================================
