@echo off
chcp 65001 > nul
setlocal

echo ========================================
echo   Vertex AI (Google Cloud Gemini) Chat Test
echo ========================================
echo.

:: ── 配置区（修改这里）────────────────────────────────────────────────
:: 方式一：Service Account JSON 文件路径（推荐）
set "SA_JSON_PATH=%~dp0vertex_sa.json"

:: 方式二：API Key（与 SA_JSON_PATH 二选一）
:: ! 注意：请使用 AI Studio Key（https://aistudio.google.com/app/apikey）
:: !        GCP Console 创建的 API Key 受"新用户"限制，无法访问 Gemini 模型
set "API_KEY="

:: 模型名称（可选）
:: SA JSON 模式可用：gemini-2.0-flash, gemini-2.5-flash
:: API Key 模式可用：gemini-2.0-flash（需要 AI Studio Key）
set "MODEL=gemini-2.0-flash"

:: 测试提示词（支持作为第一个参数传入）
set "PROMPT=%~1"
if "%PROMPT%"=="" (
    set "PROMPT=你好！请用中文简单介绍一下你自己，以及你能做什么？"
)
:: ──────────────────────────────────────────────────────────────────────

:: 判断认证方式
set "AUTH_ARG="
if "%API_KEY%" neq "" (
    set "AUTH_ARG=--key %API_KEY%"
    echo [认证] API Key 模式
) else if exist "%SA_JSON_PATH%" (
    set "AUTH_ARG=--sa %SA_JSON_PATH%"
    echo [认证] Service Account JSON: %SA_JSON_PATH%
) else (
    echo [错误] 未找到认证配置！
    echo.
    echo 请按以下步骤之一配置认证：
    echo.
    echo   方式 1 - Service Account JSON（推荐）:
    echo     将 SA JSON 文件保存为: %SA_JSON_PATH%
    echo     或编辑此 bat 文件修改 SA_JSON_PATH 变量
    echo.
    echo   方式 2 - API Key:
    echo     编辑此 bat 文件，设置 API_KEY 变量
    echo     或运行: set VERTEX_API_KEY=AIzaSy...
    echo.
    echo 详见: examples\vertex_config.txt
    echo.
    pause
    exit /b 1
)

set "EXE=%~dp0..\..\..\target\debug\examples\test_vertex_chat.exe"

echo [1/3] Building...
cargo build --example test_vertex_chat -p nl_llm
if %errorlevel% neq 0 (
    echo.
    echo [错误] Build 失败！
    pause
    exit /b 1
)
echo.

echo [2/3] 非流式请求 (generateContent)...
echo.
"%EXE%" "%PROMPT%" %AUTH_ARG% --model %MODEL%
echo.

echo [3/3] 流式请求 (streamGenerateContent)...
echo.
"%EXE%" "%PROMPT%" %AUTH_ARG% --model %MODEL% --stream
echo.

echo ========================================
echo   测试完成
echo ========================================
echo.
pause
