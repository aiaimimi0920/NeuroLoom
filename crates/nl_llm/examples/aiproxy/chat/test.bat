@echo off
chcp 65001 >nul

:: 这是测试用的占位符或者测试 Key。
:: 这里由于 AI Proxy 需要真实的积分，默认写入 xxx，方便您后续直接填写真实 key 并执行。
set AIPROXY_API_KEY=xxx

echo 正在编译 aiproxy 聊天示例...
cargo run --example aiproxy_chat
pause
