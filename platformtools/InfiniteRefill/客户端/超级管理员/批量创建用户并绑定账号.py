#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
批量创建用户并绑定账号（支持断点续传、实时进度、导出 k_ 密钥）。

目标场景：
- 创建 N 个用户（默认 200）
- 每个用户绑定 M 个账号（默认 30）
- 导出可分发的 USER_KEY（k_ 开头）

设计要点：
1) 默认使用服务端 `/admin/keys/issue` 生成 `k_`（与历史机制完全一致）。
2) 若切换本地生成模式，算法与服务端一致：`k_` + 24 字节随机 + URL-safe base64（去掉 `=`）。
3) 本地生成模式会在落盘前调用 `users/create(dry_run=true)` 预检冲突，不通过则重生 key。
4) users/create 为幂等接口，脚本可反复调用；
5) rebind 非严格幂等：为避免网络中断时重复重放导致额外搬运，脚本对“网络不确定失败”标记为 uncertain 并暂停，
   默认不自动重试该条，待人工确认后再处理（安全优先）。

示例（仅示例，不会自动执行）：
python 批量创建用户并绑定账号.py ^
  --server-url https://refill.aiaimimi.com ^
  --admin-token <token> ^
  --admin-guard <guard> ^
  --users 200 ^
  --accounts-per-user 30
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import secrets
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Any, Dict, List, Tuple


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def sha256_hex(s: str) -> str:
    return hashlib.sha256(s.encode("utf-8")).hexdigest()


def gen_user_key() -> str:
    b = secrets.token_bytes(24)
    s = base64.urlsafe_b64encode(b).decode("ascii").rstrip("=")
    return f"k_{s}"


class HttpCallError(Exception):
    def __init__(self, status: int, body: str):
        super().__init__(f"HTTP {status}: {body[:300]}")
        self.status = status
        self.body = body


class NetworkUncertainError(Exception):
    """请求结果不确定（可能已到达服务端但客户端没拿到响应）。"""


def request_json(
    *,
    url: str,
    method: str,
    headers: Dict[str, str],
    payload: Dict[str, Any] | None = None,
    timeout: int = 60,
) -> Dict[str, Any]:
    data = None
    if payload is not None:
        data = json.dumps(payload, ensure_ascii=False).encode("utf-8")

    req_headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
        "Accept": "application/json, text/plain, */*",
        "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
        "Cache-Control": "no-cache",
        "Pragma": "no-cache",
    }
    req_headers.update(headers or {})
    req = urllib.request.Request(url=url, method=method, headers=req_headers, data=data)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            text = resp.read().decode("utf-8", errors="replace")
            try:
                obj = json.loads(text)
            except Exception:
                raise HttpCallError(int(getattr(resp, "status", 200) or 200), text)
            return obj if isinstance(obj, dict) else {"ok": False, "raw": obj}
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8", errors="replace") if e.fp else str(e)
        raise HttpCallError(int(getattr(e, "code", 0) or 0), body)
    except (urllib.error.URLError, TimeoutError, ConnectionError, OSError) as e:
        raise NetworkUncertainError(str(e))


@dataclass
class Config:
    server_url: str
    admin_token: str
    admin_guard: str
    users: int
    accounts_per_user: int
    state_file: str
    keys_file: str
    label_prefix: str
    timeout: int
    dry_run: bool
    retry_uncertain: bool
    assume_uncertain_done: bool
    key_source: str


def load_state(path: str) -> Dict[str, Any]:
    if not os.path.exists(path):
        return {}
    with open(path, "r", encoding="utf-8") as f:
        obj = json.load(f)
    return obj if isinstance(obj, dict) else {}


def save_state(path: str, st: Dict[str, Any]) -> None:
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(st, f, ensure_ascii=False, indent=2)
    os.replace(tmp, path)


