@echo off
setlocal

set "FILE=%~dp0.dev.vars"

if not exist "%FILE%" (
  echo [unlock_dev_vars] MISSING: %FILE%
  exit /b 2
)

echo [unlock_dev_vars] Removing deny-delete ACL for user %USERNAME% on:
echo   %FILE%
echo.

icacls "%FILE%" /remove:d "%USERNAME%" >nul
if errorlevel 1 (
  echo [unlock_dev_vars] FAILED. You may need to own the file or run as Administrator.
  exit /b 1
)

echo [unlock_dev_vars] OK
exit /b 0
