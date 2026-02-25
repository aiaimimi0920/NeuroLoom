@echo off
cd /d "%~dp0"
echo ========================================
echo   DMXAPI Tools Test
echo ========================================

REM 从 .env.local 加载环境变量
if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="DMXAPI_API_KEY" set "DMXAPI_API_KEY=%%b"
    )
)

if "%DMXAPI_API_KEY%"=="" (
    echo 错误: 未设置 DMXAPI_API_KEY 环境变量
    echo.
    echo 请通过以下方式之一设置:
    echo   1. 设置环境变量: set DMXAPI_API_KEY=your-key
    echo   2. 在 examples/.env.local 文件中添加: DMXAPI_API_KEY=your-key
    echo.
    echo 获取密钥: https://www.dmxapi.cn
    pause
    exit /b 1
)

set "PROMPT=%~1"
if "%PROMPT%"=="" set "PROMPT=北京和上海今天的天气怎么样？"

cargo run -p nl_llm_v2 --example dmxapi_tools -- "%DMXAPI_API_KEY%" "%PROMPT%"
echo ========================================
echo   Test Complete
echo ========================================
pause
