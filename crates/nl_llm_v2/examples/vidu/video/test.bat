@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM Vidu video generation example
REM Usage: test.bat

cd /d "%~dp0\.."
if not exist .env (
    echo [ERROR] .env file not found in %~dp0\..
    echo Please copy .env.example to .env and set VIDU_API_KEY
    exit /b 1
)

REM Load .env supports both:
REM 1) KEY=VALUE
REM 2) set KEY=VALUE  (bat-style)
for /f "usebackq delims=" %%L in (".env") do (
    set "line=%%L"

    REM skip empty lines
    if "!line!"=="" (
        REM noop
    ) else (
        REM skip comments
        if /i "!line:~0,3!"=="REM" (
            REM noop
        ) else if "!line:~0,1!"=="#" (
            REM noop
        ) else if "!line:~0,1!"=="@" (
            REM noop
        ) else (
            REM normalize 'set KEY=VALUE'
            if /i "!line:~0,4!"=="set " set "line=!line:~4!"

            for /f "tokens=1,* delims==" %%A in ("!line!") do (
                if /i "%%A"=="VIDU_API_KEY" set "VIDU_API_KEY=%%B"
                if /i "%%A"=="VIDU_API_BASE_URL" set "VIDU_API_BASE_URL=%%B"
                if /i "%%A"=="VIDU_MODEL" set "VIDU_MODEL=%%B"
                if /i "%%A"=="VIDU_DURATION" set "VIDU_DURATION=%%B"
                if /i "%%A"=="VIDU_RESOLUTION" set "VIDU_RESOLUTION=%%B"
                if /i "%%A"=="VIDU_MOVEMENT_AMPLITUDE" set "VIDU_MOVEMENT_AMPLITUDE=%%B"
                if /i "%%A"=="VIDU_BGM" set "VIDU_BGM=%%B"
                if /i "%%A"=="VIDU_SEED" set "VIDU_SEED=%%B"
                if /i "%%A"=="VIDU_CALLBACK_URL" set "VIDU_CALLBACK_URL=%%B"
                if /i "%%A"=="VIDU_PAYLOAD" set "VIDU_PAYLOAD=%%B"
            )
        )
    )
)

if "%VIDU_API_KEY%"=="" (
    echo [ERROR] VIDU_API_KEY not found in .env
    exit /b 1
)

cd /d "%~dp0\..\..\.."

echo [INFO] Running Vidu video generation example...

cargo run --example vidu_video
