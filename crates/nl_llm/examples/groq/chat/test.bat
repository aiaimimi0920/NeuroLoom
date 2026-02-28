@echo off
setlocal enabledelayedexpansion

:: 注入环境测试密钥
set "GROQ_API_KEY=your_api_key_here"

echo Setting Groq API Key...
echo Running cargo test example...

:: 运行 cargo 命令执行 example
cargo run --example groq_chat
pause
