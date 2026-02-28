@echo off
REM cephalon 平台测试 - chat
REM 用法: test.bat [api_key] [prompt]
REM 密钥获取: https://cephalon.cloud/apitoken/

cd /d "%~dp0"

REM 尝试加载本地密钥配置（如果存在）
if exist "..\\local_keys.bat" call "..\\local_keys.bat"

REM 检查 API Key
if "%CEPHALON_API_KEY%"=="" (
    if "%1"=="" (
        echo 错误: 请设置 CEPHALON_API_KEY 环境变量或作为第一个参数传入
        echo 密钥获取地址: https://cephalon.cloud/apitoken/
        exit /b 1
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%CEPHALON_API_KEY%
)

REM 默认 prompt
if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   Cephalon Chat Test
echo ========================================
echo   API: https://cephalon.cloud/user-center/v1/model
echo ========================================
echo.

cargo run --example cephalon_chat -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
