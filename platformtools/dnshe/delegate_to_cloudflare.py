from __future__ import annotations

"""Batch delegate DNSHE free subdomains to Cloudflare nameservers.

Why:
  Cloudflare Email Routing requires Cloudflare to be authoritative DNS for the domain.
  For DNSHE free subdomains (e.g. artai.cc.cd), the practical way is to add NS records
  for the subdomain in DNSHE, delegating the whole zone to Cloudflare.

This script:
  - Lists subdomains from DNSHE API
  - Resolves subdomain_id for each target domain
  - (Optionally) deletes existing apex records at name == <full_domain>
  - Creates/updates NS records to the given Cloudflare nameservers

Security:
  - Reads secrets from `platformtools/dnshe/.dev.vars` (recommended) or environment variables.
  - Never writes secrets to disk.

Vars (required):
  - DNSHE_API_KEY
  - DNSHE_API_SECRET

Vars (optional):
  - DNSHE_API_BASE (default: https://api005.dnshe.com/index.php?m=domain_hub)

Usage examples (PowerShell):
  $env:DNSHE_API_KEY='cfsd_...'
  $env:DNSHE_API_SECRET='...'
  python platformtools/dnshe/delegate_to_cloudflare.py --domains artai.cc.cd,artllm.cc.cd --cf-ns kiki.ns.cloudflare.com,mark.ns.cloudflare.com --dry-run

Then run without --dry-run once the plan looks correct.
"""

import argparse
import json
import os
import sys
import urllib.parse
import urllib.request
from dataclasses import dataclass
from typing import Any, Dict, Iterable, List, Optional, Tuple


DEFAULT_BASE = "https://api005.dnshe.com/index.php?m=domain_hub"


@dataclass(frozen=True)
class DnsheConfig:
    base: str
    api_key: str
    api_secret: str


class DnsheError(RuntimeError):
    pass


