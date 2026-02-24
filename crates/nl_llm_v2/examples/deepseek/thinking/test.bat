@echo off
REM DeepSeek 推理模式测试
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

REM 检查环境变量
if "%DEEPSEEK_API_KEY%"=="" (
    if "%1"=="" (
        echo 错误: 请设置 DEEPSEEK_API_KEY 环境变量或作为第一个参数传入
        exit /b 1
    )
    set API_KEY=%1
    shift
) else (
    set API_KEY=%DEEPSEEK_API_KEY%
)

REM 默认 prompt
if "%1"=="" (
    set PROMPT=如果一个农夫有17只羊，除了9只以外都死了，农夫还有多少只羊？
) else (
    set PROMPT=%1
)

echo ========================================
echo   DeepSeek Thinking Mode Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example deepseek_thinking -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
