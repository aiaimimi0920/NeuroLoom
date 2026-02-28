@echo off
REM cloudflare 平台测试 - chat
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%CLOUDFLARE_API_KEY%"=="" (
    if "%1"=="" (
        echo Warning: No CLOUDFLARE_API_KEY provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%CLOUDFLARE_API_KEY%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   cloudflare chat Test
echo ========================================
echo.

cargo run --example cloudflare_chat -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
