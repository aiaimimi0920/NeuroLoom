@echo off
REM TokenFlux 平台基础对话测试
REM 用法: test.bat [api_key] [prompt]

pushd "%~dp0"

REM 检查环境变量
if "%TOKENFLUX_API_KEY%"=="" (
    if "%~1"=="" (
        echo 错误: 请设置 TOKENFLUX_API_KEY 环境变量或作为第一个参数传入
        exit /b 1
    )
    set API_KEY=%~1
    shift
) else (
    set API_KEY=%TOKENFLUX_API_KEY%
)

REM 默认 prompt
if "%~1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%~1
)

echo ========================================
echo   TokenFlux Chat Test (crates/nl_llm_v2)
echo ========================================
echo.

set RUST_LOG=info
cargo run --example tokenflux_chat -- "%API_KEY%" "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
popd
exit /b 0
