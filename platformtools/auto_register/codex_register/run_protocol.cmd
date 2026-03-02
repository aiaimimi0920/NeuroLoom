@echo off
setlocal

cd /d c:\Users\Public\nas_home\AI\GameEditor\NeuroLoom

rem ===== High concurrency preset =====
set CONCURRENCY=10
set SLEEP_MIN=0
set SLEEP_MAX=1

rem ===== Proxy strategy =====
set PROXY_ROTATE_SECONDS=120
set PROXY_COOLDOWN_SECONDS=120
set COOLDOWN_PROXY_ERROR_SECONDS=90
set COOLDOWN_BLOCKED_SECONDS=20
set COOLDOWN_INVALID_AUTH_SECONDS=5
set COOLDOWN_OTP_TIMEOUT_SECONDS=0
set COOLDOWN_OTHER_SECONDS=20

rem ===== OTP & network timeout =====
set OTP_TIMEOUT_SECONDS=420
set PROTOCOL_TIMEOUT_SECONDS=20

rem ===== Output strategy =====
set PROTOCOL_SUMMARY_ONLY=1
set PROTOCOL_VERBOSE=0
set MAILBOX_VERBOSE=0
set SUMMARY_PRINT_SECONDS=3
set ROLLING_WINDOW_SECONDS=180

rem Optional: stop when success reaches target (0 = unlimited)
set TARGET_SUCCESS=0

python .\platformtools\auto_register\codex_register\protocol_main.py

endlocal
