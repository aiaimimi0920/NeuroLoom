from __future__ import annotations

import base64
import concurrent.futures
import glob
import hashlib
import json
import os
import random
import re
import secrets
import socket
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
from collections import deque
from dataclasses import dataclass
from typing import Any, Deque, Dict

try:
    from curl_cffi import requests as curl_requests
except Exception:
    curl_requests = None  # type: ignore

import sys

# Ensure repo root is importable so we can `import platformtools...` when running as script.
_REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if _REPO_ROOT not in sys.path:
    sys.path.insert(0, _REPO_ROOT)

# Shared platformtools dev vars (user-managed, gitignored)
try:
    from platformtools._shared.dev_vars import load_platformtools_dev_vars
except Exception:
    load_platformtools_dev_vars = None  # type: ignore

_PLATFORMTOOLS_DEV_VARS = (
    load_platformtools_dev_vars(start_dir=os.path.dirname(__file__)) if load_platformtools_dev_vars else {}
)

# Mail provider client import
_PLAT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
_MAILCREATE_CLIENT_DIR = os.path.join(_PLAT_DIR, "mailcreate", "client")
if _MAILCREATE_CLIENT_DIR not in sys.path:
    sys.path.insert(0, _MAILCREATE_CLIENT_DIR)

from mailbox_provider import Mailbox, create_mailbox, wait_openai_code as wait_openai_code_by_provider  # type: ignore
from platformtools.auto_register.codex_register.mailbox_shared import (
    create_temp_mailbox_shared,
    wait_openai_code_shared,
)


write_lock = threading.Lock()
stats_lock = threading.Lock()
proxy_state_lock = threading.Lock()
flow_totals_lock = threading.Lock()

RUN_STARTED_AT = time.time()

_STATS: dict[str, Any] = {
    "attempt": 0,
    "success": 0,
    "fail": 0,
    "blocked": 0,
    "otp_timeout": 0,
    "proxy_error": 0,
    "invalid_auth_step": 0,
    "other": 0,
    "cooldown_wait": 0,
    "stage_auth_continue": 0,
    "stage_send_otp": 0,
    "stage_otp_validate": 0,
    "stage_create_account": 0,
    "stage_workspace": 0,
    "stage_callback": 0,
    "stage_other": 0,
    # 网络根因诊断（与错误类别正交）
    "net_local": 0,
    "net_tunnel": 0,
    "net_residential": 0,
    "net_openai": 0,
    "net_unknown": 0,
    "last_error": "",
}

_PROXY_COOLDOWN_UNTIL: dict[str, float] = {}
_SUCCESS_TS: Deque[float] = deque()
_ATTEMPT_TS: Deque[float] = deque()

# Runtime data directory
DATA_DIR = (os.environ.get("DATA_DIR") or os.path.join(os.path.dirname(__file__), "data")).strip()
if not DATA_DIR:
    DATA_DIR = os.path.join(os.path.dirname(__file__), "data")


CODEX_AUTH_DIRNAME = "codex_auth"
WAIT_UPDATE_DIRNAME = "wait_update"
ERROR_DIRNAME = "error"
RESULTS_DIRNAME = "results"


def _sanitize_instance_id(v: str) -> str:
    s = (v or "").strip()
    if not s:
        return "default"
    s = re.sub(r"[^a-zA-Z0-9_.-]+", "_", s)
    return s[:64] or "default"


INSTANCE_ID = _sanitize_instance_id(
    os.environ.get("INSTANCE_ID")
    or os.environ.get("RESULTS_INSTANCE_ID")
    or os.environ.get("HOSTNAME")
    or socket.gethostname()
)


RESULTS_SHARD_SIZE = int(os.environ.get("RESULTS_SHARD_SIZE", "200"))
if RESULTS_SHARD_SIZE <= 0:
    RESULTS_SHARD_SIZE = 200


def _data_path(*parts: str) -> str:
    return os.path.join(DATA_DIR, *parts)


def _results_dir() -> str:
    return _data_path(RESULTS_DIRNAME, INSTANCE_ID)


def _results_state_path() -> str:
    return os.path.join(_results_dir(), "results_state.json")


def _read_json(path: str) -> dict:
    try:
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f) or {}
    except Exception:
        return {}


def _write_json(path: str, obj: dict) -> None:
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(obj, f, ensure_ascii=False, separators=(",", ":"))
    os.replace(tmp, path)


def _infer_results_state() -> dict:
    try:
        os.makedirs(_results_dir(), exist_ok=True)
    except Exception:
        pass

    try:
        files = [
            p
            for p in glob.glob(os.path.join(_results_dir(), "results_*.jsonl"))
            if os.path.isfile(p)
        ]
        if not files:
            return {"shard_id": 0, "line_in_shard": 0}

        def _sid(p: str) -> int:
            m = re.search(r"results_(\d+)\.jsonl$", os.path.basename(p))
            return int(m.group(1)) if m else -1

        files.sort(key=_sid)
        last = files[-1]
        shard_id = _sid(last)
        if shard_id < 0:
            return {"shard_id": 0, "line_in_shard": 0}

        line_count = 0
        try:
            with open(last, "r", encoding="utf-8") as f:
                for _ in f:
                    line_count += 1
        except Exception:
            line_count = 0

        if line_count >= RESULTS_SHARD_SIZE:
            return {"shard_id": shard_id + 1, "line_in_shard": 0}
        return {"shard_id": shard_id, "line_in_shard": line_count}
    except Exception:
        return {"shard_id": 0, "line_in_shard": 0}


def _load_results_state() -> dict:
    st = _read_json(_results_state_path())
    if "shard_id" in st and "line_in_shard" in st:
        return st
    return _infer_results_state()


def _append_result_line(line: str) -> None:
    try:
        os.makedirs(DATA_DIR, exist_ok=True)
        os.makedirs(_results_dir(), exist_ok=True)
    except Exception:
        pass

    payload = (line or "").rstrip("\r\n") + "\n"

    st = _load_results_state()
    shard_id = int(st.get("shard_id", 0) or 0)
    line_in_shard = int(st.get("line_in_shard", 0) or 0)

    shard_path = os.path.join(_results_dir(), f"results_{shard_id:06d}.jsonl")
    with open(shard_path, "a", encoding="utf-8") as f:
        f.write(payload)

    line_in_shard += 1
    if line_in_shard >= RESULTS_SHARD_SIZE:
        shard_id += 1
        line_in_shard = 0

    _write_json(_results_state_path(), {"shard_id": shard_id, "line_in_shard": line_in_shard})


def _flow_totals_path() -> str:
    return _data_path("flow_totals.json")


def _flow_totals_read() -> dict[str, int]:
    raw = _read_json(_flow_totals_path())
    if not isinstance(raw, dict):
        raw = {}
    return {
        "protocol": int(raw.get("protocol", 0) or 0),
        "browser": int(raw.get("browser", 0) or 0),
    }


def _flow_totals_inc(flow: str, delta: int = 1) -> None:
    key = (flow or "").strip().lower()
    if key not in ("protocol", "browser"):
        return
    with flow_totals_lock:
        st = _flow_totals_read()
        st[key] = int(st.get(key, 0) or 0) + int(delta)
        _write_json(_flow_totals_path(), st)


def _flow_totals_snapshot() -> tuple[int, int]:
    st = _flow_totals_read()
    return int(st.get("protocol", 0) or 0), int(st.get("browser", 0) or 0)


# Mail provider config
MAILBOX_PROVIDER = os.environ.get("MAILBOX_PROVIDER", "auto").strip().lower()

MAILCREATE_CONFIG_FILE = os.environ.get(
    "MAILCREATE_CONFIG_FILE",
    os.path.join(DATA_DIR, "mailcreate_config.json"),
).strip()


