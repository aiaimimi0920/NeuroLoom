@echo off
setlocal enabledelayedexpansion

echo ========================================
echo   AIGoCode 模型能力检测
echo ========================================
echo.

cargo run -p nl_llm_v2 --example aigocode_capabilities -- %*
