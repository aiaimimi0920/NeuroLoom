from __future__ import annotations

"""List DNSHE subdomains for the current API credentials.

Why
---
When you manage multiple DNSHE accounts (multiple API keys), you often need to
quickly export the list of `full_domain` values so you can batch-create Cloudflare
zones and then delegate NS manually.

Credentials
-----------
Loaded from (priority):
1) `platformtools/.dev.vars` (global, recommended)
2) `platformtools/dnshe/.dev.vars` (local override)
3) environment variables

Supports profiled credentials by suffix:
- DNSHE_API_KEY_1 / DNSHE_API_SECRET_1 / DNSHE_API_BASE_1
- DNSHE_API_KEY_2 / ...

Usage
-----
python platformtools\\dnshe\\list_subdomains.py --profile 1
python platformtools\\dnshe\\list_subdomains.py --profile 2 --json
"""

import argparse
import json
import os
import sys
import urllib.parse
import urllib.request
from typing import Any, Dict, List

# Ensure repo root is importable
_REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
if _REPO_ROOT not in sys.path:
    sys.path.insert(0, _REPO_ROOT)

from platformtools._shared.dev_vars import get_var_profiled, load_platformtools_dev_vars

DNSHE_DEFAULT_BASE = "https://api005.dnshe.com/index.php?m=domain_hub"


class DnsheError(RuntimeError):
    pass


def dnshe_req(*, base: str, api_key: str, api_secret: str, endpoint: str, action: str, method: str = "GET", query: Dict[str, str] | None = None) -> Dict[str, Any]:
    base = base.rstrip("/")
    q = {"m": "domain_hub", "endpoint": endpoint, "action": action}
    if query:
        q.update({k: str(v) for k, v in query.items()})

    url = base.split("?")[0] + "?" + urllib.parse.urlencode(q)

    headers = {
        "X-API-Key": api_key,
        "X-API-Secret": api_secret,
        "Accept": "application/json",
    }

    req = urllib.request.Request(url, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            raw = resp.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as e:
        raw = e.read().decode("utf-8", errors="replace")
        raise DnsheError(f"DNSHE API HTTP {e.code}: {raw}") from e

    try:
        data = json.loads(raw) if raw else {}
    except Exception as e:
        raise DnsheError(f"DNSHE API returned non-JSON: {raw}") from e

    if isinstance(data, dict) and data.get("success") is False:
        raise DnsheError(f"DNSHE API error: {data}")

    return data


def list_subdomains(*, base: str, api_key: str, api_secret: str) -> List[Dict[str, Any]]:
    data = dnshe_req(base=base, api_key=api_key, api_secret=api_secret, endpoint="subdomains", action="list")
    subs = data.get("subdomains")
    return subs if isinstance(subs, list) else []


def main(argv: List[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--profile", default="", help="DNSHE credential profile suffix, e.g. 1/2/3 (reads DNSHE_*_<profile>)")
    ap.add_argument("--json", action="store_true", help="Output JSON")
    args = ap.parse_args(argv)

    file_vars = load_platformtools_dev_vars(start_dir=os.path.dirname(__file__))

    base = get_var_profiled(file_vars, "DNSHE_API_BASE", profile=args.profile, default=DNSHE_DEFAULT_BASE) or DNSHE_DEFAULT_BASE
    api_key = get_var_profiled(file_vars, "DNSHE_API_KEY", profile=args.profile)
    api_secret = get_var_profiled(file_vars, "DNSHE_API_SECRET", profile=args.profile)

    if not api_key or not api_secret:
        print("missing DNSHE_API_KEY/DNSHE_API_SECRET (from platformtools/.dev.vars or env)", file=sys.stderr)
        return 2

    subs = list_subdomains(base=base, api_key=api_key, api_secret=api_secret)

    full_domains = []
    for s in subs:
        fd = str(s.get("full_domain") or "").strip().lower().rstrip(".")
        if fd:
            full_domains.append(fd)

    full_domains = sorted(set(full_domains))

    if args.json:
        print(json.dumps(full_domains, ensure_ascii=False, indent=2))
    else:
        for d in full_domains:
            print(d)

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
