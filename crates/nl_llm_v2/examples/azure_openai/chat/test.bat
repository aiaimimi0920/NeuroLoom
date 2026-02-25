@echo off
cd /d "%~dp0"
echo ========================================
echo   Azure OpenAI Chat Test
echo ========================================

if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="AZURE_OPENAI_ENDPOINT" set "AZURE_OPENAI_ENDPOINT=%%b"
        if "%%a"=="AZURE_OPENAI_KEY" set "AZURE_OPENAI_KEY=%%b"
        if "%%a"=="AZURE_DEPLOYMENT" set "AZURE_DEPLOYMENT=%%b"
    )
)

if "%AZURE_OPENAI_ENDPOINT%"=="" (
    echo   [ERROR] 请在 examples\.env.local 中配置 AZURE_OPENAI_ENDPOINT
    pause
    exit /b 1
)
if "%AZURE_OPENAI_KEY%"=="" (
    echo   [ERROR] 请在 examples\.env.local 中配置 AZURE_OPENAI_KEY
    pause
    exit /b 1
)
if "%AZURE_DEPLOYMENT%"=="" set AZURE_DEPLOYMENT=gpt-4o

cargo run -p nl_llm_v2 --example azure_openai_chat -- "%AZURE_OPENAI_ENDPOINT%" "%AZURE_OPENAI_KEY%" "%AZURE_DEPLOYMENT%"
echo ========================================
echo   Test Complete
echo ========================================
pause
