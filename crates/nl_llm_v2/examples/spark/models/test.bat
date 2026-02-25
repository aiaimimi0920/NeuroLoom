@echo off
setlocal
cd /d "%~dp0"

cargo run -p nl_llm_v2 --example spark_models

endlocal
