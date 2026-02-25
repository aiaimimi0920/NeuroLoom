@echo off
setlocal
cd /d "%~dp0"

echo [INFO] Kimi OAuth 测试无需配置 KIMI_API_KEY。
echo [INFO] 将触发 Device Authorization 授权。请准备好在浏览器中确认。

cargo run -p nl_llm_v2 --example kimi_oauth_chat

endlocal
