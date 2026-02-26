@echo off
setlocal

:: 传入测试密钥
set CUSTOM_API_KEY=xxx
:: 可以在这里指定自定义 URL
set CUSTOM_BASE_URL=https://api.openai.com/v1

echo 正在编译 custom 流式聊天示例...
cargo run -p nl_llm_v2 --example custom_stream -- "%CUSTOM_API_KEY%" "请用 Rust 写一个并发下载器示例代码" "gpt-4o-mini"
if %ERRORLEVEL% neq 0 (
    echo 编译或运行失败！
    pause
    exit /b %ERRORLEVEL%
)

pause
