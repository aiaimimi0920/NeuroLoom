@echo off
REM modelscope 平台测试 - auth
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%MODELSCOPE_API_KEY%"=="" (
    if "%1"=="" (
        echo Warning: No MODELSCOPE_API_KEY provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%MODELSCOPE_API_KEY%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   modelscope auth Test
echo ========================================
echo.

cargo run --example modelscope_auth -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
