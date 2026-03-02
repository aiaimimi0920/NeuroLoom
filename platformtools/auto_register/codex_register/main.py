from __future__ import annotations

import base64
import hashlib
import secrets
import urllib.parse
import urllib.request
import urllib.error
from dataclasses import dataclass
from typing import Any, Dict
from collections import deque
import undetected_chromedriver as uc

try:
    from curl_cffi import requests as curl_requests
except Exception:
    curl_requests = None  # type: ignore
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait, Select
from selenium.webdriver.support import expected_conditions as EC
from selenium.common.exceptions import TimeoutException
from selenium.webdriver.common.keys import Keys
from selenium.webdriver.common.action_chains import ActionChains
import time
import random
import string
import os
import re
import json
import glob
import socket
from urllib.parse import urlparse, parse_qs
from urllib import request
import tempfile
import shutil
import concurrent.futures
import threading
import socket

write_lock = threading.Lock()
driver_init_lock = threading.Lock()

# Runtime data directory (results / proxies / screenshots / logs).
# - In local dev: defaults to ./data next to this file
# - In container: set DATA_DIR=/data and mount a volume
DATA_DIR = (os.environ.get("DATA_DIR") or os.path.join(os.path.dirname(__file__), "data")).strip()
if not DATA_DIR:
    DATA_DIR = os.path.join(os.path.dirname(__file__), "data")


def _sanitize_instance_id(v: str) -> str:
    s = (v or "").strip()
    if not s:
        return "default"
    s = re.sub(r"[^a-zA-Z0-9_.-]+", "_", s)
    return s[:64] or "default"


# Multi-container: isolate results shards/state by instance, while keeping
# codex_auth/wait_update shared.
INSTANCE_ID = _sanitize_instance_id(
    os.environ.get("INSTANCE_ID")
    or os.environ.get("RESULTS_INSTANCE_ID")
    or os.environ.get("HOSTNAME")
    or socket.gethostname()
)

# Data sub dirs
# - codex_auth: per-account auth json files (one file per registered account)
# - wait_update: a copy of each auth json for downstream pickup
# - need_fix_auth / fixed_success / fixed_fail: placeholders for future flows
# - error: screenshots + tiny logs (keeps last N)
# - results: sharded jsonl outputs
CODEX_AUTH_DIRNAME = "codex_auth"
WAIT_UPDATE_DIRNAME = "wait_update"
NEED_FIX_AUTH_DIRNAME = "need_fix_auth"
FIXED_SUCCESS_DIRNAME = "fixed_success"
FIXED_FAIL_DIRNAME = "fixed_fail"
ERROR_DIRNAME = "error"
RESULTS_DIRNAME = "results"

# -----------------------------------------------------------------------------
# Probe + refill (方案A：producer 容器内同时运行探测/续杯)
# -----------------------------------------------------------------------------
# 固定的 wham/usage URL（用户要求固定用 chatgpt.com）
WHAM_USAGE_URL = "https://chatgpt.com/backend-api/wham/usage"

TARGET_POOL_SIZE = int(os.environ.get("TARGET_POOL_SIZE", "10"))
if TARGET_POOL_SIZE <= 0:
    TARGET_POOL_SIZE = 10

TRIGGER_REMAINING = int(os.environ.get("TRIGGER_REMAINING", "2"))
if TRIGGER_REMAINING < 0:
    TRIGGER_REMAINING = 0

# 当 pool_size=10 时，>=8 无效（401/429）才触发续杯；与 InfiniteRefill 客户端一致
TRIGGER_INVALID_THRESHOLD = max(0, TARGET_POOL_SIZE - TRIGGER_REMAINING)

ENABLE_PROBE = int(os.environ.get("ENABLE_PROBE", "0"))  # 1=开启；默认关闭以免影响现有注册

# 方案A 扩展：同一进程内启用“修缮者”（repairer）循环，消费 need_fix_auth/ 队列。
# 说明：该队列是“待修复的 auth json（老号登录换新 token）”。
ENABLE_REPAIRER = int(os.environ.get("ENABLE_REPAIRER", "0"))  # 1=开启；默认关闭
REPAIRER_POLL_SECONDS = float(os.environ.get("REPAIRER_POLL_SECONDS", "5"))
if REPAIRER_POLL_SECONDS < 0.2:
    REPAIRER_POLL_SECONDS = 0.2

# 测试开关：修缮流程处理后保留输入文件（不删除）。
# - 1: 测试模式启用（每个文件每次进程仅处理一次，便于重复验证）
# - 0: 正常模式（处理后按既有策略删除）
REPAIRER_TEST_KEEP_INPUT = int(os.environ.get("REPAIRER_TEST_KEEP_INPUT", "0"))

PROBE_INTERVAL_SECONDS = int(os.environ.get("PROBE_INTERVAL_SECONDS", "300"))
if PROBE_INTERVAL_SECONDS <= 0:
    PROBE_INTERVAL_SECONDS = 300

PROBE_TIMEOUT_SECONDS = int(os.environ.get("PROBE_TIMEOUT_SECONDS", "30"))
if PROBE_TIMEOUT_SECONDS <= 0:
    PROBE_TIMEOUT_SECONDS = 30

TOPUP_COOLDOWN_SECONDS = int(os.environ.get("TOPUP_COOLDOWN_SECONDS", "600"))
if TOPUP_COOLDOWN_SECONDS < 0:
    TOPUP_COOLDOWN_SECONDS = 0

PROBE_LOCAL_COOLDOWN_MAX_SECONDS = int(os.environ.get("PROBE_LOCAL_COOLDOWN_MAX_SECONDS", "1800"))
if PROBE_LOCAL_COOLDOWN_MAX_SECONDS < 0:
    PROBE_LOCAL_COOLDOWN_MAX_SECONDS = 0

# 可选：将 codex_auth 的变更同步到另一个目录（用于“配置同步/二次同步”链路）。
# - 写入新 auth：copy 到同步目录
# - 删除失效 auth：同步目录也删除
CODEX_AUTH_SYNC_DIR = (os.environ.get("CODEX_AUTH_SYNC_DIR") or "").strip()
if CODEX_AUTH_SYNC_DIR:
    try:
        CODEX_AUTH_SYNC_DIR = os.path.abspath(CODEX_AUTH_SYNC_DIR)
    except Exception:
        pass

# Refill server
REFILL_SERVER_URL = (os.environ.get("REFILL_SERVER_URL") or os.environ.get("INFINITE_REFILL_SERVER_URL") or "").strip().rstrip("/")
REFILL_UPLOAD_KEY = (os.environ.get("REFILL_UPLOAD_KEY") or os.environ.get("INFINITE_REFILL_UPLOAD_KEY") or "").strip()

# Results sharding: also write results into DATA_DIR/results/ as jsonl shards.
# Default shard size: 200 lines/file.
RESULTS_SHARD_SIZE = int(os.environ.get("RESULTS_SHARD_SIZE", "200"))
if RESULTS_SHARD_SIZE <= 0:
    RESULTS_SHARD_SIZE = 200


def _data_path(*parts: str) -> str:
    return os.path.join(DATA_DIR, *parts)


def _legacy_results_root_dir() -> str:
    """Legacy results dir (root).

    We now write to per-instance subdir under this directory.
    """

    return _data_path(RESULTS_DIRNAME)


def _results_dir() -> str:
    """Per-instance results directory to avoid cross-container write conflicts."""

    return _data_path(RESULTS_DIRNAME, INSTANCE_ID)


def _results_state_path() -> str:
    """Per-instance state to avoid cross-container write conflicts."""

    return os.path.join(_results_dir(), "results_state.json")


def _legacy_results_state_path() -> str:
    # Before multi-instance layout, state lived at DATA_DIR/results_state.json
    return _data_path("results_state.json")


def _migrate_legacy_results_layout() -> None:
    """Move legacy root shard/state into this instance dir.

    This keeps the data directory clean after we introduced results/<instance_id>/.
    """

    legacy_root = _legacy_results_root_dir()
    instance_dir = _results_dir()

    try:
        legacy_shards = [
            p
            for p in glob.glob(os.path.join(legacy_root, "results_*.jsonl"))
            if os.path.isfile(p)
        ]
    except Exception:
        legacy_shards = []

    legacy_state = _legacy_results_state_path()
    legacy_state_exists = os.path.isfile(legacy_state)

    if not legacy_shards and not legacy_state_exists:
        return

    # If instance already has shards/state, do NOT mix.
    try:
        instance_has_shards = bool(
            [
                p
                for p in glob.glob(os.path.join(instance_dir, "results_*.jsonl"))
                if os.path.isfile(p)
            ]
        )
    except Exception:
        instance_has_shards = False

    if instance_has_shards or os.path.isfile(_results_state_path()):
        return

    try:
        os.makedirs(instance_dir, exist_ok=True)
    except Exception:
        pass

    for p in legacy_shards:
        try:
            shutil.move(p, os.path.join(instance_dir, os.path.basename(p)))
        except Exception:
            pass

    if legacy_state_exists:
        try:
            shutil.move(legacy_state, _results_state_path())
        except Exception:
            pass


def _read_json(path: str) -> dict:
    try:
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f) or {}
    except FileNotFoundError:
        return {}
    except Exception:
        return {}


def _write_json(path: str, obj: dict) -> None:
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(obj, f, ensure_ascii=False, separators=(",", ":"))
    os.replace(tmp, path)


def _infer_results_state() -> dict:
    """Infer last shard id + current line count from existing shard files.

    Used when results_state.json is missing.
    """
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

        # If last shard is already full, next write should go to a new shard.
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
    """Append a jsonl line to shard (200 lines/shard by default).

    Must be called under write_lock.

    NOTE:
      We do not maintain or write any monolith results.jsonl.
    """
    try:
        os.makedirs(DATA_DIR, exist_ok=True)
    except Exception:
        pass
    try:
        os.makedirs(_results_dir(), exist_ok=True)
    except Exception:
        pass

    # Normalize to exactly one trailing newline.
    payload = (line or "").rstrip("\r\n") + "\n"

    # shard append
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




def _keep_last_n_files(pattern: str, *, keep: int = 10) -> None:
    if KEEP_ERROR_ARTIFACTS:
        return
    try:
        items = [p for p in glob.glob(pattern) if os.path.isfile(p)]
        items.sort(key=lambda p: os.path.getmtime(p), reverse=True)
        for p in items[keep:]:
            try:
                os.remove(p)
            except Exception:
                pass
    except Exception:
        pass


# Dump page source/body text to help debug missing CSS/JS scenarios.
# Retained per-kind (last 10) to avoid disk growth.
DUMP_PAGE_BODY = (os.environ.get("DUMP_PAGE_BODY", "1") or "").strip().lower() not in ("0", "false", "no")
DEBUG_TRACE = (os.environ.get("DEBUG_TRACE", "1") or "").strip().lower() not in ("0", "false", "no")

# IMPORTANT: screenshots/html dumps are critical for debugging; do NOT delete by default.
KEEP_ERROR_ARTIFACTS = (os.environ.get("KEEP_ERROR_ARTIFACTS", "1") or "").strip().lower() not in ("0", "false", "no")

# Click-snapshot debug mode:
#   DEBUG_CLICK_SNAP=1 -> save screenshot + html/text before/after each click helper call.
DEBUG_CLICK_SNAP = (os.environ.get("DEBUG_CLICK_SNAP", "0") or "").strip().lower() in ("1", "true", "yes", "on")

# Limit noise when click-snap is enabled; keep 1 to snapshot every click, >1 to sample.
try:
    CLICK_SNAP_EVERY = max(1, int((os.environ.get("CLICK_SNAP_EVERY", "1") or "1").strip()))
except Exception:
    CLICK_SNAP_EVERY = 1

_CLICK_SNAP_COUNTER = 0


def _dbg(step: str, msg: str = "", *, driver=None) -> None:
    if not DEBUG_TRACE:
        return
    ts = time.strftime("%H:%M:%S", time.localtime(int(time.time())))
    url = ""
    title = ""
    try:
        if driver is not None:
            url = str(getattr(driver, "current_url", "") or "")
            title = str(getattr(driver, "title", "") or "")
    except Exception:
        url = ""
        title = ""

    tid = ""
    try:
        tid = threading.current_thread().name
    except Exception:
        tid = ""

    head = f"[{ts}]"
    if tid:
        head += f" [{tid}]"
    head += f" [{step}]"

    if msg:
        head += f" {msg}"
    if url:
        head += f" | url={url}"
    if title:
        head += f" | title={title}"
    print(head)


def _dump_page_body(*, driver, kind: str, message: str = "") -> None:
    if not DUMP_PAGE_BODY:
        return

    ts = int(time.time())
    err_dir = _data_path(ERROR_DIRNAME, INSTANCE_ID)

    try:
        os.makedirs(err_dir, exist_ok=True)
    except Exception:
        pass

    url = ""
    title = ""
    try:
        url = str(getattr(driver, "current_url", "") or "")
    except Exception:
        url = ""
    try:
        title = str(getattr(driver, "title", "") or "")
    except Exception:
        title = ""

    # also return a snippet to stdout for immediate debugging
    body_text_snippet = ""

    try:
        meta = os.path.join(err_dir, f"page_{kind}_{ts}.meta.txt")
        with open(meta, "w", encoding="utf-8") as f:
            if url:
                f.write(f"url={url}\n")
            if title:
                f.write(f"title={title}\n")
            if message:
                f.write(message + "\n")
        _keep_last_n_files(os.path.join(err_dir, f"page_{kind}_*.meta.txt"), keep=10)
    except Exception:
        pass

    try:
        html = os.path.join(err_dir, f"page_{kind}_{ts}.html")
        with open(html, "w", encoding="utf-8") as f:
            f.write(str(getattr(driver, "page_source", "") or ""))
        _keep_last_n_files(os.path.join(err_dir, f"page_{kind}_*.html"), keep=10)
    except Exception:
        pass

    try:
        body_html = driver.execute_script("return document && document.body ? document.body.outerHTML : '';")
        p = os.path.join(err_dir, f"body_{kind}_{ts}.html")
        with open(p, "w", encoding="utf-8") as f:
            f.write(str(body_html or ""))
        _keep_last_n_files(os.path.join(err_dir, f"body_{kind}_*.html"), keep=10)
    except Exception:
        pass

    try:
        body_text = driver.execute_script("return document && document.body ? document.body.innerText : '';")
        p = os.path.join(err_dir, f"text_{kind}_{ts}.txt")
        bt = str(body_text or "")
        with open(p, "w", encoding="utf-8") as f:
            f.write(bt)
        _keep_last_n_files(os.path.join(err_dir, f"text_{kind}_*.txt"), keep=10)

        # keep a short snippet for console
        body_text_snippet = bt[:4000]
    except Exception:
        pass

    if DEBUG_TRACE:
        print(f"[dump] kind={kind} dir={err_dir}")
        if body_text_snippet:
            print("[dump] body.innerText (first 4000 chars):\n" + body_text_snippet)


