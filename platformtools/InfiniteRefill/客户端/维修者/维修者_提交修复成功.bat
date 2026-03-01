@echo off
setlocal EnableExtensions
chcp 65001 >nul

REM 维修者：提交修复成功（POST /v1/repairs/submit-fixed）
REM
REM 用法：
REM   维修者_提交修复成功.bat 服务器地址 维修者密钥 artwork_id 修复后json路径
REM
set "服务器地址=%~1"
set "维修者密钥=%~2"
set "ARTWORK_ID=%~3"
set "JSON_FILE=%~4"

if "%服务器地址%"=="" goto :用法
if "%维修者密钥%"=="" goto :用法
if "%ARTWORK_ID%"=="" goto :用法
if "%JSON_FILE%"=="" goto :用法

if not exist "%JSON_FILE%" (
  echo [ERROR] 文件不存在：%JSON_FILE%
  exit /b 2
)

echo [INFO] 服务器地址=%服务器地址%
echo [INFO] artwork_id=%ARTWORK_ID%
echo [INFO] file=%JSON_FILE%
echo.

REM 用 PowerShell 读取“修复后作品 JSON”，封装为请求体：{artwork_id,fixed_artwork}
powershell -NoProfile -Command "$art=Get-Content -Raw -LiteralPath '%JSON_FILE%' | ConvertFrom-Json; $body=@{ artwork_id='%ARTWORK_ID%'; fixed_artwork=$art } | ConvertTo-Json -Depth 100; Write-Output $body" ^
  | curl -sS -X POST "%服务器地址%/v1/repairs/submit-fixed" ^
      -H "X-Upload-Key: %维修者密钥%" ^
      -H "Content-Type: application/json" ^
      --data-binary @-

echo.
exit /b 0

:用法
echo.
echo 用法：%~nx0 服务器地址 维修者密钥 artwork_id 修复后json路径
echo 示例：%~nx0 http://127.0.0.1:8788 k_xxx acc_demo_1 .\fixed.json
echo.
echo 注意：fixed_artwork 内必须包含 account_id，且必须等于 artwork_id。
echo.
exit /b 1
