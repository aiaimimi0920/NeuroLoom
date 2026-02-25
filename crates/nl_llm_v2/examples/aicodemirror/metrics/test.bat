@echo off
setlocal enabledelayedexpansion

echo ========================================
echo   AICodeMirror 指标收集
echo ========================================
echo.

cargo run -p nl_llm_v2 --example aicodemirror_metrics -- %*
