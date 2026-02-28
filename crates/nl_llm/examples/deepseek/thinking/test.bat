@echo off
REM deepseek 平台测试 - thinking
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%DEEPSEEK_API_KEY%"=="" (
    if "%1"=="" (
        echo Warning: No DEEPSEEK_API_KEY provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%DEEPSEEK_API_KEY%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   deepseek thinking Test
echo ========================================
echo.

cargo run --example deepseek_thinking -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
