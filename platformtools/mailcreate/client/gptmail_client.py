"""GPTMail Client (mail.chatgpt.org.uk)

This module implements a thin wrapper around the public GPTMail API described at:
- https://www.chatgpt.org.uk/2025/11/gptmailapiapi.html

It is intended to be used as an alternative/backup mailbox provider.

API summary:
- POST /api/generate-email
- GET  /api/emails?email=<address>
- GET  /api/email/<id>

Auth:
- Header: X-API-Key: <key>

Response format:
- {"success": bool, "data": ..., "error": ""}

NOTE:
- The public test key `gpt-test` may hit quota limits (429-like behavior surfaced as
  {success:false,error:"Daily quota exceeded"}).
"""

from __future__ import annotations

import json
import re
import time
import urllib.parse
import urllib.request
from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Tuple


class GPTMailError(RuntimeError):
    def __init__(self, message: str, *, status: Optional[int] = None, payload: Optional[Dict[str, Any]] = None):
        super().__init__(message)
        self.status = status
        self.payload = payload or {}


@dataclass
class GPTMailConfig:
    base_url: str = "https://mail.chatgpt.org.uk"
    api_key: str = ""
    timeout_seconds: int = 30


def _req(
    cfg: GPTMailConfig,
    method: str,
    path: str,
    *,
    query: Optional[Dict[str, str]] = None,
    json_body: Optional[Dict[str, Any]] = None,
) -> Tuple[int, str, Dict[str, str]]:
    url = cfg.base_url.rstrip("/") + path
    if query:
        url = url + ("?" + urllib.parse.urlencode(query))

    headers = {
        "User-Agent": "gptmail-client/0.1",
        "Accept": "application/json, text/plain, */*",
    }
    if cfg.api_key:
        headers["X-API-Key"] = cfg.api_key

    body_bytes: Optional[bytes] = None
    if json_body is not None:
        headers["Content-Type"] = "application/json"
        body_bytes = json.dumps(json_body).encode("utf-8")

    req = urllib.request.Request(url, data=body_bytes, headers=headers, method=method)

    try:
        with urllib.request.urlopen(req, timeout=cfg.timeout_seconds) as resp:
            text = resp.read().decode("utf-8", errors="replace")
            return resp.getcode(), text, dict(resp.headers)
    except urllib.error.HTTPError as e:
        text = e.read().decode("utf-8", errors="replace")
        return e.code, text, dict(e.headers)
    except Exception as e:
        raise GPTMailError(f"request failed: {method} {url}: {e}") from e


def _parse_envelope(status: int, text: str) -> Dict[str, Any]:
    try:
        data = json.loads(text or "{}")
    except Exception:
        raise GPTMailError(f"invalid JSON response (status={status}): {text[:300]}")

    if isinstance(data, dict) and data.get("success") is True:
        return data

    err = ""
    if isinstance(data, dict):
        err = str(data.get("error") or "")

    # Normalize common quota error
    if "quota" in err.lower() or "Daily quota exceeded" in err:
        raise GPTMailError(err or "quota exceeded", status=status, payload=data)

    raise GPTMailError(err or f"request failed (status={status})", status=status, payload=data)


class GPTMailClient:
    def __init__(self, cfg: GPTMailConfig):
        if not cfg.api_key:
            raise GPTMailError("GPTMail api_key is required (X-API-Key)")
        self.cfg = cfg

    def generate_email(self, *, prefix: Optional[str] = None, domain: Optional[str] = None) -> str:
        payload: Dict[str, Any] = {}
        if prefix:
            payload["prefix"] = prefix
        if domain:
            payload["domain"] = domain

        # GPTMail supports GET(random) or POST(specified). We'll use POST for deterministic prefix.
        status, text, _ = _req(self.cfg, "POST", "/api/generate-email", json_body=payload)
        env = _parse_envelope(status, text)
        email = str(((env.get("data") or {}) if isinstance(env, dict) else {}).get("email") or "").strip()
        if not email:
            raise GPTMailError(f"generate_email returned empty email: {env}", status=status, payload=env)
        return email

    def list_emails(self, *, email: str) -> List[Dict[str, Any]]:
        status, text, _ = _req(self.cfg, "GET", "/api/emails", query={"email": email})
        env = _parse_envelope(status, text)
        data = env.get("data") or {}
        emails = data.get("emails") or []
        if not isinstance(emails, list):
            return []
        return [e for e in emails if isinstance(e, dict)]

    def get_email(self, *, mail_id: str) -> Dict[str, Any]:
        status, text, _ = _req(self.cfg, "GET", f"/api/email/{urllib.parse.quote(str(mail_id))}")
        env = _parse_envelope(status, text)
        data = env.get("data")
        return data if isinstance(data, dict) else {}


_CODE_RE = re.compile(r"(?<!\d)(\d{6})(?!\d)")


def extract_6digit_code(text: str) -> Optional[str]:
    m = _CODE_RE.search(text or "")
    return m.group(1) if m else None


def wait_for_6digit_code_gptmail(
    client: GPTMailClient,
    *,
    email: str,
    from_contains: Optional[str] = None,
    timeout_seconds: int = 120,
    poll_seconds: float = 3.0,
) -> str:
    deadline = time.time() + timeout_seconds
    seen_ids: set[str] = set()

    while time.time() < deadline:
        items = client.list_emails(email=email)

        for e in items:
            mail_id = str(e.get("id") or "").strip()
            if not mail_id or mail_id in seen_ids:
                continue
            seen_ids.add(mail_id)

            if from_contains:
                src = str(e.get("from_address") or e.get("from") or "")
                if from_contains.lower() not in src.lower():
                    continue

            # subject/content/html_content
            for k in ("subject", "content", "html_content"):
                code = extract_6digit_code(str(e.get(k) or ""))
                if code:
                    return code

            # fetch detail raw_content if needed
            detail = client.get_email(mail_id=mail_id)
            for k in ("subject", "content", "html_content", "raw_content"):
                code = extract_6digit_code(str(detail.get(k) or ""))
                if code:
                    return code

        time.sleep(poll_seconds)

    raise GPTMailError(f"timeout waiting for 6-digit code (timeout={timeout_seconds}s)")
