@echo off
REM ocoolAI 平台基础对话测试
REM 用法: test.bat

cd /d "%~dp0"

REM 设置 API Key（明文展示方便测试）
set OCOOLAI_API_KEY=sk-URZYQTy7XMIRx6xm4a2297E0E3Ae4fE1B01172A58f71372d

REM 默认 prompt
if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己，并告诉我你能提供哪些帮助。
) else (
    set PROMPT=%1
)

echo ========================================
echo   ocoolAI Chat Test
echo ========================================
echo.
echo API Key: %OCOOLAI_API_KEY%
echo Prompt: %PROMPT%
echo.

cargo run --example ocoolai_chat -- "%OCOOLAI_API_KEY%" "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
