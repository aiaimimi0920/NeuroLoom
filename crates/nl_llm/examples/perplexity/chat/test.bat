@echo off
REM perplexity 平台测试 - chat
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%PERPLEXITY_API_KEY%"=="" (
    if "%1"=="" (
        echo Warning: No PERPLEXITY_API_KEY provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%PERPLEXITY_API_KEY%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   perplexity chat Test
echo ========================================
echo.

cargo run --example perplexity_chat -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
