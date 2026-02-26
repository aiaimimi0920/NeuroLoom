@echo off
setlocal enabledelayedexpansion

:: 注入环境测试密钥（请修改为您的实际密钥，请勿提交包含真实密钥的代码）
set "SORA_API_KEY=your_api_key_here"

echo Setting Sora Video API Key...
echo Running cargo test example...

:: 运行 cargo 命令执行 example
cargo run --example sora_video
