@echo off
chcp 65001 >nul
setlocal

echo ========================================
echo   OpenAI Chat Test (nl_llm_new)
echo ========================================
echo.

set "PROMPT=%~1"
if "%PROMPT%"=="" set "PROMPT=Hello!"

if "%OPENAI_API_KEY%"=="" (
    echo [!] 请设置环境变量: set OPENAI_API_KEY=sk-...
    echo     或编辑此 bat 文件添加 API Key
    pause
    exit /b 1
)

echo API Key: %OPENAI_API_KEY:~0,8%...
echo.

set "PROJECT_ROOT=%~dp0..\..\..\.."
set "EXE=%PROJECT_ROOT%\target\debug\examples\openai_chat.exe"

echo [1/3] Building...
cargo build --example openai_chat -p nl_llm_new
if %errorlevel% neq 0 (
    echo Build failed!
    pause
    exit /b 1
)
echo.

echo [2/3] Non-streaming request...
echo.
"%EXE%" "%PROMPT%" --key %OPENAI_API_KEY%
echo.

echo [3/3] Streaming request...
echo.
"%EXE%" "%PROMPT%" --key %OPENAI_API_KEY% --stream
echo.

echo Done!
pause