def _load_json_config(path: str) -> dict:
    try:
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f)
    except FileNotFoundError:
        return {}


_MAILCREATE_CFG = _load_json_config(MAILCREATE_CONFIG_FILE)

MAILCREATE_BASE_URL = (
    os.environ.get("MAILCREATE_BASE_URL")
    or _PLATFORMTOOLS_DEV_VARS.get("MAILCREATE_BASE_URL")
    or str(_MAILCREATE_CFG.get("MAILCREATE_BASE_URL") or "https://mail.aiaimimi.com")
)
MAILCREATE_CUSTOM_AUTH = (
    os.environ.get("MAILCREATE_CUSTOM_AUTH")
    or _PLATFORMTOOLS_DEV_VARS.get("MAILCREATE_CUSTOM_AUTH")
    or str(_MAILCREATE_CFG.get("MAILCREATE_CUSTOM_AUTH") or "")
).strip()
MAILCREATE_DOMAIN = (
    os.environ.get("MAILCREATE_DOMAIN")
    or _PLATFORMTOOLS_DEV_VARS.get("MAILCREATE_DOMAIN")
    or str(_MAILCREATE_CFG.get("MAILCREATE_DOMAIN") or "")
).strip()

GPTMAIL_BASE_URL = (
    os.environ.get("GPTMAIL_BASE_URL")
    or _PLATFORMTOOLS_DEV_VARS.get("GPTMAIL_BASE_URL")
    or "https://mail.chatgpt.org.uk"
).strip()
GPTMAIL_API_KEY = (
    os.environ.get("GPTMAIL_API_KEY")
    or _PLATFORMTOOLS_DEV_VARS.get("GPTMAIL_API_KEY")
    or ""
).strip()
GPTMAIL_KEYS_FILE = os.environ.get(
    "GPTMAIL_KEYS_FILE",
    os.path.join(DATA_DIR, "gptmail_keys.txt"),
).strip()
GPTMAIL_PREFIX = os.environ.get("GPTMAIL_PREFIX", "").strip() or None
GPTMAIL_DOMAIN = os.environ.get("GPTMAIL_DOMAIN", "").strip() or None


def _env_bool(name: str, default: bool) -> bool:
    raw = os.environ.get(name)
    if raw is None:
        return default
    return (raw or "").strip().lower() not in ("0", "false", "no", "off", "")


# Protocol mode config
PROTOCOL_IMPERSONATE = (os.environ.get("PROTOCOL_IMPERSONATE", "chrome") or "chrome").strip() or "chrome"
PROTOCOL_TIMEOUT_SECONDS = int(os.environ.get("PROTOCOL_TIMEOUT_SECONDS", "30") or "30")
if PROTOCOL_TIMEOUT_SECONDS <= 0:
    PROTOCOL_TIMEOUT_SECONDS = 30

# 采用浏览器版本同级别的激进邮箱验证码等待：默认 120s
OTP_TIMEOUT_SECONDS = int(os.environ.get("OTP_TIMEOUT_SECONDS", "120") or "120")
if OTP_TIMEOUT_SECONDS <= 0:
    OTP_TIMEOUT_SECONDS = 120

PROXY_ROTATE_SECONDS = int(os.environ.get("PROXY_ROTATE_SECONDS", "600") or "600")
if PROXY_ROTATE_SECONDS < 0:
    PROXY_ROTATE_SECONDS = 0

# 兼容旧配置：PROXY_COOLDOWN_SECONDS 作为兜底默认值
PROXY_COOLDOWN_SECONDS = int(os.environ.get("PROXY_COOLDOWN_SECONDS", "300") or "300")
if PROXY_COOLDOWN_SECONDS < 0:
    PROXY_COOLDOWN_SECONDS = 0

COOLDOWN_PROXY_ERROR_SECONDS = int(
    os.environ.get("COOLDOWN_PROXY_ERROR_SECONDS", str(PROXY_COOLDOWN_SECONDS)) or str(PROXY_COOLDOWN_SECONDS)
)
if COOLDOWN_PROXY_ERROR_SECONDS < 0:
    COOLDOWN_PROXY_ERROR_SECONDS = 0

COOLDOWN_BLOCKED_SECONDS = int(os.environ.get("COOLDOWN_BLOCKED_SECONDS", "120") or "120")
if COOLDOWN_BLOCKED_SECONDS < 0:
    COOLDOWN_BLOCKED_SECONDS = 0

COOLDOWN_INVALID_AUTH_SECONDS = int(os.environ.get("COOLDOWN_INVALID_AUTH_SECONDS", "30") or "30")
if COOLDOWN_INVALID_AUTH_SECONDS < 0:
    COOLDOWN_INVALID_AUTH_SECONDS = 0

COOLDOWN_OTP_TIMEOUT_SECONDS = int(os.environ.get("COOLDOWN_OTP_TIMEOUT_SECONDS", "0") or "0")
if COOLDOWN_OTP_TIMEOUT_SECONDS < 0:
    COOLDOWN_OTP_TIMEOUT_SECONDS = 0

COOLDOWN_OTHER_SECONDS = int(os.environ.get("COOLDOWN_OTHER_SECONDS", "60") or "60")
if COOLDOWN_OTHER_SECONDS < 0:
    COOLDOWN_OTHER_SECONDS = 0

SUMMARY_PRINT_SECONDS = int(os.environ.get("SUMMARY_PRINT_SECONDS", "5") or "5")
if SUMMARY_PRINT_SECONDS <= 0:
    SUMMARY_PRINT_SECONDS = 5

PROTOCOL_SUMMARY_ONLY = _env_bool("PROTOCOL_SUMMARY_ONLY", True)
PROTOCOL_VERBOSE = _env_bool("PROTOCOL_VERBOSE", False)

PROTOCOL_CHECK_GEO = (os.environ.get("PROTOCOL_CHECK_GEO", "1") or "1").strip().lower() not in (
    "0",
    "false",
    "no",
)
PROTOCOL_BLOCKED_LOCS = {
    x.strip().upper()
    for x in (os.environ.get("PROTOCOL_BLOCKED_LOCS", "CN,HK") or "CN,HK").split(",")
    if x.strip()
}

TARGET_SUCCESS = int(os.environ.get("TARGET_SUCCESS", "0") or "0")
if TARGET_SUCCESS < 0:
    TARGET_SUCCESS = 0

ROLLING_WINDOW_SECONDS = int(os.environ.get("ROLLING_WINDOW_SECONDS", "300") or "300")
if ROLLING_WINDOW_SECONDS <= 0:
    ROLLING_WINDOW_SECONDS = 300

# 多进程隔离代理池：
# - PROXY_PARTITION_TOTAL=2, PROXY_PARTITION_INDEX=0/1
#   两个进程各用一半代理，避免互相抢同一出口。
PROXY_PARTITION_TOTAL = int(os.environ.get("PROXY_PARTITION_TOTAL", "0") or "0")
PROXY_PARTITION_INDEX = int(os.environ.get("PROXY_PARTITION_INDEX", "0") or "0")
if PROXY_PARTITION_TOTAL < 0:
    PROXY_PARTITION_TOTAL = 0
if PROXY_PARTITION_TOTAL > 0:
    PROXY_PARTITION_INDEX = PROXY_PARTITION_INDEX % PROXY_PARTITION_TOTAL

# 当代理池不可用时是否允许直连兜底（容器场景建议开启）
PROTOCOL_ALLOW_DIRECT_FALLBACK = _env_bool("PROTOCOL_ALLOW_DIRECT_FALLBACK", True)
# 强制忽略代理文件，全部走直连（用于容器网络与代理不兼容场景）
PROTOCOL_IGNORE_PROXIES = _env_bool("PROTOCOL_IGNORE_PROXIES", False)


