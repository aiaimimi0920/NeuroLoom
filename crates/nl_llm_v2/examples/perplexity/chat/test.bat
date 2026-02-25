@echo off
cd /d "%~dp0"
echo ========================================
echo   Perplexity AI Chat Test
echo ========================================
echo.
echo  请设置环境变量: PERPLEXITY_API_KEY=pplx-xxxx
echo  或在 examples\.env.local 中配置
echo  获取密钥: https://www.perplexity.ai (Settings -^> API)
echo.

if exist "%~dp0..\..\.env.local" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%~dp0..\..\.env.local") do (
        if "%%a"=="PERPLEXITY_API_KEY" set "PERPLEXITY_API_KEY=%%b"
    )
)

if "%PERPLEXITY_API_KEY%"=="" (
    echo   [ERROR] 请配置 PERPLEXITY_API_KEY
    pause
    exit /b 1
)

cargo run -p nl_llm_v2 --example perplexity_chat -- "%PERPLEXITY_API_KEY%"
echo ========================================
echo   Test Complete
echo ========================================
pause
