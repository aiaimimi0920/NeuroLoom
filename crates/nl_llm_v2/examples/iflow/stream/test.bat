@echo off
REM iflow 平台测试 - stream
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%IFLOW_COOKIE%"=="" (
    if "%1"=="" (
        echo Warning: No IFLOW_COOKIE provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%IFLOW_COOKIE%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   iflow stream Test
echo ========================================
echo.

cargo run --example iflow_stream -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
