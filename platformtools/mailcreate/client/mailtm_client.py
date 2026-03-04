"""Mail.tm Client (api.mail.tm)

This module implements a client for the Mail.tm public temporary email service.
Mail.tm uses the Hydra API format for its REST endpoints.

It is intended to be used as a third mailbox provider alongside MailCreate and GPTMail.

API summary:
- GET  /domains          → list available domains (Hydra format)
- POST /accounts         → create account {"address": ..., "password": ...}
- POST /token            → authenticate    {"address": ..., "password": ...} → {"token": "..."}
- GET  /messages         → list messages    (Bearer auth, Hydra format)
- GET  /messages/{id}    → read message     (Bearer auth)

Auth:
- Bearer token obtained via POST /token

NOTE:
- Mail.tm is a free public service; domains and availability may change without notice.
- Some Mail.tm domains may already be blocked by OpenAI signup.
- Rate limits may apply for high-volume usage.
"""

from __future__ import annotations

import json
import re
import secrets
import time
import urllib.parse
import urllib.request
from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Tuple


MAILTM_DEFAULT_BASE = "https://api.mail.tm"


class MailTmError(RuntimeError):
    def __init__(self, message: str, *, status: Optional[int] = None):
        super().__init__(message)
        self.status = status


@dataclass
class MailTmConfig:
    api_base: str = MAILTM_DEFAULT_BASE
    timeout_seconds: int = 15


# ---------------------------------------------------------------------------
# Low-level HTTP helpers (stdlib only, same pattern as gptmail_client.py)
# ---------------------------------------------------------------------------

def _headers(*, token: str = "", use_json: bool = False) -> Dict[str, str]:
    h: Dict[str, str] = {
        "Accept": "application/json",
        "User-Agent": "mailtm-client/0.1",
    }
    if use_json:
        h["Content-Type"] = "application/json"
    if token:
        h["Authorization"] = f"Bearer {token}"
    return h


def _req(
    cfg: MailTmConfig,
    method: str,
    path: str,
    *,
    token: str = "",
    json_body: Optional[Dict[str, Any]] = None,
) -> Tuple[int, str]:
    url = cfg.api_base.rstrip("/") + path

    hdrs = _headers(token=token, use_json=(json_body is not None))

    body_bytes: Optional[bytes] = None
    if json_body is not None:
        body_bytes = json.dumps(json_body).encode("utf-8")

    req = urllib.request.Request(url, data=body_bytes, headers=hdrs, method=method)

    try:
        with urllib.request.urlopen(req, timeout=cfg.timeout_seconds) as resp:
            text = resp.read().decode("utf-8", errors="replace")
            return resp.getcode(), text
    except urllib.error.HTTPError as e:
        text = e.read().decode("utf-8", errors="replace")
        return e.code, text
    except Exception as e:
        raise MailTmError(f"request failed: {method} {url}: {e}") from e


# ---------------------------------------------------------------------------
# Mail.tm Client
# ---------------------------------------------------------------------------