def _save_error_artifacts(*, driver, kind: str, message: str = "") -> None:
    """Save screenshot + a tiny text log, and keep only last 10 of each kind."""
    ts = int(time.time())

    # Per-instance error dir to avoid cross-container clobbering + retention races.
    err_dir = _data_path(ERROR_DIRNAME, INSTANCE_ID)

    try:
        os.makedirs(err_dir, exist_ok=True)
    except Exception:
        pass

    try:
        # screenshot
        png = os.path.join(err_dir, f"error_{kind}_{ts}.png")
        driver.save_screenshot(png)
        _keep_last_n_files(os.path.join(err_dir, f"error_{kind}_*.png"), keep=10)
    except Exception:
        pass

    try:
        # text log (url + msg)
        txt = os.path.join(err_dir, f"error_{kind}_{ts}.txt")
        url = ""
        try:
            url = str(getattr(driver, "current_url", "") or "")
        except Exception:
            url = ""
        with open(txt, "w", encoding="utf-8") as f:
            if url:
                f.write(f"url={url}\n")
            if message:
                f.write(message + "\n")
        _keep_last_n_files(os.path.join(err_dir, f"error_{kind}_*.txt"), keep=10)
    except Exception:
        pass


def _click_with_debug(driver, el, *, tag: str, note: str = "") -> None:
    """Click helper with optional pre/post snapshots for debug diagnosis."""

    global _CLICK_SNAP_COUNTER

    if DEBUG_CLICK_SNAP:
        _CLICK_SNAP_COUNTER += 1
        do_snap = (_CLICK_SNAP_COUNTER % CLICK_SNAP_EVERY == 0)
    else:
        do_snap = False

    safe_tag = re.sub(r"[^a-zA-Z0-9_.-]+", "_", str(tag or "click"))[:64] or "click"

    if do_snap:
        try:
            _dbg("click", f"before tag={safe_tag} note={note}", driver=driver)
            _save_error_artifacts(driver=driver, kind=f"click_before_{safe_tag}", message=note)
            _dump_page_body(driver=driver, kind=f"click_before_{safe_tag}", message=note)
        except Exception:
            pass

    try:
        el.click()
    except Exception:
        driver.execute_script("arguments[0].click();", el)

    if do_snap:
        try:
            _dbg("click", f"after tag={safe_tag} note={note}", driver=driver)
            _save_error_artifacts(driver=driver, kind=f"click_after_{safe_tag}", message=note)
            _dump_page_body(driver=driver, kind=f"click_after_{safe_tag}", message=note)
        except Exception:
            pass
# -----------------------------------------------------------------------------
# Mail provider abstraction (multi-provider)
# -----------------------------------------------------------------------------
# This project supports switching mailbox providers.
#
# Providers:
# - mailcreate (default): our self-hosted Cloudflare temp-mail service
# - gptmail: public GPTMail API (https://mail.chatgpt.org.uk)
#
# Configure via environment variables:
# - MAILBOX_PROVIDER         (default: auto) values: auto | mailcreate | gptmail
#
# MailCreate provider env:
# - MAILCREATE_BASE_URL      (default: https://mail.aiaimimi.com)
# - MAILCREATE_CUSTOM_AUTH   (header x-custom-auth; required unless server sets DISABLE_CUSTOM_AUTH_CHECK=true)
# - MAILCREATE_DOMAIN        (optional; if empty/omitted, server picks from DEFAULT_DOMAINS)
#
# GPTMail provider env:
# - GPTMAIL_BASE_URL         (default: https://mail.chatgpt.org.uk)
# - GPTMAIL_API_KEY          (required; header X-API-Key)
# - GPTMAIL_PREFIX           (optional; email prefix)
# - GPTMAIL_DOMAIN           (optional; if omitted GPTMail picks random active domain)
#
# Provider implementation lives at:
#   [`mailbox_provider.py`](../../mailcreate/client/mailbox_provider.py:1)
#
import sys

# Ensure repo root is importable so we can `import platformtools...` even when
# this file is executed via a script path.
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

_PLAT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
_MAILCREATE_CLIENT_DIR = os.path.join(_PLAT_DIR, "mailcreate", "client")
if _MAILCREATE_CLIENT_DIR not in sys.path:
    sys.path.insert(0, _MAILCREATE_CLIENT_DIR)

from mailbox_provider import Mailbox, create_mailbox, wait_openai_code as wait_openai_code_by_provider  # type: ignore

MAILBOX_PROVIDER = os.environ.get("MAILBOX_PROVIDER", "auto").strip().lower()

# Registration flow mode:
# - browser: existing Selenium flow
# - protocol: HTTP protocol flow based on curl_cffi session + OAuth callback exchange
REGISTER_FLOW_MODE = (os.environ.get("REGISTER_FLOW_MODE", "browser") or "browser").strip().lower()
if REGISTER_FLOW_MODE not in ("browser", "protocol"):
    REGISTER_FLOW_MODE = "browser"

# 注册主流程是否强制要求代理。
# - 1: 无代理直接判定本轮失败（默认）
# - 0: 允许直连兜底（不建议）
REGISTER_PROXY_REQUIRED = (os.environ.get("REGISTER_PROXY_REQUIRED", "1") or "1").strip().lower() not in (
    "0",
    "false",
    "no",
)

# Protocol-flow knobs
PROTOCOL_IMPERSONATE = (os.environ.get("PROTOCOL_IMPERSONATE", "chrome") or "chrome").strip() or "chrome"
PROTOCOL_TIMEOUT_SECONDS = int(os.environ.get("PROTOCOL_TIMEOUT_SECONDS", "30") or "30")
if PROTOCOL_TIMEOUT_SECONDS <= 0:
    PROTOCOL_TIMEOUT_SECONDS = 30

PROTOCOL_CHECK_GEO = (os.environ.get("PROTOCOL_CHECK_GEO", "1") or "1").strip().lower() not in ("0", "false", "no")
PROTOCOL_BLOCKED_LOCS = {
    x.strip().upper()
    for x in (os.environ.get("PROTOCOL_BLOCKED_LOCS", "CN,HK") or "CN,HK").split(",")
    if x.strip()
}


def _load_json_config(path: str) -> dict:
    try:
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f)
    except FileNotFoundError:
        return {}


# MailCreate provider config
# Priority order:
#   1) Environment variables
#   2) Optional local config file (NOT committed): data/mailcreate_config.json
MAILCREATE_CONFIG_FILE = os.environ.get(
    "MAILCREATE_CONFIG_FILE",
    os.path.join(DATA_DIR, "mailcreate_config.json"),
).strip()
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

# IMPORTANT: Email Routing catch-all is zone-level.
# If you configure multiple domains on the MailCreate Worker (env `DOMAINS`),
# you can omit MAILCREATE_DOMAIN to let the server pick a random domain.
# (This reduces the risk of single-domain bans in downstream signup flows.)
MAILCREATE_DOMAIN = (
    os.environ.get("MAILCREATE_DOMAIN")
    or _PLATFORMTOOLS_DEV_VARS.get("MAILCREATE_DOMAIN")
    or str(_MAILCREATE_CFG.get("MAILCREATE_DOMAIN") or "")
).strip()

# GPTMail provider config
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
# Multi-key support: if GPTMAIL_API_KEY is empty, load keys from this file.
# Format: one key per line, supports '# [EXHAUSTED]' comments.
GPTMAIL_KEYS_FILE = os.environ.get(
    "GPTMAIL_KEYS_FILE",
    os.path.join(DATA_DIR, "gptmail_keys.txt"),
).strip()
GPTMAIL_PREFIX = os.environ.get("GPTMAIL_PREFIX", "").strip() or None
GPTMAIL_DOMAIN = os.environ.get("GPTMAIL_DOMAIN", "").strip() or None


def create_temp_mailbox() -> tuple[str, str]:
    """Create a new temp mailbox.

    Returns:
      (email_address, mailbox_ref)

    mailbox_ref semantics:
    - mailcreate: address_jwt
    - gptmail: email
    """

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
    return mb.email, mb.ref


def wait_openai_code(*, address_jwt: str, timeout_seconds: int = 180) -> str:
    """Wait for OpenAI 6-digit verification code.

    NOTE:
      `address_jwt` is kept for backward-compatibility naming.
      It actually means `mailbox_ref` in multi-provider mode.
    """

    ref = str(address_jwt or "").strip()
    ref_prefix = ref.split(":", 1)[0] if ":" in ref else "unknown"
    print(
        f"[mailbox] wait_openai_code start provider={MAILBOX_PROVIDER} ref_prefix={ref_prefix} "
        f"mailcreate_auth_set={bool(MAILCREATE_CUSTOM_AUTH)} gptmail_api_key_set={bool(GPTMAIL_API_KEY)}"
    )

    try:
        code = wait_openai_code_by_provider(
            provider=MAILBOX_PROVIDER,
            mailbox_ref=address_jwt,
            mailcreate_base_url=MAILCREATE_BASE_URL,
            mailcreate_custom_auth=MAILCREATE_CUSTOM_AUTH,
            gptmail_base_url=GPTMAIL_BASE_URL,
            gptmail_api_key=GPTMAIL_API_KEY,
            gptmail_keys_file=GPTMAIL_KEYS_FILE,
            timeout_seconds=timeout_seconds,
        )
    except Exception as e:
        print(
            f"[mailbox] wait_openai_code fail ref_prefix={ref_prefix} "
            f"err_type={type(e).__name__} err={e}"
        )
        raise

    print(f"[mailbox] wait_openai_code ok ref_prefix={ref_prefix} code_len={len(str(code or ''))}")
    return code


def post(url: str, body: str, header: dict, proxy: str | None=None) -> tuple[str,dict]:
    data = body.encode("utf-8")
    req = urllib.request.Request(url, data=data, headers=header, method="POST") 
    with get_opener(proxy).open(req) as resp: 
        resp_text = resp.read().decode("utf-8")
        resp_headers = dict(resp.headers)
        return resp_text, resp_headers

def put(url: str, body: str, header: dict, proxy: str | None=None) -> tuple[str,dict]:
    data = body.encode("utf-8")
    req = urllib.request.Request(url, data=data, headers=header, method="PUT") 
    with get_opener(proxy).open(req) as resp: 
        resp_text = resp.read().decode("utf-8")
        resp_headers = dict(resp.headers)
        return resp_text, resp_headers

def get(url: str, headers: dict | None=None, proxy: str | None=None) -> tuple[str, dict]:
    for i in range(5):
        try:
            req = urllib.request.Request(url, headers = headers or {})
            with get_opener(proxy).open(req) as response:
                resp_text = response.read().decode("utf-8")
                resp_headers = dict(response.getheaders())
                return resp_text, resp_headers
        except urllib.error.HTTPError as e:
            if e.code in (401, 429):
                raise # immediately bubble up for API key rotation
            delay = random.uniform(5, 10) + (i * 2)
            print(f"GET Request HTTPError: {e.code} for {url} - Retrying in {delay:.1f}s")
            time.sleep(delay)
        except Exception as e:
            delay = random.uniform(5, 10) + (i * 2)
            print(f"GET Request error: {e} - Retrying in {delay:.1f}s")
            time.sleep(delay)
    raise RuntimeError(f"Failed to GET {url} after retries")


# -----------------------------------------------------------------------------
# Probe (wham/usage) + report + refill/topup (方案A)
# -----------------------------------------------------------------------------

def _utc_now_iso() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(int(time.time())))


def _sha256_hex_str(s: str) -> str:
    return hashlib.sha256((s or "").encode("utf-8")).hexdigest()


def _infer_account_id_from_auth(auth_obj: Any) -> str | None:
    if not isinstance(auth_obj, dict):
        return None

    v = str(auth_obj.get("account_id") or "").strip()
    if v:
        return v

    # fallback: nested claims
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
    # 对齐 new-api 的实现：[`FetchCodexWhamUsage()`](references/new-api/service/codex_wham_usage.go:11)
    return {
        "Authorization": f"Bearer {access_token}",
        "chatgpt-account-id": account_id,
        "Accept": "application/json",
        "originator": "codex_cli_rs",
    }


@dataclass
class ProbeResult:
    status_code: int | None
    note: str
    category: str
    retry_after_seconds: int = 0
    http_status: int | None = None


def _parse_retry_after_seconds_from_error_body(*, http_status: int, raw_body: str, now_ts: float | None = None) -> int:
    """Best-effort parse for 429 retry wait.

    Compatible with CLIProxyAPI's usage_limit_reached style:
    - error.resets_at (unix seconds)
    - error.resets_in_seconds
    """

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
    """Best-effort 判定：wham/usage 表示当前不可用/冷却。

    说明：你之前抓到的 200 样例里包含 rate_limit.allowed/limit_reached/used_percent。
    这里尽量兼容字段变化。
    """

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

        # primary_window.used_percent == 100
        pw = rl.get("primary_window")
        if isinstance(pw, dict):
            try:
                used_percent = pw.get("used_percent")
                if used_percent is not None and float(used_percent) >= 100:
                    return True
            except Exception:
                pass

    # 兜底：常见字段名
    for k in ("allowed", "limit_reached", "is_available"):
        if k in obj and obj.get(k) in (False, 0):
            return True

    return False


def _probe_wham_one(*, auth_obj: Any, proxy: str | None = None) -> ProbeResult:
    """Probe wham usage with structured outcome.

    status_code mapping:
      - ok => 200
      - quota0/cooldown => 429
      - invalid => 401
    """

    account_id = _infer_account_id_from_auth(auth_obj)
    access_token = _infer_access_token_from_auth(auth_obj)
    if not account_id or not access_token:
        return ProbeResult(status_code=None, note="missing account_id/access_token", category="invalid_input")

    headers = _wham_headers(access_token=access_token, account_id=account_id)

    try:
        raw, _hdr = get(WHAM_USAGE_URL, headers=headers, proxy=proxy)
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
            http_status=200,
        )

    return ProbeResult(status_code=200, note="ok", category="ok", http_status=200)


