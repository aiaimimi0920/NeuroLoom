@echo off
chcp 65001 >nul

set AIPROXY_API_KEY=xxx

echo 正在编译 aiproxy 检查模型示例...
cargo run --example aiproxy_models
pause
