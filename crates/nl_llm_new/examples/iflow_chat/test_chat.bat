@echo off
chcp 65001 >nul
cd /d "%~dp0..\.."

set CARGO=C:\Users\Administrator\.cargo\bin\cargo.exe

echo.
echo ========================================
echo iFlow Chat Test (nl_llm_new)
echo ========================================
echo Models: qwen3-max, deepseek-v3.2, glm-4.6
echo         kimi-k2, qwen3-coder-plus, deepseek-r1
echo ========================================
echo.

%CARGO% run --example iflow_chat -p nl_llm_new

echo.
pause
