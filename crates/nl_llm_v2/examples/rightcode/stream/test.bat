@echo off
cd /d "%~dp0"
echo ========================================
echo   RightCode Stream Test
echo ========================================

REM 从 .env.local 加载环境变量
if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="RIGHTCODE_API_KEY" set "RIGHTCODE_API_KEY=%%b"
    )
)

if "%RIGHTCODE_API_KEY%"=="" (
    echo 错误: 未设置 RIGHTCODE_API_KEY 环境变量
    echo.
    echo 请通过以下方式之一设置:
    echo   1. 设置环境变量: set RIGHTCODE_API_KEY=your-key
    echo   2. 在 examples/.env.local 文件中添加: RIGHTCODE_API_KEY=your-key
    echo.
    echo 获取密钥: https://right.codes
    pause
    exit /b 1
)

set "PROMPT=%~1"
if "%PROMPT%"=="" set "PROMPT=写一段简单的快速排序，不要解释"

cargo run -p nl_llm_v2 --example rightcode_stream -- "%RIGHTCODE_API_KEY%" "%PROMPT%"
echo ========================================
echo   Test Complete
echo ========================================
pause
