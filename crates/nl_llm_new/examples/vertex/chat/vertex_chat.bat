@echo off
chcp 65001 > nul
set RUST_LOG=info
set VERTEX_SA_JSON=examples\vertex\vertex_sa.json

echo ========================================
echo Vertex Chat Test (nl_llm_new) - Non-Streaming
echo ========================================

cargo run --example vertex_chat -- "Hello! Please introduce yourself in Chinese."

echo.
pause
