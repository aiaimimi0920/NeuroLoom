@echo off
chcp 65001 > nul
set RUST_LOG=info
set VERTEX_SA_JSON=examples\vertex\vertex_sa.json

echo ========================================
echo Vertex Auth Test (nl_llm_new)
echo ========================================

cargo run --example vertex_auth

echo.
pause