def _post_json_simple(*, url: str, headers: dict[str, str], payload: Any, timeout: int = 30) -> tuple[int, str]:
    data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    req = urllib.request.Request(url, data=data, headers=headers, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            status = int(getattr(resp, "status", 200))
            text = resp.read().decode("utf-8", errors="replace")
            return status, text
    except urllib.error.HTTPError as e:
        try:
            body = e.read().decode("utf-8", errors="replace")
        except Exception:
            body = str(e)
        return int(getattr(e, "code", 0) or 0), body


def _refill_url(path: str) -> str:
    base = (REFILL_SERVER_URL or "").strip().rstrip("/")
    if not base:
        return ""
    return base + path


def _report_auth_repair_failed(*, account_id: str, note: str = "auth_fix_failed") -> tuple[bool, int, str]:
    """Report repair failed.

    Single endpoint (no legacy fallback):
      POST /v1/auth/repairs/submit-failed
      body: {account_id, note}

    Requires X-Upload-Key.
    """

    base = (REFILL_SERVER_URL or "").strip().rstrip("/")
    key = (REFILL_UPLOAD_KEY or "").strip()
    if not base or not key:
        return False, 0, "missing REFILL_SERVER_URL/REFILL_UPLOAD_KEY"

    headers = {"X-Upload-Key": key, "Content-Type": "application/json"}

    url = base + "/v1/auth/repairs/submit-failed"
    st, tx = _post_json_simple(url=url, headers=headers, payload={"account_id": account_id, "note": note}, timeout=30)
    if 200 <= st < 300:
        try:
            obj = json.loads(tx) if tx else {}
        except Exception:
            obj = {}
        if isinstance(obj, dict) and obj.get("ok") is True:
            return True, st, tx[:800]

    return False, st, tx[:800]


def _report_probe_to_server(*, reports: list[dict[str, Any]]) -> None:
    if not REFILL_SERVER_URL or not REFILL_UPLOAD_KEY:
        return
    if not reports:
        return

    url = _refill_url("/v1/probe-report")
    if not url:
        return

    headers = {
        "Content-Type": "application/json",
        "X-Upload-Key": REFILL_UPLOAD_KEY,
    }

    status, text = _post_json_simple(url=url, headers=headers, payload={"reports": reports}, timeout=PROBE_TIMEOUT_SECONDS)
    if not (200 <= status < 300):
        print(f"[probe] probe-report failed: http={status} resp={text[:300]}")


def _download_json_from_url(*, url: str, timeout: int = 30) -> Any | None:
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            raw = resp.read().decode("utf-8", errors="replace")
        return json.loads(raw) if raw else None
    except Exception as e:
        print(f"[probe] download topup json failed: url={url[:180]} err={e}")
        return None


def _topup_from_server(*, reports: list[dict[str, Any]]) -> list[dict[str, Any]]:
    if not REFILL_SERVER_URL or not REFILL_UPLOAD_KEY:
        return []

    url = _refill_url("/v1/refill/topup")
    if not url:
        return []

    headers = {
        "Content-Type": "application/json",
        "X-Upload-Key": REFILL_UPLOAD_KEY,
    }

    account_ids = [
        str(it.get("account_id") or "").strip()
        for it in reports
        if isinstance(it, dict) and str(it.get("account_id") or "").strip()
    ]

    payload = {
        "target_pool_size": TARGET_POOL_SIZE,
        "reports": reports,
        "account_ids": account_ids,
    }

    status, text = _post_json_simple(url=url, headers=headers, payload=payload, timeout=PROBE_TIMEOUT_SECONDS)
    if not (200 <= status < 300):
        print(f"[probe] refill/topup failed: http={status} resp={text[:300]}")
        return []

    try:
        obj = json.loads(text) if text else {}
    except Exception:
        obj = {}

    if obj.get("ok") is not True:
        print(f"[probe] refill/topup not ok: resp={text[:300]}")
        return []

    items = obj.get("accounts")
    if not isinstance(items, list):
        return []

    out: list[dict[str, Any]] = []
    for it in items:
        if not isinstance(it, dict):
            continue

        # v2: 通过短时签名链接下载账号 JSON
        download_url = str(it.get("download_url") or "").strip()
        if not download_url:
            continue
        auth = _download_json_from_url(url=download_url, timeout=PROBE_TIMEOUT_SECONDS)
        if auth is None:
            continue
        out.append({"file_name": it.get("file_name"), "auth_json": auth})
    return out


def _list_recent_auth_files(*, limit: int) -> list[str]:
    codex_auth_dir = _data_path(CODEX_AUTH_DIRNAME)
    try:
        names = [
            os.path.join(codex_auth_dir, n)
            for n in os.listdir(codex_auth_dir)
            if n.lower().endswith(".json") and os.path.isfile(os.path.join(codex_auth_dir, n))
        ]
    except Exception:
        return []

    # newest first
    try:
        names.sort(key=lambda p: os.path.getmtime(p), reverse=True)
    except Exception:
        pass

    if limit > 0:
        return names[:limit]
    return names


def _sync_codex_auth_copy(*, src_path: str) -> None:
    """Optional: mirror codex_auth changes into CODEX_AUTH_SYNC_DIR."""

    if not CODEX_AUTH_SYNC_DIR:
        return

    try:
        os.makedirs(CODEX_AUTH_SYNC_DIR, exist_ok=True)
    except Exception:
        return

    try:
        dst = os.path.join(CODEX_AUTH_SYNC_DIR, os.path.basename(src_path))
        shutil.copy2(src_path, dst)
    except Exception:
        pass


def _sync_codex_auth_delete(*, filename: str) -> None:
    if not CODEX_AUTH_SYNC_DIR:
        return
    try:
        p = os.path.join(CODEX_AUTH_SYNC_DIR, filename)
        if os.path.isfile(p):
            os.remove(p)
    except Exception:
        pass


def _write_auth_obj_to_codex_auth(*, auth_obj: Any, prefix: str = "topup") -> str | None:
    if not isinstance(auth_obj, (dict, list, str, int, float, bool)) and auth_obj is not None:
        # still try dumpable objects
        pass

    ts_ms = int(time.time() * 1000)
    rand = secrets.token_hex(3)

    acc_id = _infer_account_id_from_auth(auth_obj) or "unknown"
    safe_acc = re.sub(r"[^a-zA-Z0-9_.-]+", "_", acc_id)[:64] or "unknown"

    codex_auth_dir = _data_path(CODEX_AUTH_DIRNAME)
    os.makedirs(codex_auth_dir, exist_ok=True)

    path = os.path.join(codex_auth_dir, f"codex-{prefix}-{safe_acc}-{INSTANCE_ID}-{ts_ms}-{rand}.json")
    try:
        with open(path, "w", encoding="utf-8") as f:
            f.write(json.dumps(auth_obj, ensure_ascii=False, indent=2))
        _sync_codex_auth_copy(src_path=path)
        return path
    except Exception as e:
        print(f"[probe] write topup auth failed: {e}")
        return None


def _probe_loop() -> None:
    if PROBE_INTERVAL_SECONDS < 5:
        interval = 5
    else:
        interval = PROBE_INTERVAL_SECONDS

    max_files = int(os.environ.get("PROBE_MAX_FILES", str(TARGET_POOL_SIZE)))
    if max_files <= 0:
        max_files = TARGET_POOL_SIZE

    probe_proxy = (os.environ.get("PROBE_PROXY") or "").strip() or None

    copy_topup_to_wait_update = int(os.environ.get("TOPUP_COPY_TO_WAIT_UPDATE", "0"))

    last_topup_at = 0.0
    cooldown_until_by_account: dict[str, float] = {}

    print(
        f"[probe] enabled=1 interval={interval}s max_files={max_files} target_pool={TARGET_POOL_SIZE} invalid_threshold={TRIGGER_INVALID_THRESHOLD}"
    )

    while True:
        try:
            paths = _list_recent_auth_files(limit=max_files)
            if not paths:
                time.sleep(interval)
                continue

            reports_for_probe: list[dict[str, Any]] = []
            reports_for_topup: list[dict[str, Any]] = []
            invalid_paths: list[tuple[str, str]] = []  # (file_name, abs_path)
            invalid_like = 0

            for p in paths:
                name = os.path.basename(p)
                try:
                    auth_obj = _read_json(p)
                except Exception:
                    continue

                account_id = _infer_account_id_from_auth(auth_obj)
                if not account_id:
                    continue

                email_hash = _sha256_hex_str(account_id)

                now_ts = time.time()
                cd_until = float(cooldown_until_by_account.get(account_id) or 0.0)
                if cd_until > now_ts:
                    retry_after = int(max(0, cd_until - now_ts))
                    result = ProbeResult(
                        status_code=429,
                        note="local_cooldown",
                        category="cooldown_local",
                        retry_after_seconds=retry_after,
                        http_status=None,
                    )
                else:
                    result = _probe_wham_one(auth_obj=auth_obj, proxy=probe_proxy)
                    if result.retry_after_seconds > 0:
                        wait_seconds = result.retry_after_seconds
                        if PROBE_LOCAL_COOLDOWN_MAX_SECONDS > 0:
                            wait_seconds = min(wait_seconds, PROBE_LOCAL_COOLDOWN_MAX_SECONDS)
                        cooldown_until_by_account[account_id] = time.time() + max(0, wait_seconds)

                status_code = result.status_code
                note = result.note

                it: dict[str, Any] = {
                    "email_hash": email_hash,
                    "account_id": account_id,
                    "probed_at": _utc_now_iso(),
                    "probe_category": result.category,
                    "probe_note": result.note,
                }
                if status_code is not None:
                    it["status_code"] = int(status_code)
                if result.retry_after_seconds > 0:
                    it["retry_after_seconds"] = int(result.retry_after_seconds)
                if result.http_status is not None:
                    it["upstream_status"] = int(result.http_status)

                # report probe (no file_name field)
                reports_for_probe.append(it)

                # topup wants file_name for audit only
                it2 = dict(it)
                it2["file_name"] = name
                reports_for_topup.append(it2)

                if status_code in (401, 429):
                    invalid_like += 1
                    if result.category != "cooldown_local":
                        invalid_paths.append((name, p))

                # minimal local log
                if status_code is not None and status_code != 200:
                    print(f"[probe] {name} -> {status_code} ({note}) cat={result.category} retry={result.retry_after_seconds}s")

            healthy_count = sum(1 for r in reports_for_probe if int(r.get("status_code") or 0) == 200)
            pool_size = min(TARGET_POOL_SIZE, healthy_count)
            need_topup = pool_size < TARGET_POOL_SIZE

            if need_topup and invalid_like > 0:
                reports_bad = [
                    r
                    for r in reports_for_probe
                    if int(r.get("status_code") or 0) in (401, 429)
                ]
                _report_probe_to_server(reports=reports_bad)

            now_ts = time.time()
            if need_topup and (now_ts - last_topup_at >= TOPUP_COOLDOWN_SECONDS):
                print(f"[probe] triggering topup: pool={pool_size} invalid_like={invalid_like} probed={len(reports_for_probe)}")
                got = _topup_from_server(reports=reports_for_topup)

                if got:
                    # 回灌 + 删除失效：拿到 N 个 replacement，则删除 N 个失效文件。
                    # 说明：服务端也会基于同逻辑校验并决定下发数量；本地按下发数量删除。
                    del_count = min(len(got), len(invalid_paths))

                    with write_lock:
                        # 1) 写入 replacement
                        for item in got:
                            auth_json = item.get("auth_json")
                            if auth_json is None:
                                continue
                            out_path = _write_auth_obj_to_codex_auth(auth_obj=auth_json, prefix="topup")
                            if out_path and copy_topup_to_wait_update == 1:
                                try:
                                    wait_update_dir = _data_path(WAIT_UPDATE_DIRNAME)
                                    os.makedirs(wait_update_dir, exist_ok=True)
                                    shutil.copy2(out_path, os.path.join(wait_update_dir, os.path.basename(out_path)))
                                except Exception:
                                    pass

                        # 2) 删除被替换的失效文件（及同步目录）
                        for (fname, fpath) in invalid_paths[:del_count]:
                            try:
                                # 仅允许删除 codex_auth 目录下的文件
                                codex_auth_dir = os.path.abspath(_data_path(CODEX_AUTH_DIRNAME))
                                ap = os.path.abspath(fpath)
                                if ap.startswith(codex_auth_dir + os.sep) and os.path.isfile(ap):
                                    os.remove(ap)
                                    _sync_codex_auth_delete(filename=fname)
                            except Exception:
                                pass

                    print(f"[probe] topup received={len(got)} deleted_invalid={del_count}")
                else:
                    print("[probe] topup received=0")

                last_topup_at = now_ts

        except Exception as e:
            print(f"[probe] loop error: {e}")

        time.sleep(interval)

def get_email(proxy: str | None = None) -> tuple[str, str]:
    """Compatibility wrapper.

    Returns:
      (email, address_jwt)

    Note:
      `proxy` is ignored here because the mailbox API call is performed by our
      MailCreate client (direct HTTPS) and does not use the Selenium proxy.
    """

    _ = proxy
    print("[mailbox] mailbox_direct=true action=create_temp_mailbox")
    return create_temp_mailbox()


def get_oai_code(*, address_jwt: str, timeout_seconds: int = 180, proxy: str | None = None) -> str:
    """Compatibility wrapper.

    Note:
      `proxy` is ignored here for the same reason as [`get_email()`](platformtools/auto_register/codex_register/main.py:1).
    """

    _ = proxy
    print("[mailbox] mailbox_direct=true action=wait_openai_code")
    return wait_openai_code(address_jwt=address_jwt, timeout_seconds=timeout_seconds)


def _proxy_dict_for_requests(proxy: str | None) -> dict[str, str] | None:
    p = str(proxy or "").strip()
    if not p:
        return None
    return {"http": p, "https": p}


def _decode_cookie_json_prefix(raw_cookie: str) -> dict[str, Any]:
    """Decode first JWT-like segment from cookie and parse as JSON."""

    v = str(raw_cookie or "").strip()
    if not v:
        return {}

    head = v.split(".", 1)[0]
    # OpenAI cookies may use standard base64 (not always urlsafe/padded).
    for use_urlsafe in (False, True):
        try:
            pad = "=" * ((4 - (len(head) % 4)) % 4)
            blob = (head + pad).encode("ascii")
            decoded = (
                base64.urlsafe_b64decode(blob)
                if use_urlsafe
                else base64.b64decode(blob)
            )
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

        # 非重定向且未拿到 callback
        break

    raise RuntimeError("protocol flow did not reach localhost callback")


def register_protocol(proxy: str | None = None) -> tuple[str, str]:
    """Protocol-based register flow (no browser automation).

    Mirrors the reference flow using curl_cffi session + auth/openai endpoints,
    then reuses submit_callback_url for token exchange/output JSON format.
    """

    if curl_requests is None:
        raise RuntimeError("protocol flow requires curl_cffi; please install curl_cffi")

    proxies = _proxy_dict_for_requests(proxy)
    sess = curl_requests.Session(
        proxies=proxies,
        impersonate=PROTOCOL_IMPERSONATE,
    )

    if PROTOCOL_CHECK_GEO:
        try:
            trace_resp = sess.get("https://cloudflare.com/cdn-cgi/trace", timeout=10)
            trace_txt = str(getattr(trace_resp, "text", "") or "")
            loc_m = re.search(r"^loc=(.+)$", trace_txt, re.MULTILINE)
            ip_m = re.search(r"^ip=(.+)$", trace_txt, re.MULTILINE)
            loc = (loc_m.group(1) if loc_m else "").strip().upper()
            ip = (ip_m.group(1) if ip_m else "").strip()
            if loc:
                print(f"[protocol] trace loc={loc} ip={ip}")
            if loc and loc in PROTOCOL_BLOCKED_LOCS:
                raise RuntimeError(f"protocol flow blocked geo loc={loc}")
        except RuntimeError:
            raise
        except Exception as e:
            print(f"[protocol] trace check failed: {e}")

    email, address_jwt = get_email(proxy)
    print("Email obtained:", email)

    oauth = generate_oauth_url()
    print("OAuth URL:", oauth.auth_url)

    # Hit authorize first to establish cookies (oai-did etc.)
    sess.get(oauth.auth_url, timeout=PROTOCOL_TIMEOUT_SECONDS)

    did = str(sess.cookies.get("oai-did") or "").strip()
    if not did:
        raise RuntimeError("protocol flow missing oai-did cookie")

    # Sentinel token
    sentinel_req = json.dumps({"p": "", "id": did, "flow": "authorize_continue"}, ensure_ascii=False, separators=(",", ":"))
    sen_resp = curl_requests.post(
        "https://sentinel.openai.com/backend-api/sentinel/req",
        headers={
            "origin": "https://sentinel.openai.com",
            "referer": "https://sentinel.openai.com/backend-api/sentinel/frame.html?sv=20260219f9f6",
            "content-type": "text/plain;charset=UTF-8",
        },
        data=sentinel_req,
        proxies=proxies,
        impersonate=PROTOCOL_IMPERSONATE,
        timeout=PROTOCOL_TIMEOUT_SECONDS,
    )
    if int(getattr(sen_resp, "status_code", 0) or 0) != 200:
        raise RuntimeError(f"sentinel req failed: http={sen_resp.status_code}")

    try:
        sentinel_token = str((sen_resp.json() or {}).get("token") or "").strip()
    except Exception:
        sentinel_token = ""
    if not sentinel_token:
        raise RuntimeError("sentinel token missing")

    sentinel_header_value = json.dumps(
        {
            "p": "",
            "t": "",
            "c": sentinel_token,
            "id": did,
            "flow": "authorize_continue",
        },
        ensure_ascii=False,
        separators=(",", ":"),
    )

    signup_body = json.dumps(
        {
            "username": {"value": email, "kind": "email"},
            "screen_hint": "signup",
        },
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
        raise RuntimeError(f"authorize/continue failed: http={signup_resp.status_code} body={str(getattr(signup_resp, 'text', '') or '')[:300]}")

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
        raise RuntimeError(f"send-otp failed: http={otp_send_resp.status_code} body={str(getattr(otp_send_resp, 'text', '') or '')[:300]}")

    code = get_oai_code(address_jwt=address_jwt, timeout_seconds=180, proxy=proxy)
    print("Verification Code:", code)

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
        raise RuntimeError(f"email-otp/validate failed: http={otp_verify_resp.status_code} body={str(getattr(otp_verify_resp, 'text', '') or '')[:300]}")

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
        raise RuntimeError(f"create_account failed: http={create_account_resp.status_code} body={str(getattr(create_account_resp, 'text', '') or '')[:400]}")

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
        raise RuntimeError(f"workspace/select failed: http={select_resp.status_code} body={str(getattr(select_resp, 'text', '') or '')[:300]}")

    try:
        continue_url = str((select_resp.json() or {}).get("continue_url") or "").strip()
    except Exception:
        continue_url = ""
    if not continue_url:
        raise RuntimeError("workspace/select missing continue_url")

    callback_url = _follow_redirects_for_callback(sess=sess, start_url=continue_url, max_hops=8)

    reg_email, config_json = submit_callback_url(
        callback_url=callback_url,
        expected_state=oauth.state,
        code_verifier=oauth.code_verifier,
        redirect_uri=oauth.redirect_uri,
        proxy=proxy,
        mailbox_ref=address_jwt,
        password=pwd,
        first_name=first_name,
        last_name=last_name,
        birthdate=birthdate,
    )

    return reg_email, config_json


AUTH_URL = "https://auth.openai.com/oauth/authorize"
TOKEN_URL = "https://auth.openai.com/oauth/token"
CLIENT_ID = "app_EMoamEEZ73f0CkXaXp7hrann"

DEFAULT_CALLBACK_PORT = 1455
DEFAULT_REDIRECT_URI = f"http://localhost:{DEFAULT_CALLBACK_PORT}/auth/callback"
DEFAULT_SCOPE = "openid email profile offline_access"


def _b64url_no_pad(raw: bytes) -> str:
    return base64.urlsafe_b64encode(raw).decode("ascii").rstrip("=")


def _sha256_b64url_no_pad(s: str) -> str:
    return _b64url_no_pad(hashlib.sha256(s.encode("ascii")).digest())


def _random_state(nbytes: int = 16) -> str:
    return secrets.token_urlsafe(nbytes)


def _pkce_verifier() -> str:
    # RFC 7636 allows 43..128 chars; urlsafe token is fine.
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

    # Query takes precedence; fragment is a fallback.
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

    # Handle malformed callback payloads where state is appended with '#'.
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
    # WARNING: no signature verification; this only decodes claims to extract fields.
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
    proxy_handler = urllib.request.ProxyHandler({'http': proxy, 'https': proxy})
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
    for _ in range(4):
        try:
            with get_opener(proxy).open(req, timeout=timeout) as resp:
                raw = resp.read()
                if resp.status != 200:
                    raise RuntimeError(
                        f"token exchange failed: {resp.status}: {raw.decode('utf-8', 'replace')}"
                    )
                return json.loads(raw.decode("utf-8"))
        except urllib.error.HTTPError as exc:
            raw = exc.read()
            raise RuntimeError(
                f"token exchange failed: {exc.code}: {raw.decode('utf-8', 'replace')}"
            ) from exc
        except Exception as e:
            print(f"POST Request error: {e}")
            time.sleep(2)
            
    raise RuntimeError("Failed to post form after max retries")


@dataclass(frozen=True)
class OAuthStart:
    auth_url: str
    state: str
    code_verifier: str
    redirect_uri: str


def generate_oauth_url(
    *,
    redirect_uri: str = DEFAULT_REDIRECT_URI,
    scope: str = DEFAULT_SCOPE,
) -> OAuthStart:
    """
    1) Generate oauth URL -> return a URL that can pull up authorization.

    You must keep the returned `state` and `code_verifier` and pass them into
    `submit_callback_url`.
    """
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
    # mailbox_ref: 用于后续“修缮者”读取邮箱验证码；应为 mailbox_provider.py 约定的编码格式
    #   - mailcreate:<jwt>
    #   - gptmail:<email>
    mailbox_ref: str = "",
    password: str = "",
    first_name: str = "",
    last_name: str = "",
    birthdate: str = "",
) -> tuple[str, str]:
    """
    2) Submit call back url -> takes the full callback URL, exchanges the code for
       tokens, and returns a JSON string "config" payload.
    """
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
        proxy=proxy
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
    expired_rfc3339 = time.strftime(
        "%Y-%m-%dT%H:%M:%SZ", time.gmtime(now + max(expires_in, 0))
    )
    now_rfc3339 = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(now))

    # Construct the JSON format exactly as requested by user
    config = dict(claims)
    config.update({
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
    })

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

    # Optional: persist mailbox ref for future repairer runs.
    # Keep it as-is (opaque ref) to avoid mixing provider logic here.
    if mailbox_ref and str(mailbox_ref).strip():
        config["mailbox_ref"] = str(mailbox_ref).strip()

    return email, json.dumps(config, ensure_ascii=False, separators=(",", ":"))


