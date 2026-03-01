from __future__ import annotations

import os
import sys

# Ensure repo root is importable
_REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if _REPO_ROOT not in sys.path:
    sys.path.insert(0, _REPO_ROOT)

"""End-to-end inbound test for Cloudflare Email Routing -> Email Worker -> D1 -> MailCreate API.

What it does:
1) Calls MailCreate to create a new address (should default to mail.aiaimimi.com).
2) Attempts to deliver a test email via SMTP directly to Cloudflare Email Routing MX.
3) Polls MailCreate /api/mails until the message appears.

Notes:
- This test may fail if your network blocks outbound SMTP (port 25).
- Even if SMTP succeeds, Email Routing delivery may take some time.

Usage:
  python platformtools\\mailcreate\\client\\test_inbound_email_routing.py

Env/dev-vars:
- Reads platformtools/.dev.vars via load_platformtools_dev_vars
  - MAILCREATE_BASE_URL
  - MAILCREATE_CUSTOM_AUTH (still sent, but Worker currently bypasses auth gate)
"""

import random
import smtplib
import ssl
import string
import time
from email.message import EmailMessage

from platformtools._shared.dev_vars import get_var, load_platformtools_dev_vars
from platformtools.mailcreate.client.mailcreate_client import MailCreateClient, MailCreateConfig


MX_HOSTS = [
    "route1.mx.cloudflare.net",
    "route2.mx.cloudflare.net",
    "route3.mx.cloudflare.net",
]


def _rand_token(n: int = 10) -> str:
    alphabet = string.ascii_lowercase + string.digits
    return "".join(random.choice(alphabet) for _ in range(n))


def send_smtp(*, to_addr: str, subject: str, body: str) -> None:
    last_err: Exception | None = None

    for host in MX_HOSTS:
        try:
            print(f"[smtp] connecting: {host}:25")
            with smtplib.SMTP(host, 25, timeout=25) as s:
                s.ehlo()

                # Opportunistic STARTTLS if offered
                if s.has_extn("starttls"):
                    ctx = ssl.create_default_context()
                    s.starttls(context=ctx)
                    s.ehlo()

                msg = EmailMessage()
                msg["From"] = "test-sender@example.com"
                msg["To"] = to_addr
                msg["Subject"] = subject
                msg.set_content(body)

                s.send_message(msg)
                print(f"[smtp] sent via {host}")
                return
        except Exception as e:
            print(f"[smtp] failed via {host}: {e}")
            last_err = e
            continue

    raise RuntimeError(f"SMTP send failed via all MX hosts: {last_err}")


def main() -> int:
    vars = load_platformtools_dev_vars(start_dir="platformtools/mailcreate/client")
    base = get_var(vars, "MAILCREATE_BASE_URL")
    auth = get_var(vars, "MAILCREATE_CUSTOM_AUTH")

    client = MailCreateClient(MailCreateConfig(base_url=base, custom_auth=auth, timeout_seconds=30))

    # create new mailbox
    r = client.new_address()
    addr = str(r.get("address") or "").strip()
    jwt = str(r.get("jwt") or "").strip()
    if not addr or not jwt:
        raise RuntimeError(f"new_address returned invalid payload: {r}")

    token = _rand_token(12)
    subject = f"mailcreate-e2e {token}"
    body = f"Hello from e2e test. token={token}"

    print("[mailcreate] address:", addr)
    print("[mailcreate] subject:", subject)

    # send inbound mail
    send_smtp(to_addr=addr, subject=subject, body=body)

    # poll inbox
    deadline = time.time() + 180
    seen: set[int] = set()

    while time.time() < deadline:
        data = client.list_mails(jwt=jwt, limit=20, offset=0)
        items = data.get("results") or []
        for it in items:
            try:
                mid = int(it.get("id"))
            except Exception:
                continue
            if mid in seen:
                continue
            seen.add(mid)

            # fetch raw
            full = client.get_mail(jwt=jwt, mail_id=mid)
            raw = str(full.get("raw") or "")
            if token in raw or token in str(it.get("raw") or ""):
                print("[ok] received mail_id:", mid)
                return 0

        print(f"[poll] no match yet (seen={len(seen)}), sleep 6s")
        time.sleep(6)

    raise RuntimeError("timeout waiting for inbound email")


if __name__ == "__main__":
    raise SystemExit(main())
