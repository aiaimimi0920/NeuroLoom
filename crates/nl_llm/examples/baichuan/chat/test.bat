@echo off
setlocal enabledelayedexpansion
chcp 65001 > nul

cd /d "%~dp0\..\..\.."

:: 检查是否配置了外部的 baichuan_key.txt
set LOCAL_KEY_FILE=baichuan_key.txt
set API_KEY=
if exist "%LOCAL_KEY_FILE%" (
    set /p API_KEY=<"%LOCAL_KEY_FILE%"
)

if "%API_KEY%"=="" (
    if "%1"=="" (
        echo 警告: 未找到 baichuan_key.txt 且未提供参数作为 API Key。
        echo 请在 nl_llm 目录下创建 baichuan_key.txt 存放您的密钥。
        echo.
        echo 假装正在进行白盒测试（使用无效密钥可能导致 401 Authentication Error）
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
    )
)

set PROMPT=你好，请介绍一下百川大模型家族。
if not "%2"=="" set PROMPT=%2

echo [1/2] 编译基础对话示例...
echo.
cargo build --example baichuan_chat
if %errorlevel% neq 0 (
    echo [错误] 编译失败！
    pause
    exit /b %errorlevel%
)

echo.
echo [2/2] 执行 Baichuan 基础对话示例...
echo =======================================================
cargo run --example baichuan_chat -- "%API_KEY%" "%PROMPT%"
echo =======================================================

echo.
pause
