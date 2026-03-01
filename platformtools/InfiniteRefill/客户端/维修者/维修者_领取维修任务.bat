@echo off
setlocal
chcp 65001 >nul

REM 维修者：领取维修区任务（POST /v1/repairs/claim）
REM
REM 用法：
REM   维修者_领取维修任务.bat 服务器地址 维修者密钥 [count]
REM
set "服务器地址=%~1"
set "维修者密钥=%~2"
set "COUNT=%~3"

if "%服务器地址%"=="" goto :用法
if "%维修者密钥%"=="" goto :用法
if "%COUNT%"=="" set "COUNT=1"

echo [INFO] 服务器地址=%服务器地址%
echo [INFO] count=%COUNT%
echo.

curl -sS -X POST "%服务器地址%/v1/repairs/claim" ^
  -H "X-Upload-Key: %维修者密钥%" ^
  -H "Content-Type: application/json" ^
  -d "{\"count\":%COUNT%}"

echo.
exit /b 0

:用法
echo.
echo 用法：%~nx0 服务器地址 维修者密钥 [count]
echo 示例：%~nx0 http://127.0.0.1:8788 k_xxx 1
echo.
echo 说明：维修者密钥使用 X-Upload-Key（与热心群众同凭据体系）。
echo.
exit /b 1
