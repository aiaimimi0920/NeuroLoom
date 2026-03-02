@echo off
setlocal
cd /d "C:\Users\Public\nas_home\AI\GameEditor\NeuroLoom\platformtools\auto_register\codex_register"

REM ===== 单浏览器单客户端本机调试模式 =====
REM 目标：
REM 1) 不并发（CONCURRENCY=1）
REM 2) 非无头（HEADLESS=0）
REM 3) 失败后保留浏览器现场（DEBUG_KEEP_BROWSER_ON_FAIL=1）
REM 4) 失败后自动等待，不闪退（DEBUG_WAIT_ON_FAIL=1）
REM 5) 关闭探测/修缮后台线程，避免干扰（ENABLE_PROBE=0, ENABLE_REPAIRER=0）

set CONCURRENCY=1
set HEADLESS=0
set DEBUG_TRACE=1
set DUMP_PAGE_BODY=1
set KEEP_ERROR_ARTIFACTS=1
set ANONYMOUS_MODE=1
set DEBUG_KEEP_BROWSER_ON_FAIL=1
set DEBUG_WAIT_ON_FAIL=1
set ENABLE_PROBE=0
set ENABLE_REPAIRER=0
set SLEEP_MIN=0
set SLEEP_MAX=0

echo [debug] using local windows browser mode, single worker, non-headless
echo [debug] on failure browser will be kept and process will wait for manual inspection
echo [debug] press Ctrl+C in this terminal when you finish observing and want to continue
echo.

python main.py
