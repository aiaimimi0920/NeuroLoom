@echo off
REM gemini_cli 平台测试 - auth
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%GEMINI_CLI_API_KEY%"=="" (
    if "%1"=="" (
        echo Warning: No GEMINI_CLI_API_KEY provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%GEMINI_CLI_API_KEY%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   gemini_cli auth Test
echo ========================================
echo.

cargo run --example gemini_cli_auth -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
