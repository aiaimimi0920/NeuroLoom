@echo off
chcp 65001 >nul
cd /d "%~dp0..\..\.."

echo.
echo ========================================
echo iFlow Chat Test (nl_llm_new) - Streaming
echo ========================================
echo Models: qwen3-max, deepseek-v3.2, glm-4.6
echo         kimi-k2, qwen3-coder-plus, deepseek-r1
echo ========================================
echo.

cargo run --example iflow_chat -p nl_llm_new -- "你好！介绍一下如果我用Rust写异步SSE你会给我什么建议？" --stream

echo.
pause