def _log(msg: str, *, force: bool = False) -> None:
    if PROTOCOL_SUMMARY_ONLY and not force and not PROTOCOL_VERBOSE:
        return
    print(msg, flush=True)


def _classify_error(exc: Exception | str) -> str:
    s = str(exc or "").lower()
    if "timeout waiting for 6-digit code" in s:
        return "otp_timeout"
    if "invalid_auth_step" in s:
        return "invalid_auth_step"
    if "just a moment" in s or "authorize/continue failed: http=403" in s:
        return "blocked"
    if "blocked geo" in s:
        return "blocked"
    if "proxy" in s or "ssl" in s or "curl: (28)" in s or "curl: (35)" in s or "curl: (56)" in s:
        return "proxy_error"
    if "timed out" in s or "couldn't connect" in s or "failed to connect" in s:
        return "proxy_error"
    return "other"


def _infer_stage_from_error(err_text: str) -> str:
    s = (err_text or "").lower()
    if "authorize/continue" in s or "invalid_auth_step" in s:
        return "stage_auth_continue"
    if "send-otp" in s or "passwordless/send-otp" in s:
        return "stage_send_otp"
    if "email-otp/validate" in s:
        return "stage_otp_validate"
    if "create_account" in s:
        return "stage_create_account"
    if "workspace/select" in s or "workspace id missing" in s:
        return "stage_workspace"
    if "callback" in s or "continue_url" in s:
        return "stage_callback"
    return "stage_other"


def _infer_net_cause(err_text: str) -> str:
    """将异常文本粗分到链路位置，帮助定位网络波动来源。

    注意：这是启发式分类，不是 100% 精确归因。
    """
    s = (err_text or "").lower()
    if not s:
        return "net_unknown"

    # 1) OpenAI 侧：挑战/封控/5xx/相关端点失败
    if (
        "just a moment" in s
        or "captcha" in s
        or "cf-chl" in s
        or "cf-ray" in s
        or "authorize/continue failed" in s
        or "send-otp failed" in s
        or "email-otp/validate failed" in s
        or "create_account failed" in s
        or "workspace/select failed" in s
        or "sentinel req failed" in s
        or "auth.openai.com" in s
        or "sentinel.openai.com" in s
    ):
        return "net_openai"

    # 2) 本地出口/系统网络栈
    if (
        "getaddrinfo failed" in s
        or "name or service not known" in s
        or "temporary failure in name resolution" in s
        or "no such host" in s
        or "network is unreachable" in s
        or "no route to host" in s
        or "winerror 10051" in s
        or "winerror 10065" in s
        or "dns" in s
    ):
        return "net_local"

    # 3) 本地隧道/翻墙代理链路（与住宅代理前的一跳）
    if (
        "http=407" in s
        or "proxy authentication" in s
        or "proxyconnect" in s
        or "tunnel connection failed" in s
        or "socks" in s
        or "connect to proxy" in s
        or ("proxy" in s and "failed to connect" in s)
    ):
        return "net_tunnel"

    # 4) 远端住宅代理节点波动（超时/TLS/连接重置等）
    if (
        "curl: (28)" in s
        or "curl: (35)" in s
        or "curl: (56)" in s
        or "timed out" in s
        or "timeout was reached" in s
        or "ssl" in s
        or "tls" in s
        or "connection reset" in s
        or "recv failure" in s
        or "empty reply from server" in s
        or "failed to connect" in s
        or "couldn't connect" in s
    ):
        # 若明确提到 proxy，优先归到隧道层；否则归住宅出口层
        if "proxy" in s:
            return "net_tunnel"
        return "net_residential"

    return "net_unknown"



def _stats_inc(
    kind: str,
    err: Exception | str | None = None,
    *,
    stage: str | None = None,
    net_cause: str | None = None,
) -> None:
    with stats_lock:
        now_ts = time.time()
        if kind == "attempt":
            _STATS["attempt"] = int(_STATS.get("attempt", 0)) + 1
            _ATTEMPT_TS.append(now_ts)
            _trim_ts(now_ts)
            return

        if kind == "success":
            _STATS["success"] = int(_STATS.get("success", 0)) + 1
            _SUCCESS_TS.append(now_ts)
            _trim_ts(now_ts)
            _flow_totals_inc("protocol", 1)
            return

        # cooldown_wait 不是失败，只代表“当前无可用代理，等待冷却结束”。
        if kind == "cooldown_wait":
            _STATS["cooldown_wait"] = int(_STATS.get("cooldown_wait", 0)) + 1
            return

        _STATS["fail"] = int(_STATS.get("fail", 0)) + 1
        if kind in _STATS:
            _STATS[kind] = int(_STATS.get(kind, 0)) + 1
        else:
            _STATS["other"] = int(_STATS.get("other", 0)) + 1

        if err is not None:
            _STATS["last_error"] = str(err)

        stg = stage or _infer_stage_from_error(str(err or ""))
        if stg in _STATS:
            _STATS[stg] = int(_STATS.get(stg, 0)) + 1
        else:
            _STATS["stage_other"] = int(_STATS.get("stage_other", 0)) + 1

        nc = (net_cause or _infer_net_cause(str(err or ""))).strip().lower()
        if nc in _STATS:
            _STATS[nc] = int(_STATS.get(nc, 0)) + 1
        else:
            _STATS["net_unknown"] = int(_STATS.get("net_unknown", 0)) + 1


def _trim_ts(now_ts: float) -> None:
    cutoff = now_ts - float(ROLLING_WINDOW_SECONDS)
    while _SUCCESS_TS and _SUCCESS_TS[0] < cutoff:
        _SUCCESS_TS.popleft()
    while _ATTEMPT_TS and _ATTEMPT_TS[0] < cutoff:
        _ATTEMPT_TS.popleft()


def _stats_snapshot() -> dict[str, Any]:
    with stats_lock:
        now_ts = time.time()
        _trim_ts(now_ts)
        st = dict(_STATS)
        st["rolling_success"] = len(_SUCCESS_TS)
        st["rolling_attempt"] = len(_ATTEMPT_TS)
        st["rolling_window_seconds"] = ROLLING_WINDOW_SECONDS
        return st


def _proxy_is_cooled_down(proxy: str, now_ts: float | None = None) -> bool:
    now_v = time.time() if now_ts is None else now_ts
    with proxy_state_lock:
        until = float(_PROXY_COOLDOWN_UNTIL.get(proxy, 0.0) or 0.0)
    return until > now_v


def _proxy_mark_cooldown(proxy: str, seconds: int) -> None:
    if not proxy or seconds <= 0:
        return
    with proxy_state_lock:
        _PROXY_COOLDOWN_UNTIL[proxy] = time.time() + float(seconds)


def _cooldown_seconds_for_error_class(err_cls: str) -> int:
    c = (err_cls or "").strip().lower()
    if c == "proxy_error":
        return COOLDOWN_PROXY_ERROR_SECONDS
    if c == "blocked":
        return COOLDOWN_BLOCKED_SECONDS
    if c == "invalid_auth_step":
        return COOLDOWN_INVALID_AUTH_SECONDS
    if c == "otp_timeout":
        return COOLDOWN_OTP_TIMEOUT_SECONDS
    return COOLDOWN_OTHER_SECONDS


def _pick_proxy(
    *,
    proxies: list[str],
    current_proxy: str | None,
    assigned_at: float,
) -> tuple[str | None, float]:
    now_ts = time.time()

    if not proxies:
        return None, now_ts

    if current_proxy:
        if (
            PROXY_ROTATE_SECONDS > 0
            and (now_ts - assigned_at) < PROXY_ROTATE_SECONDS
            and not _proxy_is_cooled_down(current_proxy, now_ts)
            and current_proxy in proxies
        ):
            return current_proxy, assigned_at

    ready = [p for p in proxies if not _proxy_is_cooled_down(p, now_ts)]
    if not ready:
        return None, assigned_at

    if current_proxy and len(ready) > 1 and current_proxy in ready:
        ready = [p for p in ready if p != current_proxy]

    return random.choice(ready), now_ts


