@echo off
setlocal EnableDelayedExpansion
chcp 65001 >nul

set "API_KEY=AIzaSyBtCqclXkiT7l9spBtTDGFLr_ssZikiY54"
set "MODEL=gemini-2.5-flash"
set "PROMPT=%~1"
if "%PROMPT%"=="" set "PROMPT=Hello!"

echo ========================================
echo   Google AI Studio Chat Test (nl_llm_new)
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
    echo Get your key from: https://aistudio.google.com/app/apikey
    pause
    exit /b 1
)

echo API Key: %API_KEY:~0,8%...
echo Model: %MODEL%
echo.

set "PROJECT_ROOT=%~dp0..\..\..\.."
set "EXE=%PROJECT_ROOT%\target\debug\examples\google_ai_studio_chat.exe"

echo Building...
cargo build --example google_ai_studio_chat -p nl_llm_new
if !errorlevel! neq 0 (
    echo Build failed!
    pause
    exit /b 1
)
echo.

echo Running non-streaming test...
"%EXE%" "%PROMPT%" --key %API_KEY% --model %MODEL%
echo.

echo Running streaming test...
"%EXE%" "%PROMPT%" --key %API_KEY% --model %MODEL% --stream
echo.

echo Done!
pause
