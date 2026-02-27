@echo off
chcp 65001 > nul
set A302_API_KEY=sk-lRQzthyTyLZ5zoREfLdi13xY4sbZWQjlgui7aFzB9D2hv38B
cargo run -p nl_llm_v2 --example a302_stream
pause
