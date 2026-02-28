@echo off
set RUST_LOG=debug

:: User's provided Coze API Key
set COZE_API_KEY=pat_9qJeaFVzdWSocuFJuX4hIph9oLeVbPNKozBs6zHUmZdK3olxA47o0FNRYEy4TPGJ

echo =========================================
echo 运行 Coze Chat (Non-Stream) 示例
echo 此测试将展示 LlmClient 内部进行流式重组并返回合并的字符串
echo 如果尚未发布正确的 Bot ID，它将正确抛出 Bot नॉट_found (4200) 错误
echo =========================================

cargo run -p nl_llm --example coze_chat

pause
