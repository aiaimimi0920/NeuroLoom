@echo off
setlocal
chcp 65001 >nul

set "服务器地址=%~1"
set "管理员令牌=%~2"
set "管理员护卫码=%~3"
set "输出文件=%~4"

if "%服务器地址%"=="" goto :用法
if "%管理员令牌%"=="" goto :用法
if "%管理员护卫码%"=="" goto :用法
if "%输出文件%"=="" set "输出文件=backup_dump.json"

echo [INFO] 服务器地址=%服务器地址%
echo [INFO] 输出文件=%输出文件%
echo.

curl -sS "%服务器地址%/admin/backup/export" ^
  -H "Authorization: Bearer %管理员令牌%" ^
  -H "X-Admin-Guard: %管理员护卫码%" ^
  -H "Content-Type: application/json" > "%输出文件%"

echo [OK] 已写入：%输出文件%
echo.
exit /b 0

:用法
echo 用法：%~nx0 服务器地址 管理员令牌 管理员护卫码 [输出文件]
echo 示例：%~nx0 https://127.0.0.1:8787 dev_admin_token_123 my-strong-guard-string backup_dump.json
echo.
echo 说明：仅导出“有效数据”（不含日志 probes，不含 invalid/exhausted 库，不含任何 token 字段）。
echo.
exit /b 1
