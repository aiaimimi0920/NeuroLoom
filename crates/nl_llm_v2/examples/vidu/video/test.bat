@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM Vidu 官方平台测试 - video
REM
REM 约定：
REM - 优先从环境变量 VIDU_API_KEY 读取
REM - 或者在 examples\vidu\.env 中写入 key（该文件会被 gitignore 忽略）
REM   - 支持两种写法：
REM     1) set VIDU_API_KEY=xxxx
REM     2) VIDU_API_KEY=xxxx
REM - 如需切换域名（api.vidu.cn / api.vidu.com），可在 examples\vidu\.env 里额外写：
REM     set VIDU_API_BASE_URL=https://api.vidu.com
REM
REM 用法：
REM   test.bat [api_key] [prompt] [img1] [img2] [img3] [img4] [img5] [img6]
REM
REM 说明：
REM - 1 张图片 => img2video
REM - 2 张图片 => start-end2video
REM - 3+ 张图片 => reference2video

cd /d "%~dp0"

REM 尝试从本地 .env 文件加载密钥（不提交）
REM 注意：.env 扩展名不一定能被 cmd 直接 call 执行，因此这里改为“解析文件内容并 set 变量”。
if exist "..\.env" call :load_env_file "..\.env"

REM 1) API KEY
set "API_KEY=%VIDU_API_KEY%"
if not "%API_KEY%"=="" goto _have_key

if "%~1"=="" (
    echo Error: No VIDU_API_KEY provided.
    echo Please set VIDU_API_KEY env var, pass as arg, or create examples\vidu\.env
    exit /b 1
)

set "API_KEY=%~1"
shift

:_have_key

REM 2) Prompt（不要用变量名 PROMPT，避免与 cmd 内置提示符变量冲突）
set "VIDU_PROMPT=A cinematic scene."
if not "%~1"=="" (
    set "VIDU_PROMPT=%~1"
    shift
)

REM 3) Images
if "%~1"=="" (
    echo Error: img1 is required.
    echo Example:
    echo   test.bat ^<api_key^> "prompt" "https://.../a.png" "https://.../b.png"
    exit /b 1
)

set "IMG1=%~1"
shift
set "IMG2=%~1"
shift
set "IMG3=%~1"
shift
set "IMG4=%~1"
shift
set "IMG5=%~1"
shift
set "IMG6=%~1"

echo ========================================
echo   Vidu video Test
echo ========================================
echo.
echo   prompt: %VIDU_PROMPT%
echo   img1  : %IMG1%
echo   img2  : %IMG2%
echo   img3  : %IMG3%
echo ========================================
echo.

REM 切回仓库根目录再运行 cargo（从 crates/nl_llm_v2/examples/vidu/video 回到 workspace root 需要上跳 5 层）
REM 这里使用 %CD%（因为前面已 cd /d "%~dp0"），避免某些环境下 %~dp0 解析异常。
set "REPO_ROOT=%CD%\..\..\..\..\.."
for %%I in ("%REPO_ROOT%") do set "REPO_ROOT=%%~fI"

REM 诊断：显示工作目录与关键变量（不打印 key）
echo repo_root: %REPO_ROOT%
if defined VIDU_API_BASE_URL echo VIDU_API_BASE_URL: %VIDU_API_BASE_URL%
if defined VIDU_API_KEY (
    echo VIDU_API_KEY: set
) else (
    echo VIDU_API_KEY: not set
)

REM 为避免 cargo 输出的 Running 行明文包含 key，这里不把 key 作为 CLI 参数传入。
REM Rust 示例会优先从环境变量 VIDU_API_KEY 读取。
cargo run --manifest-path "%REPO_ROOT%\Cargo.toml" -p nl_llm_v2 --example vidu_video -- "%VIDU_PROMPT%" "%IMG1%" "%IMG2%" "%IMG3%" "%IMG4%" "%IMG5%" "%IMG6%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
pause
exit /b 0

:load_env_file
REM 解析简单的 .env 文件：支持 `set KEY=VALUE` 或 `KEY=VALUE`
REM - 会忽略空行、以 REM/# 开头的注释行、以及以 @ 开头的 batch 指令行
set "ENV_FILE=%~1"
if not exist "%ENV_FILE%" exit /b 0

for /f "usebackq delims=" %%L in ("%ENV_FILE%") do (
    set "LINE=%%L"

    REM 跳过空行
    if "!LINE!"=="" (
        REM noop
    ) else (
        REM 跳过注释 / batch 指令行
        if /i "!LINE:~0,3!"=="REM" (
            REM noop
        ) else if "!LINE:~0,1!"=="#" (
            REM noop
        ) else if "!LINE:~0,1!"=="@" (
            REM noop
        ) else (
            set "KV=!LINE!"
            if /i "!KV:~0,4!"=="set " set "KV=!KV:~4!"

            for /f "tokens=1* delims==" %%A in ("!KV!") do (
                set "K=%%A"
                set "V=%%B"
            )

            REM 去掉值两侧引号（如果有）
            if "!V:~0,1!"=="\"" if "!V:~-1!"=="\"" set "V=!V:~1,-1!"

            if not "!K!"=="" (
                set "!K!=!V!"
            )
        )
    )
)

exit /b 0
