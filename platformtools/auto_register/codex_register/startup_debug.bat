@echo off
setlocal
cd /d "C:\Users\Public\nas_home\AI\GameEditor\NeuroLoom\platformtools\auto_register\codex_register\browser_version"

REM 单实例保护（原子）：避免重复运行 startup_debug.bat 导致多个浏览器窗口并发弹出
set "DEBUG_LOCK_DIR=%TEMP%\codex_register_startup_debug.lockdir"

REM 若锁存在但实际上没有 main.py 进程，视为脏锁并自动清理
if exist "%DEBUG_LOCK_DIR%" (
  powershell -NoProfile -Command "$ps = Get-CimInstance Win32_Process -Filter \"Name='python.exe'\" | Where-Object { $_.CommandLine -match '(?i)\bmain\.py\b' }; if(@($ps).Count -gt 0){ exit 9 } else { exit 0 }"
  if not errorlevel 9 (
    rmdir /S /Q "%DEBUG_LOCK_DIR%" >nul 2>nul
  )
)

2>nul mkdir "%DEBUG_LOCK_DIR%"
if errorlevel 1 (
  echo [debug] 检测到已有本地调试实例在运行，拒绝重复启动，防止重复弹窗。
  echo [debug] 如需重启，请先 Ctrl+C 停止当前调试；若异常退出可手动删除:
  echo         %DEBUG_LOCK_DIR%
  exit /b 1
)

REM ===== 单浏览器单客户端本机调试模式 =====
REM 目标：
REM 1) 不并发（CONCURRENCY=1）
REM 2) 非无头（HEADLESS=0）
REM 3) 失败后保留浏览器现场（DEBUG_KEEP_BROWSER_ON_FAIL=1）
REM 4) 失败后自动等待，不闪退（DEBUG_WAIT_ON_FAIL=1）
REM 5) 关闭探测/修缮后台线程，避免干扰（ENABLE_PROBE=0, ENABLE_REPAIRER=0）

set CONCURRENCY=1
set MAX_ATTEMPTS_PER_WORKER=1
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

REM 调试时启用流量节省：阻止图片/CSS/字体
set BLOCK_IMAGES=2
set BLOCK_CSS=2
set BLOCK_FONTS=2

REM 调试浏览器窗口尺寸（宽,高）
set BROWSER_WINDOW_SIZE=500,600

REM 强制 uc 使用与本机 Chrome 主版本一致，避免自动选到过高版本驱动
set USE_UNDETECTED_CHROMEDRIVER=1
set CHROME_VERSION_MAIN=145

REM 邮箱输入阶段：调试模式下放慢并增加重试，避免“控件慢出现”导致提前失败
set DEBUG_EMAIL_PREWAIT_SECONDS=6
set DEBUG_EMAIL_WAIT_ROUNDS=5
set DEBUG_EMAIL_RETRY_SLEEP_SECONDS=2.5
set SMART_WAIT_CHALLENGE_GRACE_SECONDS=12

echo [debug] using local windows browser mode, single worker, non-headless
echo [debug] on failure browser will be kept and process will wait for manual inspection
echo [debug] press Ctrl+C in this terminal when you finish observing

echo.
python main.py
set "EXIT_CODE=%ERRORLEVEL%"

if exist "%DEBUG_LOCK_DIR%" rmdir "%DEBUG_LOCK_DIR%" >nul 2>nul
exit /b %EXIT_CODE%
