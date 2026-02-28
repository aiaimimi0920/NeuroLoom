@echo off
REM ocoolAI 平台流式输出测试
REM 用法: test.bat [prompt]

cd /d "%~dp0"

REM 设置 API Key（明文展现方便测试）
set OCOOLAI_API_KEY=sk-URZYQTy7XMIRx6xm4a2297E0E3Ae4fE1B01172A58f71372d

REM 默认 prompt
if "%1"=="" (
    set PROMPT=请写一首关于人工智能的短诗，大约4-6行。
) else (
    set PROMPT=%1
)

echo ========================================
echo   ocoolAI Stream Test
echo ========================================
echo.
echo API Key: %OCOOLAI_API_KEY%
echo Prompt: %PROMPT%
echo.

cargo run --example ocoolai_stream -- "%OCOOLAI_API_KEY%" "%PROMPT%"

echo.
echo ========================================
echo   Stream Test Complete
echo ========================================
