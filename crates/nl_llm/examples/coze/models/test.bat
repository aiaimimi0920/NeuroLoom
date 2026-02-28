@echo off
set RUST_LOG=info

set COZE_API_KEY=pat_9qJeaFVzdWSocuFJuX4hIph9oLeVbPNKozBs6zHUmZdK3olxA47o0FNRYEy4TPGJ

echo =========================================
echo 运行 Coze Models Extension 测试
echo =========================================

cargo run -p nl_llm --example coze_models

pause
