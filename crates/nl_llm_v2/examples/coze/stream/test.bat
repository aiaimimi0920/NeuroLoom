@echo off
set RUST_LOG=info

set COZE_API_KEY=pat_9qJeaFVzdWSocuFJuX4hIph9oLeVbPNKozBs6zHUmZdK3olxA47o0FNRYEy4TPGJ

echo =========================================
echo 运行 Coze Streaming 示例
echo =========================================

cargo run -p nl_llm_v2 --example coze_stream

pause
