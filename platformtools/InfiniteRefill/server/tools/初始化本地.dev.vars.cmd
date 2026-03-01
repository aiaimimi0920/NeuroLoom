@echo off
setlocal
chcp 65001 >nul

REM 说明：
REM - 如果 server\.dev.vars 不存在，则从 server\.dev.vars.example 复制生成。
REM - 该文件被 gitignore 忽略，你可以在其中填写真实 secrets。

set "EXAMPLE=server\.dev.vars.example"
set "TARGET=server\.dev.vars"

if exist "%TARGET%" (
  echo [OK] 已存在：%TARGET%
  echo [INFO] 如需重置，请手动删除 %TARGET% 后重新运行本脚本。
  exit /b 0
)

if not exist "%EXAMPLE%" (
  echo [ERROR] 找不到模板文件：%EXAMPLE%
  exit /b 1
)

copy "%EXAMPLE%" "%TARGET%" >nul
if errorlevel 1 (
  echo [ERROR] 复制失败：%EXAMPLE% -> %TARGET%
  exit /b 1
)

echo [OK] 已生成：%TARGET%
echo [NEXT] 请打开 %TARGET% 填写真实密钥（ADMIN_TOKEN/R2_* 等）。
exit /b 0
