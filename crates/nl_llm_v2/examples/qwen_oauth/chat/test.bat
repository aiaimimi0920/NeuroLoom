@echo off
setlocal
cd /d "%~dp0"

echo [INFO] Qwen OAuth 测试无需配置 QWEN_API_KEY。
echo [INFO] 将触发 portal.qwen.ai 交互式授权流程，如果这是你第一次运行，请留意弹出的网页或控制台中显示的 User Code。

cargo run -p nl_llm_v2 --example qwen_oauth_chat

endlocal
