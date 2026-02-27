@echo off
REM iflow 平台测试 - auth
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%IFLOW_COOKIE%"=="" (
    if "%1"=="" (
        echo Warning: No IFLOW_COOKIE provided.
        set API_KEY=BXAuth=eyJraWQiOiIxMDAxMjY5NjMyOCIsImFsZyI6IlJTMjU2In0..kH0_SL-oxnTN8Sa2IyGzdzJN44SM7pi_pLF0h-W4pRISSv431KgD_6btfzSD3Dt7Monat4LppDy1JLFGx48-P_7eAUCJQIcPzft-3hONBbjDDTtFOhob1p1cMpdP_26CC-ubtwhjeZcnC8wi0UxsEyVyFuRVZbT2xqICYck6jo_OTcIuOgUvGmq9mi7U2OASaHbfkNoJavYiIEQKCLkU6RM4d1AD0Rp2Fkxp4pYO0jAZc2Bm-bd-jNyTrjhs4JAxykb5VGrWWyefP9uZd7rp57P40qkoMWsQeFD7DqXbssuruqUmQLdA21LqBjgcshch7cPNHwjGlL5T7szD4oCFAg;
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%IFLOW_COOKIE%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   iflow auth Test
echo ========================================
echo.

cargo run --example iflow_auth -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
