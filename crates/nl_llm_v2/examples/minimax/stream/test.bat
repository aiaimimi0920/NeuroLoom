@echo off
setlocal enabledelayedexpansion
chcp 65001 >nul

for /f "tokens=1,* delims==" %%a in ('type ..\..\..\..\..\..\.env.local 2^>nul ^| findstr /B "MINIMAX_API_KEY="') do set MINIMAX_API_KEY=%%b

if "%MINIMAX_API_KEY%"=="" (
    echo [INFO] MINIMAX_API_KEY not found in .env.local, using hardcoded key for testing.
    set MINIMAX_API_KEY=sk-api-Gdkn7tVftqD54dp7Ydw4E6SaqCQOPmK4Fsk40bArWXEVwbFYguEptB-HGPBrSS_3PEUGuF_kLoyiIM8S1HwsqfZuo3sx7vrDg07BXKg-4vzJ9Gjx5P4abOA
)

if "%~1"=="" (
    cargo run -p nl_llm_v2 --example minimax_stream -- "%MINIMAX_API_KEY%"
) else (
    cargo run -p nl_llm_v2 --example minimax_stream -- "%MINIMAX_API_KEY%" %*
)
