"""MailCreate Client

A minimal Python client for the Cloudflare temp email Worker deployed at a custom domain.

This client is designed to be reused by multiple automation projects (e.g. Codex auto-register,
other platform auto-register) so they all share the same mailbox API surface.

Endpoints are implemented against the Worker routes:
- Open settings: GET /open_api/settings
- Create address: POST /api/new_address
- Mails: GET /api/mails, GET /api/mail/:mail_id, DELETE /api/mails/:id

Auth model:
- All non-/open_api/* endpoints require header x-custom-auth when PASSWORDS is configured.
- /api/* endpoints additionally require Authorization: Bearer <address_jwt> for most reads.

Ref:
- Worker middleware and auth: [`worker.ts`](platformtools/mailcreate/server/cloudflare_temp_email/worker/src/worker.ts:38)
"""

from __future__ import annotations

import json
import re
import time
import urllib.parse
import urllib.request
from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Tuple


@dataclass
class MailCreateConfig:
    base_url: str
    custom_auth: str
    timeout_seconds: int = 30


class MailCreateError(RuntimeError):
    pass


def _req(
    config: MailCreateConfig,
    method: str,
    path: str,
    *,
    headers: Optional[Dict[str, str]] = None,
    json_body: Optional[Dict[str, Any]] = None,
) -> Tuple[int, str, Dict[str, str]]:
    url = config.base_url.rstrip("/") + path

    body_bytes: Optional[bytes] = None
    base_headers = {
        "User-Agent": "mailcreate-client/0.1",
        "Accept": "application/json, text/plain, */*",
    }

    if headers:
        base_headers.update(headers)

    if json_body is not None:
        body_bytes = json.dumps(json_body).encode("utf-8")
        base_headers["Content-Type"] = "application/json"

    req = urllib.request.Request(url, data=body_bytes, headers=base_headers, method=method)

    try:
        with urllib.request.urlopen(req, timeout=config.timeout_seconds) as resp:
            text = resp.read().decode("utf-8", errors="replace")
            return resp.getcode(), text, dict(resp.headers)
    except urllib.error.HTTPError as e:
        text = e.read().decode("utf-8", errors="replace")
        return e.code, text, dict(e.headers)
    except Exception as e:
        raise MailCreateError(f"request failed: {method} {url}: {e}") from e


class MailCreateClient:
    def __init__(self, config: MailCreateConfig):
        self.config = config

    def health_check(self) -> str:
        code, text, _ = _req(
            self.config,
            "GET",
            "/health_check",
            headers={"x-custom-auth": self.config.custom_auth},
        )
        if code != 200:
            raise MailCreateError(f"health_check failed: {code} {text}")
        return text

    def open_settings(self) -> Dict[str, Any]:
        # /open_api/settings is still protected by x-custom-auth if PASSWORDS configured.
        code, text, _ = _req(
            self.config,
            "GET",
            "/open_api/settings",
            headers={"x-custom-auth": self.config.custom_auth},
        )
        if code != 200:
            raise MailCreateError(f"open_settings failed: {code} {text}")
        return json.loads(text)

    def new_address(self, *, name: Optional[str] = None, domain: Optional[str] = None) -> Dict[str, Any]:
        payload: Dict[str, Any] = {}
        if name is not None:
            payload["name"] = name
        if domain is not None:
            payload["domain"] = domain

        code, text, _ = _req(
            self.config,
            "POST",
            "/api/new_address",
            headers={"x-custom-auth": self.config.custom_auth},
            json_body=payload,
        )
        if code != 200:
            raise MailCreateError(f"new_address failed: {code} {text}")
        return json.loads(text)

    def list_mails(self, *, jwt: str, limit: int = 20, offset: int = 0) -> Dict[str, Any]:
        q = urllib.parse.urlencode({"limit": str(limit), "offset": str(offset)})
        code, text, _ = _req(
            self.config,
            "GET",
            f"/api/mails?{q}",
            headers={
                "x-custom-auth": self.config.custom_auth,
                "Authorization": f"Bearer {jwt}",
            },
        )
        if code != 200:
            raise MailCreateError(f"list_mails failed: {code} {text}")
        return json.loads(text)

    def get_mail(self, *, jwt: str, mail_id: int) -> Dict[str, Any]:
        code, text, _ = _req(
            self.config,
            "GET",
            f"/api/mail/{mail_id}",
            headers={
                "x-custom-auth": self.config.custom_auth,
                "Authorization": f"Bearer {jwt}",
            },
        )
        if code != 200:
            raise MailCreateError(f"get_mail failed: {code} {text}")
        return json.loads(text) if text else {}

    def delete_mail(self, *, jwt: str, mail_id: int) -> Dict[str, Any]:
        code, text, _ = _req(
            self.config,
            "DELETE",
            f"/api/mails/{mail_id}",
            headers={
                "x-custom-auth": self.config.custom_auth,
                "Authorization": f"Bearer {jwt}",
            },
        )
        if code != 200:
            raise MailCreateError(f"delete_mail failed: {code} {text}")
        return json.loads(text) if text else {"success": True}


_CODE_RE = re.compile(r"(?<!\d)(\d{6})(?!\d)")


def extract_6digit_code(text: str) -> Optional[str]:
    m = _CODE_RE.search(text or "")
    return m.group(1) if m else None


def wait_for_6digit_code(
    client: MailCreateClient,
    *,
    jwt: str,
    from_contains: Optional[str] = None,
    timeout_seconds: int = 120,
    poll_seconds: float = 3.0,
) -> str:
    """Poll mailbox until a 6-digit code appears in subject/raw.

    Notes:
    - raw_mails.raw is stored; subject extraction depends on upstream parser.
    - For maximum reliability, search within raw email text as well.
    """

    deadline = time.time() + timeout_seconds
    last_seen_ids: set[int] = set()

    while time.time() < deadline:
        data = client.list_mails(jwt=jwt, limit=20, offset=0)
        emails: List[Dict[str, Any]] = data.get("results") or data.get("emails") or data.get("data") or []

        # Some API variants return { results, count } via handleListQuery.
        if isinstance(data, dict) and "results" in data and isinstance(data["results"], list):
            emails = data["results"]

        for e in emails:
            try:
                mail_id = int(e.get("id"))
            except Exception:
                continue

            if mail_id in last_seen_ids:
                continue

            last_seen_ids.add(mail_id)

            # Optional filter
            if from_contains:
                src = (e.get("source") or e.get("from") or "")
                if from_contains.lower() not in str(src).lower():
                    continue

            subject = str(e.get("subject") or "")
            code = extract_6digit_code(subject)
            if code:
                return code

            # Fetch full raw to search
            full = client.get_mail(jwt=jwt, mail_id=mail_id)
            raw = str(full.get("raw") or "")
            code = extract_6digit_code(raw)
            if code:
                return code

        time.sleep(poll_seconds)

    raise MailCreateError(f"timeout waiting for 6-digit code (timeout={timeout_seconds}s)")
