@echo off
setlocal enabledelayedexpansion

echo ========================================
echo   AICodeMirror 并发控制测试
echo ========================================
echo.

cargo run -p nl_llm_v2 --example aicodemirror_concurrency -- %*
