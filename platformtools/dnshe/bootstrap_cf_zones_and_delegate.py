from __future__ import annotations

"""Bootstrap Cloudflare zones for DNSHE free domains and delegate DNS to Cloudflare.

Goal
----
For each domain (e.g. artai.cc.cd):
1) Create (or find) a Cloudflare zone (type=full) so Cloudflare becomes authoritative.
2) Fetch Cloudflare-assigned nameservers for that zone.
3) Use DNSHE API to set apex NS records for the domain, delegating DNS to Cloudflare.

Why this matters
----------------
Cloudflare Email Routing requires Cloudflare to be authoritative DNS for the domain.
Delegating via NS is the correct approach.

Security model
--------------
Secrets are loaded from:
- `platformtools/dnshe/.dev.vars` (recommended, gitignored), then
- process environment variables.

Required variables
------------------
Cloudflare:
- CF_API_TOKEN       (API token with Zone:Edit for your account)
- CF_ACCOUNT_ID      (your Cloudflare account id)

DNSHE:
- DNSHE_API_KEY
- DNSHE_API_SECRET

Optional variables
------------------
- DNSHE_API_BASE     (default: https://api005.dnshe.com/index.php?m=domain_hub)

Usage
-----
1) Copy example:
   - See: `platformtools/dnshe/.dev.vars.example`

2) Dry run:
   python platformtools/dnshe/bootstrap_cf_zones_and_delegate.py --domains artai.cc.cd --dry-run

3) Apply:
   python platformtools/dnshe/bootstrap_cf_zones_and_delegate.py --domains artai.cc.cd

Notes
-----
- This only delegates DNS. You still need to enable Email Routing + catch-all -> your Email Worker
  for each zone in Cloudflare Dashboard.
- DNSHE API docs show dns_records supports A/AAAA/CNAME/MX/TXT; however the DNSHE UI supports NS.
  This script will try to create NS via API. If DNSHE API rejects NS, it will fail fast.
"""

import argparse
import json
import os
import sys
import time
import urllib.parse
import urllib.request
from dataclasses import dataclass
from typing import Any, Dict, Iterable, List, Optional

# Ensure repo root is importable so we can `import platformtools...` even when
# this file is executed as a script path.
_REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
if _REPO_ROOT not in sys.path:
    sys.path.insert(0, _REPO_ROOT)


DNSHE_DEFAULT_BASE = "https://api005.dnshe.com/index.php?m=domain_hub"
CF_API_BASE = "https://api.cloudflare.com/client/v4"


class BootstrapError(RuntimeError):
    pass


@dataclass(frozen=True)
class DnsheConfig:
    base: str
    api_key: str
    api_secret: str


@dataclass(frozen=True)
class CfConfig:
    # Prefer API Token; if empty, fall back to Global API Key.
    api_token: str
    account_id: str
    auth_email: str = ""
    global_api_key: str = ""


# -------------------------
# DNSHE API
# -------------------------