def ensure_state(cfg: Config) -> Dict[str, Any]:
    st = load_state(cfg.state_file)
    if not st:
        st = {
            "version": 1,
            "created_at": utc_now(),
            "updated_at": utc_now(),
            "server_url": cfg.server_url,
            "users": cfg.users,
            "accounts_per_user": cfg.accounts_per_user,
            "label_prefix": cfg.label_prefix,
            "records": [],
            "notes": [],
        }
        save_state(cfg.state_file, st)
        return st

    for k, v in (
        ("server_url", cfg.server_url),
        ("users", cfg.users),
        ("accounts_per_user", cfg.accounts_per_user),
    ):
        if str(st.get(k)) != str(v):
            raise SystemExit(
                f"[FATAL] 断点文件参数不一致: {k} state={st.get(k)} cli={v}。"
                f"请更换 --state-file 或手工确认后修改状态文件。"
            )

    st.setdefault("records", [])
    st.setdefault("notes", [])
    return st


def export_keys(path: str, st: Dict[str, Any]) -> None:
    rows = []
    for r in st.get("records", []):
        rows.append(
            {
                "index": r.get("index"),
                "key": r.get("key"),
                "key_hash": r.get("key_hash"),
                "stage": r.get("stage"),
                "updated_at": r.get("updated_at"),
            }
        )
    rows.sort(key=lambda x: int(x.get("index", 0)))
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        for x in rows:
            if str(x.get("key", "")).startswith("k_"):
                f.write(f"{x['index']},{x['key']},{x['key_hash']},{x['stage']}\n")


def headers(cfg: Config) -> Dict[str, str]:
    return {
        "Authorization": f"Bearer {cfg.admin_token}",
        "X-Admin-Guard": cfg.admin_guard,
        "Content-Type": "application/json",
    }


def post_issue_user_key(cfg: Config) -> Tuple[str, str]:
    payload = {
        "type": "user",
        "count": 1,
        "label": cfg.label_prefix,
    }
    resp = request_json(
        url=f"{cfg.server_url}/admin/keys/issue",
        method="POST",
        headers=headers(cfg),
        payload=payload,
        timeout=cfg.timeout,
    )
    if not bool(resp.get("ok", False)):
        raise HttpCallError(200, json.dumps(resp, ensure_ascii=False))
    keys = resp.get("keys")
    if not isinstance(keys, list) or len(keys) < 1:
        raise HttpCallError(200, json.dumps({"error": "missing_keys_in_issue_response", "raw": resp}, ensure_ascii=False))
    k = str(keys[0] or "").strip()
    if not k.startswith("k_"):
        raise HttpCallError(200, json.dumps({"error": "invalid_key_prefix", "key": k}, ensure_ascii=False))
    return k, sha256_hex(k)


def post_users_create(cfg: Config, key_hash: str) -> Dict[str, Any]:
    # 目标每人 30 账号时，若基础上限 20，则 delta=+10
    account_limit_delta = cfg.accounts_per_user - 20
    payload = {
        "key_hashes": [key_hash],
        "label_prefix": cfg.label_prefix,
        "account_limit_delta": account_limit_delta,
        "dry_run": False,
    }
    return request_json(
        url=f"{cfg.server_url}/admin/users/create",
        method="POST",
        headers=headers(cfg),
        payload=payload,
        timeout=cfg.timeout,
    )


def post_users_rebind(cfg: Config, key_hash: str) -> Dict[str, Any]:
    # 关键：source_owner_hashes 不留空，避免服务端自动选择“所有用户 owner”。
    # 这里放一个固定 dummy + include_public_pool=true，只从公共池(-1)抽取。
    dummy_owner = "0" * 64
    payload = {
        "key_hashes": [key_hash],
        "accounts_per_user": cfg.accounts_per_user,
        "source_owner_hashes": [dummy_owner],
        "include_public_pool": True,
        "label_prefix": cfg.label_prefix,
        "dry_run": False,
    }
    return request_json(
        url=f"{cfg.server_url}/admin/users/rebind",
        method="POST",
        headers=headers(cfg),
        payload=payload,
        timeout=cfg.timeout,
    )


def stage_done_count(st: Dict[str, Any], stage: str) -> int:
    c = 0
    for r in st.get("records", []):
        if r.get("stage") == stage:
            c += 1
    return c