def create_proxy_extension(proxy: str) -> str | None:
    match = re.search(r"http://([^:]+):([^@]+)@([^:]+):(\d+)", proxy)
    if not match:
        return None
    user, pwd, host, port = match.groups()
    
    manifest_json = """
    {
        "version": "1.0.0",
        "manifest_version": 2,
        "name": "Chrome Proxy",
        "permissions": [
            "proxy",
            "tabs",
            "unlimitedStorage",
            "storage",
            "<all_urls>",
            "webRequest",
            "webRequestBlocking"
        ],
        "background": {
            "scripts": ["background.js"]
        },
        "minimum_chrome_version":"22.0.0"
    }
    """

    background_js = """
    var config = {
            mode: "fixed_servers",
            rules: {
              singleProxy: {
                scheme: "http",
                host: "%s",
                port: parseInt(%s)
              },
              bypassList: ["localhost", "127.0.0.1", "<local>"]
            }
          };

    chrome.proxy.settings.set({value: config, scope: "regular"}, function() {});

    function callbackFn(details) {
        return {
            authCredentials: {
                username: "%s",
                password: "%s"
            }
        };
    }

    chrome.webRequest.onAuthRequired.addListener(
                callbackFn,
                {urls: ["<all_urls>"]},
                ['blocking']
    );
    """ % (host, port, user, pwd)
    
    plugin_dir = tempfile.mkdtemp(prefix="proxy_auth_")
    with open(os.path.join(plugin_dir, "manifest.json"), "w", encoding="utf-8") as f:
        f.write(manifest_json)
    with open(os.path.join(plugin_dir, "background.js"), "w", encoding="utf-8") as f:
        f.write(background_js)
        
    return plugin_dir

from selenium import webdriver
from selenium.webdriver.chrome.service import Service
from selenium.webdriver.chrome.options import Options

def new_driver(proxy: str | None = None):
    options = Options()

    # Headless defaults to ON for servers/containers.
    # Set HEADLESS=0 to show the browser window for debugging/observing repair flow.
    headless = int(os.environ.get("HEADLESS", "1"))
    if headless != 0:
        options.add_argument('--headless')

    options.add_argument('--no-sandbox')
    options.add_argument('--disable-dev-shm-usage')
    options.add_argument('--disable-gpu')
    options.add_argument('--window-size=1920,1080')
    options.add_argument('--user-agent=Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36')
    options.add_argument('--enable-features=NetworkService,NetworkServiceInProcess')
    
    # Disable background telemetry and optimization guide to save proxy traffic
    options.add_argument('--disable-features=OptimizationGuideModelDownloading,OptimizationHintsFetching,OptimizationTargetPrediction,OptimizationGuideModelExecution')
    options.add_argument('--disable-background-networking')
    options.add_argument('--disable-sync')
    options.add_argument('--disable-component-update')
    options.add_argument('--disable-domain-reliability')
    options.add_argument('--disable-client-side-phishing-detection')
    options.add_argument('--disable-default-apps')
    options.add_argument('--no-default-browser-check')
    options.add_argument('--disable-features=TranslateUI')

    # Hard block a few extremely noisy Chrome background endpoints by loopback.
    # This prevents wasting proxy traffic on requests like:
    #   optimizationguide-pa.googleapis.com
    #
    # NOTE: This is based on a known-good historical rule set from legacy `tools/oai_register/main.py`.
    # Controlled via env:
    #   BLOCK_GOOGLE_OPT_GUIDE=2  (default: 2)
    #   BLOCK_NOISY_HOSTS=2       (default: 2)
    host_rule_entries: list[str] = []

    block_opt = int(os.environ.get("BLOCK_GOOGLE_OPT_GUIDE", "2"))
    if block_opt == 2:
        host_rule_entries.extend([
            "MAP optimizationguide-pa.googleapis.com 127.0.0.1",
            "MAP optimizationguide-pa.googleapis.com:443 127.0.0.1",
            "MAP optimizationguide-pa.googleapis.com:80 127.0.0.1",
        ])

    # Extra background endpoints historically known to burn proxy bandwidth.
    # Keep this list conservative; enable/disable via env.
    block_noisy = int(os.environ.get("BLOCK_NOISY_HOSTS", "2"))
    if block_noisy == 2:
        host_rule_entries.extend([
            "MAP update.googleapis.com 127.0.0.1",
            "MAP browser-intake-datadoghq.com 127.0.0.1",
            "MAP *.gvt1.com 127.0.0.1",
            "MAP *.cloudflarestream.com 127.0.0.1",
        ])

    if host_rule_entries:
        options.add_argument(f"--host-resolver-rules={', '.join(host_rule_entries)}")

    # Anti-detect features for standard selenium
    options.add_argument('--disable-blink-features=AutomationControlled')
    options.add_experimental_option("excludeSwitches", ["enable-automation"])
    options.add_experimental_option('useAutomationExtension', False)
    
    # Traffic saver defaults to ON (2) unless explicitly disabled.
    # 2 = block, 1 = allow
    block_images = int(os.environ.get("BLOCK_IMAGES", "2"))
    block_css = int(os.environ.get("BLOCK_CSS", "2"))
    block_fonts = int(os.environ.get("BLOCK_FONTS", "2"))
    
    prefs = {}
    if block_images == 2:
        prefs["profile.managed_default_content_settings.images"] = 2
        options.add_argument('--blink-settings=imagesEnabled=false')
    if block_css == 2:
        prefs["profile.managed_default_content_settings.stylesheet"] = 2
    if block_fonts == 2:
        prefs["profile.managed_default_content_settings.fonts"] = 2

    if prefs:
        print(f"Traffic Saver Mode Active: Images={block_images==2}, CSS={block_css==2}, Fonts={block_fonts==2}")
        options.add_experimental_option("prefs", prefs)
    
    proxy_dir = None
    if proxy and "@" in proxy:
        proxy_dir = create_proxy_extension(proxy)
        if proxy_dir:
            options.add_argument(f"--load-extension={proxy_dir}")
            options.add_argument(f"--disable-extensions-except={proxy_dir}")
    elif proxy:
        options.add_argument(f'--proxy-server={proxy}')

    # Always bypass localhost loopback for OAuth redirect capture.
    if proxy:
        options.add_argument("--proxy-bypass-list=<-loopback>;localhost;127.0.0.1")
        
    options.add_argument('--log-level=3')
    options.add_argument('--disable-crash-reporter')
    options.add_argument('--disable-in-process-stack-traces')
    options.page_load_strategy = 'eager' # Don't wait for all resources to download
    
    service = Service()
    driver = webdriver.Chrome(service=service, options=options)

    # Aggressive network blocking (CDP). This is more reliable than prefs alone.
    try:
        driver.execute_cdp_cmd("Network.enable", {})
        blocked_urls: list[str] = []
        if block_images == 2:
            blocked_urls.extend([
                "*.png",
                "*.jpg",
                "*.jpeg",
                "*.gif",
                "*.webp",
                "*.avif",
                "*.svg",
                "*.ico",
            ])
        if block_css == 2:
            blocked_urls.extend(["*.css"])
        if block_fonts == 2:
            blocked_urls.extend([
                "*.woff",
                "*.woff2",
                "*.ttf",
                "*.otf",
                "*.eot",
            ])
        if blocked_urls:
            driver.execute_cdp_cmd("Network.setBlockedURLs", {"urls": blocked_urls})
    except Exception:
        # CDP may fail on some driver builds; prefs still apply.
        pass

    # Execute CDP command to hide webdriver property
    driver.execute_cdp_cmd("Page.addScriptToEvaluateOnNewDocument", {
        "source": """
            Object.defineProperty(navigator, 'webdriver', {
                get: () => undefined
            })
        """
    })

    return driver, proxy_dir

def generate_name() -> tuple[str, str]:
    first = ["Neo", "John", "Sarah", "Michael", "Emma", "David", "James", "Robert", "Mary", "William", "Richard", "Thomas", "Charles", "Christopher", "Daniel", "Matthew", "Anthony", "Mark", "Donald", "Steven", "Paul", "Andrew", "Joshua", "Kenneth", "Kevin", "Brian", "George", "Edward", "Ronald", "Timothy"]
    last = ["Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis", "Rodriguez", "Martinez", "Hernandez", "Lopez", "Gonzalez", "Wilson", "Anderson", "Thomas", "Taylor", "Moore", "Jackson", "Martin", "Lee", "Perez", "Thompson", "White"]
    return random.choice(first), random.choice(last)

def generate_pwd(length=12) -> str:
    chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@*&"
    return "".join(random.choice(chars) for _ in range(length)) + "A1@"

