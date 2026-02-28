@echo off
REM xai 平台测试 - models
REM 用法: test.bat [api_key]
REM 也可在 examples\xai\.env 中设置 XAI_API_KEY

cd /d "%~dp0"

REM 尝试从本地 .env 文件加载密钥
if exist "..\\.env" call "..\\.env"

if "%XAI_API_KEY%"=="" (
    if "%1"=="" (
        echo Error: No XAI_API_KEY provided.
        echo Please set XAI_API_KEY env var, pass as arg, or create examples\xai\.env
        exit /b 1
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%XAI_API_KEY%
)

echo ========================================
echo   xai models Test
echo ========================================
echo.

cargo run --example xai_models -- %API_KEY%

echo.
echo ========================================
echo   Test Complete
echo ========================================
