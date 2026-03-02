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
import quopri
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


# Prefer deterministic extraction:
# 1) HTML <title> 中的 6 位数字（用户指定优先）
# 2) 语义锚点后的 6 位 OTP
_TITLE_6_RE = re.compile(r"(?is)<title[^>]*>.*?(\d{6}).*?</title>")
_CODE_CONTEXT_RE = re.compile(
    r"(?is)(?:verification\s*code|verify\s*code|security\s*code|one[-\s]*time\s*code|otp\s*code|验证码|校验码|代码为|代码是|code\s*(?:is|:))[^0-9]{0,40}(\d{6})"
)


def _normalize_qp_soft_breaks(s: str) -> str:
    return (s or "").replace("=\r\n", "").replace("=\n", "")


def _decode_quoted_printable(s: str) -> str:
    try:
        b = (s or "").encode("utf-8", errors="ignore")
        return quopri.decodestring(b).decode("utf-8", errors="ignore")
    except Exception:
        return s or ""


def extract_6digit_code(text: str) -> Optional[str]:
    raw = text or ""
    s1 = _normalize_qp_soft_breaks(raw)
    s2 = _normalize_qp_soft_breaks(_decode_quoted_printable(raw))

    for s in (s1, s2):
        m_title = _TITLE_6_RE.search(s)
        if m_title:
            return m_title.group(1)

    for s in (s1, s2):
        m_ctx = _CODE_CONTEXT_RE.search(s)
        if m_ctx:
            return m_ctx.group(1)

    return None


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
    poll_rounds = 0
    last_batch_size = 0
    last_sources: List[str] = []

    while time.time() < deadline:
        poll_rounds += 1
        items = client.list_emails(email=email)

        # Prefer newest mails first.
        try:
            items = sorted(items, key=lambda it: int(str(it.get("id") or "0")), reverse=True)
        except Exception:
            pass

        last_batch_size = len(items)
        last_sources = []
        for _e in items[:5]:
            try:
                _src = str((_e.get("from_address") or _e.get("from") or "")).strip()
                _sid = str(_e.get("id") or "").strip()
                if _src or _sid:
                    last_sources.append(f"{_sid}:{_src}")
            except Exception:
                continue

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

    raise GPTMailError(
        "timeout waiting for 6-digit code "
        f"(timeout={timeout_seconds}s rounds={poll_rounds} seen_ids={len(seen_ids)} "
        f"last_batch={last_batch_size} from_contains={from_contains!r} "
        f"email={email} last_sources={last_sources})"
    )
