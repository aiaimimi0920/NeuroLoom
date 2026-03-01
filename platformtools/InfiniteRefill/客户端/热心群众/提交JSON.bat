@echo off
setlocal EnableExtensions EnableDelayedExpansion
chcp 65001 >nul

REM 热心群众：提交 JSON（支持文件/文件夹；带合规检查）
REM
REM 用法：
REM   提交JSON.bat 服务器地址 上传密钥 接口 路径
REM
REM 参数：
REM - 接口：register 或 report
REM - 路径：单个 .json 文件，或包含多个 .json 的文件夹
REM
REM 合规检查：
REM - 必须是合法 JSON
REM - register：必须包含 accounts 数组；每项至少有 email_hash(64hex) 与 seen_at
REM - report：必须包含 reports 数组；每项至少有 email_hash(64hex) 与 status_code 与 probed_at
REM - 已禁用“敏感字段”拦截：允许包含 access_token/refresh_token/id_token

set "服务器地址=%~1"
set "上传密钥=%~2"
set "接口=%~3"
set "路径=%~4"

if "%服务器地址%"=="" goto :用法
if "%上传密钥%"=="" goto :用法
if "%接口%"=="" goto :用法
if "%路径%"=="" goto :用法

set "接口=%接口%"
if /I "%接口%"=="register" (
  set "URL_PATH=/v1/accounts/register"
) else if /I "%接口%"=="report" (
  set "URL_PATH=/v1/probe-report"
) else (
  echo [ERROR] 接口参数只能是 register 或 report
  exit /b 2
)

if not exist "%路径%" (
  echo [ERROR] 路径不存在：%路径%
  exit /b 3
)

set /a TOTAL=0
set /a OK=0
set /a BAD=0

REM 判断是文件还是文件夹
if exist "%路径%\*" (
  echo [INFO] 模式：文件夹
  for %%F in ("%路径%\*.json") do (
    call :处理单个文件 "%%~fF" "%%~nxF"
  )
) else (
  echo [INFO] 模式：单文件
  call :处理单个文件 "%路径%" "%~nx4"
)

echo.
echo [DONE] total=%TOTAL% ok=%OK% bad=%BAD%
exit /b 0

:处理单个文件
set "FILE=%~1"
set "BASE=%~2"
set /a TOTAL+=1

REM 1) （已禁用）敏感字段检测：允许 access_token/refresh_token/id_token

REM 2) JSON + 结构校验
powershell -NoProfile -Command "try{ $raw=Get-Content -Raw -LiteralPath '%FILE%'; $obj=$raw|ConvertFrom-Json; if('%接口%'.ToLower() -eq 'register'){ if(-not $obj.accounts -or -not ($obj.accounts -is [System.Collections.IEnumerable])){ throw 'missing accounts[]' }; foreach($it in @($obj.accounts)){ if(-not $it.email_hash -or ($it.email_hash.ToString() -notmatch '^[a-f0-9]{64}$')){ throw 'bad email_hash' }; if(-not $it.seen_at){ throw 'missing seen_at' } } } else { if(-not $obj.reports -or -not ($obj.reports -is [System.Collections.IEnumerable])){ throw 'missing reports[]' }; foreach($it in @($obj.reports)){ if(-not $it.email_hash -or ($it.email_hash.ToString() -notmatch '^[a-f0-9]{64}$')){ throw 'bad email_hash' }; if($null -eq $it.status_code){ throw 'missing status_code' }; if(-not $it.probed_at){ throw 'missing probed_at' } } } exit 0 }catch{ Write-Output ('ERR:' + $_.Exception.Message); exit 9 }" >"%TEMP%\_json_check.txt" 2>nul
if not "%ERRORLEVEL%"=="0" (
  set /a BAD+=1
  for /f "usebackq delims=" %%E in ("%TEMP%\_json_check.txt") do set "ERRLINE=%%E"
  echo [SKIP] %BASE%：JSON/结构不合规（%ERRLINE%）
  exit /b 0
)

REM 3) 发送
echo [POST] %BASE% -> %URL_PATH%
curl -sS -X POST "%服务器地址%%URL_PATH%" ^
  -H "X-Upload-Key: %上传密钥%" ^
  -H "Content-Type: application/json" ^
  --data-binary "@%FILE%" >nul

if "%ERRORLEVEL%"=="0" (
  set /a OK+=1
) else (
  set /a BAD+=1
  echo [FAIL] %BASE%：curl 失败（exit=%ERRORLEVEL%）
)

exit /b 0

:用法
echo.
echo 用法：%~nx0 服务器地址 上传密钥 接口 路径
echo.
echo 接口：
echo   register  ^(POST /v1/accounts/register^)
echo   report    ^(POST /v1/probe-report^)
echo.
echo 路径：
echo   - 单个 json 文件：C:\path\x.json
echo   - 或文件夹：C:\path\folder
echo.
echo 示例：
echo   %~nx0 https://127.0.0.1:8787 qwersasdf register .\register.json
echo   %~nx0 https://127.0.0.1:8787 qwersasdf report   .\reports\
echo.
exit /b 1
