@echo off
setlocal enabledelayedexpansion
chcp 65001 >nul

for /f "tokens=1,* delims==" %%a in ('type ..\..\..\..\..\..\.env.local 2^>nul ^| findstr /B "MINIMAX_CN_API_KEY="') do set MINIMAX_CN_API_KEY=%%b

if "%MINIMAX_CN_API_KEY%"=="" (
    echo [INFO] MINIMAX_CN_API_KEY not found in .env.local, using hardcoded key for testing.
    set MINIMAX_CN_API_KEY=sk-api-3FbXKkZ1Slh5uBJgplYWXcE3jMVsDudWYDUm1plLqfHEYSiXf9mRYeP96O4a8zEHVFgWfsyRFjJnVcdqAvdVi91MQijpaEu462JS7nby85ae17p7MQlEwEI
)

if "%~1"=="" (
    cargo run -p nl_llm_v2 --example minimax_cn_auth -- "%MINIMAX_CN_API_KEY%"
) else (
    cargo run -p nl_llm_v2 --example minimax_cn_auth -- "%~1"
)
