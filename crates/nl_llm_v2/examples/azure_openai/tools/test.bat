@echo off
cd /d "%~dp0"
echo ========================================
echo   Azure OpenAI Tools Test
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
    pause
    exit /b 1
)

set "DEPLOYMENT=%~1"
if "%DEPLOYMENT%"=="" set "DEPLOYMENT=gpt-4o"

set "PROMPT=%~2"
if "%PROMPT%"=="" set "PROMPT=北京和上海今天的天气怎么样？"

echo Endpoint: %AZURE_OPENAI_ENDPOINT%
echo Deployment: %DEPLOYMENT%
echo Prompt: %PROMPT%
echo.

cargo run -p nl_llm_v2 --example azure_openai_tools -- "%AZURE_OPENAI_API_KEY%" "%AZURE_OPENAI_ENDPOINT%" "%DEPLOYMENT%" "%PROMPT%"
echo ========================================
echo   Test Complete
echo ========================================
pause
