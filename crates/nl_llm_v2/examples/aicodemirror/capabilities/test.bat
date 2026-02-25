@echo off
setlocal enabledelayedexpansion

echo ========================================
echo   AICodeMirror 模型能力检测
echo ========================================
echo.

cargo run -p nl_llm_v2 --example aicodemirror_capabilities -- %*