def _summary_loop() -> None:
    while True:
        time.sleep(SUMMARY_PRINT_SECONDS)
        st = _stats_snapshot()
        elapsed = max(1.0, time.time() - RUN_STARTED_AT)
        speed_h = float(st.get("success", 0)) * 3600.0 / elapsed

        proxies = load_proxies()
        total_proxy = len(proxies)
        now_ts = time.time()
        cooling = 0
        if total_proxy > 0:
            with proxy_state_lock:
                cooling = sum(1 for p in proxies if float(_PROXY_COOLDOWN_UNTIL.get(p, 0.0) or 0.0) > now_ts)

        target_txt = f"/{TARGET_SUCCESS}" if TARGET_SUCCESS > 0 else ""
        rw = max(1, int(st.get("rolling_window_seconds", ROLLING_WINDOW_SECONDS)))
        rolling_success = int(st.get("rolling_success", 0) or 0)
        rolling_attempt = int(st.get("rolling_attempt", 0) or 0)
        rolling_h = rolling_success * 3600.0 / float(rw)
        rolling_sr = (rolling_success / float(rolling_attempt)) if rolling_attempt > 0 else 0.0

        protocol_total, browser_total = _flow_totals_snapshot()

        print(
            (
                f"[SUMMARY] 完成 {st.get('success', 0)}{target_txt} | 尝试 {st.get('attempt', 0)} | 失败 {st.get('fail', 0)} "
                f"| 封控 {st.get('blocked', 0)} | OTP超时 {st.get('otp_timeout', 0)} "
                f"| 代理错 {st.get('proxy_error', 0)} | invalid_step {st.get('invalid_auth_step', 0)} "
                f"| 阶段(auth/send/otpv/create/ws/cb/other)="
                f"{st.get('stage_auth_continue', 0)}/{st.get('stage_send_otp', 0)}/{st.get('stage_otp_validate', 0)}/"
                f"{st.get('stage_create_account', 0)}/{st.get('stage_workspace', 0)}/{st.get('stage_callback', 0)}/{st.get('stage_other', 0)} "
                f"| 根因(local/tunnel/res/openai/unk)="
                f"{st.get('net_local', 0)}/{st.get('net_tunnel', 0)}/{st.get('net_residential', 0)}/{st.get('net_openai', 0)}/{st.get('net_unknown', 0)} "
                f"| 速度(累计){speed_h:.0f}/h | 速度({rw}s){rolling_h:.0f}/h | 成功率({rw}s){rolling_sr*100:.1f}% "
                f"| 代理冷却 {cooling}/{total_proxy} | 总生成(协议/浏览器) {protocol_total}/{browser_total}"
            ),
            flush=True,
        )


AUTH_URL = "https://auth.openai.com/oauth/authorize"
TOKEN_URL = "https://auth.openai.com/oauth/token"
CLIENT_ID = "app_EMoamEEZ73f0CkXaXp7hrann"
DEFAULT_CALLBACK_PORT = 1455
DEFAULT_REDIRECT_URI = f"http://localhost:{DEFAULT_CALLBACK_PORT}/auth/callback"
DEFAULT_SCOPE = "openid email profile offline_access"


@dataclass(frozen=True)
class OAuthStart:
    auth_url: str
    state: str
    code_verifier: str
    redirect_uri: str


_MAIL_DOMAIN_HEALTH_ORDER = [
    d.strip().lower()
    for d in (os.environ.get("MAIL_DOMAIN_HEALTH_ORDER") or "mail.aiaimimi.com,aimiaimi.cc.cd,mimiaiai.cc.cd,aiaimimi.cc.cd,aiaiai.cc.cd").split(",")
    if d.strip()
]
_MAILBOX_PICK_TRIES = int(os.environ.get("MAILBOX_PICK_TRIES", "3") or "3")
if _MAILBOX_PICK_TRIES <= 0:
    _MAILBOX_PICK_TRIES = 1


def _pick_mailcreate_with_health() -> Mailbox:
    domains: list[str] = []
    seen: set[str] = set()

    def _add_domain(raw: str) -> None:
        d = str(raw or "").strip().lower()
        if not d or d in seen:
            return
        seen.add(d)
        domains.append(d)

    _add_domain(MAILCREATE_DOMAIN)
    for dom in _MAIL_DOMAIN_HEALTH_ORDER:
        _add_domain(dom)

    if not domains:
        return create_mailbox(
            provider="mailcreate",
            mailcreate_base_url=MAILCREATE_BASE_URL,
            mailcreate_custom_auth=MAILCREATE_CUSTOM_AUTH,
            mailcreate_domain="",
            gptmail_base_url=GPTMAIL_BASE_URL,
            gptmail_api_key=GPTMAIL_API_KEY,
            gptmail_keys_file=GPTMAIL_KEYS_FILE,
            gptmail_prefix=GPTMAIL_PREFIX,
            gptmail_domain=GPTMAIL_DOMAIN,
        )

    tries = max(1, _MAILBOX_PICK_TRIES)
    tries = min(tries, len(domains))
    picked_domains = random.sample(domains, k=tries)

    last_err: Exception | None = None
    for dom in picked_domains:
        try:
            return create_mailbox(
                provider="mailcreate",
                mailcreate_base_url=MAILCREATE_BASE_URL,
                mailcreate_custom_auth=MAILCREATE_CUSTOM_AUTH,
                mailcreate_domain=dom,
                gptmail_base_url=GPTMAIL_BASE_URL,
                gptmail_api_key=GPTMAIL_API_KEY,
                gptmail_keys_file=GPTMAIL_KEYS_FILE,
                gptmail_prefix=GPTMAIL_PREFIX,
                gptmail_domain=GPTMAIL_DOMAIN,
            )
        except Exception as e:
            last_err = e
            continue

    if last_err is not None:
        return create_mailbox(
            provider="mailcreate",
            mailcreate_base_url=MAILCREATE_BASE_URL,
            mailcreate_custom_auth=MAILCREATE_CUSTOM_AUTH,
            mailcreate_domain="",
            gptmail_base_url=GPTMAIL_BASE_URL,
            gptmail_api_key=GPTMAIL_API_KEY,
            gptmail_keys_file=GPTMAIL_KEYS_FILE,
            gptmail_prefix=GPTMAIL_PREFIX,
            gptmail_domain=GPTMAIL_DOMAIN,
        )

    raise RuntimeError("failed to pick mailcreate domain")


def create_temp_mailbox() -> tuple[str, str]:
    return create_temp_mailbox_shared()


def wait_openai_code(*, mailbox_ref: str, timeout_seconds: int = 180) -> str:
    return wait_openai_code_shared(mailbox_ref=mailbox_ref, timeout_seconds=timeout_seconds)


def _b64url_no_pad(raw: bytes) -> str:
    return base64.urlsafe_b64encode(raw).decode("ascii").rstrip("=")


def _sha256_b64url_no_pad(s: str) -> str:
    return _b64url_no_pad(hashlib.sha256(s.encode("ascii")).digest())


def _random_state(nbytes: int = 16) -> str:
    return secrets.token_urlsafe(nbytes)


def _pkce_verifier() -> str:
    return secrets.token_urlsafe(64)


