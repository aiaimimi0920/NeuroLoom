@echo off
REM kimi 平台测试 - models
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%KIMI_API_KEY%"=="" (
    if "%1"=="" (
        echo Warning: No KIMI_API_KEY provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%KIMI_API_KEY%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   kimi models Test
echo ========================================
echo.

cargo run --example kimi_models -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