def _req(
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

    body_bytes: Optional[bytes] = None
    headers = {
        "X-API-Key": cfg.api_key,
        "X-API-Secret": cfg.api_secret,
        "Accept": "application/json",
    }

    if json_body is not None:
        body_bytes = json.dumps(json_body).encode("utf-8")
        headers["Content-Type"] = "application/json"

    req = urllib.request.Request(url, data=body_bytes, headers=headers, method=method)

    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            raw = resp.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as e:
        raw = e.read().decode("utf-8", errors="replace")
        raise DnsheError(f"DNSHE API HTTP {e.code}: {raw}") from e
    except Exception as e:
        raise DnsheError(f"DNSHE API request failed: {method} {url}: {e}") from e

    try:
        data = json.loads(raw) if raw else {}
    except Exception as e:
        raise DnsheError(f"DNSHE API returned non-JSON: {raw}") from e

    if isinstance(data, dict) and data.get("success") is False:
        raise DnsheError(f"DNSHE API error: {data}")

    return data


def list_subdomains(cfg: DnsheConfig) -> List[Dict[str, Any]]:
    data = _req(cfg, endpoint="subdomains", action="list", method="GET")
    subs = data.get("subdomains")
    if not isinstance(subs, list):
        return []
    return subs


def get_subdomain_id_by_full_domain(subs: Iterable[Dict[str, Any]], full_domain: str) -> Optional[int]:
    target = (full_domain or "").strip().lower().rstrip(".")
    for s in subs:
        fd = str(s.get("full_domain") or "").strip().lower().rstrip(".")
        if fd == target:
            try:
                return int(s.get("id"))
            except Exception:
                return None
    return None


def list_dns_records(cfg: DnsheConfig, subdomain_id: int) -> List[Dict[str, Any]]:
    data = _req(
        cfg,
        endpoint="dns_records",
        action="list",
        method="GET",
        query={"subdomain_id": str(subdomain_id)},
    )
    recs = data.get("records")
    if not isinstance(recs, list):
        return []
    return recs


def delete_dns_record(cfg: DnsheConfig, record_id: int) -> None:
    _req(
        cfg,
        endpoint="dns_records",
        action="delete",
        method="POST",
        json_body={"record_id": int(record_id)},
    )


def create_dns_record(
    cfg: DnsheConfig,
    *,
    subdomain_id: int,
    rtype: str,
    content: str,
    name: Optional[str] = None,
    ttl: int = 600,
    priority: Optional[int] = None,
) -> int:
    body: Dict[str, Any] = {
        "subdomain_id": int(subdomain_id),
        "type": str(rtype).upper(),
        "content": str(content),
        "ttl": int(ttl),
    }
    if name is not None:
        body["name"] = name
    if priority is not None:
        body["priority"] = int(priority)

    data = _req(cfg, endpoint="dns_records", action="create", method="POST", json_body=body)
    rid = data.get("record_id") or data.get("id")
    try:
        return int(rid)
    except Exception:
        # DNSHE doc says record_id exists; if not, still return -1
        return -1


def update_dns_record(cfg: DnsheConfig, *, record_id: int, content: str, ttl: int = 600, priority: Optional[int] = None) -> None:
    body: Dict[str, Any] = {"record_id": int(record_id), "content": str(content), "ttl": int(ttl)}
    if priority is not None:
        body["priority"] = int(priority)
    _req(cfg, endpoint="dns_records", action="update", method="POST", json_body=body)


def normalize_domain_list(domains_csv: str) -> List[str]:
    parts = [p.strip().lower().rstrip(".") for p in (domains_csv or "").split(",")]
    return [p for p in parts if p]


def normalize_ns_list(ns_csv: str) -> List[str]:
    parts = [p.strip().lower().rstrip(".") for p in (ns_csv or "").split(",")]
    parts = [p for p in parts if p]
    # de-dup while preserving order
    seen: set[str] = set()
    out: List[str] = []
    for p in parts:
        if p not in seen:
            seen.add(p)
            out.append(p)
    return out


def _apex_name_variants(full_domain: str) -> List[str]:
    # Different backends treat apex name differently.
    # - UI suggests using '@' for apex.
    # - API examples show absolute names (test.example.com).
    # We'll consider both.
    fd = full_domain.strip().lower().rstrip(".")
    return ["@", fd]


def _record_name_eq(record_name: str, full_domain: str) -> bool:
    rn = (record_name or "").strip().lower().rstrip(".")
    fd = full_domain.strip().lower().rstrip(".")
    return rn in ("@", fd)


def delegate_one(
    cfg: DnsheConfig,
    *,
    full_domain: str,
    cf_nameservers: List[str],
    ttl: int,
    purge_apex: bool,
    dry_run: bool,
) -> None:
    subs = list_subdomains(cfg)
    sid = get_subdomain_id_by_full_domain(subs, full_domain)
    if not sid:
        raise DnsheError(f"subdomain not found in DNSHE list: {full_domain}")

    recs = list_dns_records(cfg, sid)

    apex_recs = [
        r
        for r in recs
        if _record_name_eq(str(r.get("name") or ""), full_domain)
    ]

    if purge_apex and apex_recs:
        for r in apex_recs:
            rid = r.get("id")
            if rid is None:
                continue
            if dry_run:
                print(f"[dry-run] delete record id={rid} type={r.get('type')} name={r.get('name')} content={r.get('content')}")
            else:
                delete_dns_record(cfg, int(rid))

        # refresh list after deletions
        if not dry_run:
            recs = list_dns_records(cfg, sid)

    # Ensure NS records exist for apex
    existing_ns: List[Dict[str, Any]] = [
        r
        for r in recs
        if str(r.get("type") or "").upper() == "NS" and _record_name_eq(str(r.get("name") or ""), full_domain)
    ]

    # Map existing content -> record
    by_content: Dict[str, Dict[str, Any]] = {}
    for r in existing_ns:
        c = str(r.get("content") or "").strip().lower().rstrip(".")
        by_content[c] = r

    for ns in cf_nameservers:
        if ns in by_content:
            rid = int(by_content[ns].get("id"))
            if dry_run:
                print(f"[dry-run] keep/update NS record id={rid} name={by_content[ns].get('name')} content={ns} ttl={ttl}")
            else:
                update_dns_record(cfg, record_id=rid, content=ns, ttl=ttl)
            continue

        # create new NS record
        created = False
        for name_variant in _apex_name_variants(full_domain):
            if dry_run:
                print(f"[dry-run] create NS name={name_variant} content={ns} ttl={ttl} (subdomain_id={sid})")
                created = True
                break
            try:
                create_dns_record(cfg, subdomain_id=sid, rtype="NS", content=ns, name=name_variant, ttl=ttl)
                created = True
                break
            except DnsheError:
                # Try next name variant
                continue

        if not created:
            raise DnsheError(f"failed to create NS record for {full_domain} -> {ns}")


def main(argv: List[str]) -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--domains", required=True, help="Comma-separated domains, e.g. artai.cc.cd,artllm.cc.cd")
    p.add_argument("--cf-ns", required=True, help="Comma-separated Cloudflare NS, e.g. kiki.ns.cloudflare.com,mark.ns.cloudflare.com")
    p.add_argument("--ttl", type=int, default=600)
    p.add_argument("--purge-apex", action="store_true", help="Delete existing apex records before creating NS (recommended)")
    p.add_argument("--no-purge-apex", action="store_true", help="Do NOT delete existing apex records")
    p.add_argument("--dry-run", action="store_true")

    args = p.parse_args(argv)

    domains = normalize_domain_list(args.domains)
    if not domains:
        print("no domains provided", file=sys.stderr)
        return 2

    ns_list = normalize_ns_list(args.cf_ns)
    if len(ns_list) < 2:
        print("need at least 2 Cloudflare nameservers", file=sys.stderr)
        return 2

    def _load_dotenv(path: str) -> Dict[str, str]:
        out: Dict[str, str] = {}
        try:
            with open(path, "r", encoding="utf-8") as f:
                for raw in f:
                    line = raw.strip()
                    if not line or line.startswith("#"):
                        continue
                    if "=" not in line:
                        continue
                    k, v = line.split("=", 1)
                    out[k.strip()] = v.strip()
        except FileNotFoundError:
            return {}
        return out

    dev_vars_path = os.path.join(os.path.dirname(__file__), ".dev.vars")
    file_vars = _load_dotenv(dev_vars_path)

    def _get(name: str, default: str = "") -> str:
        return (file_vars.get(name) or os.environ.get(name) or default).strip()

    base = _get("DNSHE_API_BASE", DEFAULT_BASE) or DEFAULT_BASE
    api_key = _get("DNSHE_API_KEY")
    api_secret = _get("DNSHE_API_SECRET")

    if not api_key or not api_secret:
        print("missing DNSHE_API_KEY or DNSHE_API_SECRET (from .dev.vars or env)", file=sys.stderr)
        return 2

    cfg = DnsheConfig(base=base, api_key=api_key, api_secret=api_secret)

    purge_apex = bool(args.purge_apex) and not bool(args.no_purge_apex)
    if not args.purge_apex and not args.no_purge_apex:
        # default to purge, because delegation should not coexist with apex A/MX/TXT etc.
        purge_apex = True

    for d in domains:
        print(f"== Delegating {d} -> {', '.join(ns_list)} (ttl={args.ttl}, purge_apex={purge_apex}, dry_run={args.dry_run})")
        delegate_one(
            cfg,
            full_domain=d,
            cf_nameservers=ns_list,
            ttl=int(args.ttl),
            purge_apex=purge_apex,
            dry_run=bool(args.dry_run),
        )

    print("done")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
