@echo off
setlocal EnableDelayedExpansion

:: 设置固定的 API 密钥，优先读取外部配置文件以免泄露
set "AK_FILE=%~dp0..\..\..\..\..\tencent_ak.txt"
set "SK_FILE=%~dp0..\..\..\..\..\tencent_sk.txt"

if exist "%AK_FILE%" (
    for /f "usebackq tokens=*" %%a in ("%AK_FILE%") do set "TENCENT_SECRET_ID=%%~a"
    echo Loaded TENCENT_SECRET_ID from %AK_FILE%
) else (
    set "TENCENT_SECRET_ID=YOUR_TENCENT_SECRET_ID"
    echo Using default/placeholder TENCENT_SECRET_ID
)

if exist "%SK_FILE%" (
    for /f "usebackq tokens=*" %%a in ("%SK_FILE%") do set "TENCENT_SECRET_KEY=%%~a"
    echo Loaded TENCENT_SECRET_KEY from %SK_FILE%
) else (
    set "TENCENT_SECRET_KEY=YOUR_TENCENT_SECRET_KEY"
    echo Using default/placeholder TENCENT_SECRET_KEY
)

cd ..\..\..

echo Running Tencent TI chat example...
cargo run --example tencent_ti_chat

pause