def enter_birthday(driver) -> str:
    # We no longer handle birthday here. The JS in the name step handles both name and birthday if they are on the same explicit page.
    # Otherwise, this is just the fallback blind tab entry.
    try:
        # Standard blind tab entry fallback
        birthday_input = driver.switch_to.active_element
        birthday_input.send_keys("1")
        birthday_input.send_keys(Keys.TAB)
        birthday_input = driver.switch_to.active_element
        birthday_input.send_keys("1")
        birthday_input.send_keys(Keys.TAB)
        birthday_input = driver.switch_to.active_element
        birthday_input.send_keys("2000")
        birthday_input.send_keys(Keys.ENTER)
    except Exception:
        pass
    
    return "2000-01-01"

def smart_wait(driver, by, value, timeout=20, *, debug_kind: str = "", debug_message: str = ""):
    """Wait for an element.

    While waiting, it also checks for OpenAI's “Oops, an error occurred!” overlay
    and clicks “Try again” automatically.

    In debug mode, when it fails it will dump the current page body/source and
    raise a RuntimeError (NOT TimeoutException), so logs won't be filled with
    generic timeout errors.
    """

    if debug_kind:
        _dbg("wait", f"{debug_kind} by={by} value={value!r} timeout={timeout}s", driver=driver)

    end_time = time.time() + timeout
    while time.time() < end_time:
        try:
            # Check for the "Try again" button and click it if it appears
            try_again_btns = driver.find_elements(
                By.XPATH,
                "//button[contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'try again')]",
            )
            if try_again_btns and try_again_btns[0].is_displayed():
                _dbg("overlay", "Detected 'Oops' overlay; clicking 'Try again'", driver=driver)
                _click_with_debug(driver, try_again_btns[0], tag="overlay_try_again", note="smart_wait oops overlay")
                time.sleep(2)  # Wait for page to reload/recover
                continue

            # Check for the actual target element
            el = driver.find_element(by, value)
            if el.is_displayed() and el.is_enabled():
                if debug_kind:
                    _dbg("wait", f"{debug_kind} ok", driver=driver)
                return el
        except Exception:
            pass
        time.sleep(0.5)

    if debug_kind:
        msg = debug_message or f"wait failed for {by}={value}"
        try:
            _dump_page_body(driver=driver, kind=debug_kind, message=msg)
        except Exception:
            pass
        try:
            _save_error_artifacts(driver=driver, kind=debug_kind, message=msg)
        except Exception:
            pass
        raise RuntimeError(f"wait failed: {debug_kind}; page dumped")

    raise TimeoutException(f"Timeout waiting for {by}={value}")

