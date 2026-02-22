@echo off
chcp 65001 >nul
cd /d "%~dp0.."

set CARGO=C:\Users\Administrator\.cargo\bin\cargo.exe

echo.
echo ============================================
echo  nl_llm_new Examples - Build All
echo ============================================
echo.
echo 可用示例 (按 provider 分组):
echo.
echo   iflow_chat        - iFlow 对话 (Cookie Auth)
echo   iflow_auth        - iFlow 认证测试
echo   iflow_models      - iFlow 模型列表
echo   vertex_chat       - Vertex AI (SA JSON)
echo   google_ai_studio_chat - Google AI Studio (API Key)
echo   claude_chat       - Claude (API Key)
echo   openai_chat       - OpenAI (API Key)
echo   antigravity_chat  - Antigravity (TODO: OAuth)
echo   gemini_cli_chat   - Gemini CLI (TODO: OAuth)
echo.
echo ============================================
echo.

echo Building all examples...
%CARGO% build --examples -p nl_llm_new

if %ERRORLEVEL% == 0 (
    echo.
    echo ===== Build Succeeded! =====
    echo.
    echo 运行示例:
    echo   cargo run --example iflow_chat -p nl_llm_new
    echo   cargo run --example vertex_chat -p nl_llm_new
    echo   cargo run --example claude_chat -p nl_llm_new -- --key sk-ant-...
    echo   ...
) else (
    echo.
    echo ===== Build Failed! =====
)

echo.
pause
