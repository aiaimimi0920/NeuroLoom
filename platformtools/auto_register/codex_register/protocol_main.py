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
_PLAT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
_MAILCREATE_CLIENT_DIR = os.path.join(_PLAT_DIR, "mailcreate", "client")
if _MAILCREATE_CLIENT_DIR not in sys.path:
    sys.path.insert(0, _MAILCREATE_CLIENT_DIR)

from mailbox_provider import Mailbox, create_mailbox, wait_openai_code as wait_openai_code_by_provider  # type: ignore


write_lock = threading.Lock()
stats_lock = threading.Lock()
proxy_state_lock = threading.Lock()

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

# Codex usage probe (aligned with main/browser_version behavior)
WHAM_USAGE_URL = "https://chatgpt.com/backend-api/wham/usage"
PROBE_LOCAL_COOLDOWN_MAX_SECONDS = int(os.environ.get("PROBE_LOCAL_COOLDOWN_MAX_SECONDS", "1800") or "1800")
if PROBE_LOCAL_COOLDOWN_MAX_SECONDS < 0:
    PROBE_LOCAL_COOLDOWN_MAX_SECONDS = 0


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


def _append_jsonl(path: str, obj: Any) -> None:
    try:
        os.makedirs(os.path.dirname(path), exist_ok=True)
    except Exception:
        pass
    try:
        line = json.dumps(obj, ensure_ascii=False, separators=(",", ":"))
        with open(path, "a", encoding="utf-8") as f:
            f.write(line + "\n")
    except Exception:
        pass


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

OTP_TIMEOUT_SECONDS = int(os.environ.get("OTP_TIMEOUT_SECONDS", "120") or "120")
if OTP_TIMEOUT_SECONDS <= 0:
    OTP_TIMEOUT_SECONDS = 120

# 注册主流程是否强制要求代理。
# - 1: 无代理直接判定本轮失败（默认）
# - 0: 允许直连兜底
REGISTER_PROXY_REQUIRED = (os.environ.get("REGISTER_PROXY_REQUIRED", "1") or "1").strip().lower() not in (
    "0",
    "false",
    "no",
)

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