def register(driver, proxy=None) -> tuple[str, str]:
    _dbg("register", "start", driver=driver)

    email, address_jwt = get_email(proxy)
    _dbg("mailbox", f"obtained email={email} ref={address_jwt}", driver=driver)
    print("Email obtained:", email)

    oauth = generate_oauth_url()
    url = oauth.auth_url
    _dbg("oauth", "generated oauth url", driver=driver)
    print("OAuth URL:", url)

    _dbg("nav", "driver.get(oauth_url)", driver=driver)
    driver.get(url)

    try:
        WebDriverWait(driver, 60).until(EC.url_contains("auth.openai.com"))
        _dbg("page", "reach oai sign up page", driver=driver)
        print("Reach oai sign up page")
    except TimeoutException:
        _dump_page_body(driver=driver, kind="wait_auth_openai", message="URL did not contain auth.openai.com")
        raise RuntimeError("did not reach auth.openai.com; page dumped")

    # click sign up
    sign_up_button = smart_wait(
        driver,
        By.XPATH,
        "//*[self::button or self::a][contains(normalize-space(), 'Sign up') or contains(normalize-space(), '注册') or contains(normalize-space(), 'Sign Up') or contains(normalize-space(), 'sign up') or contains(normalize-space(), 'SignUp')]",
        timeout=20,
        debug_kind="signup_button",
        debug_message="sign up button not found",
    )
    _dbg("ui", "click sign up", driver=driver)
    _click_with_debug(driver, sign_up_button, tag="signup_button", note="register click sign up")
    _dbg("ui", "sign up clicked", driver=driver)
    print("Sign up clicked")

    # fill email
    email_input = smart_wait(
        driver,
        By.ID,
        "_r_f_-email",
        timeout=20,
        debug_kind="email_input",
        debug_message="email input not found",
    )
    _dbg("ui", "reach email input", driver=driver)
    email_input.clear()
    _dbg("ui", f"fill email={email}", driver=driver)
    print("Reach email input")
    # Speed: send full string instead of per-char typing.
    email_input.send_keys(email)
    email_input.send_keys(Keys.ENTER)
    _dbg("ui", "email ENTER pressed", driver=driver)
    print("Enter pressed")

    # fill password
    pwd_input = smart_wait(
        driver,
        By.ID,
        "_r_u_-new-password",
        timeout=30,
        debug_kind="password_input",
        debug_message=f"password input not found; email={email}",
    )
    _dbg("ui", "reach password input", driver=driver)
    print("Reach password input")
    pwd = generate_pwd()
    _dbg("ui", "fill password", driver=driver)
    # Speed: send full string instead of per-char typing.
    pwd_input.send_keys(pwd)
    pwd_input.send_keys(Keys.ENTER)
    _dbg("ui", "password ENTER pressed", driver=driver)
    print("Enter pressed")
    
    code = get_oai_code(address_jwt=address_jwt, timeout_seconds=180, proxy=proxy)
    # Dump page + pause shortly to reduce race where code changes due to resend.
    try:
        _dbg("mail", f"got verification code={code} mailbox_ref={address_jwt}", driver=driver)
        _dump_page_body(driver=driver, kind="code_before_fill", message=f"code={code} mailbox_ref={address_jwt}")
    except Exception:
        pass
    time.sleep(1.0)
    print("Verification Code:", code)
    try:
        code_input = smart_wait(
            driver,
            By.ID,
            "_r_4_-code",
            timeout=10,
            debug_kind="code_input",
            debug_message="Timeout waiting for code input",
        )
        print("Reach code input")

        # Defensive: ensure the code input is empty.
        try:
            code_input.clear()
        except Exception:
            try:
                code_input.send_keys(Keys.CONTROL + "a")
                code_input.send_keys(Keys.BACKSPACE)
            except Exception:
                pass

        # Sanity-check: the page should be sending to the same email.
        try:
            page_txt = driver.execute_script("return document && document.body ? document.body.innerText : ''; ")
            m = re.search(
                r"sent\s+to\s+([A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,})",
                str(page_txt or ""),
                flags=re.IGNORECASE,
            )
            if m:
                page_email = (m.group(1) or "").strip().lower()
                if page_email and page_email != (email or "").strip().lower():
                    _dbg("code", f"page email mismatch page={page_email} expected={email}", driver=driver)
                    _dump_page_body(driver=driver, kind="code_email_mismatch", message=f"page={page_email} expected={email}")
        except Exception:
            pass

        # Speed: send code in one shot.
        code_input.send_keys(code)
        code_input.send_keys(Keys.ENTER)
        print("Enter pressed")

        # If the page shows "Incorrect code", try one resend then re-fetch code.
        try:
            time.sleep(1.0)
            txt = driver.execute_script("return document && document.body ? document.body.innerText : '';")
            if "incorrect code" in str(txt or "").lower():
                _dbg("code", "detected incorrect code, trying resend once", driver=driver)
                _dump_page_body(driver=driver, kind="code_incorrect", message="incorrect code after submit")

                try:
                    resend_btn = smart_wait(
                        driver,
                        By.XPATH,
                        "//button[contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'resend')]",
                        timeout=10,
                        debug_kind="resend_button",
                        debug_message="resend button not found",
                    )
                    _click_with_debug(driver, resend_btn, tag="resend_button", note="incorrect code -> resend")
                    time.sleep(2)
                except Exception:
                    pass

                # re-fetch code and submit again (single retry)
                code2 = get_oai_code(address_jwt=address_jwt, timeout_seconds=180, proxy=proxy)
                _dbg("mail", f"re-fetched verification code={code2}", driver=driver)
                try:
                    _dump_page_body(driver=driver, kind="code_retry", message=f"code2={code2}")
                except Exception:
                    pass

                code_input2 = smart_wait(
                    driver,
                    By.ID,
                    "_r_4_-code",
                    timeout=10,
                    debug_kind="code_input_retry",
                    debug_message="code input not found on retry",
                )
                try:
                    code_input2.clear()
                except Exception:
                    pass
                # Speed: send code in one shot.
                code_input2.send_keys(code2)
                code_input2.send_keys(Keys.ENTER)
                time.sleep(1.0)
        except Exception:
            pass

    except TimeoutException:
        print("Reach new code input")
        code_inputs = WebDriverWait(driver, 10).until(
            lambda d: d.find_elements(
                By.CSS_SELECTOR,
                'div[role="group"] input[inputmode="numeric"][maxlength="1"]'
            )
        )
        for current, digit in zip(code_inputs, code):
            WebDriverWait(driver, 1).until(EC.element_to_be_clickable(current))
            _click_with_debug(driver, current, tag="otp_digit_box", note="register segmented otp input")
            current.clear()
            current.send_keys(digit)
        driver.switch_to.active_element.send_keys(Keys.ENTER)
        
    first_name, last_name = generate_name()
    full_name_str = first_name + " " + last_name
    
    print("Filling Name and Birthday (robust v4)")
 
    # Default target birthday (stable & adult)
    birthdate = "1995-01-15"
    explicit_form_detected = False
    bday_filled = False
 
    def _force_set_birthday_iso(iso_yyyy_mm_dd: str) -> dict | None:
        """Force-set the react-aria DateField hidden input used by /about-you.

        NOTE: This can be overwritten by React because the hidden input is
        controlled by internal state. Prefer `_fill_about_you_birthday_segments()`
        which simulates real user typing into the contenteditable segments.
        """

        js = r"""
        return (function(v){
          try {
            const inp = document.querySelector('input[type="hidden"][name="birthday"]');
            if (!inp) return {ok:false, reason:'no_hidden_birthday'};
            const prev = inp.value;
            inp.value = v;
            try { inp.setAttribute('value', v); } catch (e) {}
            try { inp.dispatchEvent(new Event('input', {bubbles:true})); } catch (e) {}
            try { inp.dispatchEvent(new Event('change', {bubbles:true})); } catch (e) {}

            const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(v || '');
            if (m) {
              const group = document.querySelector('div[role="group"][id$="-birthday"]');
              if (group) {
                const setSeg = (t, txt) => {
                  const el = group.querySelector('div[data-type="' + t + '"][contenteditable="true"]');
                  if (el) {
                    try { el.textContent = txt; } catch (e) {}
                    try { el.dispatchEvent(new Event('input', {bubbles:true})); } catch (e) {}
                    try { el.dispatchEvent(new Event('change', {bubbles:true})); } catch (e) {}
                  }
                };
                setSeg('month', m[2]);
                setSeg('day', m[3]);
                setSeg('year', m[1]);
              }
            }

            return {ok:true, prev:prev, now:inp.value};
          } catch (e) {
            return {ok:false, reason:String(e)};
          }
        })(arguments[0]);
        """

        try:
            return driver.execute_script(js, iso_yyyy_mm_dd)
        except Exception:
            return None

    def _fill_about_you_birthday_segments(*, iso_yyyy_mm_dd: str) -> dict | None:
        """Fill /about-you birthday by typing into react-aria contenteditable segments.

        This is the only method we've observed that updates the hidden
        `input[name=birthday]` reliably.
        """

        try:
            yyyy, mm, dd = (iso_yyyy_mm_dd or "").split("-")
        except Exception:
            return None

        try:
            group = driver.find_element(By.CSS_SELECTOR, 'div[role="group"][id$="-birthday"]')
            seg_month = group.find_element(By.CSS_SELECTOR, 'div[contenteditable="true"][data-type="month"]')
            seg_day = group.find_element(By.CSS_SELECTOR, 'div[contenteditable="true"][data-type="day"]')
            seg_year = group.find_element(By.CSS_SELECTOR, 'div[contenteditable="true"][data-type="year"]')

            def _type_seg(el, text: str) -> None:
                try:
                    el.click()
                except Exception:
                    try:
                        driver.execute_script("arguments[0].focus();", el)
                    except Exception:
                        pass
                try:
                    el.send_keys(Keys.CONTROL + "a")
                    el.send_keys(Keys.BACKSPACE)
                except Exception:
                    pass
                for ch in text:
                    el.send_keys(ch)
                    time.sleep(0.02)

            # NOTE: react-aria numeric segments sometimes behave like fixed-width fields.
            # Send the raw digits without leading zeros to reduce weird carry/overflow.
            _type_seg(seg_month, str(int(mm)))
            _type_seg(seg_day, str(int(dd)))
            _type_seg(seg_year, str(int(yyyy)))

            try:
                seg_year.send_keys(Keys.TAB)
            except Exception:
                pass

            hidden_now = driver.execute_script(
                'var el=document.querySelector("input[type=\\"hidden\\"][name=\\"birthday\\"]"); return el? (el.value||"") : "";'
            )
            return {
                "ok": True,
                "iso": iso_yyyy_mm_dd,
                "hidden": str(hidden_now or ""),
            }
        except Exception as e:
            return {"ok": False, "reason": str(e)}

    def _force_submit_about_you_form(*, full_name: str, iso_yyyy_mm_dd: str) -> dict | None:
        """Force-submit the /about-you form with a guaranteed birthday payload.

        We observed react-aria DateField can display our typed segments but still
        keeps the hidden `input[name=birthday]` at its default (TODAY), which the
        server rejects. This function removes the existing hidden birthday input
        and injects a new one right before submitting the form.
        """

        js = r"""
        return (function(nameV, bdayV){
          try {
            const form = document.querySelector('form[action="/about-you"]');
            if (!form) return {ok:false, reason:'no_form'};

            // set name
            try {
              const nameInp = form.querySelector('input[name="name"]');
              if (nameInp) {
                nameInp.value = nameV || '';
                try { nameInp.dispatchEvent(new Event('input', {bubbles:true})); } catch (e) {}
                try { nameInp.dispatchEvent(new Event('change', {bubbles:true})); } catch (e) {}
              }
            } catch (e) {}

            // force birthday hidden input
            let removed = 0;
            try {
              const olds = form.querySelectorAll('input[type="hidden"][name="birthday"]');
              removed = olds ? olds.length : 0;
              olds && olds.forEach(n => { try { n.remove(); } catch (e) {} });
            } catch (e) {}

            try {
              const inp = document.createElement('input');
              inp.type = 'hidden';
              inp.name = 'birthday';
              inp.value = bdayV || '';
              form.appendChild(inp);
            } catch (e) {}

            // also ensure consent hidden exists
            try {
              const c = form.querySelector('input[type="hidden"][name="isExplicitConsentRequired"]');
              if (!c) {
                const ci = document.createElement('input');
                ci.type = 'hidden';
                ci.name = 'isExplicitConsentRequired';
                ci.value = 'false';
                form.appendChild(ci);
              }
            } catch (e) {}

            // submit (simulate real button click; React Router action is required)
            try {
              const btn = form.querySelector('button[type="submit"], input[type="submit"]');
              if (btn) {
                btn.click();
              } else {
                form.submit();
              }
            } catch (e) {
              return {ok:false, reason:'submit_failed:' + String(e), removed:removed};
            }
            return {ok:true, removed:removed, birthday:bdayV};
          } catch (e) {
            return {ok:false, reason:String(e)};
          }
        })(arguments[0], arguments[1]);
        """;

        try:
            return driver.execute_script(js, full_name, iso_yyyy_mm_dd)
        except Exception:
            return None
  
    def _is_visible(el) -> bool:
        try:
            return el.is_displayed() and el.is_enabled()
        except Exception:
            return False

    def _attrs_text(el) -> str:
        parts: list[str] = []
        for k in ("id", "name", "placeholder", "aria-label", "autocomplete", "type"):
            try:
                v = (el.get_attribute(k) or "").strip()
                if v:
                    parts.append(v)
            except Exception:
                pass
        return " ".join(parts).lower()

    def _safe_focus(el) -> None:
        try:
            _click_with_debug(driver, el, tag="safe_focus", note="focus input before typing")
            return
        except Exception:
            pass
        try:
            driver.execute_script("arguments[0].focus();", el)
        except Exception:
            pass

    def _safe_clear(el) -> None:
        _safe_focus(el)
        try:
            el.send_keys(Keys.CONTROL + "a")
            el.send_keys(Keys.BACKSPACE)
        except Exception:
            pass

    def _set_value_js(el, value: str) -> bool:
        try:
            return bool(
                driver.execute_script(
                    """
                    const el = arguments[0];
                    const value = arguments[1];
                    try { el.focus(); } catch (e) {}
                    try { el.value = value; } catch (e) { return false; }
                    try { el.dispatchEvent(new Event('input', { bubbles: true })); } catch (e) {}
                    try { el.dispatchEvent(new Event('change', { bubbles: true })); } catch (e) {}
                    return true;
                    """,
                    el,
                    value,
                )
            )
        except Exception:
            return False

    def _pick_best(elements, keywords: list[str], *, forbid: list[str] | None = None):
        forbid = forbid or []
        best = None
        best_score = 0
        for el in elements:
            txt = _attrs_text(el)
            if any(bad in txt for bad in forbid):
                continue
            score = 0
            for kw in keywords:
                if kw in txt:
                    score += 2
            if score > best_score:
                best_score = score
                best = el
        return best

    try:
        # Wait for the profile step to render; avoid long fixed sleeps.
        t_end = time.time() + 20
        while time.time() < t_end:
            if driver.find_elements(By.CSS_SELECTOR, "input, select"):
                break
            time.sleep(0.5)

        inputs = [el for el in driver.find_elements(By.CSS_SELECTOR, "input") if _is_visible(el)]
        selects = [el for el in driver.find_elements(By.CSS_SELECTOR, "select") if _is_visible(el)]

        forbid_name = ["email", "password", "code", "otp", "verification", "phone"]

        first_input = _pick_best(inputs, ["first", "given"], forbid=forbid_name)
        last_input = _pick_best(inputs, ["last", "family", "surname"], forbid=forbid_name)
        full_name_input = _pick_best(inputs, ["full name", "fullname"], forbid=forbid_name) or _pick_best(inputs, ["name"], forbid=forbid_name)

        name_filled = False
        if first_input and last_input:
            _safe_clear(first_input)
            first_input.send_keys(first_name)
            _safe_clear(last_input)
            last_input.send_keys(last_name)
            name_filled = True
        elif full_name_input:
            _safe_clear(full_name_input)
            full_name_input.send_keys(full_name_str)
            name_filled = True

        yyyy, mm, dd = birthdate.split("-")
        forbid_bday = ["email", "password", "code", "otp"]

        bday_input = _pick_best(inputs, ["birth", "birthday", "date of birth", "dob", "birthdate"], forbid=forbid_bday)
        month_sel = _pick_best(selects, ["month"], forbid=forbid_bday)
        day_sel = _pick_best(selects, ["day"], forbid=forbid_bday)
        year_sel = _pick_best(selects, ["year"], forbid=forbid_bday)

        def _ph(el) -> str:
            try:
                return (el.get_attribute("placeholder") or "").strip().lower()
            except Exception:
                return ""

        month_inp = next((el for el in inputs if _ph(el) in ("mm", "month")), None)
        day_inp = next((el for el in inputs if _ph(el) in ("dd", "day")), None)
        year_inp = next((el for el in inputs if _ph(el) in ("yyyy", "year")), None)

        # (bday_filled defined outside try)
        bday_filled = False
        if month_sel and day_sel and year_sel:
            try:
                Select(month_sel).select_by_value(str(int(mm)))
            except Exception:
                try:
                    Select(month_sel).select_by_visible_text(str(int(mm)))
                except Exception:
                    pass
            try:
                Select(day_sel).select_by_value(str(int(dd)))
            except Exception:
                try:
                    Select(day_sel).select_by_visible_text(str(int(dd)))
                except Exception:
                    pass
            try:
                Select(year_sel).select_by_value(yyyy)
            except Exception:
                try:
                    Select(year_sel).select_by_visible_text(yyyy)
                except Exception:
                    pass
            bday_filled = True
        elif month_inp and day_inp and year_inp:
            _safe_clear(month_inp)
            month_inp.send_keys(f"{int(mm):02d}")
            _safe_clear(day_inp)
            day_inp.send_keys(f"{int(dd):02d}")
            _safe_clear(year_inp)
            year_inp.send_keys(yyyy)
            bday_filled = True
        elif bday_input:
            btype = (bday_input.get_attribute("type") or "").strip().lower()
            if btype == "date":
                if not _set_value_js(bday_input, birthdate):
                    _safe_clear(bday_input)
                    bday_input.send_keys(birthdate)
            else:
                masked = f"{int(mm):02d}/{int(dd):02d}/{yyyy}"
                _safe_clear(bday_input)
                for ch in f"{int(mm):02d}{int(dd):02d}{yyyy}":
                    bday_input.send_keys(ch)
                    time.sleep(0.03)
                _set_value_js(bday_input, masked)
            bday_filled = True
        else:
            # /about-you uses react-aria contenteditable segments + a hidden input.
            # DOM-level value assignment gets overwritten; type into segments.
            try:
                if driver.find_elements(By.CSS_SELECTOR, 'input[type="hidden"][name="birthday"]'):
                    r = _fill_about_you_birthday_segments(iso_yyyy_mm_dd=birthdate)
                    _dbg("about-you", f"fill birthday segments result={r}", driver=driver)
                    bday_filled = bool(r and r.get("hidden") == birthdate)
            except Exception:
                pass

        explicit_form_detected = bool(name_filled or bday_filled)

        # Trigger blur/validation
        try:
            driver.switch_to.active_element.send_keys(Keys.TAB)
        except Exception:
            pass

    except Exception as e:
        print(f"Name/Birthday filling error (v4): {e}")

    # Safety net: if we're on /about-you and birthday wasn't filled, force-set the hidden input.
    # This page defaults birthday to TODAY (invalid for signup), causing:
    #   "We can't create an account with that info. Try again."
    if not bday_filled:
        try:
            if "auth.openai.com/about-you" in str(getattr(driver, "current_url", "") or ""):
                r = _fill_about_you_birthday_segments(iso_yyyy_mm_dd=birthdate)
                _dbg("about-you", f"post-fill birthday segments result={r}", driver=driver)
                bday_filled = bool(r and r.get("hidden") == birthdate)
        except Exception:
            pass

    if not explicit_form_detected and not bday_filled:
        # Fallback: old blind tab entry (last resort)
        birthdate = enter_birthday(driver)

    print("Reach birthday input")

    # If we're on the /about-you page, force-submit it (most reliable).
    try:
        u0 = str(getattr(driver, "current_url", "") or "")
        if "auth.openai.com/about-you" in u0 or driver.find_elements(By.CSS_SELECTOR, 'form[action="/about-you"]'):
            rsub = _force_submit_about_you_form(full_name=full_name_str, iso_yyyy_mm_dd=birthdate)
            _dbg("about-you", f"force submit about-you result={rsub}", driver=driver)
            try:
                time.sleep(1)
                _dump_page_body(driver=driver, kind="about_you_force_submit", message=f"birthdate={birthdate} result={rsub}")
            except Exception:
                pass
    except Exception:
        pass

    def _click_final_continue_if_present() -> bool:
        xpaths = [
            "//button[(contains(., 'Agree') or contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'continue')) and not(contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'continue with'))]",
            "//button[contains(normalize-space(.), '继续') or contains(normalize-space(.), '同意') or contains(normalize-space(.), '允许') or contains(normalize-space(.), '授权')]",
            "//button[contains(normalize-space(.), '继续操作') or contains(normalize-space(.), '继续并授权')]",
        ]
        for xp in xpaths:
            el = _find_visible(driver, By.XPATH, xp)
            if not el:
                continue
            _click_with_debug(driver, el, tag="continue_button", note=f"final-continue xpath={xp[:90]}")
            return True
        return False

    continue_button = None
    try:
        # Final confirmation page click (if we are not on about-you / force submit didn't navigate)
        print("Clicking continue/agree button")
        if not _click_final_continue_if_present():
            continue_button = smart_wait(
                driver,
                By.XPATH,
                (
                    "//button[(contains(., 'Agree') or "
                    "contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'continue')) "
                    "and not(contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'continue with'))]"
                ),
                timeout=10,
                debug_kind="continue_button",
                debug_message="continue/agree button not found",
            )
            _click_with_debug(driver, continue_button, tag="continue_button_fallback", note="register continue/agree fallback")
        time.sleep(1)

        # Dump after click to see where we landed (helps debug accidental navigation to terms page).
        try:
            _dbg("ui", "after continue/agree click", driver=driver)
            _dump_page_body(driver=driver, kind="after_continue", message="after continue/agree click")
        except Exception:
            pass

        # If we are still stuck on /about-you, it usually means validation failed
        # (e.g. birthday still set to today's date -> "We can't create an account with that info").
        # Try to force-set an adult birthday and submit once more before waiting callback.
        try:
            time.sleep(0.5)
            u_now = str(getattr(driver, "current_url", "") or "")
            if "auth.openai.com/about-you" in u_now:
                txt = driver.execute_script(
                    "return document && document.body ? (document.body.innerText || '') : '';"
                )
                if "we can't create an account with that info" in str(txt or "").lower():
                    _dbg("about-you", "validation error detected; retrying with forced adult birthday", driver=driver)
                    _dump_page_body(driver=driver, kind="about_you_validation", message="validation error after continue")
                    r = _fill_about_you_birthday_segments(iso_yyyy_mm_dd=birthdate)
                    _dbg("about-you", f"retry birthday segments result={r}", driver=driver)

                    # Prefer form.submit() over clicking the button again.
                    try:
                        driver.execute_script(
                            "var f=document.querySelector('form[action=\"/about-you\"]'); if(f) f.submit();"
                        )
                    except Exception:
                        _click_with_debug(driver, continue_button, tag="continue_button_retry", note="about-you retry continue")

                    time.sleep(1)
                    _dump_page_body(driver=driver, kind="after_continue_retry", message=f"after retry continue on about-you force={r}")
        except Exception:
            pass

    except Exception:
        print("Continue button missing, skip ENTER fallback to avoid mis-navigation")
        try:
            _dbg("ui", "continue button missing; do not press ENTER", driver=driver)
            _dump_page_body(driver=driver, kind="continue_missing_no_enter", message="continue button missing; ENTER disabled")
        except Exception:
            pass
        
    def _maybe_recover_from_terms_page() -> None:
        """Sometimes we accidentally land on openai.com policies pages (CSS/JS blocked).

        Try to return to auth flow, and prefer the tab whose URL is auth.openai.com.
        """

        try:
            handles = list(driver.window_handles or [])
        except Exception:
            handles = []

        # Prefer auth.openai.com tab if multiple windows exist.
        try:
            if len(handles) > 1:
                best = None
                for h in handles:
                    try:
                        driver.switch_to.window(h)
                        u = str(getattr(driver, "current_url", "") or "")
                        if "auth.openai.com" in u or "localhost:1455" in u:
                            best = h
                            break
                    except Exception:
                        continue
                if best:
                    driver.switch_to.window(best)
        except Exception:
            pass

        try:
            u = str(getattr(driver, "current_url", "") or "")
            if "openai.com/policies" in u:
                _dbg("recover", f"landed on policies page: {u}", driver=driver)
                _dump_page_body(driver=driver, kind="policies_page", message=u)
                try:
                    driver.back()
                    time.sleep(1)
                except Exception:
                    pass
        except Exception:
            pass

    try:
        # Give it a few tries; sometimes a transient nav goes to policies page.
        for _ in range(3):
            _maybe_recover_from_terms_page()
            try:
                WebDriverWait(driver, 20).until(EC.url_contains("localhost:1455"))
                break
            except TimeoutException:
                continue
        else:
            raise TimeoutException("callback not reached")

    except TimeoutException:
        print("Timeout waiting for callback URL. Capturing screenshot...")
        try:
            _dump_page_body(driver=driver, kind="callback_timeout", message=str(getattr(driver, "current_url", "") or ""))
        except Exception:
            pass
        _save_error_artifacts(driver=driver, kind="callback", message="Timeout waiting for callback URL to localhost")
        raise RuntimeError("Blocked: Timeout waiting for callback URL to localhost.")
        
    callback_url = driver.current_url
    print("Success Callback URL Captured.")
    
    reg_email, call_back = submit_callback_url(
        callback_url=callback_url,
        expected_state=oauth.state,
        code_verifier=oauth.code_verifier,
        redirect_uri=oauth.redirect_uri,
        proxy=proxy,
        mailbox_ref=address_jwt,
        password=pwd,
        first_name=first_name,
        last_name=last_name,
        birthdate=birthdate,
    )
    return reg_email, call_back

# -----------------------------------------------------------------------------
# Repairer (修缮者)：消费 need_fix_auth/ 队列，老号登录换新 token
# -----------------------------------------------------------------------------

def _repairer_dirs() -> tuple[str, str, str, str]:
    need = _data_path(NEED_FIX_AUTH_DIRNAME)
    proc = os.path.join(need, "_processing")
    okd = _data_path(FIXED_SUCCESS_DIRNAME)
    bad = _data_path(FIXED_FAIL_DIRNAME)
    return need, proc, okd, bad


def _repairer_results_dir() -> str:
    return _results_dir()


def _append_jsonl(path: str, obj: dict[str, Any]) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    line = json.dumps(obj, ensure_ascii=False, separators=(",", ":"))
    with open(path, "a", encoding="utf-8") as f:
        f.write(line + "\n")


def _read_json_any(path: str) -> Any:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def _write_json_any(path: str, obj: Any) -> None:
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(obj, f, ensure_ascii=False, indent=2)
    os.replace(tmp, path)


