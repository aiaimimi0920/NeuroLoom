@echo off
REM aicodemirror 平台测试 - capabilities
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%AICODEMIRROR_API_KEY%"=="" (
    if "%1"=="" (
        echo Warning: No AICODEMIRROR_API_KEY provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%AICODEMIRROR_API_KEY%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   aicodemirror capabilities Test
echo ========================================
echo.

cargo run --example aicodemirror_capabilities -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
