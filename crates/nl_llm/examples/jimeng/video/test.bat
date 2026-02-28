@echo off
setlocal enabledelayedexpansion

:: 注入环境测试密钥（请修改为您的实际密钥，请勿提交包含真实密钥的代码）
set "JIMENG_ACCESS_KEY=your_access_key_here"
set "JIMENG_SECRET_KEY=your_secret_key_here"

echo Setting Jimeng API Keys...
echo Running cargo test example...

:: 运行 cargo 命令执行 example
cargo run --example jimeng_video
