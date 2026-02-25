@echo off
REM qwen_coder 平台测试 - stream
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%QWEN_CODER_API_KEY%"=="" (
    if "%1"=="" (
        echo Warning: No QWEN_CODER_API_KEY provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%QWEN_CODER_API_KEY%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   qwen_coder stream Test
echo ========================================
echo.

cargo run --example qwen_coder_stream -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
