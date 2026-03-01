@echo off
setlocal
chcp 65001 >nul

set "服务器地址=%~1"
set "管理员令牌=%~2"
set "管理员护卫码=%~3"

if "%服务器地址%"=="" goto :用法
if "%管理员令牌%"=="" goto :用法
if "%管理员护卫码%"=="" goto :用法

echo [INFO] 服务器地址=%服务器地址%
echo.

curl -sS "%服务器地址%/admin/stats" ^
  -H "Authorization: Bearer %管理员令牌%" ^
  -H "X-Admin-Guard: %管理员护卫码%"

echo.
exit /b 0

:用法
echo 用法：%~nx0 服务器地址 管理员令牌 管理员护卫码
echo 示例：%~nx0 https://127.0.0.1:8787 dev_admin_token_123 my-strong-guard-string
echo.
exit /b 1
