@echo off
cd /d "%~dp0"
echo ========================================
echo   Azure OpenAI Stream Test
echo ========================================

if "%AZURE_OPENAI_API_KEY%"=="" (
    echo 错误: 未设置 AZURE_OPENAI_API_KEY 环境变量
    echo.
    echo 请设置:
    echo   set AZURE_OPENAI_API_KEY=your-key
    echo   set AZURE_OPENAI_ENDPOINT=https://YOUR-RESOURCE.openai.azure.com
    pause
    exit /b 1
)

if "%AZURE_OPENAI_ENDPOINT%"=="" (
    echo 错误: 未设置 AZURE_OPENAI_ENDPOINT 环境变量
    echo.
    echo 示例: https://YOUR-RESOURCE.openai.azure.com
    pause
    exit /b 1
)

set "DEPLOYMENT=%~1"
if "%DEPLOYMENT%"=="" set "DEPLOYMENT=gpt-4o"

echo Endpoint: %AZURE_OPENAI_ENDPOINT%
echo Deployment: %DEPLOYMENT%
echo.

cargo run -p nl_llm_v2 --example azure_openai_stream -- "%AZURE_OPENAI_API_KEY%" "%AZURE_OPENAI_ENDPOINT%" "%DEPLOYMENT%"
echo ========================================
echo   Test Complete
echo ========================================
pause
