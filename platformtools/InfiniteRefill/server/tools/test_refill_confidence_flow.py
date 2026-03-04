#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
本地集成冒烟：待置信区流程（先采信 -> 回放 -> 判真/判伪）

依赖环境变量：
- SERVER_URL
- USER_KEY_A   (首报用户)
- USER_KEY_B   (回放验证用户)
- ADMIN_TOKEN  (可选，仅用于清理与观测)

说明：
- 该脚本不依赖 pytest，直接 python 执行。
- 主要验证接口语义，不强依赖真实 OpenAI 可用性。
"""

import json
import os
import sys
import time
import urllib.request
import urllib.error


def req_json(method: str, url: str, body: dict | None = None, headers: dict | None = None):
    data = None
    if body is not None:
        data = json.dumps(body, ensure_ascii=False).encode("utf-8")
    h = {"Content-Type": "application/json"}
    if headers:
        h.update(headers)
    req = urllib.request.Request(url, data=data, headers=h, method=method)
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            text = r.read().decode("utf-8", errors="replace")
            return r.getcode(), text, json.loads(text) if text else {}
    except urllib.error.HTTPError as e:
        text = e.read().decode("utf-8", errors="replace")
        try:
            obj = json.loads(text)
        except Exception:
            obj = {"raw": text}
        return e.code, text, obj


def must_env(name: str) -> str:
    v = (os.environ.get(name) or "").strip()
    if not v:
        print(f"[ERROR] missing env: {name}")
        sys.exit(2)
    return v


def main() -> int:
    base = must_env("SERVER_URL").rstrip("/")
    key_a = must_env("USER_KEY_A")
    key_b = must_env("USER_KEY_B")

    # 1) A 上报坏号，触发“先采信”
    bad_acc = f"acc_conf_{int(time.time())}"
    body_a = {
        "target_pool_size": 1,
        "reports": [
            {
                "account_id": bad_acc,
                "status_code": 401,
                "probed_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                "note": "smoke_first_report",
            }
        ],
    }
    c1, _t1, o1 = req_json("POST", f"{base}/v1/refill/topup", body_a, {"X-User-Key": key_a})
    print("[STEP1]", c1, o1.get("ok"), o1.get("replaced_from_requested"))

    if c1 >= 400 or not o1.get("ok"):
        print("[ERROR] step1 failed")
        return 1

    # 2) B 再次上报同号，标记 replay_from_confidence=true，用于判真
    body_b = {
        "target_pool_size": 1,
        "reports": [
            {
                "account_id": bad_acc,
                "status_code": 401,
                "probed_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                "note": "smoke_replay_report",
                "replay_from_confidence": True,
            }
        ],
    }
    c2, _t2, o2 = req_json("POST", f"{base}/v1/refill/topup", body_b, {"X-User-Key": key_b})
    print("[STEP2]", c2, o2.get("ok"), o2.get("replaced_from_requested"))
    if c2 >= 400 or not o2.get("ok"):
        print("[ERROR] step2 failed")
        return 1

    # 3) 读取管理统计（可选）
    admin = (os.environ.get("ADMIN_TOKEN") or "").strip()
    if admin:
        c3, _t3, o3 = req_json("GET", f"{base}/admin/confidence/stats", None, {"Authorization": f"Bearer {admin}"})
        print("[STEP3]", c3, o3.get("ok"), o3.get("confidence_queue"))

    print("[OK] confidence smoke done")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
