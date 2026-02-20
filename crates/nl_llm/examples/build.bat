@echo off
chcp 65001 >nul
cd /d "%~dp0"

set CARGO=C:\Users\Administrator\.cargo\bin\cargo.exe

echo.
echo ========================================
echo Build iFlow Test Examples
echo ========================================
echo.

cd ..\..\..

echo [1/2] Building project...
%CARGO% build
if %ERRORLEVEL% neq 0 (
    echo [Error] Build failed
    pause
    exit /b 1
)

echo.
echo [2/2] Building examples...
%CARGO% build --examples
if %ERRORLEVEL% neq 0 (
    echo [Error] Examples build failed
    pause
    exit /b 1
)

echo.
echo ========================================
echo Build Complete!
echo ========================================
echo.
echo Test scripts (in crates\nl_llm\examples):
echo   test_auth.bat   - Test iFlow auth
echo   test_chat.bat   - Test iFlow chat
echo   test_models.bat - Query available models
echo.
echo Config: iflow_config.txt
echo.

pause
