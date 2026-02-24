@echo off
REM vertex 平台测试 - chat
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%GOOGLE_APPLICATION_CREDENTIALS_JSON%"=="" (
    if "%1"=="" (
        echo Warning: No GOOGLE_APPLICATION_CREDENTIALS_JSON provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%GOOGLE_APPLICATION_CREDENTIALS_JSON%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   vertex chat Test
echo ========================================
echo.

cargo run -p nl_llm_v2 --example vertex_chat -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
