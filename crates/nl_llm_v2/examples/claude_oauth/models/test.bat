@echo off
REM claude_oauth 平台测试 - models
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%CLAUDE_OAUTH_API_KEY%"=="" (
    if "%1"=="" (
        echo Warning: No CLAUDE_OAUTH_API_KEY provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%CLAUDE_OAUTH_API_KEY%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   claude_oauth models Test
echo ========================================
echo.

cargo run --example claude_oauth_models -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