def dnshe_req(
    cfg: DnsheConfig,
    *,
    endpoint: str,
    action: str,
    method: str = "GET",
    query: Optional[Dict[str, str]] = None,
    json_body: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    base = cfg.base.rstrip("/")
    q = {"m": "domain_hub", "endpoint": endpoint, "action": action}
    if query:
        q.update({k: str(v) for k, v in query.items()})

    url = base.split("?")[0] + "?" + urllib.parse.urlencode(q)

    headers = {
        "X-API-Key": cfg.api_key,
        "X-API-Secret": cfg.api_secret,
        "Accept": "application/json",
    }

    body_bytes: Optional[bytes] = None
    if json_body is not None:
        body_bytes = json.dumps(json_body).encode("utf-8")
        headers["Content-Type"] = "application/json"

    req = urllib.request.Request(url, data=body_bytes, headers=headers, method=method)

    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            raw = resp.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as e:
        raw = e.read().decode("utf-8", errors="replace")
        raise BootstrapError(f"DNSHE API HTTP {e.code}: {raw}") from e

    try:
        data = json.loads(raw) if raw else {}
    except Exception as e:
        raise BootstrapError(f"DNSHE API returned non-JSON: {raw}") from e

    if isinstance(data, dict) and data.get("success") is False:
        raise BootstrapError(f"DNSHE API error: {data}")

    return data


def dnshe_list_subdomains(cfg: DnsheConfig) -> List[Dict[str, Any]]:
    data = dnshe_req(cfg, endpoint="subdomains", action="list", method="GET")
    subs = data.get("subdomains")
    return subs if isinstance(subs, list) else []


def dnshe_get_subdomain_id(subs: Iterable[Dict[str, Any]], full_domain: str) -> int:
    target = full_domain.strip().lower().rstrip(".")
    for s in subs:
        fd = str(s.get("full_domain") or "").strip().lower().rstrip(".")
        if fd == target:
            try:
                return int(s.get("id"))
            except Exception:
                break
    raise BootstrapError(f"DNSHE subdomain not found: {full_domain}")


def dnshe_list_records(cfg: DnsheConfig, subdomain_id: int) -> List[Dict[str, Any]]:
    data = dnshe_req(
        cfg,
        endpoint="dns_records",
        action="list",
        method="GET",
        query={"subdomain_id": str(subdomain_id)},
    )
    recs = data.get("records")
    return recs if isinstance(recs, list) else []


def dnshe_delete_record(cfg: DnsheConfig, record_id: int) -> None:
    dnshe_req(
        cfg,
        endpoint="dns_records",
        action="delete",
        method="POST",
        json_body={"record_id": int(record_id)},
    )


def dnshe_create_record(
    cfg: DnsheConfig,
    *,
    subdomain_id: int,
    rtype: str,
    name: str,
    content: str,
    ttl: int = 600,
    priority: Optional[int] = None,
) -> int:
    body: Dict[str, Any] = {
        "subdomain_id": int(subdomain_id),
        "type": str(rtype).upper(),
        "name": name,
        "content": content,
        "ttl": int(ttl),
    }
    if priority is not None:
        body["priority"] = int(priority)

    data = dnshe_req(cfg, endpoint="dns_records", action="create", method="POST", json_body=body)
    rid = data.get("record_id") or data.get("id")
    return int(rid) if rid is not None else -1


def apex_match(record_name: str, full_domain: str) -> bool:
    rn = (record_name or "").strip().lower().rstrip(".")
    fd = full_domain.strip().lower().rstrip(".")
    return rn in ("@", fd)


# -------------------------
# Cloudflare API
# -------------------------

def cf_req(
    cfg: CfConfig,
    *,
    method: str,
    path: str,
    query: Optional[Dict[str, str]] = None,
    json_body: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    url = CF_API_BASE.rstrip("/") + path
    if query:
        url = url + "?" + urllib.parse.urlencode(query)

    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json",
    }

    # Auth strategy:
    # - Prefer Global API Key if provided (X-Auth-Email + X-Auth-Key)
    # - Else use API Token (Bearer)
    #
    # This makes it safe if CF_API_TOKEN is accidentally set while user intends to
    # use the Global API Key.
    if getattr(cfg, "auth_email", "") and getattr(cfg, "global_api_key", ""):
        headers["X-Auth-Email"] = str(getattr(cfg, "auth_email")).strip()
        headers["X-Auth-Key"] = str(getattr(cfg, "global_api_key")).strip()
    elif cfg.api_token:
        headers["Authorization"] = f"Bearer {cfg.api_token}"
    else:
        raise BootstrapError("missing Cloudflare credentials (need CF_API_TOKEN or CF_AUTH_EMAIL+CF_GLOBAL_API_KEY)")

    body_bytes: Optional[bytes] = None
    if json_body is not None:
        body_bytes = json.dumps(json_body).encode("utf-8")

    req = urllib.request.Request(url, data=body_bytes, headers=headers, method=method)

    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            raw = resp.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as e:
        raw = e.read().decode("utf-8", errors="replace")
        raise BootstrapError(f"Cloudflare API HTTP {e.code}: {raw}") from e

    try:
        data = json.loads(raw) if raw else {}
    except Exception as e:
        raise BootstrapError(f"Cloudflare API returned non-JSON: {raw}") from e

    if isinstance(data, dict) and data.get("success") is False:
        raise BootstrapError(f"Cloudflare API error: {data}")

    return data


def cf_find_zone(cfg: CfConfig, *, name: str) -> Optional[Dict[str, Any]]:
    data = cf_req(cfg, method="GET", path="/zones", query={"name": name, "per_page": "50"})
    res = data.get("result")
    if isinstance(res, list) and res:
        return res[0]
    return None


def cf_create_zone(cfg: CfConfig, *, name: str, jump_start: bool = False) -> Dict[str, Any]:
    body = {
        "name": name,
        "account": {"id": cfg.account_id},
        "type": "full",
        "jump_start": bool(jump_start),
    }
    data = cf_req(cfg, method="POST", path="/zones", json_body=body)
    zone = data.get("result")
    if not isinstance(zone, dict):
        raise BootstrapError(f"Cloudflare zone create returned unexpected payload: {data}")
    return zone


def cf_get_or_create_zone(cfg: CfConfig, *, name: str) -> Dict[str, Any]:
    existing = cf_find_zone(cfg, name=name)
    if existing:
        return existing
    return cf_create_zone(cfg, name=name)


def normalize_domains(domains_csv: str) -> List[str]:
    parts = [p.strip().lower().rstrip(".") for p in (domains_csv or "").split(",")]
    return [p for p in parts if p]


def main(argv: List[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--domains", required=True, help="Comma-separated domains (10 domains pool)")
    ap.add_argument("--ttl", type=int, default=600)
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument(
        "--skip-dnshe",
        action="store_true",
        help="Only create/find Cloudflare zones and print assigned nameservers; do NOT touch DNSHE.",
    )
    ap.add_argument("--purge-apex", action="store_true", help="Delete existing apex records before setting NS (recommended)")
    ap.add_argument(
        "--dnshe-profile",
        default="",
        help="DNSHE credential profile suffix, e.g. 1/2/3 (reads DNSHE_*_<profile> from platformtools/.dev.vars)",
    )
    args = ap.parse_args(argv)

    domains = normalize_domains(args.domains)
    if not domains:
        print("no domains provided", file=sys.stderr)
        return 2

    # Load dev vars from:
    # - platformtools/.dev.vars (recommended, global), then
    # - platformtools/dnshe/.dev.vars (local override), then
    # - process environment.
    from platformtools._shared.dev_vars import get_var, get_var_profiled, load_platformtools_dev_vars

    file_vars = load_platformtools_dev_vars(start_dir=os.path.dirname(__file__))

    # Cloudflare does not need profile.
    def _get(name: str, default: str = "") -> str:
        return get_var(file_vars, name, default)

    # DNSHE can be profiled by suffix (DNSHE_*_1 / DNSHE_*_2 / DNSHE_*_3)
    def _get_dnshe(name: str, default: str = "", profile: str = "") -> str:
        return get_var_profiled(file_vars, name, profile=profile, default=default)

    cf_token = _get("CF_API_TOKEN")
    cf_account_id = _get("CF_ACCOUNT_ID")
    cf_auth_email = _get("CF_AUTH_EMAIL")
    cf_global_api_key = _get("CF_GLOBAL_API_KEY")

    if not cf_account_id:
        print("missing CF_ACCOUNT_ID (from .dev.vars or env)", file=sys.stderr)
        return 2

    if not cf_token and not (cf_auth_email and cf_global_api_key):
        print(
            "missing Cloudflare creds: need CF_API_TOKEN or (CF_AUTH_EMAIL + CF_GLOBAL_API_KEY) (from .dev.vars or env)",
            file=sys.stderr,
        )
        return 2

    dnshe: Optional[DnsheConfig] = None
    subs: List[Dict[str, Any]] = []

    if not args.skip_dnshe:
        dnshe_key = _get_dnshe("DNSHE_API_KEY", profile=str(args.dnshe_profile))
        dnshe_secret = _get_dnshe("DNSHE_API_SECRET", profile=str(args.dnshe_profile))
        if not dnshe_key or not dnshe_secret:
            print("missing DNSHE_API_KEY or DNSHE_API_SECRET (from .dev.vars or env)", file=sys.stderr)
            return 2

        dnshe_base = _get_dnshe("DNSHE_API_BASE", DNSHE_DEFAULT_BASE, profile=str(args.dnshe_profile)) or DNSHE_DEFAULT_BASE
        dnshe = DnsheConfig(base=dnshe_base, api_key=dnshe_key, api_secret=dnshe_secret)

        # Cache DNSHE subdomain listing once
        subs = dnshe_list_subdomains(dnshe)

    cf = CfConfig(
        api_token=cf_token,
        account_id=cf_account_id,
        auth_email=cf_auth_email,
        global_api_key=cf_global_api_key,
    )

    ns_map: Dict[str, List[str]] = {}

    for d in domains:
        print(f"== {d}")

        # 1) Create/find zone
        if args.dry_run:
            print(f"[dry-run] cloudflare: get_or_create_zone name={d}")
            zone = {"name": d, "id": "(dry-run)", "name_servers": ["ns1", "ns2"]}
        else:
            zone = cf_get_or_create_zone(cf, name=d)

        ns = zone.get("name_servers")
        if not isinstance(ns, list) or len(ns) < 2:
            # If we got a lightweight zone object, re-fetch
            if not args.dry_run:
                z2 = cf_find_zone(cf, name=d) or {}
                ns = z2.get("name_servers")
            if not isinstance(ns, list) or len(ns) < 2:
                raise BootstrapError(f"Cloudflare zone missing name_servers for {d}: {zone}")

        ns_list = [str(x).strip().lower().rstrip(".") for x in ns if str(x).strip()]
        ns_list = ns_list[:2]
        ns_map[d] = ns_list
        print(f"cloudflare zone id={zone.get('id')} ns={ns_list[0]},{ns_list[1]}")

        if args.skip_dnshe:
            continue

        assert dnshe is not None

        # 2) DNSHE: set NS records on apex
        sid = dnshe_get_subdomain_id(subs, d)
        recs = dnshe_list_records(dnshe, sid)

        if args.purge_apex:
            for r in recs:
                if not apex_match(str(r.get("name") or ""), d):
                    continue
                rid = r.get("id")
                if rid is None:
                    continue
                if args.dry_run:
                    print(f"[dry-run] dnshe: delete record id={rid} type={r.get('type')} name={r.get('name')} content={r.get('content')}")
                else:
                    dnshe_delete_record(dnshe, int(rid))

            if not args.dry_run:
                recs = dnshe_list_records(dnshe, sid)

        existing_ns = [
            r
            for r in recs
            if str(r.get("type") or "").upper() == "NS" and apex_match(str(r.get("name") or ""), d)
        ]
        existing_contents = {str(r.get("content") or "").strip().lower().rstrip(".") for r in existing_ns}

        for n in ns_list:
            if n in existing_contents:
                print(f"dnshe ns already present: {n}")
                continue
            if args.dry_run:
                print(f"[dry-run] dnshe: create NS name=@ content={n} ttl={args.ttl} (subdomain_id={sid})")
            else:
                # Try apex as '@'
                try:
                    dnshe_create_record(dnshe, subdomain_id=sid, rtype="NS", name="@", content=n, ttl=int(args.ttl))
                except BootstrapError as e:
                    # DNSHE API doc currently doesn't claim NS is supported; many accounts can only do this via UI.
                    raise BootstrapError(
                        f"DNSHE API does not accept NS records (got: {e}). "
                        f"You need to set NS delegation manually in DNSHE UI for {d} -> {ns_list[0]}, {ns_list[1]}."
                    ) from e

        print("ok")
        time.sleep(0.2)

    if args.skip_dnshe:
        print("\nNS_MAP_JSON=" + json.dumps(ns_map, ensure_ascii=False))

    print("all done")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