def _parse_callback_url(callback_url: str) -> Dict[str, str]:
    candidate = callback_url.strip()
    if not candidate:
        return {
            "code": "",
            "state": "",
            "error": "",
            "error_description": "",
        }

    if "://" not in candidate:
        if candidate.startswith("?"):
            candidate = f"http://localhost{candidate}"
        elif any(ch in candidate for ch in "/?#") or ":" in candidate:
            candidate = f"http://{candidate}"
        elif "=" in candidate:
            candidate = f"http://localhost/?{candidate}"

    parsed = urllib.parse.urlparse(candidate)
    query = urllib.parse.parse_qs(parsed.query, keep_blank_values=True)
    fragment = urllib.parse.parse_qs(parsed.fragment, keep_blank_values=True)

    for key, values in fragment.items():
        if key not in query or not query[key] or not (query[key][0] or "").strip():
            query[key] = values

    def get1(k: str) -> str:
        v = query.get(k, [""])
        return (v[0] or "").strip()

    code = get1("code")
    state = get1("state")
    error = get1("error")
    error_description = get1("error_description")

    if code and not state and "#" in code:
        code, state = code.split("#", 1)

    if not error and error_description:
        error, error_description = error_description, ""

    return {
        "code": code,
        "state": state,
        "error": error,
        "error_description": error_description,
    }


def _jwt_claims_no_verify(id_token: str) -> Dict[str, Any]:
    if not id_token or id_token.count(".") < 2:
        return {}
    payload_b64 = id_token.split(".")[1]
    pad = "=" * ((4 - (len(payload_b64) % 4)) % 4)
    try:
        payload = base64.urlsafe_b64decode((payload_b64 + pad).encode("ascii"))
        return json.loads(payload.decode("utf-8"))
    except Exception:
        return {}


def _to_int(v: Any) -> int:
    try:
        return int(v)
    except (TypeError, ValueError):
        return 0


def get_opener(proxy: str | None = None):
    if not proxy:
        return urllib.request.build_opener()
    proxy_handler = urllib.request.ProxyHandler({"http": proxy, "https": proxy})
    return urllib.request.build_opener(proxy_handler)


def _post_form(url: str, data: Dict[str, str], timeout: int = 30, proxy: str | None = None) -> Dict[str, Any]:
    body = urllib.parse.urlencode(data).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=body,
        method="POST",
        headers={
            "Content-Type": "application/x-www-form-urlencoded",
            "Accept": "application/json",
        },
    )
    with get_opener(proxy).open(req, timeout=timeout) as resp:
        raw = resp.read()
        if resp.status != 200:
            raise RuntimeError(
                f"token exchange failed: {resp.status}: {raw.decode('utf-8', 'replace')}"
            )
        return json.loads(raw.decode("utf-8"))


def generate_oauth_url(*, redirect_uri: str = DEFAULT_REDIRECT_URI, scope: str = DEFAULT_SCOPE) -> OAuthStart:
    state = _random_state()
    code_verifier = _pkce_verifier()
    code_challenge = _sha256_b64url_no_pad(code_verifier)

    params = {
        "client_id": CLIENT_ID,
        "response_type": "code",
        "redirect_uri": redirect_uri,
        "scope": scope,
        "state": state,
        "code_challenge": code_challenge,
        "code_challenge_method": "S256",
        "prompt": "login",
        "id_token_add_organizations": "true",
        "codex_cli_simplified_flow": "true",
    }
    auth_url = f"{AUTH_URL}?{urllib.parse.urlencode(params)}"
    return OAuthStart(
        auth_url=auth_url,
        state=state,
        code_verifier=code_verifier,
        redirect_uri=redirect_uri,
    )


def submit_callback_url(
    *,
    callback_url: str,
    expected_state: str,
    code_verifier: str,
    redirect_uri: str = DEFAULT_REDIRECT_URI,
    proxy: str | None = None,
    mailbox_ref: str = "",
    password: str = "",
    first_name: str = "",
    last_name: str = "",
    birthdate: str = "",
) -> tuple[str, str]:
    cb = _parse_callback_url(callback_url)
    if cb["error"]:
        desc = cb["error_description"]
        raise RuntimeError(f"oauth error: {cb['error']}: {desc}".strip())

    if not cb["code"]:
        raise ValueError("callback url missing ?code=")
    if not cb["state"]:
        raise ValueError("callback url missing ?state=")
    if cb["state"] != expected_state:
        raise ValueError("state mismatch")

    token_resp = _post_form(
        TOKEN_URL,
        {
            "grant_type": "authorization_code",
            "client_id": CLIENT_ID,
            "code": cb["code"],
            "redirect_uri": redirect_uri,
            "code_verifier": code_verifier,
        },
        timeout=30,
        proxy=proxy,
    )

    access_token = (token_resp.get("access_token") or "").strip()
    refresh_token = (token_resp.get("refresh_token") or "").strip()
    id_token = (token_resp.get("id_token") or "").strip()
    expires_in = _to_int(token_resp.get("expires_in"))

    claims = _jwt_claims_no_verify(id_token)
    email = str(claims.get("email") or "").strip()
    auth_claims = claims.get("https://api.openai.com/auth") or {}
    account_id = str(auth_claims.get("chatgpt_account_id") or "").strip()

    now = int(time.time())
    expired_rfc3339 = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(now + max(expires_in, 0)))
    now_rfc3339 = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(now))

    config = dict(claims)
    config.update(
        {
            "type": "codex",
            "email": email,
            "expired": expired_rfc3339,
            "disabled": False,
            "id_token": id_token,
            "access_token": access_token,
            "refresh_token": refresh_token,
            "password": password,
            "birthdate": birthdate,
            "client_id": CLIENT_ID,
            "last_name": last_name,
            "account_id": account_id,
            "first_name": first_name,
            "session_id": claims.get("session_id", ""),
            "last_refresh": now_rfc3339,
            "pwd_auth_time": claims.get("pwd_auth_time", int(time.time() * 1000)),
            "https://api.openai.com/auth": auth_claims,
            "https://api.openai.com/profile": claims.get("https://api.openai.com/profile", {}),
        }
    )

    # 强制 schema 并集：即使上游响应缺字段，也保留关键键。
    schema_defaults = {
        "refresh_token": "",
        "session_id": "",
        "password": "",
        "birthdate": "",
        "first_name": "",
        "last_name": "",
        "mailbox_ref": "",
    }
    for _k, _v in schema_defaults.items():
        if _k not in config:
            config[_k] = _v

    if mailbox_ref and str(mailbox_ref).strip():
        config["mailbox_ref"] = str(mailbox_ref).strip()

    return email, json.dumps(config, ensure_ascii=False, separators=(",", ":"))


def _proxy_dict_for_requests(proxy: str | None) -> dict[str, str] | None:
    p = str(proxy or "").strip()
    if not p:
        return None
    return {"http": p, "https": p}


def _decode_cookie_json_prefix(raw_cookie: str) -> dict[str, Any]:
    v = str(raw_cookie or "").strip()
    if not v:
        return {}

    head = v.split(".", 1)[0]
    for use_urlsafe in (False, True):
        try:
            pad = "=" * ((4 - (len(head) % 4)) % 4)
            blob = (head + pad).encode("ascii")
            decoded = base64.urlsafe_b64decode(blob) if use_urlsafe else base64.b64decode(blob)
            obj = json.loads(decoded.decode("utf-8"))
            if isinstance(obj, dict):
                return obj
        except Exception:
            continue

    return {}


def _follow_redirects_for_callback(*, sess, start_url: str, max_hops: int = 8) -> str:
    cur = str(start_url or "").strip()
    if not cur:
        raise RuntimeError("missing continue_url")

    for _ in range(max_hops):
        resp = sess.get(cur, allow_redirects=False, timeout=PROTOCOL_TIMEOUT_SECONDS)
        status = int(getattr(resp, "status_code", 0) or 0)
        loc = str(resp.headers.get("Location") or "").strip()

        if loc and "localhost:1455" in loc:
            return loc

        if status in (301, 302, 303, 307, 308) and loc:
            if loc.startswith("/"):
                pu = urllib.parse.urlparse(cur)
                loc = f"{pu.scheme}://{pu.netloc}{loc}"
            cur = loc
            continue

        break

    raise RuntimeError("protocol flow did not reach localhost callback")


