@echo off
REM doubaoseed 平台测试 - models
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%DOUBAOSEED_API_KEY%"=="" (
    if "%1"=="" (
        echo Warning: No DOUBAOSEED_API_KEY provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%DOUBAOSEED_API_KEY%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   doubaoseed models Test
echo ========================================
echo.

cargo run --example doubaoseed_models -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