def print_progress(st: Dict[str, Any], start_ts: float) -> None:
    total = int(st.get("users", 0) or 0)
    done = stage_done_count(st, "rebind_done")
    uncertain = stage_done_count(st, "rebind_uncertain")
    elapsed = max(1e-6, time.time() - start_ts)
    speed = done / elapsed * 60.0
    eta_min = ((total - done) / speed) if speed > 1e-9 else -1
    eta_text = f"{eta_min:.1f}m" if eta_min >= 0 else "--"
    print(
        f"[PROGRESS] done={done}/{total} uncertain={uncertain} "
        f"speed={speed:.2f} users/min elapsed={elapsed/60:.1f}m eta={eta_text}",
        flush=True,
    )


def get_or_create_record(st: Dict[str, Any], idx: int) -> Tuple[Dict[str, Any], bool]:
    for r in st.get("records", []):
        if int(r.get("index", 0)) == idx:
            return r, False

    rec = {
        "index": idx,
        "key": "",
        "key_hash": "",
        "stage": "init",
        "created_at": utc_now(),
        "updated_at": utc_now(),
        "last_error": "",
        "attempts": 0,
    }
    st["records"].append(rec)
    return rec, True


def post_users_create_dry_run(cfg: Config, key_hash: str) -> Dict[str, Any]:
    payload = {
        "key_hashes": [key_hash],
        "label_prefix": cfg.label_prefix,
        "account_limit_delta": 0,
        "dry_run": True,
    }
    return request_json(
        url=f"{cfg.server_url}/admin/users/create",
        method="POST",
        headers=headers(cfg),
        payload=payload,
        timeout=cfg.timeout,
    )


def _is_hash_available_for_user_create(cfg: Config, key_hash: str) -> bool:
    probe = post_users_create_dry_run(cfg, key_hash)
    if not bool(probe.get("ok", False)):
        raise HttpCallError(200, json.dumps(probe, ensure_ascii=False))
    exists_count = int(probe.get("exists_count", 0) or 0)
    return exists_count <= 0


def ensure_unique_key_for_record(cfg: Config, st: Dict[str, Any], rec: Dict[str, Any], *, max_tries: int = 12) -> None:
    # 本地 + 远端双重去重：避免与已有用户 key_hash 冲突。
    local_used = {
        str(r.get("key_hash", "")).strip().lower()
        for r in st.get("records", [])
        if r is not rec
    }

    for _ in range(max_tries):
        k = str(rec.get("key") or "").strip()
        h = str(rec.get("key_hash") or "").strip().lower()

        if not k.startswith("k_") or len(h) != 64:
            k = gen_user_key()
            h = sha256_hex(k)

        if h in local_used:
            rec["key"] = gen_user_key()
            rec["key_hash"] = sha256_hex(rec["key"])
            rec["updated_at"] = utc_now()
            continue

        if not _is_hash_available_for_user_create(cfg, h):
            rec["key"] = gen_user_key()
            rec["key_hash"] = sha256_hex(rec["key"])
            rec["updated_at"] = utc_now()
            continue

        rec["key"] = k
        rec["key_hash"] = h
        rec["updated_at"] = utc_now()
        return

    raise SystemExit("[FATAL] 无法在限定次数内生成无冲突 USER_KEY，请重试。")


