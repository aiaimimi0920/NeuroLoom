@echo off
cd /d "%~dp0"
set API_KEY=edc616b0056d1425f7a3ce62c659bf6c:MDU0M2RmZTFiZWVkNTQ5MWYyOGFmODhl
set MODEL=xopglm5
cargo run --example xfyun_maas_models -- %API_KEY% %MODEL%
