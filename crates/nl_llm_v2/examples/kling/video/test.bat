@echo off
setlocal

cd /d "%~dp0\.."
if exist .env (
    for /f "usebackq tokens=1,* delims==" %%A in (".env") do (
        set "%%A=%%B"
    )
) else (
    echo [ERROR] .env file not found in %~dp0\..
    echo Please create an .env file with KLING_CREDENTIALS=AccessKey^|SecretKey
    exit /b 1
)

cd /d "%~dp0\..\..\.."
echo [INFO] Running Kling Video generation example...
cargo run --example kling_video
