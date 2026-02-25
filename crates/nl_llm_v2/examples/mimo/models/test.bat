@echo off
REM mimo 平台测试 - models
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%MIMO_API_KEY%"=="" (
    if "%1"=="" (
        echo Warning: No MIMO_API_KEY provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%MIMO_API_KEY%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   mimo models Test
echo ========================================
echo.

cargo run --example mimo_models -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
