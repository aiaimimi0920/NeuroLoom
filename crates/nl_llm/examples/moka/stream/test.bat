@echo off
chcp 65001 >nul

:: 测试用的占位符 Key
set MOKA_API_KEY=xxx

echo 正在编译 MokaAI 流式聊天示例...
cargo run --example moka_stream
pause