def generate_name() -> tuple[str, str]:
    first = [
        "Neo",
        "John",
        "Sarah",
        "Michael",
        "Emma",
        "David",
        "James",
        "Robert",
        "Mary",
        "William",
        "Richard",
        "Thomas",
        "Charles",
        "Christopher",
        "Daniel",
        "Matthew",
        "Anthony",
        "Mark",
        "Donald",
        "Steven",
        "Paul",
        "Andrew",
        "Joshua",
        "Kenneth",
        "Kevin",
        "Brian",
        "George",
        "Edward",
        "Ronald",
        "Timothy",
    ]
    last = [
        "Smith",
        "Johnson",
        "Williams",
        "Brown",
        "Jones",
        "Garcia",
        "Miller",
        "Davis",
        "Rodriguez",
        "Martinez",
        "Hernandez",
        "Lopez",
        "Gonzalez",
        "Wilson",
        "Anderson",
        "Thomas",
        "Taylor",
        "Moore",
        "Jackson",
        "Martin",
        "Lee",
        "Perez",
        "Thompson",
        "White",
    ]
    return random.choice(first), random.choice(last)


def generate_pwd(length: int = 12) -> str:
    chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@*&"
    return "".join(random.choice(chars) for _ in range(length)) + "A1@"


def register_protocol(proxy: str | None = None) -> tuple[str, str]:
    if curl_requests is None:
        raise RuntimeError("protocol flow requires curl_cffi; please install curl_cffi")

    def _mailtm_headers(*, token: str = "", use_json: bool = False) -> Dict[str, str]:
        headers = {"Accept": "application/json"}
        if use_json:
            headers["Content-Type"] = "application/json"
        if token:
            headers["Authorization"] = f"Bearer {token}"
        return headers

    def _mailtm_domains(*, api_base: str, px: dict[str, str] | None, imp: str) -> list[str]:
        resp = curl_requests.get(
            f"{api_base}/domains",
            headers=_mailtm_headers(),
            proxies=px,
            impersonate=imp,
            timeout=15,
        )
        if int(getattr(resp, "status_code", 0) or 0) != 200:
            raise RuntimeError(f"获取 Mail.tm 域名失败，状态码: {resp.status_code}")

        data = resp.json()
        if isinstance(data, list):
            items = data
        elif isinstance(data, dict):
            items = data.get("hydra:member") or data.get("items") or []
        else:
            items = []

        domains: list[str] = []
        for item in items:
            if not isinstance(item, dict):
                continue
            domain = str(item.get("domain") or "").strip()
            is_active = item.get("isActive", True)
            is_private = item.get("isPrivate", False)
            if domain and is_active and not is_private:
                domains.append(domain)
        return domains

    def _create_mailtm_mailbox(*, api_base: str, px: dict[str, str] | None, imp: str) -> tuple[str, str, str]:
        domains = _mailtm_domains(api_base=api_base, px=px, imp=imp)
        if not domains:
            raise RuntimeError("Mail.tm 没有可用域名")

        for _ in range(5):
            local = f"oc{secrets.token_hex(5)}"
            domain = random.choice(domains)
            email = f"{local}@{domain}"
            password = secrets.token_urlsafe(18)

            create_resp = curl_requests.post(
                f"{api_base}/accounts",
                headers=_mailtm_headers(use_json=True),
                json={"address": email, "password": password},
                proxies=px,
                impersonate=imp,
                timeout=15,
            )
            if int(getattr(create_resp, "status_code", 0) or 0) not in (200, 201):
                continue

            token_resp = curl_requests.post(
                f"{api_base}/token",
                headers=_mailtm_headers(use_json=True),
                json={"address": email, "password": password},
                proxies=px,
                impersonate=imp,
                timeout=15,
            )
            if int(getattr(token_resp, "status_code", 0) or 0) == 200:
                token = str((token_resp.json() or {}).get("token") or "").strip()
                if token:
                    return email, token, password

        raise RuntimeError("Mail.tm 邮箱创建成功但获取 Token 失败")

    def _poll_oai_code(
        *,
        api_base: str,
        token: str,
        email: str,
        px: dict[str, str] | None,
        imp: str,
        timeout_seconds: int,
    ) -> str:
        url_list = f"{api_base}/messages"
        regex = r"(?<!\d)(\d{6})(?!\d)"
        seen_ids: set[str] = set()

        loops = max(1, int(timeout_seconds / 3))
        _log(f"[protocol] waiting otp for {email}", force=True)

        for _ in range(loops):
            try:
                resp = curl_requests.get(
                    url_list,
                    headers=_mailtm_headers(token=token),
                    proxies=px,
                    impersonate=imp,
                    timeout=15,
                )
                if int(getattr(resp, "status_code", 0) or 0) != 200:
                    time.sleep(3)
                    continue

                data = resp.json()
                if isinstance(data, list):
                    messages = data
                elif isinstance(data, dict):
                    messages = data.get("hydra:member") or data.get("messages") or []
                else:
                    messages = []

                for msg in messages:
                    if not isinstance(msg, dict):
                        continue
                    msg_id = str(msg.get("id") or "").strip()
                    if not msg_id or msg_id in seen_ids:
                        continue
                    seen_ids.add(msg_id)

                    read_resp = curl_requests.get(
                        f"{api_base}/messages/{msg_id}",
                        headers=_mailtm_headers(token=token),
                        proxies=px,
                        impersonate=imp,
                        timeout=15,
                    )
                    if int(getattr(read_resp, "status_code", 0) or 0) != 200:
                        continue

                    mail_data = read_resp.json()
                    sender = str(((mail_data.get("from") or {}).get("address") or "")).lower()
                    subject = str(mail_data.get("subject") or "")
                    intro = str(mail_data.get("intro") or "")
                    text = str(mail_data.get("text") or "")
                    html = mail_data.get("html") or ""
                    if isinstance(html, list):
                        html = "\n".join(str(x) for x in html)
                    content = "\n".join([subject, intro, text, str(html)])

                    if "openai" not in sender and "openai" not in content.lower():
                        continue

                    m = re.search(regex, content)
                    if m:
                        return m.group(1)
            except Exception:
                pass

            time.sleep(3)

        return ""

    api_base = (os.environ.get("PROTOCOL_MAILTM_BASE", "https://api.mail.tm") or "https://api.mail.tm").strip().rstrip("/")
    proxies = _proxy_dict_for_requests(proxy)

    imp_seed = (os.environ.get("PROTOCOL_IMPERSONATE_POOL", "chrome,chrome110,chrome116,safari") or "chrome,chrome110,chrome116,safari")
    # edge 在部分 curl_cffi 版本会在真实请求阶段抛出 not supported，这里默认剔除
    imp_pool = [x.strip() for x in imp_seed.split(",") if x.strip() and x.strip().lower() != "edge"]
    if not imp_pool:
        imp_pool = ["chrome"]

    imp_value = str(PROTOCOL_IMPERSONATE or "chrome").strip().lower()
    current_impersonate = random.choice(imp_pool) if imp_value in ("auto", "random") else str(PROTOCOL_IMPERSONATE or "chrome")

    try:
        sess = curl_requests.Session(proxies=proxies, impersonate=current_impersonate)
    except Exception as e:
        if "not supported" in str(e).lower() and str(current_impersonate).lower() != "chrome":
            current_impersonate = "chrome"
            sess = curl_requests.Session(proxies=proxies, impersonate=current_impersonate)
        else:
            raise

    if PROTOCOL_CHECK_GEO:
        trace_resp = sess.get("https://cloudflare.com/cdn-cgi/trace", timeout=10)
        trace_txt = str(getattr(trace_resp, "text", "") or "")
        loc_m = re.search(r"^loc=(.+)$", trace_txt, re.MULTILINE)
        ip_m = re.search(r"^ip=(.+)$", trace_txt, re.MULTILINE)
        loc = (loc_m.group(1) if loc_m else "").strip().upper()
        ip = (ip_m.group(1) if ip_m else "").strip()
        _log(f"[protocol] loc={loc} ip={ip}")
        if loc and loc in PROTOCOL_BLOCKED_LOCS:
            raise RuntimeError(f"protocol flow blocked geo loc={loc}")

    email, mailbox_token, mailbox_password = _create_mailtm_mailbox(api_base=api_base, px=proxies, imp=current_impersonate)
    _log(f"Email obtained: {email}")

    oauth = generate_oauth_url()
    _log(f"OAuth URL: {oauth.auth_url}")

    signup_body = json.dumps(
        {"username": {"value": email, "kind": "email"}, "screen_hint": "signup"},
        ensure_ascii=False,
        separators=(",", ":"),
    )

    signup_resp = None
    for _auth_try in range(2):
        # 重新打一次 OAuth 起点，确保 did/cookie/sentinel 同步
        try:
            sess.get(oauth.auth_url, timeout=PROTOCOL_TIMEOUT_SECONDS)
        except Exception as e:
            if "not supported" in str(e).lower() and str(current_impersonate).lower() != "chrome":
                current_impersonate = "chrome"
                sess = curl_requests.Session(proxies=proxies, impersonate=current_impersonate)
                _log(f"[protocol] fallback impersonate={current_impersonate}", force=True)
                sess.get(oauth.auth_url, timeout=PROTOCOL_TIMEOUT_SECONDS)
            else:
                raise

        did = str(sess.cookies.get("oai-did") or "").strip()
        if not did:
            raise RuntimeError("protocol flow missing oai-did cookie")

        sentinel_req = json.dumps(
            {"p": "", "id": did, "flow": "authorize_continue"},
            ensure_ascii=False,
            separators=(",", ":"),
        )
        # 与参考3保持一致：sentinel 使用独立请求（非 session）
        sen_resp = curl_requests.post(
            "https://sentinel.openai.com/backend-api/sentinel/req",
            headers={
                "origin": "https://sentinel.openai.com",
                "referer": "https://sentinel.openai.com/backend-api/sentinel/frame.html?sv=20260219f9f6",
                "content-type": "text/plain;charset=UTF-8",
            },
            data=sentinel_req,
            proxies=proxies,
            impersonate=current_impersonate,
            timeout=PROTOCOL_TIMEOUT_SECONDS,
        )
        if int(getattr(sen_resp, "status_code", 0) or 0) != 200:
            raise RuntimeError(f"sentinel req failed: http={sen_resp.status_code}")

        sentinel_token = str((sen_resp.json() or {}).get("token") or "").strip()
        if not sentinel_token:
            raise RuntimeError("sentinel token missing")

        sentinel_header_value = json.dumps(
            {"p": "", "t": "", "c": sentinel_token, "id": did, "flow": "authorize_continue"},
            ensure_ascii=False,
            separators=(",", ":"),
        )

        signup_resp = sess.post(
            "https://auth.openai.com/api/accounts/authorize/continue",
            headers={
                "referer": "https://auth.openai.com/create-account",
                "accept": "application/json",
                "content-type": "application/json",
                "openai-sentinel-token": sentinel_header_value,
            },
            data=signup_body,
            timeout=PROTOCOL_TIMEOUT_SECONDS,
        )

        _st = int(getattr(signup_resp, "status_code", 0) or 0)
        _body = str(getattr(signup_resp, "text", "") or "")
        if _st == 200:
            break

        # 命中 challenge 时，换一套指纹重试一次
        if (
            _auth_try == 0
            and _st in (403, 429)
            and "just a moment" in _body.lower()
        ):
            alt_pool = [x for x in imp_pool if str(x).strip().lower() != str(current_impersonate).strip().lower()]
            # 兜底保证 chrome 总可选
            if not any(str(x).strip().lower() == "chrome" for x in alt_pool):
                alt_pool.append("chrome")
            random.shuffle(alt_pool)
            for next_imp in alt_pool:
                try:
                    sess = curl_requests.Session(proxies=proxies, impersonate=next_imp)
                    current_impersonate = next_imp
                    _log(f"[protocol] challenge retry with impersonate={current_impersonate}", force=True)
                    break
                except Exception:
                    continue
            else:
                # 没有任何可用指纹则保持现状，交给后续报错
                pass
            if _auth_try == 0 and current_impersonate:
                continue

        raise RuntimeError(
            f"authorize/continue failed: http={_st} body={_body[:300]}"
        )

    otp_send_resp = sess.post(
        "https://auth.openai.com/api/accounts/passwordless/send-otp",
        headers={
            "referer": "https://auth.openai.com/create-account/password",
            "accept": "application/json",
            "content-type": "application/json",
        },
        timeout=PROTOCOL_TIMEOUT_SECONDS,
    )
    if int(getattr(otp_send_resp, "status_code", 0) or 0) != 200:
        raise RuntimeError(
            f"send-otp failed: http={otp_send_resp.status_code} body={str(getattr(otp_send_resp, 'text', '') or '')[:300]}"
        )

    code = _poll_oai_code(
        api_base=api_base,
        token=mailbox_token,
        email=email,
        px=proxies,
        imp=current_impersonate,
        timeout_seconds=OTP_TIMEOUT_SECONDS,
    )
    if not code:
        raise RuntimeError("timeout waiting for 6-digit code")
    _log(f"Verification Code: {code}")

    otp_verify_resp = sess.post(
        "https://auth.openai.com/api/accounts/email-otp/validate",
        headers={
            "referer": "https://auth.openai.com/email-verification",
            "accept": "application/json",
            "content-type": "application/json",
        },
        data=json.dumps({"code": str(code)}),
        timeout=PROTOCOL_TIMEOUT_SECONDS,
    )
    if int(getattr(otp_verify_resp, "status_code", 0) or 0) != 200:
        raise RuntimeError(
            f"email-otp/validate failed: http={otp_verify_resp.status_code} body={str(getattr(otp_verify_resp, 'text', '') or '')[:300]}"
        )

    first_name, last_name = generate_name()
    birthdate = f"{random.randint(1980, 2002)}-0{random.randint(1, 9)}-{random.randint(10, 28)}"

    create_account_resp = sess.post(
        "https://auth.openai.com/api/accounts/create_account",
        headers={
            "referer": "https://auth.openai.com/about-you",
            "accept": "application/json",
            "content-type": "application/json",
        },
        data=json.dumps({"name": f"{first_name} {last_name}", "birthdate": birthdate}),
        timeout=PROTOCOL_TIMEOUT_SECONDS,
    )
    if int(getattr(create_account_resp, "status_code", 0) or 0) != 200:
        raise RuntimeError(
            f"create_account failed: http={create_account_resp.status_code} body={str(getattr(create_account_resp, 'text', '') or '')[:400]}"
        )

    auth_cookie = str(sess.cookies.get("oai-client-auth-session") or "").strip()
    auth_obj = _decode_cookie_json_prefix(auth_cookie)
    ws_list = auth_obj.get("workspaces") if isinstance(auth_obj, dict) else None

    workspace_id = ""
    if isinstance(ws_list, list) and ws_list and isinstance(ws_list[0], dict):
        workspace_id = str(ws_list[0].get("id") or "").strip()

    if not workspace_id:
        raise RuntimeError("workspace id missing in auth session cookie")

    select_resp = sess.post(
        "https://auth.openai.com/api/accounts/workspace/select",
        headers={
            "referer": "https://auth.openai.com/sign-in-with-chatgpt/codex/consent",
            "content-type": "application/json",
            "accept": "application/json",
        },
        data=json.dumps({"workspace_id": workspace_id}),
        timeout=PROTOCOL_TIMEOUT_SECONDS,
    )
    if int(getattr(select_resp, "status_code", 0) or 0) != 200:
        raise RuntimeError(
            f"workspace/select failed: http={select_resp.status_code} body={str(getattr(select_resp, 'text', '') or '')[:300]}"
        )

    continue_url = str((select_resp.json() or {}).get("continue_url") or "").strip()
    if not continue_url:
        raise RuntimeError("workspace/select missing continue_url")

    callback_url = _follow_redirects_for_callback(sess=sess, start_url=continue_url, max_hops=8)

    reg_email, config_json = submit_callback_url(
        callback_url=callback_url,
        expected_state=oauth.state,
        code_verifier=oauth.code_verifier,
        redirect_uri=oauth.redirect_uri,
        proxy=proxy,
        mailbox_ref=f"mailtm:{email}",
        password=mailbox_password,
        first_name=first_name,
        last_name=last_name,
        birthdate=birthdate,
    )

    return reg_email, config_json