def _stats_inc(kind: str, err: Exception | str | None = None, *, stage: str | None = None) -> None:
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

        print(
            (
                f"[SUMMARY] 完成 {st.get('success', 0)}{target_txt} | 尝试 {st.get('attempt', 0)} | 失败 {st.get('fail', 0)} "
                f"| 封控 {st.get('blocked', 0)} | OTP超时 {st.get('otp_timeout', 0)} "
                f"| 代理错 {st.get('proxy_error', 0)} | invalid_step {st.get('invalid_auth_step', 0)} "
                f"| 阶段(auth/send/otpv/create/ws/cb/other)="
                f"{st.get('stage_auth_continue', 0)}/{st.get('stage_send_otp', 0)}/{st.get('stage_otp_validate', 0)}/"
                f"{st.get('stage_create_account', 0)}/{st.get('stage_workspace', 0)}/{st.get('stage_callback', 0)}/{st.get('stage_other', 0)} "
                f"| 速度(累计){speed_h:.0f}/h | 速度({rw}s){rolling_h:.0f}/h | 成功率({rw}s){rolling_sr*100:.1f}% "
                f"| 代理冷却 {cooling}/{total_proxy}"
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


@dataclass
class ProbeResult:
    status_code: int | None
    note: str
    category: str
    retry_after_seconds: int = 0
    http_status: int | None = None


_MAIL_DOMAIN_HEALTH_ORDER = [
    d.strip().lower()
    for d in (os.environ.get("MAIL_DOMAIN_HEALTH_ORDER") or "mail.aiaimimi.com,aimiaimi.cc.cd,mimiaiai.cc.cd,aiaimimi.cc.cd,aiaiai.cc.cd").split(",")
    if d.strip()
]
_MAILBOX_PICK_TRIES = int(os.environ.get("MAILBOX_PICK_TRIES", "3") or "3")
if _MAILBOX_PICK_TRIES <= 0:
    _MAILBOX_PICK_TRIES = 1


def _domain_of_email(email: str) -> str:
    s = str(email or "").strip().lower()
    if "@" not in s:
        return ""
    return s.split("@", 1)[1].strip()


def _domain_health_score(domain: str) -> int:
    d = str(domain or "").strip().lower()
    if not d:
        return -10_000
    try:
        idx = _MAIL_DOMAIN_HEALTH_ORDER.index(d)
        return 10_000 - idx
    except ValueError:
        return 0


def _pick_mailcreate_with_health() -> Mailbox:
    candidates: list[Mailbox] = []

    # 1) 优先尝试固定高成功率域名
    if MAILCREATE_DOMAIN:
        mb = create_mailbox(
            provider="mailcreate",
            mailcreate_base_url=MAILCREATE_BASE_URL,
            mailcreate_custom_auth=MAILCREATE_CUSTOM_AUTH,
            mailcreate_domain=MAILCREATE_DOMAIN,
            gptmail_base_url=GPTMAIL_BASE_URL,
            gptmail_api_key=GPTMAIL_API_KEY,
            gptmail_keys_file=GPTMAIL_KEYS_FILE,
            gptmail_prefix=GPTMAIL_PREFIX,
            gptmail_domain=GPTMAIL_DOMAIN,
        )
        candidates.append(mb)

    # 2) 额外尝试健康域（去重）
    for dom in _MAIL_DOMAIN_HEALTH_ORDER:
        if len(candidates) >= _MAILBOX_PICK_TRIES:
            break
        if MAILCREATE_DOMAIN and dom == MAILCREATE_DOMAIN.strip().lower():
            continue
        try:
            mb = create_mailbox(
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
            candidates.append(mb)
        except Exception:
            continue

    # 3) 如果上面都没拿到，再兜底走原配置
    if not candidates:
        candidates.append(
            create_mailbox(
                provider=MAILBOX_PROVIDER,
                mailcreate_base_url=MAILCREATE_BASE_URL,
                mailcreate_custom_auth=MAILCREATE_CUSTOM_AUTH,
                mailcreate_domain=MAILCREATE_DOMAIN,
                gptmail_base_url=GPTMAIL_BASE_URL,
                gptmail_api_key=GPTMAIL_API_KEY,
                gptmail_keys_file=GPTMAIL_KEYS_FILE,
                gptmail_prefix=GPTMAIL_PREFIX,
                gptmail_domain=GPTMAIL_DOMAIN,
            )
        )

    best = max(candidates, key=lambda m: _domain_health_score(_domain_of_email(m.email)))
    return best


def create_temp_mailbox() -> tuple[str, str]:
    provider = (MAILBOX_PROVIDER or "").strip().lower()
    if provider in ("mailcreate", "self", "local"):
        mb = _pick_mailcreate_with_health()
    else:
        mb: Mailbox = create_mailbox(
            provider=MAILBOX_PROVIDER,
            mailcreate_base_url=MAILCREATE_BASE_URL,
            mailcreate_custom_auth=MAILCREATE_CUSTOM_AUTH,
            mailcreate_domain=MAILCREATE_DOMAIN,
            gptmail_base_url=GPTMAIL_BASE_URL,
            gptmail_api_key=GPTMAIL_API_KEY,
            gptmail_keys_file=GPTMAIL_KEYS_FILE,
            gptmail_prefix=GPTMAIL_PREFIX,
            gptmail_domain=GPTMAIL_DOMAIN,
        )

        # auto 情况：若最终落到 mailcreate，再做一次健康域优选
        if getattr(mb, "provider", "") == "mailcreate":
            try:
                mb = _pick_mailcreate_with_health()
            except Exception:
                pass

    return mb.email, mb.ref


def wait_openai_code(*, mailbox_ref: str, timeout_seconds: int = 180) -> str:
    return wait_openai_code_by_provider(
        provider=MAILBOX_PROVIDER,
        mailbox_ref=mailbox_ref,
        mailcreate_base_url=MAILCREATE_BASE_URL,
        mailcreate_custom_auth=MAILCREATE_CUSTOM_AUTH,
        gptmail_base_url=GPTMAIL_BASE_URL,
        gptmail_api_key=GPTMAIL_API_KEY,
        gptmail_keys_file=GPTMAIL_KEYS_FILE,
        timeout_seconds=timeout_seconds,
    )


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


def _infer_account_id_from_auth(auth_obj: Any) -> str | None:
    if not isinstance(auth_obj, dict):
        return None

    v = str(auth_obj.get("account_id") or "").strip()
    if v:
        return v

    auth_claims = auth_obj.get("https://api.openai.com/auth")
    if isinstance(auth_claims, dict):
        v2 = str(auth_claims.get("chatgpt_account_id") or "").strip()
        if v2:
            return v2

    return None


def _infer_access_token_from_auth(auth_obj: Any) -> str | None:
    if not isinstance(auth_obj, dict):
        return None
    v = str(auth_obj.get("access_token") or "").strip()
    return v or None


def _wham_headers(*, access_token: str, account_id: str) -> dict[str, str]:
    return {
        "Authorization": f"Bearer {access_token}",
        "chatgpt-account-id": account_id,
        "Accept": "application/json",
        "originator": "codex_cli_rs",
    }


def _parse_retry_after_seconds_from_error_body(*, http_status: int, raw_body: str, now_ts: float | None = None) -> int:
    if http_status != 429:
        return 0

    now = float(now_ts if now_ts is not None else time.time())

    try:
        obj = json.loads(raw_body) if raw_body else {}
    except Exception:
        obj = {}

    if not isinstance(obj, dict):
        return 0

    err = obj.get("error")
    if not isinstance(err, dict):
        return 0

    et = str(err.get("type") or "").strip()
    if et and et != "usage_limit_reached":
        return 0

    try:
        resets_at = int(err.get("resets_at") or 0)
    except Exception:
        resets_at = 0
    if resets_at > 0:
        wait = int(max(0, resets_at - int(now)))
        if wait > 0:
            return wait

    try:
        resets_in = int(err.get("resets_in_seconds") or 0)
    except Exception:
        resets_in = 0
    if resets_in > 0:
        return resets_in

    return 0


def _extract_retry_after_seconds_from_wham_obj(obj: Any) -> int:
    if not isinstance(obj, dict):
        return 0

    rl = obj.get("rate_limit")
    if isinstance(rl, dict):
        for k in ("resets_in_seconds", "retry_after_seconds", "retry_after"):
            try:
                v = int(rl.get(k) or 0)
            except Exception:
                v = 0
            if v > 0:
                return v
        for k in ("resets_at", "reset_at"):
            try:
                ts = int(rl.get(k) or 0)
            except Exception:
                ts = 0
            if ts > 0:
                wait = int(max(0, ts - int(time.time())))
                if wait > 0:
                    return wait

    for k in ("resets_in_seconds", "retry_after_seconds", "retry_after"):
        try:
            v = int(obj.get(k) or 0)
        except Exception:
            v = 0
        if v > 0:
            return v

    return 0


def _wham_usage_is_quota0(obj: Any) -> bool:
    if not isinstance(obj, dict):
        return False

    rl = obj.get("rate_limit")
    if isinstance(rl, dict):
        allowed = rl.get("allowed")
        if allowed is False:
            return True
        limit_reached = rl.get("limit_reached")
        if limit_reached is True:
            return True

        pw = rl.get("primary_window")
        if isinstance(pw, dict):
            try:
                used_percent = pw.get("used_percent")
                if used_percent is not None and float(used_percent) >= 100:
                    return True
            except Exception:
                pass

    for k in ("allowed", "limit_reached", "is_available"):
        if k in obj and obj.get(k) in (False, 0):
            return True

    return False


def _probe_wham_one(*, auth_obj: Any, proxy: str | None = None) -> ProbeResult:
    account_id = _infer_account_id_from_auth(auth_obj)
    access_token = _infer_access_token_from_auth(auth_obj)
    if not account_id or not access_token:
        return ProbeResult(status_code=None, note="missing account_id/access_token", category="invalid_input")

    headers = _wham_headers(access_token=access_token, account_id=account_id)
    req = urllib.request.Request(WHAM_USAGE_URL, headers=headers, method="GET")

    try:
        with get_opener(proxy).open(req, timeout=PROTOCOL_TIMEOUT_SECONDS) as resp:
            raw = resp.read().decode("utf-8", errors="replace")
            http_status = int(getattr(resp, "status", 200) or 200)
    except urllib.error.HTTPError as e:
        code = int(getattr(e, "code", 0) or 0)
        body = ""
        try:
            body = e.read().decode("utf-8", errors="replace")
        except Exception:
            body = ""

        if code == 401:
            return ProbeResult(status_code=401, note="http401", category="invalid_auth", http_status=401)
        if code == 429:
            retry_after = _parse_retry_after_seconds_from_error_body(http_status=429, raw_body=body)
            return ProbeResult(
                status_code=429,
                note="http429",
                category="quota_limited",
                retry_after_seconds=retry_after,
                http_status=429,
            )
        return ProbeResult(status_code=None, note=f"http{code}", category="upstream_http_error", http_status=code)
    except Exception as e:
        return ProbeResult(status_code=None, note=f"error:{e}", category="network_error")

    try:
        obj = json.loads(raw) if raw else {}
    except Exception:
        obj = {}

    if _wham_usage_is_quota0(obj):
        retry_after = _extract_retry_after_seconds_from_wham_obj(obj)
        return ProbeResult(
            status_code=429,
            note="quota0",
            category="quota_limited",
            retry_after_seconds=retry_after,
            http_status=http_status,
        )

    return ProbeResult(status_code=200, note="ok", category="ok", http_status=http_status)


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

    proxies = _proxy_dict_for_requests(proxy)
    if REGISTER_PROXY_REQUIRED and not proxies:
        raise RuntimeError("register_proxy_required no_proxy_available flow=protocol")

    proxy_id = str(proxy or "DIRECT")
    sess = curl_requests.Session(proxies=proxies, impersonate=PROTOCOL_IMPERSONATE)

    if PROTOCOL_CHECK_GEO:
        trace_resp = sess.get("https://cloudflare.com/cdn-cgi/trace", timeout=10)
        trace_txt = str(getattr(trace_resp, "text", "") or "")
        loc_m = re.search(r"^loc=(.+)$", trace_txt, re.MULTILINE)
        ip_m = re.search(r"^ip=(.+)$", trace_txt, re.MULTILINE)
        loc = (loc_m.group(1) if loc_m else "").strip().upper()
        ip = (ip_m.group(1) if ip_m else "").strip()
        _log(f"[protocol] loc={loc} ip={ip} proxy_id={proxy_id}")
        if loc and loc in PROTOCOL_BLOCKED_LOCS:
            raise RuntimeError(f"protocol flow blocked geo loc={loc}")

    _log("[mailbox] mailbox_direct=true action=create_temp_mailbox")
    email, mailbox_ref = create_temp_mailbox()
    ref_preview = (mailbox_ref[:20] + "...") if len(mailbox_ref) > 20 else mailbox_ref
    _log(f"Email obtained: {email} mailbox_ref={ref_preview}")

    oauth = generate_oauth_url()
    _log(f"OAuth URL: {oauth.auth_url}")

    sess.get(oauth.auth_url, timeout=PROTOCOL_TIMEOUT_SECONDS)

    did = str(sess.cookies.get("oai-did") or "").strip()
    if not did:
        raise RuntimeError("protocol flow missing oai-did cookie")

    sentinel_req = json.dumps(
        {"p": "", "id": did, "flow": "authorize_continue"},
        ensure_ascii=False,
        separators=(",", ":"),
    )
    sen_resp = sess.post(
        "https://sentinel.openai.com/backend-api/sentinel/req",
        headers={
            "origin": "https://sentinel.openai.com",
            "referer": "https://sentinel.openai.com/backend-api/sentinel/frame.html?sv=20260219f9f6",
            "content-type": "text/plain;charset=UTF-8",
        },
        data=sentinel_req,
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

    signup_body = json.dumps(
        {"username": {"value": email, "kind": "email"}, "screen_hint": "signup"},
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
    if int(getattr(signup_resp, "status_code", 0) or 0) != 200:
        raise RuntimeError(
            f"authorize/continue failed: http={signup_resp.status_code} body={str(getattr(signup_resp, 'text', '') or '')[:300]}"
        )

    pwd = generate_pwd()

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

    _log("[mailbox] mailbox_direct=true action=wait_openai_code")
    code = wait_openai_code(mailbox_ref=mailbox_ref, timeout_seconds=OTP_TIMEOUT_SECONDS)
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
    birthdate = "2000-02-20"

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
        mailbox_ref=mailbox_ref,
        password=pwd,
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
    proxy_file = _data_path("proxies.txt")
    if os.path.exists(proxy_file):
        with open(proxy_file, "r", encoding="utf-8") as f:
            all_proxies = [line.strip() for line in f if line.strip() and not line.startswith("#")]
        return _partition_proxies(all_proxies)
    return []


def worker(worker_id: int) -> None:
    current_proxy: str | None = None
    assigned_at = 0.0
    cooldown_until_by_account: dict[str, float] = {}

    while True:
        proxies = load_proxies()
        proxy, assigned_at = _pick_proxy(
            proxies=proxies,
            current_proxy=current_proxy,
            assigned_at=assigned_at,
        )

        if not proxy and REGISTER_PROXY_REQUIRED:
            _stats_inc("proxy_error", err="register_proxy_required no_proxy_available", stage="stage_other")
            _log(
                f"[Protocol Worker {worker_id}] [x] register_proxy_required no_proxy_available "
                f"flow=protocol"
            )
            time.sleep(1.0)
            continue

        if proxies and not proxy:
            _stats_inc("cooldown_wait")
            time.sleep(1.0)
            continue

        current_proxy = proxy
        _stats_inc("attempt")

        _log(
            f"[Protocol Worker {worker_id}] use_proxy={(proxy or 'DIRECT')} proxy_id={(proxy or 'DIRECT')} "
            f"rotate={PROXY_ROTATE_SECONDS}s cooldown={PROXY_COOLDOWN_SECONDS}s"
        )

        try:
            reg_email, res = register_protocol(proxy)
            auth_payload = json.loads(res)

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
                    f.write(json.dumps(auth_payload, indent=2, ensure_ascii=False))

                wait_update_dir = _data_path(WAIT_UPDATE_DIRNAME)
                os.makedirs(wait_update_dir, exist_ok=True)
                try:
                    import shutil

                    shutil.copy2(auth_path, os.path.join(wait_update_dir, os.path.basename(auth_path)))
                except Exception:
                    pass

            probe_result = ProbeResult(status_code=None, note="probe_skipped", category="probe_skipped")
            try:
                account_id = _infer_account_id_from_auth(auth_payload) or ""
                now_ts = time.time()
                cd_until = float(cooldown_until_by_account.get(account_id) or 0.0) if account_id else 0.0
                if account_id and cd_until > now_ts:
                    retry_after = int(max(0, cd_until - now_ts))
                    probe_result = ProbeResult(
                        status_code=429,
                        note="local_cooldown",
                        category="cooldown_local",
                        retry_after_seconds=retry_after,
                        http_status=None,
                    )
                else:
                    probe_result = _probe_wham_one(auth_obj=auth_payload, proxy=proxy)
                    if account_id and probe_result.retry_after_seconds > 0:
                        wait_seconds = probe_result.retry_after_seconds
                        if PROBE_LOCAL_COOLDOWN_MAX_SECONDS > 0:
                            wait_seconds = min(wait_seconds, PROBE_LOCAL_COOLDOWN_MAX_SECONDS)
                        cooldown_until_by_account[account_id] = time.time() + max(0, wait_seconds)

                _append_jsonl(
                    os.path.join(_results_dir(), "probe_report.jsonl"),
                    {
                        "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                        "worker_id": worker_id,
                        "email": reg_email,
                        "account_id": _infer_account_id_from_auth(auth_payload),
                        "status_code": probe_result.status_code,
                        "probe_category": probe_result.category,
                        "probe_note": probe_result.note,
                        "retry_after_seconds": probe_result.retry_after_seconds,
                        "upstream_status": probe_result.http_status,
                    },
                )
            except Exception:
                pass

            _stats_inc("success")
            _log(
                f"[Protocol Worker {worker_id}] [✓] success email={reg_email} "
                f"probe={probe_result.status_code}/{probe_result.category}"
            )

        except Exception as e:
            cls = _classify_error(e)
            stg = _infer_stage_from_error(str(e))
            _stats_inc(cls, err=e, stage=stg)
            _log(f"[Protocol Worker {worker_id}] [x] {e}")

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
