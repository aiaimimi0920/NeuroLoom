@echo off
chcp 65001 >nul
cd /d "%~dp0"
set "LOCKFILE=%~dp0_task.lock"
if not exist "%LOCKFILE%" goto :RUN
powershell -NoProfile -Command "if(((Get-Date)-(Get-Item -LiteralPath '%LOCKFILE%').LastWriteTime).TotalMinutes -lt 10){exit 1}" >nul 2>nul
if errorlevel 1 exit /b 0
del "%LOCKFILE%" >nul 2>nul
:RUN
echo %date% %time% > "%LOCKFILE%"
call 单次续杯.bat --from-task
del "%LOCKFILE%" >nul 2>nul
