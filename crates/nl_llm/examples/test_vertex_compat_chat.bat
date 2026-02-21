@echo off
setlocal EnableDelayedExpansion

REM --- Configuration (Modify here) ---
set "API_KEY="
set "BASE_URL="
set "MODEL=gemini-2.5-flash"
set "PROMPT=%~1"
if "%PROMPT%"=="" set "PROMPT=Hello!"
REM ----------------------------

echo ========================================
echo   Vertex Compat (Third-party Proxy) Chat Test
echo ========================================
echo.

where cargo >nul 2>&1
if !errorlevel! neq 0 (
    if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
        set "PATH=%PATH%;%USERPROFILE%\.cargo\bin"
    )
)

if "%API_KEY%"=="" (
    echo ERROR: No API Key configured!
    echo Please set API_KEY in this bat file.
    pause
    exit /b 1
)

if "%BASE_URL%"=="" (
    echo ERROR: No Base URL configured!
    echo Please set BASE_URL in this bat file.
    echo Example: set "BASE_URL=https://zenmux.ai/api"
    pause
    exit /b 1
)

echo API Key: %API_KEY:~0,8%...
echo Base URL: %BASE_URL%
echo Model: %MODEL%
echo.

set "PROJECT_ROOT=%~dp0..\..\.."
set "EXE=%PROJECT_ROOT%\target\debug\examples\test_vertex_compat_chat.exe"

echo Building...
cargo build --example test_vertex_compat_chat -p nl_llm
if !errorlevel! neq 0 (
    echo Build failed!
    pause
    exit /b 1
)
echo.

echo Running non-streaming test...
"%EXE%" "%PROMPT%" --key %API_KEY% --base-url %BASE_URL% --model %MODEL%
echo.

echo Running streaming test...
"%EXE%" "%PROMPT%" --key %API_KEY% --base-url %BASE_URL% --model %MODEL% --stream
echo.

echo Done!
pause
