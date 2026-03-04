from __future__ import annotations

import json
import os
import sys
from typing import Any

# Ensure repo root importable when called from sub-scripts.
_REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if _REPO_ROOT not in sys.path:
    sys.path.insert(0, _REPO_ROOT)

try:
    from platformtools._shared.dev_vars import load_platformtools_dev_vars
except Exception:
    load_platformtools_dev_vars = None  # type: ignore

_PLATFORMTOOLS_DEV_VARS = (
    load_platformtools_dev_vars(start_dir=os.path.dirname(__file__)) if load_platformtools_dev_vars else {}
)

# mailbox_provider is in platformtools/mailcreate/client
_PLAT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
_MAILCREATE_CLIENT_DIR = os.path.join(_PLAT_DIR, "mailcreate", "client")
if _MAILCREATE_CLIENT_DIR not in sys.path:
    sys.path.insert(0, _MAILCREATE_CLIENT_DIR)

from mailbox_provider import Mailbox, create_mailbox, wait_openai_code as wait_openai_code_by_provider  # type: ignore


def _load_json_config(path: str) -> dict:
    try:
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f)
    except FileNotFoundError:
        return {}
    except Exception:
        return {}


DATA_DIR = (os.environ.get("DATA_DIR") or os.path.join(os.path.dirname(__file__), "data")).strip()
if not DATA_DIR:
    DATA_DIR = os.path.join(os.path.dirname(__file__), "data")

MAILBOX_PROVIDER = os.environ.get("MAILBOX_PROVIDER", "auto").strip().lower()

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

# Mail.tm provider config
MAILTM_API_BASE = (
    os.environ.get("MAILTM_API_BASE")
    or _PLATFORMTOOLS_DEV_VARS.get("MAILTM_API_BASE")
    or "https://api.mail.tm"
).strip()

_MAIL_DOMAIN_HEALTH_ORDER = [
    d.strip().lower()
    for d in (
        os.environ.get("MAIL_DOMAIN_HEALTH_ORDER")
        or "mail.aiaimimi.com,aimiaimi.cc.cd,mimiaiai.cc.cd,aiaimimi.cc.cd,aiaiai.cc.cd"
    ).split(",")
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
            mailtm_api_base=MAILTM_API_BASE,
        )

    tries = max(1, _MAILBOX_PICK_TRIES)
    tries = min(tries, len(domains))
    picked_domains = __import__("random").sample(domains, k=tries)

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
                mailtm_api_base=MAILTM_API_BASE,
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
            mailtm_api_base=MAILTM_API_BASE,
        )

    raise RuntimeError("failed to pick mailcreate domain")


def create_temp_mailbox_shared() -> tuple[str, str]:
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
            mailtm_api_base=MAILTM_API_BASE,
        )
        if getattr(mb, "provider", "") == "mailcreate":
            try:
                mb = _pick_mailcreate_with_health()
            except Exception:
                pass

    return mb.email, mb.ref


def wait_openai_code_shared(*, mailbox_ref: str, timeout_seconds: int = 180) -> str:
    return wait_openai_code_by_provider(
        provider=MAILBOX_PROVIDER,
        mailbox_ref=mailbox_ref,
        mailcreate_base_url=MAILCREATE_BASE_URL,
        mailcreate_custom_auth=MAILCREATE_CUSTOM_AUTH,
        gptmail_base_url=GPTMAIL_BASE_URL,
        gptmail_api_key=GPTMAIL_API_KEY,
        gptmail_keys_file=GPTMAIL_KEYS_FILE,
        mailtm_api_base=MAILTM_API_BASE,
        timeout_seconds=timeout_seconds,
    )
