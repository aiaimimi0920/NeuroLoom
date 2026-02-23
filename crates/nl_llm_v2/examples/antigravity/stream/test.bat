@echo off
REM antigravity 平台测试 - stream
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%ANTIGRAVITY_API_KEY%"=="" (
    if "%1"=="" (
        echo Warning: No ANTIGRAVITY_API_KEY provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%ANTIGRAVITY_API_KEY%
)

if "%1"=="" (
    set PROMPT=你好！请给我讲一个长篇故事。
) else (
    set PROMPT=%1
)

echo ========================================
echo   antigravity stream Test
echo ========================================
echo.

cargo run --example antigravity_stream -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
