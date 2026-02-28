from __future__ import annotations

"""Check Cloudflare zone status + current public NS for domains.

This script is meant to answer: "Cloudflare side is configured correctly?"

It verifies:
- Cloudflare zone exists and is ACTIVE
- Cloudflare assigned name_servers (authoritative NS)
- Public DNS currently returns NS records (via Cloudflare DoH)
- Whether public NS matches Cloudflare assigned NS

Secrets:
- Loaded from `platformtools/.dev.vars` (preferred) or env.
- Never prints secrets.
"""

import argparse
import json
import os
import random
import sys
import urllib.parse
import urllib.request
from typing import Any, Dict, List, Optional, Tuple

# Ensure repo root importable
_REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
if _REPO_ROOT not in sys.path:
    sys.path.insert(0, _REPO_ROOT)

from platformtools._shared.dev_vars import get_var, load_platformtools_dev_vars

CF_API_BASE = "https://api.cloudflare.com/client/v4"
DOH_URL = "https://cloudflare-dns.com/dns-query"


class CheckError(RuntimeError):
    pass


def _http_json(*, method: str, url: str, headers: Dict[str, str], body: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
    data: Optional[bytes] = None
    if body is not None:
        data = json.dumps(body).encode("utf-8")
        headers = {**headers, "Content-Type": "application/json"}

    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            raw = resp.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as e:
        raw = e.read().decode("utf-8", errors="replace")
        raise CheckError(f"HTTP {e.code}: {raw}") from e
    except Exception as e:
        # Some Windows proxy stacks can surface as odd FileNotFoundError/URLError.
        raise CheckError(f"request failed: {method} {url}: {e}") from e

    try:
        return json.loads(raw) if raw else {}
    except Exception as e:
        raise CheckError(f"Non-JSON response: {raw}") from e


def _cf_headers(*, file_vars: Dict[str, str]) -> Dict[str, str]:
    # Prefer Global API Key if present, else API token.
    email = get_var(file_vars, "CF_AUTH_EMAIL")
    gkey = get_var(file_vars, "CF_GLOBAL_API_KEY")
    token = get_var(file_vars, "CF_API_TOKEN")

    if email and gkey:
        return {
            "Accept": "application/json",
            "X-Auth-Email": email,
            "X-Auth-Key": gkey,
        }

    if token:
        return {
            "Accept": "application/json",
            "Authorization": f"Bearer {token}",
        }

    raise CheckError("missing Cloudflare creds: need CF_API_TOKEN or (CF_AUTH_EMAIL + CF_GLOBAL_API_KEY)")


def cf_get_zone_by_name(*, headers: Dict[str, str], name: str) -> Optional[Dict[str, Any]]:
    q = urllib.parse.urlencode({"name": name, "per_page": "1"})
    url = f"{CF_API_BASE}/zones?{q}"
    data = _http_json(method="GET", url=url, headers=headers)
    if not isinstance(data, dict) or not data.get("success"):
        return None
    result = data.get("result")
    if not isinstance(result, list) or not result:
        return None
    z = result[0]
    return z if isinstance(z, dict) else None


def doh_ns(*, name: str) -> List[str]:
    q = urllib.parse.urlencode({"name": name, "type": "NS"})
    url = f"{DOH_URL}?{q}"
    data = _http_json(method="GET", url=url, headers={"Accept": "application/dns-json"})
    ans = data.get("Answer")
    if not isinstance(ans, list):
        return []
    out: List[str] = []
    for a in ans:
        if not isinstance(a, dict):
            continue
        if int(a.get("type") or 0) != 2:
            continue
        v = str(a.get("data") or "").strip().lower().rstrip(".")
        if v:
            out.append(v)
    # de-dup
    seen: set[str] = set()
    uniq: List[str] = []
    for x in out:
        if x not in seen:
            seen.add(x)
            uniq.append(x)
    return uniq


def normalize_domains(domains_csv: str) -> List[str]:
    parts = [p.strip().lower().rstrip(".") for p in (domains_csv or "").split(",")]
    return [p for p in parts if p]


def _ns_match(cf_ns: List[str], dns_ns: List[str]) -> bool:
    a = sorted([x.strip().lower().rstrip(".") for x in cf_ns if x.strip()])
    b = sorted([x.strip().lower().rstrip(".") for x in dns_ns if x.strip()])
    return a == b and len(a) > 0


def main(argv: List[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--domains", required=True, help="Comma-separated domains")
    ap.add_argument("--sample", type=int, default=0, help="If >0, randomly sample N domains from the list")
    args = ap.parse_args(argv)

    domains = normalize_domains(args.domains)
    if not domains:
        print("no domains", file=sys.stderr)
        return 2

    if args.sample and args.sample > 0 and args.sample < len(domains):
        random.seed(0xC0FFEE)
        domains = random.sample(domains, k=int(args.sample))

    file_vars = load_platformtools_dev_vars(start_dir=os.path.dirname(__file__))
    headers = _cf_headers(file_vars=file_vars)

    for d in domains:
        print(f"==== {d}")
        z = cf_get_zone_by_name(headers=headers, name=d)
        if not z:
            print("  CF: zone NOT FOUND")
            continue

        status = str(z.get("status") or "")
        plan = str(((z.get("plan") or {}) if isinstance(z.get("plan"), dict) else {}).get("name") or "")
        cf_ns_raw = z.get("name_servers")
        cf_ns = [str(x).strip().lower().rstrip(".") for x in (cf_ns_raw if isinstance(cf_ns_raw, list) else []) if str(x).strip()]

        print(f"  CF: status={status} plan={plan} ns={','.join(cf_ns) if cf_ns else '(none)'}")

        try:
            dns_ns = doh_ns(name=d)
        except CheckError as e:
            print(f"  DNS: ERROR {e}")
            continue

        print(f"  DNS: ns={','.join(dns_ns) if dns_ns else '(none)'}")

        if cf_ns and dns_ns:
            print(f"  MATCH: {'YES' if _ns_match(cf_ns, dns_ns) else 'NO'}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
