@echo off
setlocal
chcp 65001 >nul

set "服务器地址=%~1"
set "管理员令牌=%~2"
set "管理员护卫码=%~3"
set "数量=%~4"
set "标签=%~5"

if "%服务器地址%"=="" goto :用法
if "%管理员令牌%"=="" goto :用法
if "%管理员护卫码%"=="" goto :用法
if "%数量%"=="" set "数量=1"

echo [INFO] 服务器地址=%服务器地址%
echo [INFO] 数量=%数量%
if not "%标签%"=="" echo [INFO] 标签=%标签%
echo.

set "TMP=%TEMP%\issue_user_keys_%RANDOM%.json"
(
  echo {"type":"user","count":%数量%,"label":"%标签%","bind_pool_size":10}
) > "%TMP%"

curl -sS -X POST "%服务器地址%/admin/keys/issue" ^
  -H "Authorization: Bearer %管理员令牌%" ^
  -H "X-Admin-Guard: %管理员护卫码%" ^
  -H "Content-Type: application/json" ^
  --data-binary "@%TMP%"

del "%TMP%" >nul 2>nul

echo.
exit /b 0

:用法
echo 用法：%~nx0 服务器地址 管理员令牌 管理员护卫码 数量 [标签]
echo 示例：%~nx0 https://127.0.0.1:8787 dev_admin_token_123 my-strong-guard-string 30 2026-02-special
echo.
echo 说明：
echo - 服务端会自动生成新的 USER_KEY，并只返回一次明文。
echo - bind_pool_size 仅表示默认绑定/预留 10 槽位（不涉及任何第三方 token）。
echo.
exit /b 1
