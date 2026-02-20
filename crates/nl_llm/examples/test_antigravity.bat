@echo off
chcp 65001 >nul
cd /d "%~dp0"

set CARGO=C:\Users\Administrator\.cargo\bin\cargo.exe

echo.
echo ========================================
echo Antigravity (Gemini Code Assist) Auth Test
echo ========================================
echo.

if not exist "..\..\..\target\debug\examples\test_antigravity_auth.exe" (
    echo Building...
    %CARGO% build --example test_antigravity_auth
)

..\..\..\target\debug\examples\test_antigravity_auth.exe

echo.
pause
