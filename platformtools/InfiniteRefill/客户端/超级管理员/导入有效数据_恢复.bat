@echo off
setlocal
chcp 65001 >nul

set "服务器地址=%~1"
set "管理员令牌=%~2"
set "管理员护卫码=%~3"
set "备份文件=%~4"

if "%服务器地址%"=="" goto :用法
if "%管理员令牌%"=="" goto :用法
if "%管理员护卫码%"=="" goto :用法
if "%备份文件%"=="" goto :用法

echo [INFO] 服务器地址=%服务器地址%
echo [INFO] 备份文件=%备份文件%
echo.

curl -sS -X POST "%服务器地址%/admin/backup/import" ^
  -H "Authorization: Bearer %管理员令牌%" ^
  -H "X-Admin-Guard: %管理员护卫码%" ^
  -H "Content-Type: application/json" ^
  --data-binary "@%备份文件%"

echo.
exit /b 0

:用法
echo 用法：%~nx0 服务器地址 管理员令牌 管理员护卫码 备份文件
echo 示例：%~nx0 https://127.0.0.1:8787 dev_admin_token_123 my-strong-guard-string backup_dump.json
echo.
echo 说明：请求体应为：{"dump":{...}}，即导出接口原样返回的 JSON。
echo.
exit /b 1
