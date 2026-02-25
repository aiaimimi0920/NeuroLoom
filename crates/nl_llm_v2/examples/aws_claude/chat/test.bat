@echo off
cd /d "%~dp0"
echo ========================================
echo   AWS Claude Chat Test
echo ========================================
echo.
echo  请设置以下环境变量之一:
echo    API Key 模式: AWS_BEDROCK_API_KEY=xxx
echo    AK/SK 模式:   AWS_ACCESS_KEY_ID + AWS_SECRET_ACCESS_KEY
echo.

if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="AWS_BEDROCK_API_KEY" set "AWS_BEDROCK_API_KEY=%%b"
    )
)

if "%AWS_BEDROCK_API_KEY%"=="" (
    echo   [ERROR] 请在 examples\.env.local 中配置 AWS_BEDROCK_API_KEY
    echo   或设置环境变量 AWS_BEDROCK_API_KEY
    pause
    exit /b 1
)

cargo run -p nl_llm_v2 --example aws_claude_chat -- "%AWS_BEDROCK_API_KEY%"
echo ========================================
echo   Test Complete
echo ========================================
pause