def _deep_merge_keep_old_when_missing(old: Any, new: Any) -> Any:
    """Merge dicts recursively.

    Policy:
    - If key exists in `new`, it overwrites `old`.
    - If key does NOT exist in `new`, keep from `old`.
    - For nested dicts: recurse.
    - For lists / scalars: replace.

    Additionally, for `email` / `password`, we treat empty-string from `new` as "missing"
    and keep old values.
    """

    if isinstance(old, dict) and isinstance(new, dict):
        out: dict[str, Any] = dict(old)
        for k, v in new.items():
            if k in out and isinstance(out.get(k), dict) and isinstance(v, dict):
                out[k] = _deep_merge_keep_old_when_missing(out.get(k), v)
            else:
                out[k] = v

        for k in ("email", "password"):
            if k in old and (k not in new or str(new.get(k) or "").strip() == ""):
                out[k] = old.get(k)

        return out

    return new if new is not None else old


def _repairer_claim_one_file() -> str | None:
    need, proc, _okd, _bad = _repairer_dirs()
    if not os.path.isdir(need):
        return None
    os.makedirs(proc, exist_ok=True)

    try:
        names = [
            n
            for n in os.listdir(need)
            if n.lower().endswith(".json") and os.path.isfile(os.path.join(need, n))
        ]
    except Exception:
        return None

    if not names:
        return None

    # oldest first for stability
    try:
        names.sort(key=lambda n: os.path.getmtime(os.path.join(need, n)))
    except Exception:
        pass

    for name in names:
        src = os.path.join(need, name)
        dst = os.path.join(proc, name)
        try:
            os.replace(src, dst)
            return dst
        except FileNotFoundError:
            continue
        except PermissionError:
            continue
        except OSError:
            continue

    return None


def _repairer_release_stale_processing(*, stale_seconds: int = 1800) -> None:
    _need, proc, _okd, _bad = _repairer_dirs()
    if not os.path.isdir(proc):
        return

    now = time.time()
    for name in os.listdir(proc):
        if not name.lower().endswith(".json"):
            continue
        path = os.path.join(proc, name)
        try:
            st = os.stat(path)
        except Exception:
            continue
        if now - st.st_mtime < stale_seconds:
            continue

        # move back for retry
        try:
            os.replace(path, os.path.join(_data_path(NEED_FIX_AUTH_DIRNAME), name))
        except Exception:
            pass


def _find_visible(driver, by, value):
    try:
        els = driver.find_elements(by, value)
    except Exception:
        return None
    for el in els:
        try:
            if el.is_displayed() and el.is_enabled():
                return el
        except Exception:
            continue
    return None


def _click_if_found(driver, xpath: str) -> bool:
    try:
        el = _find_visible(driver, By.XPATH, xpath)
        if not el:
            return False
        _click_with_debug(driver, el, tag="click_if_found", note=f"xpath={xpath[:120]}")
        return True
    except Exception:
        return False


def _wait_for_any(driver, *, timeout_seconds: int, predicates: list[callable]) -> Any:
    end = time.time() + timeout_seconds
    last_exc: Exception | None = None
    while time.time() < end:
        for p in predicates:
            try:
                v = p()
                if v:
                    return v
            except Exception as e:
                last_exc = e
        time.sleep(0.4)
    raise RuntimeError(f"timeout waiting for condition: {last_exc}")


def _wait_code_try_candidates(*, candidates: list[str], timeout_seconds: int) -> tuple[str, str]:
    """Try multiple encoded mailbox_ref until we can fetch a 6-digit code.

    Special policy:
    - If GPTMail is quota-limited (or all keys exhausted), we should NOT mark the
      auth as "unrepairable". Caller can treat it as a transient "no_quota" case.
    """

    last_err: Exception | None = None
    last_err_str = ""

    for ref in candidates:
        r = str(ref or "").strip()
        if not r:
            continue
        try:
            code = wait_openai_code_by_provider(
                provider="auto",
                mailbox_ref=r,
                mailcreate_base_url=MAILCREATE_BASE_URL,
                mailcreate_custom_auth=MAILCREATE_CUSTOM_AUTH,
                gptmail_base_url=GPTMAIL_BASE_URL,
                gptmail_api_key=GPTMAIL_API_KEY,
                gptmail_keys_file=GPTMAIL_KEYS_FILE,
                timeout_seconds=timeout_seconds,
            )
            return str(code), r
        except Exception as e:
            last_err = e
            last_err_str = str(e)

            # Normalize quota-like failures for caller.
            s = last_err_str.lower()
            if "all gptmail keys are exhausted" in s or "quota" in s or "daily quota" in s:
                raise RuntimeError("no_quota_for_otp")

            # If we are using GPTMail and just cannot get a code (deliverability/empty inbox),
            # treat it as a real repair failure.
            if "timeout waiting for 6-digit code" in s:
                raise RuntimeError("otp_timeout")

            continue

    raise RuntimeError(f"failed to fetch openai code from all mailbox_ref candidates: {last_err}")


def _repairer_drive_login_and_get_callback_url(*, driver, oauth: OAuthStart, email: str, password: str, mailbox_ref_candidates: list[str]) -> tuple[str, str]:
    """Drive OpenAI login flow until OAuth redirects to callback URL.

    Returns:
      (callback_url, chosen_mailbox_ref)
    """

    driver.get(oauth.auth_url)

    try:
        WebDriverWait(driver, 60).until(EC.url_contains("auth.openai.com"))
    except Exception:
        raise RuntimeError("did not reach auth.openai.com")

    # Step: fill email
    email_input = None
    try:
        email_input = smart_wait(driver, By.ID, "_r_f_-email", timeout=15)
    except Exception:
        email_input = _find_visible(driver, By.CSS_SELECTOR, 'input[type="email"]')

    if not email_input:
        raise RuntimeError("email input not found")

    try:
        email_input.clear()
    except Exception:
        pass
    email_input.send_keys(str(email))
    email_input.send_keys(Keys.ENTER)

    def _password_input():
        return _find_visible(driver, By.CSS_SELECTOR, 'input[type="password"]')

    # Some flows require clicking "Continue" then "Continue with password".
    for _ in range(60):
        pwd_inp = _password_input()
        if pwd_inp:
            try:
                pwd_inp.clear()
            except Exception:
                pass
            pwd_inp.send_keys(str(password))
            pwd_inp.send_keys(Keys.ENTER)
            break

        if _click_if_found(
            driver,
            "//button[contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'continue with password') or contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'password')]",
        ):
            time.sleep(0.6)
            continue

        _click_if_found(driver, "//button[contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'continue')]")
        time.sleep(0.6)

    def _has_callback() -> bool:
        try:
            return "localhost:1455" in str(getattr(driver, "current_url", "") or "")
        except Exception:
            return False

    def _code_input():
        # 常见单输入框
        selectors = [
            'input[id*="code"]',
            'input[name*="code"]',
            'input[autocomplete="one-time-code"]',
            'input[inputmode="numeric"][maxlength="6"]',
            'input[aria-label*="code" i]',
            'input[placeholder*="code" i]',
        ]

        for sel in selectors:
            el = _find_visible(driver, By.CSS_SELECTOR, sel)
            if el:
                return el

        # 常见分段输入框
        try:
            group = driver.find_elements(By.CSS_SELECTOR, 'div[role="group"] input[inputmode="numeric"][maxlength="1"]')
            if group:
                return group
        except Exception:
            pass

        return None

    def _has_risk_text_hint() -> bool:
        try:
            txt = str(driver.execute_script("return document && document.body ? (document.body.innerText || '') : ''; ") or "").lower()
        except Exception:
            txt = ""

        hints = [
            "verification code",
            "enter code",
            "check your email",
            "email a code",
            "send code",
            "verify it's you",
            "验证码",
            "发送验证码",
            "邮箱验证码",
            "请输入验证码",
        ]
        return any(h in txt for h in hints)

    def _click_send_code_if_needed() -> bool:
        """有风控时可能先出现“发送验证码”按钮，先触发发送，再等待输入框出现。"""

        send_code_xpaths = [
            "//*[self::button or self::a or self::span][contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'send code')]",
            "//*[self::button or self::a or self::span][contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'email me a code')]",
            "//*[self::button or self::a or self::span][contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'send verification')]",
            "//*[self::button or self::a or self::span][contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'get code')]",
            "//*[self::button or self::a or self::span][contains(., '发送验证码') or contains(., '验证码') or contains(., '发送代码')]",
            "//button[contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'continue') and not(contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'with'))]",
        ]

        for xp in send_code_xpaths:
            if _click_if_found(driver, xp):
                time.sleep(1.0)
                return True

        return False

    def _await_callback_or_code_stage() -> Any:
        """分类讨论：
        - 无风控：直接跳 callback
        - 有风控：先触发发送验证码，再等待 code input 出现
        """

        if _has_callback():
            return "CALLBACK"

        ci = _code_input()
        if ci:
            return ci

        # 如果页面存在风控提示，主动触发“发送验证码”按钮
        if _has_risk_text_hint():
            _click_send_code_if_needed()
            ci2 = _code_input()
            if ci2:
                return ci2

        return None

    v = _wait_for_any(driver, timeout_seconds=80, predicates=[_await_callback_or_code_stage])

    chosen_ref = ""

    if v != "CALLBACK":
        code, chosen_ref = _wait_code_try_candidates(candidates=mailbox_ref_candidates, timeout_seconds=180)

        if isinstance(v, list):
            for cur, digit in zip(v, str(code)):
                try:
                    _click_with_debug(driver, cur, tag="repairer_otp_digit_box", note="repairer segmented otp input")
                    cur.clear()
                except Exception:
                    pass
                cur.send_keys(str(digit))
            try:
                driver.switch_to.active_element.send_keys(Keys.ENTER)
            except Exception:
                pass
        else:
            try:
                v.clear()
            except Exception:
                pass
            v.send_keys(str(code))
            v.send_keys(Keys.ENTER)

    try:
        WebDriverWait(driver, 60).until(EC.url_contains("localhost:1455"))
    except Exception:
        raise RuntimeError("timeout waiting for oauth callback")

    return str(getattr(driver, "current_url", "") or ""), chosen_ref


def _repair_one_auth_file(path: str, *, proxy: str | None) -> tuple[bool, str, str | None]:
    """Repair one auth json file.

    Returns:
      (ok, reason, out_path)
    """

    name = os.path.basename(path)
    auth_obj = _read_json_any(path)

    if not isinstance(auth_obj, dict):
        return False, "invalid_json_not_object", None

    email = str(auth_obj.get("email") or "").strip()
    password = str(auth_obj.get("password") or "").strip()
    account_id = str(auth_obj.get("account_id") or "").strip()

    if not account_id:
        # fallback from nested claims field
        try:
            account_id = str((auth_obj.get("https://api.openai.com/auth") or {}).get("chatgpt_account_id") or "").strip()
        except Exception:
            account_id = ""

    if not email:
        return False, "missing_email", None
    if not password:
        return False, "missing_password", None

    # Prepare mailbox_ref candidates (encoded refs for mailbox_provider)
    candidates: list[str] = []

    # 1) previously persisted mailbox_ref (from our own submit_callback_url)
    mr0 = str(auth_obj.get("mailbox_ref") or "").strip()
    if mr0:
        candidates.append(mr0)

    # 2) best-effort: mailcreate jwt mint for existing address (preferred for repair)
    # If admin creds exist, we can poll MailCreate for OpenAI OTP reliably.

    # 3) best-effort: if user provides mailcreate admin creds, they can mint jwt for existing address
    # (Optional; errors ignored.)
    try:
        mc_custom = (
            os.environ.get("MAILCREATE_CUSTOM_AUTH")
            or _PLATFORMTOOLS_DEV_VARS.get("MAILCREATE_CUSTOM_AUTH")
            or str(_MAILCREATE_CFG.get("MAILCREATE_CUSTOM_AUTH") or "")
            or ""
        ).strip()
        mc_admin = (
            os.environ.get("MAILCREATE_ADMIN_AUTH")
            or _PLATFORMTOOLS_DEV_VARS.get("MAILCREATE_ADMIN_AUTH")
            or str(_MAILCREATE_CFG.get("MAILCREATE_ADMIN_AUTH") or "")
            or ""
        ).strip()
        if mc_custom and mc_admin and str(MAILCREATE_BASE_URL or "").strip():
            # admin endpoints
            base = str(MAILCREATE_BASE_URL).strip().rstrip("/")

            def _http_json(*, url: str, method: str = "GET", headers: dict[str, str] | None = None, payload: Any | None = None, timeout: int = 30) -> tuple[int, str, Any]:
                hdr = dict(headers or {})
                data = None
                if payload is not None:
                    data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
                    hdr.setdefault("Content-Type", "application/json")
                req = urllib.request.Request(url, data=data, headers=hdr, method=method)
                try:
                    with urllib.request.urlopen(req, timeout=timeout) as resp:
                        st = int(getattr(resp, "status", 200))
                        text = resp.read().decode("utf-8", errors="replace")
                        try:
                            obj = json.loads(text) if text else {}
                        except Exception:
                            obj = {}
                        return st, text, obj
                except urllib.error.HTTPError as e:
                    text = e.read().decode("utf-8", errors="replace")
                    try:
                        obj = json.loads(text) if text else {}
                    except Exception:
                        obj = {}
                    return int(getattr(e, "code", 0) or 0), text, obj

            admin_headers = {"x-custom-auth": mc_custom, "x-admin-auth": mc_admin, "Accept": "application/json"}
            q = urllib.parse.urlencode({"limit": "50", "offset": "0", "query": email})
            st1, _tx1, obj1 = _http_json(url=f"{base}/admin/address?{q}", method="GET", headers=admin_headers)
            addr_id = None
            if 200 <= st1 < 300 and isinstance(obj1, dict) and isinstance(obj1.get("results"), list):
                target = email.strip().lower()
                for it in obj1.get("results"):
                    if isinstance(it, dict) and str(it.get("name") or "").strip().lower() == target:
                        try:
                            addr_id = int(it.get("id"))
                        except Exception:
                            addr_id = None
                        break

            if addr_id:
                st2, _tx2, obj2 = _http_json(url=f"{base}/admin/show_password/{int(addr_id)}", method="GET", headers=admin_headers)
                if 200 <= st2 < 300 and isinstance(obj2, dict):
                    jwt = str(obj2.get("jwt") or "").strip()
                    if jwt:
                        candidates.append(f"mailcreate:{jwt}")
    except Exception:
        pass

    # 4) last resort: try gptmail by email
    candidates.append(f"gptmail:{email}")

    # de-dup candidates, keep order
    seen: set[str] = set()
    candidates = [c for c in candidates if c and (c not in seen and not seen.add(c))]

    # probe for log (optional)
    try:
        result = _probe_wham_one(auth_obj=auth_obj, proxy=None)
    except Exception:
        result = ProbeResult(status_code=None, note="probe_failed", category="probe_failed")

    _append_jsonl(
        os.path.join(_repairer_results_dir(), "repairer_probe.jsonl"),
        {
            "ts": _utc_now_iso(),
            "file": name,
            "account_id": account_id,
            "email": email,
            "status_code": result.status_code,
            "note": result.note,
            "probe_category": result.category,
            "retry_after_seconds": result.retry_after_seconds,
            "upstream_status": result.http_status,
        },
    )

    driver = None
    proxy_dir = None
    try:
        with driver_init_lock:
            driver, proxy_dir = new_driver(proxy)

        oauth = generate_oauth_url()
        callback_url, chosen_ref = _repairer_drive_login_and_get_callback_url(
            driver=driver,
            oauth=oauth,
            email=email,
            password=password,
            mailbox_ref_candidates=candidates,
        )

        # exchange callback -> new token json
        reg_email, config_json = submit_callback_url(
            callback_url=callback_url,
            expected_state=oauth.state,
            code_verifier=oauth.code_verifier,
            redirect_uri=oauth.redirect_uri,
            proxy=proxy,
            mailbox_ref=(chosen_ref or (mr0 or "")),
            password=password,
            first_name=str(auth_obj.get("first_name") or ""),
            last_name=str(auth_obj.get("last_name") or ""),
            birthdate=str(auth_obj.get("birthdate") or ""),
        )

        try:
            new_obj = json.loads(config_json)
        except Exception:
            new_obj = {}

        merged = _deep_merge_keep_old_when_missing(auth_obj, new_obj)

        # Write outputs
        # 约定：修缮成功后不写入本地 token 池（禁止进入 codex_auth）。
        # 成功产物只进入 fixed_success，后续由 uploader 负责上传 fixed_success 目录。
        ts_ms = int(time.time() * 1000)
        rand = secrets.token_hex(3)
        safe_acc = re.sub(r"[^a-zA-Z0-9_.-]+", "_", (account_id or "unknown"))[:64] or "unknown"
        out_name = f"codex-repaired-{safe_acc}-{INSTANCE_ID}-{ts_ms}-{rand}.json"

        fixed_success_path = os.path.join(_data_path(FIXED_SUCCESS_DIRNAME), out_name)

        with write_lock:
            os.makedirs(_data_path(FIXED_SUCCESS_DIRNAME), exist_ok=True)
            _write_json_any(fixed_success_path, merged)

        _append_jsonl(
            os.path.join(_repairer_results_dir(), "repairer_success.jsonl"),
            {"ts": _utc_now_iso(), "file": name, "account_id": account_id, "email": reg_email, "out": out_name},
        )

        return True, "ok", fixed_success_path

    finally:
        if driver:
            try:
                driver.quit()
            except Exception:
                pass
        if proxy_dir and os.path.exists(proxy_dir):
            try:
                shutil.rmtree(proxy_dir, ignore_errors=True)
            except Exception:
                pass