class MailTmClient:
    def __init__(self, cfg: Optional[MailTmConfig] = None):
        self.cfg = cfg or MailTmConfig()

    # -- domains -----------------------------------------------------------

    def get_domains(self) -> List[str]:
        """Fetch available public domains from Mail.tm."""
        status, text = _req(self.cfg, "GET", "/domains")
        if status != 200:
            raise MailTmError(f"get_domains failed: status={status} body={text[:300]}", status=status)

        try:
            data = json.loads(text)
        except Exception:
            raise MailTmError(f"get_domains invalid JSON: {text[:300]}", status=status)

        # Hydra format: {"hydra:member": [...]} or plain list
        items: list = []
        if isinstance(data, list):
            items = data
        elif isinstance(data, dict):
            items = data.get("hydra:member") or data.get("items") or data.get("member") or []

        domains: List[str] = []
        for item in items:
            if not isinstance(item, dict):
                continue
            domain = str(item.get("domain") or "").strip()
            is_active = item.get("isActive", True)
            is_private = item.get("isPrivate", False)
            if domain and is_active and not is_private:
                domains.append(domain)

        return domains

    # -- account -----------------------------------------------------------

    def create_account(self, email: str, password: str) -> Dict[str, Any]:
        """Create a Mail.tm account."""
        status, text = _req(
            self.cfg, "POST", "/accounts",
            json_body={"address": email, "password": password},
        )
        if status not in (200, 201):
            raise MailTmError(f"create_account failed: status={status} body={text[:300]}", status=status)

        try:
            return json.loads(text)
        except Exception:
            raise MailTmError(f"create_account invalid JSON: {text[:300]}", status=status)

    def get_token(self, email: str, password: str) -> str:
        """Authenticate and obtain a Bearer token."""
        status, text = _req(
            self.cfg, "POST", "/token",
            json_body={"address": email, "password": password},
        )
        if status != 200:
            raise MailTmError(f"get_token failed: status={status} body={text[:300]}", status=status)

        try:
            data = json.loads(text)
        except Exception:
            raise MailTmError(f"get_token invalid JSON: {text[:300]}", status=status)

        token = str(data.get("token") or "").strip()
        if not token:
            raise MailTmError(f"get_token returned empty token: {data}", status=status)
        return token

    # -- convenience: create mailbox in one shot ---------------------------

    def create_mailbox(self, *, max_retries: int = 5) -> Tuple[str, str, str]:
        """Create a new mailbox: returns (email, token, password).

        1. Fetch available domains
        2. Generate random local part
        3. Create account + obtain token
        """
        domains = self.get_domains()
        if not domains:
            raise MailTmError("no available domains from Mail.tm")

        import random

        last_err: Optional[Exception] = None
        for _ in range(max_retries):
            local = f"oc{secrets.token_hex(6)}"
            domain = random.choice(domains)
            email = f"{local}@{domain}"
            password = secrets.token_urlsafe(18)

            try:
                self.create_account(email, password)
            except MailTmError as e:
                last_err = e
                # 422 usually means address taken or invalid; retry with new address
                if e.status in (422, 409, 400):
                    continue
                raise

            try:
                token = self.get_token(email, password)
            except MailTmError as e:
                last_err = e
                continue

            return email, token, password

        raise MailTmError(f"create_mailbox failed after {max_retries} retries: {last_err}")

    # -- messages ----------------------------------------------------------

    def list_messages(self, *, token: str) -> List[Dict[str, Any]]:
        """List messages in the inbox."""
        status, text = _req(self.cfg, "GET", "/messages", token=token)
        if status != 200:
            raise MailTmError(f"list_messages failed: status={status}", status=status)

        try:
            data = json.loads(text)
        except Exception:
            return []

        # Hydra format
        if isinstance(data, list):
            messages = data
        elif isinstance(data, dict):
            messages = data.get("hydra:member") or data.get("messages") or data.get("member") or []
        else:
            messages = []

        return [m for m in messages if isinstance(m, dict)]

    def get_message(self, *, token: str, msg_id: str) -> Dict[str, Any]:
        """Read a single message by ID."""
        # Mail.tm uses /messages/{id} but the id may contain /api/messages/ prefix
        path = msg_id if msg_id.startswith("/") else f"/messages/{msg_id}"
        status, text = _req(self.cfg, "GET", path, token=token)
        if status != 200:
            return {}

        try:
            data = json.loads(text)
            return data if isinstance(data, dict) else {}
        except Exception:
            return {}


# ---------------------------------------------------------------------------
# OTP polling
# ---------------------------------------------------------------------------

_CODE_RE = re.compile(r"(?<!\d)(\d{6})(?!\d)")


def wait_for_6digit_code_mailtm(
    client: MailTmClient,
    *,
    token: str,
    email: str,
    from_contains: Optional[str] = None,
    timeout_seconds: int = 120,
    poll_seconds: float = 3.0,
) -> str:
    """Poll Mail.tm inbox for a 6-digit verification code from OpenAI."""
    deadline = time.time() + timeout_seconds
    seen_ids: set[str] = set()
    poll_rounds = 0

    while time.time() < deadline:
        poll_rounds += 1
        try:
            messages = client.list_messages(token=token)
        except Exception:
            time.sleep(poll_seconds)
            continue

        for msg in messages:
            if not isinstance(msg, dict):
                continue
            msg_id = str(msg.get("id") or msg.get("@id") or "").strip()
            if not msg_id or msg_id in seen_ids:
                continue
            seen_ids.add(msg_id)

            # Check sender filter
            if from_contains:
                sender_obj = msg.get("from") or {}
                if isinstance(sender_obj, dict):
                    sender = str(sender_obj.get("address") or "").lower()
                else:
                    sender = str(sender_obj or "").lower()
                if from_contains.lower() not in sender:
                    continue

            # Quick check intro/subject
            for field in ("subject", "intro"):
                text = str(msg.get(field) or "")
                m = _CODE_RE.search(text)
                if m:
                    return m.group(1)

            # Fetch full message for text/html
            try:
                detail = client.get_message(token=token, msg_id=msg_id)
            except Exception:
                continue

            for field in ("subject", "intro", "text", "html"):
                raw = detail.get(field)
                if raw is None:
                    continue
                if isinstance(raw, list):
                    raw = "\n".join(str(x) for x in raw)
                text = str(raw or "")
                m = _CODE_RE.search(text)
                if m:
                    return m.group(1)

        time.sleep(poll_seconds)

    raise MailTmError(
        f"timeout waiting for 6-digit code "
        f"(timeout={timeout_seconds}s rounds={poll_rounds} seen_ids={len(seen_ids)} "
        f"from_contains={from_contains!r} email={email})"
    )
