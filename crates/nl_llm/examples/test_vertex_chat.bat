@echo off
chcp 65001 > nul
setlocal EnableDelayedExpansion

echo ========================================
echo   Vertex AI (Google Cloud Gemini) Chat Test
echo ========================================
echo.

REM --- Configuration (Modify here) ---
REM Method 1: Service Account JSON file path (recommended)
set "SA_JSON_PATH=%~dp0vertex_sa.json"

REM Method 2: API Key (alternative to SA_JSON_PATH)
REM Note: Use AI Studio Key from https://aistudio.google.com/app/apikey
set "API_KEY=AIzaSyBtCqclXkiT7l9spBtTDGFLr_ssZikiY54"

REM Method 3: Custom Base URL (ONLY for third-party Vertex-compatible services like zenmux.ai)
REM Leave EMPTY for standard Google AI Studio API Key
REM When set, uses Vertex-style path: /v1/publishers/google/models/
set "BASE_URL="

REM Model name (available: gemini-2.5-flash, gemini-2.5-pro, gemini-1.5-flash, gemini-1.5-pro)
set "MODEL=gemini-2.5-flash"

REM Test prompt (can be passed as first argument)
set "PROMPT=%~1"
if "%PROMPT%"=="" (
    set "PROMPT=Hello! Please introduce yourself briefly."
)
REM ----------------------------

REM Build BASE_URL argument
set "BASE_URL_ARG="
if "%BASE_URL%" neq "" (
    set "BASE_URL_ARG=--base-url %BASE_URL%"
)

REM Determine authentication method
set "AUTH_ARG="
set "AUTH_MODE=0"

if "%API_KEY%" neq "" (
    set "AUTH_ARG=--key %API_KEY%"
    set "AUTH_MODE=1"
    echo [Auth] API Key mode
    if "%BASE_URL%" neq "" (
        echo [Base URL] %BASE_URL%
    ) else (
        echo [Base URL] https://generativelanguage.googleapis.com ^(default^)
    )
)

if "%AUTH_MODE%"=="0" (
    if exist "%SA_JSON_PATH%" (
        set "AUTH_ARG=--sa %SA_JSON_PATH%"
        set "AUTH_MODE=1"
        echo [Auth] Service Account JSON: %SA_JSON_PATH%
        if "%BASE_URL%" neq "" (
            echo [Base URL] %BASE_URL%
        )
    )
)

if "%AUTH_MODE%"=="0" (
    echo [Error] No authentication configured!
    echo.
    echo Please configure authentication by one of these methods:
    echo.
    echo   Method 1 - Service Account JSON ^(recommended^):
    echo     Save SA JSON file as: %SA_JSON_PATH%
    echo.
    echo   Method 2 - API Key:
    echo     Edit this bat file and set API_KEY variable
    echo.
    pause
    exit /b 1
)

set "EXE=%~dp0..\..\..\target\debug\examples\test_vertex_chat.exe"

echo [1/3] Building...
cargo build --example test_vertex_chat -p nl_llm
if !errorlevel! neq 0 (
    echo.
    echo [Error] Build failed!
    pause
    exit /b 1
)
echo.

echo [2/3] Non-streaming request (generateContent)...
echo.
"%EXE%" "%PROMPT%" %AUTH_ARG% --model %MODEL% %BASE_URL_ARG%
echo.

echo [3/3] Streaming request (streamGenerateContent)...
echo.
"%EXE%" "%PROMPT%" %AUTH_ARG% --model %MODEL% --stream %BASE_URL_ARG%
echo.

echo ========================================
echo   Test completed
echo ========================================
echo.
pause
