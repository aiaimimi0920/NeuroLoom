@echo off
cd /d "%~dp0"
echo ========================================
echo   AWS Claude (Bedrock) Models
echo ========================================
cargo run -p nl_llm_v2 --example aws_claude_models
echo ========================================
echo   Test Complete
echo ========================================
pause