def run(cfg: Config) -> int:
    st = ensure_state(cfg)

    if cfg.assume_uncertain_done:
        for r in st.get("records", []):
            if r.get("stage") == "rebind_uncertain":
                r["stage"] = "rebind_done"
                r["updated_at"] = utc_now()
                r["last_error"] = "marked_done_by_assume_uncertain_done"
        st["updated_at"] = utc_now()
        save_state(cfg.state_file, st)

    start_ts = time.time()
    total = int(st.get("users", 0) or 0)

    uncertain_exists = any(r.get("stage") == "rebind_uncertain" for r in st.get("records", []))
    if uncertain_exists and not cfg.retry_uncertain:
        print(
            "[STOP] 检测到 rebind_uncertain 记录。为避免潜在重复搬运，默认暂停。\n"
            "       如已人工确认可重试，请加参数 --retry-uncertain。\n"
            "       若确认已成功可直接标记完成，请加参数 --assume-uncertain-done。",
            flush=True,
        )
        export_keys(cfg.keys_file, st)
        return 2

    for idx in range(1, total + 1):
        rec, created_new = get_or_create_record(st, idx)
        stage = str(rec.get("stage") or "init")

        # 仅在“确定尚未发起 create”的阶段准备 key，避免更换已使用 key。
        if created_new or (stage == "init" and int(rec.get("attempts", 0) or 0) == 0):
            if str(cfg.key_source).lower() == "server":
                # 即便由服务端发 key，也做一次 dry_run 可用性校验，避免极小概率冲突或脏历史数据影响。
                ok = False
                for _ in range(12):
                    k, h = post_issue_user_key(cfg)
                    if _is_hash_available_for_user_create(cfg, h):
                        rec["key"] = k
                        rec["key_hash"] = h
                        rec["updated_at"] = utc_now()
                        ok = True
                        break
                if not ok:
                    raise SystemExit("[FATAL] 服务端发放的 USER_KEY 连续冲突，请稍后重试。")
            else:
                # local: 与服务端同算法 + dry_run 去重预检
                if not str(rec.get("key") or "").startswith("k_"):
                    rec["key"] = gen_user_key()
                    rec["key_hash"] = sha256_hex(str(rec["key"]))
                ensure_unique_key_for_record(cfg, st, rec)
            save_state(cfg.state_file, st)
            stage = str(rec.get("stage") or "init")

        if stage == "rebind_done":
            continue
        if stage == "rebind_uncertain" and not cfg.retry_uncertain:
            continue

        key = str(rec.get("key") or "").strip()
        key_hash = str(rec.get("key_hash") or "").strip().lower()
        if not key.startswith("k_") or len(key_hash) != 64:
            rec["stage"] = "init"
            rec["last_error"] = "invalid_local_key_or_hash"

        print(f"[USER {idx}/{total}] stage={rec.get('stage')} key={key[:12]}...", flush=True)

        if cfg.dry_run:
            rec["stage"] = "dry_run"
            rec["updated_at"] = utc_now()
            save_state(cfg.state_file, st)
            continue

        # Step-1: users/create（幂等）
        try:
            rec["attempts"] = int(rec.get("attempts", 0) or 0) + 1
            resp_create = post_users_create(cfg, key_hash)
            if not bool(resp_create.get("ok", False)):
                raise HttpCallError(200, json.dumps(resp_create, ensure_ascii=False))
            rec["stage"] = "user_created"
            rec["last_error"] = ""
            rec["updated_at"] = utc_now()
            save_state(cfg.state_file, st)
            print(
                f"[USER {idx}/{total}] users/create ok created={resp_create.get('created_count')} enabled_existing={resp_create.get('enabled_existing_count')}",
                flush=True,
            )
        except HttpCallError as e:
            rec["last_error"] = f"users_create_http_{e.status}: {e.body[:400]}"
            rec["updated_at"] = utc_now()
            save_state(cfg.state_file, st)
            print(f"[ERROR] users/create failed idx={idx} {rec['last_error']}", flush=True)
            return 1
        except NetworkUncertainError as e:
            rec["stage"] = "create_uncertain"
            rec["last_error"] = f"users_create_network_uncertain: {e}"
            rec["updated_at"] = utc_now()
            st["notes"].append({"ts": utc_now(), "idx": idx, "event": "create_uncertain"})
            save_state(cfg.state_file, st)
            print(
                f"[STOP] users/create 网络不确定失败 idx={idx}，已保存断点。"
                "请直接重跑脚本（users/create 为幂等）。",
                flush=True,
            )
            return 3

        # Step-2: users/rebind（非严格幂等；网络不确定时不自动重放）
        try:
            resp_rebind = post_users_rebind(cfg, key_hash)
            if not bool(resp_rebind.get("ok", False)):
                raise HttpCallError(200, json.dumps(resp_rebind, ensure_ascii=False))

            rec["stage"] = "rebind_done"
            rec["bound_count"] = int(resp_rebind.get("moved_total", 0) or 0)
            rec["last_error"] = ""
            rec["updated_at"] = utc_now()
            save_state(cfg.state_file, st)

            print(
                f"[USER {idx}/{total}] rebind ok moved={rec.get('bound_count')} accounts_per_user={cfg.accounts_per_user}",
                flush=True,
            )
            print_progress(st, start_ts)

        except HttpCallError as e:
            rec["last_error"] = f"rebind_http_{e.status}: {e.body[:400]}"
            rec["updated_at"] = utc_now()
            save_state(cfg.state_file, st)
            print(f"[ERROR] rebind failed idx={idx} {rec['last_error']}", flush=True)
            return 1
        except NetworkUncertainError as e:
            rec["stage"] = "rebind_uncertain"
            rec["last_error"] = f"rebind_network_uncertain: {e}"
            rec["updated_at"] = utc_now()
            st["notes"].append({"ts": utc_now(), "idx": idx, "event": "rebind_uncertain"})
            save_state(cfg.state_file, st)
            print(
                f"[STOP] rebind 网络不确定失败 idx={idx}。"
                "已标记 rebind_uncertain 并停止，避免自动重放造成重复搬运。",
                flush=True,
            )
            return 4

    st["updated_at"] = utc_now()
    save_state(cfg.state_file, st)
    export_keys(cfg.keys_file, st)

    done = stage_done_count(st, "rebind_done")
    uncertain = stage_done_count(st, "rebind_uncertain")
    print("=" * 72, flush=True)
    print(f"[DONE] rebind_done={done}/{total} rebind_uncertain={uncertain}", flush=True)
    print(f"[OUT ] keys_file={os.path.abspath(cfg.keys_file)}", flush=True)
    print(f"[OUT ] state_file={os.path.abspath(cfg.state_file)}", flush=True)
    print("=" * 72, flush=True)
    return 0 if uncertain == 0 and done == total else 5


