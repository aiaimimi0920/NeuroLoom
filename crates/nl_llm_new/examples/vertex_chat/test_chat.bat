@echo off
setlocal EnableDelayedExpansion
chcp 65001 >nul

set "SA_JSON_PATH=%~dp0..\vertex\vertex_sa.json"
set "MODEL=gemini-2.5-flash"
set "PROMPT=%~1"
if "%PROMPT%"=="" set "PROMPT=Hello!"

echo ========================================
echo   Vertex AI Chat Test (nl_llm_new)
echo ========================================
echo.

where cargo >nul 2>&1
if !errorlevel! neq 0 (
    if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
        set "PATH=%PATH%;%USERPROFILE%\.cargo\bin"
    )
)

echo Checking SA file: %SA_JSON_PATH%
if not exist "%SA_JSON_PATH%" (
    echo ERROR: File not found!
    echo Please place vertex_sa.json in examples\vertex\
    pause
    exit /b 1
)
echo File found, proceeding...
echo.

set "PROJECT_ROOT=%~dp0..\..\..\.."
set "EXE=%PROJECT_ROOT%\target\debug\examples\vertex_chat.exe"

echo Building...
cargo build --example vertex_chat -p nl_llm_new
if !errorlevel! neq 0 (
    echo Build failed!
    pause
    exit /b 1
)
echo.

echo Running non-streaming test...
"%EXE%" "%PROMPT%" --sa "%SA_JSON_PATH%" --model %MODEL%
echo.

echo Running streaming test...
"%EXE%" "%PROMPT%" --sa "%SA_JSON_PATH%" --model %MODEL% --stream
echo.

echo Done!
pause
