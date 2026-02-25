@echo off
REM Kimi 余额查询测试
REM 用法: test.bat

cd /d "%~dp0"

REM 检查环境变量
if "%KIMI_API_KEY%"=="" (
    echo 错误: 请设置 KIMI_API_KEY 环境变量
    exit /b 1
)

echo ========================================
echo   Kimi Balance Query Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example kimi_balance

echo.
echo ========================================
echo   Test Complete
echo ========================================
