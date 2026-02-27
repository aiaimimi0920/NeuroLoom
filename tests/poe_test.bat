@echo off
chcp 65001 > nul
echo ==============================================
echo 正在准备测试 Poe Provider
echo ==============================================
set POE_API_KEY=VEgG8gVETAQo_SA-A6jv_eZ7wHPnbRQvyMlVVa_asEo
echo 检测到测试密钥 (已配置): %POE_API_KEY%

cd %~dp0\..\crates\nl_llm_v2
echo.
echo 执行 cargo test --test poe_test -- --nocapture...
echo.
cargo test --test poe_test -- --nocapture
pause
