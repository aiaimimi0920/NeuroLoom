@echo off
setlocal enabledelayedexpansion

:: 为防止包含真实秘钥，推送到线上的将使用占位符
set "TOGETHER_API_KEY=your_api_key_here"

echo Setting Together API Key...
echo Running cargo test example...

:: 运行 cargo 命令执行 example
cargo run --example together_chat
pause
