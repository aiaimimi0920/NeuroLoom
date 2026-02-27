@echo off
cd /d "%~dp0"
if "%1"=="" ( set API_KEY=xxx ) else ( set API_KEY=%1 & shift )
if "%1"=="" ( set MODEL=llama3 ) else ( set MODEL=%1 & shift )
if "%1"=="" ( set PROMPT=???????Ollama ) else ( set PROMPT=%1 )
echo ========================================
echo   Ollama stream Test
echo ========================================
cargo run --example ollama_stream -- %API_KEY% %MODEL% "%PROMPT%"
echo ========================================