def parse_args(argv: List[str]) -> Config:
    p = argparse.ArgumentParser(description="批量创建用户并绑定账号（断点续传 + 进度输出）")
    p.add_argument("--server-url", required=True, help="例如 https://refill.aiaimimi.com")
    p.add_argument("--admin-token", required=True, help="管理员令牌")
    p.add_argument("--admin-guard", required=True, help="管理员护卫码")
    p.add_argument("--users", type=int, default=200, help="用户数，默认 200")
    p.add_argument("--accounts-per-user", type=int, default=30, help="每用户账号数，默认 30")
    p.add_argument("--state-file", default="./out/bulk_users_200x30.state.json", help="断点状态文件")
    p.add_argument("--keys-file", default="./out/bulk_users_200x30.keys.csv", help="导出 key 文件")
    p.add_argument("--label-prefix", default="ops-batch-user", help="服务端 user label 前缀")
    p.add_argument("--timeout", type=int, default=90, help="单请求超时秒数")
    p.add_argument("--dry-run", action="store_true", help="只生成本地断点和 key，不调用服务端")
    p.add_argument("--retry-uncertain", action="store_true", help="允许重试 rebind_uncertain（有重复搬运风险）")
    p.add_argument("--assume-uncertain-done", action="store_true", help="将 rebind_uncertain 直接标记完成（人工确认后使用）")
    p.add_argument("--key-source", choices=["server", "local"], default="server", help="key 来源：server=服务端生成(默认)，local=本地生成")

    a = p.parse_args(argv)

    users = max(1, min(2000, int(a.users)))
    apu = max(1, min(500, int(a.accounts_per_user)))
    server = str(a.server_url).rstrip("/").strip()
    if not (server.startswith("http://") or server.startswith("https://")):
        raise SystemExit("--server-url 必须以 http:// 或 https:// 开头")

    return Config(
        server_url=server,
        admin_token=str(a.admin_token).strip(),
        admin_guard=str(a.admin_guard).strip(),
        users=users,
        accounts_per_user=apu,
        state_file=str(a.state_file),
        keys_file=str(a.keys_file),
        label_prefix=str(a.label_prefix).strip() or "ops-batch-user",
        timeout=max(10, int(a.timeout)),
        dry_run=bool(a.dry_run),
        retry_uncertain=bool(a.retry_uncertain),
        assume_uncertain_done=bool(a.assume_uncertain_done),
        key_source=str(a.key_source).strip().lower(),
    )


def main(argv: List[str]) -> int:
    cfg = parse_args(argv)
    return run(cfg)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
