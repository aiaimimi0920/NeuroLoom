@echo off
setlocal

set "FILE=%~dp0.dev.vars"

if not exist "%FILE%" (
  echo [lock_dev_vars] MISSING: %FILE%
  echo Create it first (copy from platformtools\.dev.vars.example).
  exit /b 2
)

echo [lock_dev_vars] Applying ACL deny-delete for user %USERNAME% on:
echo   %FILE%
echo.
echo This helps prevent accidental deletion of the secrets file.
echo NOTE: This may also block renaming/moving the file until unlocked.
echo.

icacls "%FILE%" /deny "%USERNAME%":(D) >nul
if errorlevel 1 (
  echo [lock_dev_vars] FAILED. You may need to own the file or run as Administrator.
  exit /b 1
)

echo [lock_dev_vars] OK
exit /b 0
