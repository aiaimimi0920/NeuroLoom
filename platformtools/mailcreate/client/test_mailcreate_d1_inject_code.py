from __future__ import annotations

"""Inject a fake inbound email into Cloudflare D1 and verify MailCreate can poll & extract a 6-digit code.

Why
- Outbound SMTP from this machine can be rejected by Cloudflare MX (PTR / reverse lookup).
- This test validates the *rest* of the pipeline:
  new_address -> D1 raw_mails insert -> /api/mails + /api/mail -> code extraction.

It does NOT validate real Email Routing delivery.

Usage:
  python platformtools\\mailcreate\\client\\test_mailcreate_d1_inject_code.py
"""

import json
import os
import random
import subprocess
import sys
import time

# Ensure repo root is importable
_REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if _REPO_ROOT not in sys.path:
    sys.path.insert(0, _REPO_ROOT)

from platformtools._shared.dev_vars import get_var, load_platformtools_dev_vars
from platformtools.mailcreate.client.mailcreate_client import (
    MailCreateClient,
    MailCreateConfig,
    wait_for_6digit_code,
)


def _rand_code() -> str:
    return f"{random.randint(0, 999999):06d}"


def _run_wrangler_sql(*, sql: str) -> None:
    worker_dir = os.path.join(
        _REPO_ROOT,
        "platformtools",
        "mailcreate",
        "server",
        "cloudflare_temp_email",
        "worker",
    )

    # Use Wrangler from the worker project so it picks up wrangler.toml and auth.
    # On Windows, calling "npx" directly may fail depending on PATH resolution.
    # Use shell=True so cmd.exe resolves npx.cmd.
    cmd = (
        "npx wrangler d1 execute cloudflare-temp-email --remote "
        + "--command "
        + json.dumps(sql)
    )

    p = subprocess.run(cmd, cwd=worker_dir, capture_output=True, text=True, shell=True)
    if p.returncode != 0:
        raise RuntimeError(
            "wrangler d1 execute failed\n"
            f"cmd={cmd}\n"
            f"stdout=\n{p.stdout}\n"
            f"stderr=\n{p.stderr}\n"
        )


def main() -> int:
    vars = load_platformtools_dev_vars(start_dir="platformtools/mailcreate/client")
    base = get_var(vars, "MAILCREATE_BASE_URL")
    auth = get_var(vars, "MAILCREATE_CUSTOM_AUTH")

    client = MailCreateClient(MailCreateConfig(base_url=base, custom_auth=auth, timeout_seconds=30))

    # 1) Create address
    r = client.new_address()
    addr = str(r.get("address") or "").strip()
    jwt = str(r.get("jwt") or "").strip()
    if not addr or not jwt:
        raise RuntimeError(f"new_address returned invalid payload: {r}")

    code = _rand_code()
    token = f"inject-{int(time.time())}"

    raw = (
        "From: OpenAI <no-reply@openai.com>\n"
        f"To: <{addr}>\n"
        f"Subject: Verify your email ({token})\n"
        "Content-Type: text/plain; charset=utf-8\n"
        "\n"
        f"Your verification code is {code}.\n"
    )

    # 2) Insert into D1
    # raw_mails columns: message_id, source, address, raw
    # Keep message_id stable-ish.
    message_id = f"<{token}@mailcreate.local>"

    # Basic SQL escaping for single quotes.
    raw_sql = raw.replace("'", "''")
    source_sql = "OpenAI <no-reply@openai.com>".replace("'", "''")
    addr_sql = addr.replace("'", "''")
    msgid_sql = message_id.replace("'", "''")

    sql = (
        "INSERT INTO raw_mails (message_id, source, address, raw) VALUES ("
        f"'{msgid_sql}', '{source_sql}', '{addr_sql}', '{raw_sql}'"
        ");"
    )

    _run_wrangler_sql(sql=sql)

    # 3) Poll via API for code
    got = wait_for_6digit_code(client, jwt=jwt, from_contains="openai", timeout_seconds=60, poll_seconds=2.0)

    print("address=", addr)
    print("expected_code=", code)
    print("got_code=", got)

    if got != code:
        raise RuntimeError("code mismatch")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
