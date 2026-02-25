@echo off
cd /d "%~dp0"
echo ========================================
echo   AWS Claude AK/SK Chat Test
echo ========================================
echo.
echo  请设置以下环境变量:
echo    AWS_ACCESS_KEY_ID=AKIA...
echo    AWS_SECRET_ACCESS_KEY=xxxxx
echo    AWS_REGION=us-east-1  (可选)
echo.

if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="AWS_ACCESS_KEY_ID" set "AWS_ACCESS_KEY_ID=%%b"
        if "%%a"=="AWS_SECRET_ACCESS_KEY" set "AWS_SECRET_ACCESS_KEY=%%b"
        if "%%a"=="AWS_REGION" set "AWS_REGION=%%b"
    )
)

if "%AWS_ACCESS_KEY_ID%"=="" (
    echo   [ERROR] 请在 examples\.env.local 中配置 AWS_ACCESS_KEY_ID
    pause
    exit /b 1
)
if "%AWS_SECRET_ACCESS_KEY%"=="" (
    echo   [ERROR] 请在 examples\.env.local 中配置 AWS_SECRET_ACCESS_KEY
    pause
    exit /b 1
)
if "%AWS_REGION%"=="" set AWS_REGION=us-east-1

cargo run -p nl_llm_v2 --example aws_claude_ak_chat -- "%AWS_ACCESS_KEY_ID%" "%AWS_SECRET_ACCESS_KEY%" "%AWS_REGION%"
echo ========================================
echo   Test Complete
echo ========================================
pause
