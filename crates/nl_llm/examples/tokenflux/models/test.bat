@echo off
REM TokenFlux 平台模型列表测试
REM 用法: test.bat [api_key]

cd /d "%~dp0"

if "%TOKENFLUX_API_KEY%"=="" (
    if "%1"=="" (
        echo 错误: 请设置 TOKENFLUX_API_KEY 环境变量或作为第一个参数传入
        exit /b 1
    )
    set API_KEY=%1
) else (
    set API_KEY=%TOKENFLUX_API_KEY%
)

echo ========================================
echo   TokenFlux Models Test
echo ========================================

echo Using API Key: %API_KEY:~0,8%...
echo.

cargo run --example tokenflux_models -- "%API_KEY%"

if %ERRORLEVEL% neq 0 (
    echo.
    echo ❌ 测试失败
    exit /b %ERRORLEVEL%
)

echo.
echo ✅ 测试通过
