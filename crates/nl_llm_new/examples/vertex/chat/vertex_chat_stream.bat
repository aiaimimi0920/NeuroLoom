@echo off
chcp 65001 > nul
set RUST_LOG=info
set VERTEX_SA_JSON=examples\vertex\vertex_sa.json

echo ========================================
echo Vertex CLI Chat Test (nl_llm_new) - Streaming
echo ========================================

cargo run --example vertex_chat -- "Hello! Please introduce yourself in Chinese and explain what you can do." --stream

echo.
pause
