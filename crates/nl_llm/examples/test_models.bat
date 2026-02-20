@echo off
chcp 65001 >nul
cd /d "%~dp0"

set CARGO=C:\Users\Administrator\.cargo\bin\cargo.exe

echo.
echo ========================================
echo iFlow Models Query
echo ========================================
echo.

if not exist "..\..\..\target\debug\examples\test_iflow_models.exe" (
    echo Building...
    %CARGO% build --example test_iflow_models
)

..\..\..\target\debug\examples\test_iflow_models.exe

echo.
pause