def _partition_proxies(proxies: list[str]) -> list[str]:
    if PROXY_PARTITION_TOTAL <= 0:
        return proxies
    out = [p for i, p in enumerate(proxies) if (i % PROXY_PARTITION_TOTAL) == PROXY_PARTITION_INDEX]
    return out


def load_proxies() -> list[str]:
    if PROTOCOL_IGNORE_PROXIES:
        return []
    proxy_file = _data_path("proxies.txt")
    if os.path.exists(proxy_file):
        with open(proxy_file, "r", encoding="utf-8") as f:
            all_proxies = [line.strip() for line in f if line.strip() and not line.startswith("#")]
        return _partition_proxies(all_proxies)
    return []


def worker(worker_id: int) -> None:
    current_proxy: str | None = None
    assigned_at = 0.0

    while True:
        proxies = load_proxies()
        proxy, assigned_at = _pick_proxy(
            proxies=proxies,
            current_proxy=current_proxy,
            assigned_at=assigned_at,
        )

        if proxies and not proxy:
            if PROTOCOL_ALLOW_DIRECT_FALLBACK:
                # 代理池全部冷却/不可用时，允许直连继续跑，避免容器长时间0产出
                proxy = None
                _log(
                    f"[Protocol Worker {worker_id}] proxy pool unavailable -> fallback DIRECT",
                    force=True,
                )
            else:
                _stats_inc("cooldown_wait")
                time.sleep(1.0)
                continue

        current_proxy = proxy
        _stats_inc("attempt")

        _log(
            f"[Protocol Worker {worker_id}] use_proxy={(proxy or 'DIRECT')} rotate={PROXY_ROTATE_SECONDS}s cooldown={PROXY_COOLDOWN_SECONDS}s"
        )

        try:
            reg_email, res = register_protocol(proxy)

            with write_lock:
                _append_result_line(res)

                codex_auth_dir = _data_path(CODEX_AUTH_DIRNAME)
                os.makedirs(codex_auth_dir, exist_ok=True)

                ts_ms = int(time.time() * 1000)
                rand = secrets.token_hex(3)
                auth_path = os.path.join(
                    codex_auth_dir,
                    f"codex-{reg_email}-free-{INSTANCE_ID}-{ts_ms}-{rand}.json",
                )

                with open(auth_path, "w", encoding="utf-8") as f:
                    f.write(json.dumps(json.loads(res), indent=2, ensure_ascii=False))

                wait_update_dir = _data_path(WAIT_UPDATE_DIRNAME)
                os.makedirs(wait_update_dir, exist_ok=True)
                try:
                    import shutil

                    shutil.copy2(auth_path, os.path.join(wait_update_dir, os.path.basename(auth_path)))
                except Exception:
                    pass

            _stats_inc("success")
            _log(f"[Protocol Worker {worker_id}] [✓] success email={reg_email}")

        except Exception as e:
            cls = _classify_error(e)
            stg = _infer_stage_from_error(str(e))
            net = _infer_net_cause(str(e))
            _stats_inc(cls, err=e, stage=stg, net_cause=net)
            _log(f"[Protocol Worker {worker_id}] [x] cls={cls} stage={stg} net={net} err={e}")

            if proxy:
                cool_sec = _cooldown_seconds_for_error_class(cls)
                _proxy_mark_cooldown(proxy, cool_sec)
            current_proxy = None

        sleep_min = int(os.environ.get("SLEEP_MIN", "0"))
        sleep_max = int(os.environ.get("SLEEP_MAX", "1"))
        sleep_time = random.randint(sleep_min, sleep_max) if sleep_max >= sleep_min else sleep_min
        if sleep_time > 0:
            time.sleep(sleep_time)