def _repairer_restore_claimed_for_test(*, claimed: str, name: str) -> None:
    """测试模式：将 _processing 中的输入文件放回 need_fix_auth，方便重复测试。"""

    try:
        dst = os.path.join(_data_path(NEED_FIX_AUTH_DIRNAME), name)
        os.replace(claimed, dst)
    except Exception:
        # 回退：若原子替换失败，尽量复制保留样本
        try:
            shutil.copy2(claimed, os.path.join(_data_path(NEED_FIX_AUTH_DIRNAME), name))
        except Exception:
            pass
        try:
            os.remove(claimed)
        except Exception:
            pass



def _repairer_loop() -> None:
    need, proc, okd, bad = _repairer_dirs()
    os.makedirs(need, exist_ok=True)
    os.makedirs(proc, exist_ok=True)
    os.makedirs(okd, exist_ok=True)
    os.makedirs(bad, exist_ok=True)
    os.makedirs(_repairer_results_dir(), exist_ok=True)

    print(f"[repairer] enabled=1 need_fix_dir={need}")
    print(f"[repairer] poll_seconds={REPAIRER_POLL_SECONDS}")
    print(f"[repairer] test_keep_input={REPAIRER_TEST_KEEP_INPUT}")

    stale_seconds = int(os.environ.get("REPAIRER_STALE_SECONDS", "1800"))
    processed_once_in_test: set[str] = set()

    while True:
        try:
            _repairer_release_stale_processing(stale_seconds=stale_seconds)

            claimed = _repairer_claim_one_file()
            if not claimed:
                time.sleep(REPAIRER_POLL_SECONDS)
                continue

            name = os.path.basename(claimed)

            # 测试模式下：每个文件每次进程仅处理一次，避免无限循环刷同一文件。
            if REPAIRER_TEST_KEEP_INPUT == 1 and name in processed_once_in_test:
                _repairer_restore_claimed_for_test(claimed=claimed, name=name)
                time.sleep(REPAIRER_POLL_SECONDS)
                continue

            proxies = load_proxies()
            proxy = random.choice(proxies) if proxies else None

            ok = False
            out_path = None
            reason = ""
            try:
                ok, reason, out_path = _repair_one_auth_file(claimed, proxy=proxy)
            except Exception as e:
                ok = False
                reason = f"exception:{e}"

            if ok:
                # 正常模式：成功后删除；测试模式：回放到 need_fix_auth 便于重复测试。
                try:
                    if REPAIRER_TEST_KEEP_INPUT == 1:
                        _repairer_restore_claimed_for_test(claimed=claimed, name=name)
                        processed_once_in_test.add(name)
                    else:
                        os.remove(claimed)
                except Exception:
                    pass
                print(f"[repairer] ok file={name} out={out_path}")
                continue

            # Special: OTP quota exhausted. This is NOT an unrecoverable repair failure.
            if "no_quota_for_otp" in (reason or ""):
                _append_jsonl(
                    os.path.join(_repairer_results_dir(), "repairer_no_quota.jsonl"),
                    {"ts": _utc_now_iso(), "file": name, "reason": reason},
                )
                # 正常模式：删除；测试模式：回放输入样本，且不重复处理。
                try:
                    if REPAIRER_TEST_KEEP_INPUT == 1:
                        _repairer_restore_claimed_for_test(claimed=claimed, name=name)
                        processed_once_in_test.add(name)
                    else:
                        os.remove(claimed)
                except Exception:
                    pass
                print(f"[repairer] skip(no_quota) file={name}")
                continue

            # If we had quota to access mailbox API, but still couldn't fetch OTP (timeout),
            # this is considered a real "unrepairable" attempt and should be reported.
            # (The code path sets reason="exception:otp_timeout".)

            # failure policy:
            # - 向服务端上报“该 account_id 无法修缮”（服务端累计 3 次进墓地）
            # - 本地不归档 fixed_fail；直接删除队列文件（避免无限重试）
            try:
                auth_obj = _read_json_any(claimed)
            except Exception:
                auth_obj = {}

            acc = ""
            try:
                acc = str(auth_obj.get("account_id") or "").strip() if isinstance(auth_obj, dict) else ""
            except Exception:
                acc = ""
            if not acc:
                try:
                    acc = str((auth_obj.get("https://api.openai.com/auth") or {}).get("chatgpt_account_id") or "").strip() if isinstance(auth_obj, dict) else ""
                except Exception:
                    acc = ""

            report_ok = False
            report_http = 0
            report_resp = ""
            if acc:
                try:
                    report_ok, report_http, report_resp = _report_auth_repair_failed(account_id=acc, note=reason[:1000])
                except Exception as e:
                    report_ok, report_http, report_resp = False, 0, f"exception:{e}"

            _append_jsonl(
                os.path.join(_repairer_results_dir(), "repairer_failed.jsonl"),
                {
                    "ts": _utc_now_iso(),
                    "file": name,
                    "account_id": acc,
                    "reason": reason,
                    "report_ok": report_ok,
                    "http": report_http,
                    "resp": str(report_resp or "")[:800],
                },
            )

            try:
                if REPAIRER_TEST_KEEP_INPUT == 1:
                    _repairer_restore_claimed_for_test(claimed=claimed, name=name)
                    processed_once_in_test.add(name)
                else:
                    os.remove(claimed)
            except Exception:
                pass

            print(f"[repairer] fail file={name} reason={reason} report_ok={report_ok} http={report_http}")

        except Exception as e:
            print(f"[repairer] loop error: {e}")

        time.sleep(0.2)


def load_proxies() -> list[str]:
    proxy_file = _data_path("proxies.txt")
    if os.path.exists(proxy_file):
        with open(proxy_file, "r", encoding="utf-8") as f:
            proxies = [line.strip() for line in f if line.strip() and not line.startswith("#")]
        return proxies
    return []

def worker(worker_id: int):
    while True:
        proxies = load_proxies()
        proxy = random.choice(proxies) if proxies else None

        if proxy:
            print(f"[Worker {worker_id}] ---> 使用代理: {proxy} <---")
        elif REGISTER_PROXY_REQUIRED:
            print(
                f"[Worker {worker_id}] [x] register_proxy_required no_proxy_available "
                f"flow={REGISTER_FLOW_MODE} headless={int(os.environ.get('HEADLESS', '1') or '1')}"
            )
            time.sleep(1.0)
            continue
        else:
            print(f"[Worker {worker_id}] ---> 未配置可用代理，使用本地网络直连 <---")

        driver = None
        proxy_dir = None
        try:
            if REGISTER_FLOW_MODE == "protocol":
                print(f"[Worker {worker_id}] ---> 协议流模式（REGISTER_FLOW_MODE=protocol） <---")
                reg_email, res = register_protocol(proxy)
            else:
                with driver_init_lock:
                    driver, proxy_dir = new_driver(proxy)
                reg_email, res = register(driver, proxy)
            
            # Write outputs (sharded results + per-account json)
            with write_lock:
                _append_result_line(res)

                # Write per-account auth json file into codex_auth/
                codex_auth_dir = _data_path(CODEX_AUTH_DIRNAME)
                os.makedirs(codex_auth_dir, exist_ok=True)
                # Use a unique filename to avoid collisions when multiple containers
                # share the same data volume.
                ts_ms = int(time.time() * 1000)
                rand = secrets.token_hex(3)
                auth_path = os.path.join(
                    codex_auth_dir,
                    f"codex-{reg_email}-free-{INSTANCE_ID}-{ts_ms}-{rand}.json",
                )
                with open(auth_path, "w", encoding="utf-8") as f:
                    f.write(json.dumps(json.loads(res), indent=2, ensure_ascii=False))

                # 可选：配置同步（把 codex_auth 写入同步目录）
                try:
                    _sync_codex_auth_copy(src_path=auth_path)
                except Exception:
                    pass

                # Also copy into wait_update/ for downstream pickup
                wait_update_dir = _data_path(WAIT_UPDATE_DIRNAME)
                os.makedirs(wait_update_dir, exist_ok=True)
                try:
                    shutil.copy2(auth_path, os.path.join(wait_update_dir, os.path.basename(auth_path)))
                except Exception:
                    pass
                    
            print(
                f"[Worker {worker_id}] [✓] 注册成功，Token 已保存在 {CODEX_AUTH_DIRNAME} 并复制到 {WAIT_UPDATE_DIRNAME}，并追加到 results 分片！"
            )
            
        except RuntimeError as e:
            # Expected blocks, no stack trace needed
            print(f"[Worker {worker_id}] [x] {e} (准备换IP重试)")
        except TimeoutException as e:
            print(f"[Worker {worker_id}] [x] 页面加载超时，可能遇到风控盾拦截。 (准备换IP重试)")
        except Exception as e:
            err_str = str(e)
            if "RemoteDisconnected" in err_str or "Connection aborted" in err_str or "Max retries exceeded" in err_str or "UNEXPECTED_EOF_WHILE_READING" in err_str or "UNEXPECTED_MESSAGE" in err_str:
                print(f"[Worker {worker_id}] [x] 代理连接强制中断 (SSL/EOF断流)，准备换IP重试")
            else:
                import traceback
                trace_str = traceback.format_exc()
                print(f"[Worker {worker_id}] [x] 本次注册流程意外中止:\\n{trace_str}")
            
        finally:
            if driver:
                try:
                    driver.quit()
                except Exception:
                    pass
            if proxy_dir and os.path.exists(proxy_dir):
                shutil.rmtree(proxy_dir, ignore_errors=True)
        
        # 自由调整休眠时间
        sleep_min = int(os.environ.get("SLEEP_MIN", "5"))
        sleep_max = int(os.environ.get("SLEEP_MAX", "20"))
        sleep_time = random.randint(sleep_min, sleep_max) if sleep_max >= sleep_min else sleep_min
        print(f"[Worker {worker_id}] 任务结束。挂起 {sleep_time} 秒后开启下一轮尝试...")
        time.sleep(sleep_time)

if __name__ == "__main__":
    os.makedirs(DATA_DIR, exist_ok=True)

    # Per-instance dirs (safe for multi-container shared volume)
    os.makedirs(_results_dir(), exist_ok=True)
    os.makedirs(_data_path(ERROR_DIRNAME, INSTANCE_ID), exist_ok=True)

    # Shared dirs
    os.makedirs(_data_path(CODEX_AUTH_DIRNAME), exist_ok=True)
    os.makedirs(_data_path(WAIT_UPDATE_DIRNAME), exist_ok=True)
    os.makedirs(_data_path(NEED_FIX_AUTH_DIRNAME), exist_ok=True)
    os.makedirs(_data_path(FIXED_SUCCESS_DIRNAME), exist_ok=True)
    os.makedirs(_data_path(FIXED_FAIL_DIRNAME), exist_ok=True)

    # Move any legacy root results shards/state into this instance dir.
    _migrate_legacy_results_layout()

    proxy_file = _data_path("proxies.txt")
    
    if not os.path.exists(proxy_file):
        with open(proxy_file, "w", encoding="utf-8") as f:
            f.write("# 在此文件中添加您的代理IP池，每行一个\n")
            f.write("# 格式示例: http://192.168.1.100:8080\n")
            
    concurrency = int(os.environ.get("CONCURRENCY", "1"))
    if concurrency < 0:
        concurrency = 0

    # 方案A：同一容器同时做生产 + 探测/续杯（可通过 env 关闭）
    if ENABLE_PROBE == 1:
        try:
            t = threading.Thread(target=_probe_loop, name="probe_loop", daemon=True)
            t.start()
        except Exception as e:
            print(f"[probe] failed to start probe thread: {e}")

    # 修缮者：同一进程内后台跑 need_fix_auth 修复循环
    if ENABLE_REPAIRER == 1:
        try:
            t2 = threading.Thread(target=_repairer_loop, name="repairer_loop", daemon=True)
            t2.start()
        except Exception as e:
            print(f"[repairer] failed to start repairer thread: {e}")

    print(f"==== 守护进程启动: 无限循环多线程生成器 (并发数: {concurrency}) ====")
    print(f"INSTANCE_ID={INSTANCE_ID}")
    print(f"results 分片将写入 {_results_dir()} (每 {RESULTS_SHARD_SIZE} 条一片)")
    print(f"账号 JSON 将写入 {_data_path(CODEX_AUTH_DIRNAME)} 并复制到 {_data_path(WAIT_UPDATE_DIRNAME)}")
    print(f"代理池请直接写入 {proxy_file}")
    
    if concurrency > 0:
        with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as executor:
            for i in range(concurrency):
                executor.submit(worker, i + 1)
                # 错开启动时间，避免瞬间打满并发
                time.sleep(random.randint(2, 5))
    else:
        # Allow running repairer/probe-only mode without starting register workers.
        while True:
            time.sleep(3600)

