@echo off
setlocal EnableExtensions
chcp 65001 >nul

REM 维修者：误报恢复（POST /v1/repairs/submit-misreport）
REM
REM 用法：
REM   维修者_误报恢复.bat 服务器地址 维修者密钥 artwork_id [note]
REM
set "服务器地址=%~1"
set "维修者密钥=%~2"
set "ARTWORK_ID=%~3"
set "NOTE=%~4"

if "%服务器地址%"=="" goto :用法
if "%维修者密钥%"=="" goto :用法
if "%ARTWORK_ID%"=="" goto :用法

if "%NOTE%"=="" set "NOTE="

echo [INFO] 服务器地址=%服务器地址%
echo [INFO] artwork_id=%ARTWORK_ID%
echo.

powershell -NoProfile -Command "$body=@{ artwork_id='%ARTWORK_ID%'; note='%NOTE%' } | ConvertTo-Json -Depth 10; Write-Output $body" ^
  | curl -sS -X POST "%服务器地址%/v1/repairs/submit-misreport" ^
      -H "X-Upload-Key: %维修者密钥%" ^
      -H "Content-Type: application/json" ^
      --data-binary @-

echo.
exit /b 0

:用法
echo.
echo 用法：%~nx0 服务器地址 维修者密钥 artwork_id [note]
echo 示例：%~nx0 http://127.0.0.1:8788 k_xxx acc_demo_1 "客户端误报full"
echo.
exit /b 1
