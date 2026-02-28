@echo off
chcp 65001 >nul

set AIPROXY_API_KEY=xxx

echo 正在编译 aiproxy 流式聊天示例...
cargo run --example aiproxy_stream
pause
