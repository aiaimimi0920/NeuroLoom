@echo off
chcp 65001 >nul
setlocal

echo ========================================
echo   Claude Chat Test (nl_llm_new)
echo ========================================
echo.

set "PROMPT=%~1"
if "%PROMPT%"=="" set "PROMPT=Hello!"

if "%ANTHROPIC_API_KEY%"=="" (
    echo [!] 请设置环境变量: set ANTHROPIC_API_KEY=sk-ant-...
    echo     或编辑此 bat 文件添加 API Key
    pause
    exit /b 1
)

echo API Key: %ANTHROPIC_API_KEY:~0,8%...
echo.

set "PROJECT_ROOT=%~dp0..\..\..\.."
set "EXE=%PROJECT_ROOT%\target\debug\examples\claude_chat.exe"

echo [1/3] Building...
cargo build --example claude_chat -p nl_llm_new
if %errorlevel% neq 0 (
    echo Build failed!
    pause
    exit /b 1
)
echo.

echo [2/3] Non-streaming request...
echo.
"%EXE%" "%PROMPT%" --key %ANTHROPIC_API_KEY%
echo.

echo [3/3] Streaming request...
echo.
"%EXE%" "%PROMPT%" --key %ANTHROPIC_API_KEY% --stream
echo.

echo Done!
pause