if __name__ == "__main__":
    if curl_requests is None:
        raise RuntimeError("curl_cffi 未安装，无法运行协议流。请先安装: pip install curl_cffi")

    os.makedirs(DATA_DIR, exist_ok=True)
    os.makedirs(_results_dir(), exist_ok=True)
    os.makedirs(_data_path(ERROR_DIRNAME, INSTANCE_ID), exist_ok=True)
    os.makedirs(_data_path(CODEX_AUTH_DIRNAME), exist_ok=True)
    os.makedirs(_data_path(WAIT_UPDATE_DIRNAME), exist_ok=True)

    proxy_file = _data_path("proxies.txt")
    if not os.path.exists(proxy_file):
        with open(proxy_file, "w", encoding="utf-8") as f:
            f.write("# 在此文件中添加您的代理IP池，每行一个\n")
            f.write("# 格式示例: http://192.168.1.100:8080\n")

    concurrency = int(os.environ.get("CONCURRENCY", "1"))
    if concurrency < 1:
        concurrency = 1

    print(f"==== 协议流守护进程启动: 并发数 {concurrency} ====", flush=True)
    print(f"INSTANCE_ID={INSTANCE_ID}", flush=True)
    print(f"results 分片将写入 {_results_dir()} (每 {RESULTS_SHARD_SIZE} 条一片)", flush=True)
    print(f"账号 JSON 将写入 {_data_path(CODEX_AUTH_DIRNAME)} 并复制到 {_data_path(WAIT_UPDATE_DIRNAME)}", flush=True)
    print(f"代理池请直接写入 {proxy_file}", flush=True)
    if PROXY_PARTITION_TOTAL > 0:
        print(
            f"代理分片: index={PROXY_PARTITION_INDEX}/{PROXY_PARTITION_TOTAL} (仅使用该分片代理)",
            flush=True,
        )
    print(
        (
            f"策略: rotate={PROXY_ROTATE_SECONDS}s "
            f"cooldown(proxy/blocked/invalid/otp/other)="
            f"{COOLDOWN_PROXY_ERROR_SECONDS}/{COOLDOWN_BLOCKED_SECONDS}/{COOLDOWN_INVALID_AUTH_SECONDS}/{COOLDOWN_OTP_TIMEOUT_SECONDS}/{COOLDOWN_OTHER_SECONDS}s "
            f"otp_timeout={OTP_TIMEOUT_SECONDS}s summary_only={int(PROTOCOL_SUMMARY_ONLY)}"
        ),
        flush=True,
    )

    t_summary = threading.Thread(target=_summary_loop, name="protocol_summary", daemon=True)
    t_summary.start()

    with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as executor:
        for i in range(concurrency):
            executor.submit(worker, i + 1)
            time.sleep(0.05)
