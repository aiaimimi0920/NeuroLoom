@echo off
REM antigravity 平台测试 - models
REM 用法: test.bat [token_path]
REM 注意：models 示例需要先运行 auth 或 chat 产生 token 缓存

REM 切换到 crate 根目录（test.bat 在 examples/antigravity/models/ 下，需要向上 4 层）
cd /d "%~dp0\..\..\.."

if "%ANTIGRAVITY_API_KEY%"=="" (
    if "%1"=="" (
        echo Warning: No ANTIGRAVITY_API_KEY provided.
        set TOKEN_PATH=dummy_credential
    ) else (
        set TOKEN_PATH=%1
    )
) else (
    set TOKEN_PATH=%ANTIGRAVITY_API_KEY%
)

echo ========================================
echo   antigravity models Test
echo ========================================
echo.

cargo run --example antigravity_models -- %TOKEN_PATH%

echo.
echo ========================================
echo   Test Complete
echo ========================================
