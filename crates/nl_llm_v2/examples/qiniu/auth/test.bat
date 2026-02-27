@echo off
REM qiniu 平台测试
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%QINIU_API_KEY%"=="" (
    if "%NL_API_KEY%"=="" (
        if "%1"=="" (
            echo Warning: No QINIU_API_KEY/NL_API_KEY provided.
            set API_KEY=dummy_credential
        ) else (
            set API_KEY=%1
            shift
        )
    ) else (
        set API_KEY=%NL_API_KEY%
    )
) else (
    set API_KEY=%QINIU_API_KEY%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   qiniu TEST
echo ========================================
echo.

cargo run --example qiniu_auth -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
