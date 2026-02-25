@echo off
cd /d "%~dp0"
echo ========================================
echo   Cloudflare Workers AI Models
echo ========================================
cargo run -p nl_llm_v2 --example cloudflare_models
echo ========================================
echo   Test Complete
echo ========================================
pause
