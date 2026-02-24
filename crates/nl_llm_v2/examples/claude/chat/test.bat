@echo off
REM Claude API Key 平台测试 - chat
REM 用法: test.bat [api_key] [prompt]
setlocal

cd /d "%~dp0\..\.."

if "%ANTHROPIC_API_KEY%"=="" (
    if "%1"=="" (
        echo Warning: No ANTHROPIC_API_KEY provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%ANTHROPIC_API_KEY%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   Claude Chat Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example claude_chat -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
pause
