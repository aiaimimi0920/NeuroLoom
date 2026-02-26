@echo off
setlocal enabledelayedexpansion

:: 注入环境测试密钥
set "REPLICATE_API_TOKEN=your_api_key_here"

echo Setting Replicate Video API Key...
echo Running cargo test example...

:: 运行 cargo 命令执行 example
cargo run --example replicate_video
