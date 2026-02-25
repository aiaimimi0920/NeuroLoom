@echo off
cd /d "%~dp0"
echo ========================================
echo   Azure OpenAI Auth Test
echo ========================================
echo.
echo  首次使用请设置以下环境变量:
echo    AZURE_OPENAI_ENDPOINT=https://YOUR-RESOURCE.openai.azure.com
echo    AZURE_OPENAI_KEY=your-api-key
echo    AZURE_DEPLOYMENT=your-deployment-name  (默认: gpt-4o)
echo.

if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="AZURE_OPENAI_ENDPOINT" set "AZURE_OPENAI_ENDPOINT=%%b"
        if "%%a"=="AZURE_OPENAI_KEY" set "AZURE_OPENAI_KEY=%%b"
        if "%%a"=="AZURE_DEPLOYMENT" set "AZURE_DEPLOYMENT=%%b"
    )
)

if "%AZURE_OPENAI_ENDPOINT%"=="" (
    echo   [ERROR] 请设置 AZURE_OPENAI_ENDPOINT 环境变量
    echo   或在 examples\.env.local 中配置:
    echo     AZURE_OPENAI_ENDPOINT=https://your-resource.openai.azure.com
    echo     AZURE_OPENAI_KEY=your-api-key
    echo     AZURE_DEPLOYMENT=gpt-4o
    pause
    exit /b 1
)
if "%AZURE_OPENAI_KEY%"=="" (
    echo   [ERROR] 请设置 AZURE_OPENAI_KEY 环境变量
    pause
    exit /b 1
)
if "%AZURE_DEPLOYMENT%"=="" (
    set AZURE_DEPLOYMENT=gpt-4o
)

cargo run -p nl_llm_v2 --example azure_openai_auth -- "%AZURE_OPENAI_ENDPOINT%" "%AZURE_OPENAI_KEY%" "%AZURE_DEPLOYMENT%"
echo ========================================
echo   Test Complete
echo ========================================
pause
