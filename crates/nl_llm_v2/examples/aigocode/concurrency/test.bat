@echo off
setlocal enabledelayedexpansion

echo ========================================
echo   AIGoCode 并发控制测试
echo ========================================
echo.

cargo run -p nl_llm_v2 --example aigocode_concurrency -- %*
