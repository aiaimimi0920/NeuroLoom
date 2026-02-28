@echo off
REM siliconflow 平台测试 - chat
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%SILICONFLOW_API_KEY%"=="" (
    if "%1"=="" (
        echo Warning: No SILICONFLOW_API_KEY provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%SILICONFLOW_API_KEY%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   siliconflow chat Test
echo ========================================
echo.

cargo run --example siliconflow_chat -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
