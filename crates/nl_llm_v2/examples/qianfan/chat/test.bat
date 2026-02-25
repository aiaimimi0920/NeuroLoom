@echo off
cd /d "%~dp0"
echo ========================================
echo   百度千帆 Chat Test
echo ========================================
echo.
echo  请设置环境变量: QIANFAN_API_KEY=xxx
echo  或在 examples\.env.local 中配置:
echo    QIANFAN_API_KEY=your-api-key
echo.
echo  获取密钥: https://qianfan.cloud.baidu.com
echo.

if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="QIANFAN_API_KEY" set "QIANFAN_API_KEY=%%b"
    )
)

if "%QIANFAN_API_KEY%"=="" (
    echo   [ERROR] 请配置 QIANFAN_API_KEY
    pause
    exit /b 1
)

cargo run -p nl_llm_v2 --example qianfan_chat -- "%QIANFAN_API_KEY%"
echo ========================================
echo   Test Complete
echo ========================================
pause
