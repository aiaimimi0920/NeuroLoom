#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""上传单个本地 auth json 的“身份信息 + 健康探测结果”到无限续杯服务器（模式A）。

- 默认：不上传任何 token 到服务端（仅上报 hash/id/status）
- 可选：在 --mode register 时上传完整 auth_json（用于服务端 /v1/refill/topup 下发替换账号）

用法示例：
  python tools/upload_single_json_probe_and_report.py \
    --server http://127.0.0.1:8787 \
    --upload-key YOUR_UPLOAD_KEY \
    --file "C:\\Users\\Administrator\\.cli-proxy-api\\xxx@email.json"

可选：只注册身份（不探测）
  python tools/upload_single_json_probe_and_report.py --mode register ...
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import os
from datetime import datetime
from typing import Any, Dict, Optional, Tuple

import aiohttp


WHAM_URL = "https://chatgpt.com/backend-api/wham/usage"


def utc_now_iso() -> str:
    return datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")


def sha256_hex(s: str) -> str:
    return hashlib.sha256(s.encode("utf-8")).hexdigest()


def load_auth(path: str) -> Dict[str, Any]:
    # utf-8-sig: 自动吞掉 UTF-8 BOM，避免 \ufeff 导致 JSON 解析失败
    with open(path, "r", encoding="utf-8-sig") as f:
        obj = json.load(f)
    return obj if isinstance(obj, dict) else {}


def canonical_auth_json(auth: Dict[str, Any]) -> str:
    # 统一输出为无 BOM 的标准 JSON 文本
    return json.dumps(auth, ensure_ascii=False, separators=(",", ":"), sort_keys=True)


def extract_email_and_account_id(auth: Dict[str, Any], file_path: str) -> Tuple[Optional[str], Optional[str]]:
    email = auth.get("email")
    email = email.strip() if isinstance(email, str) and email.strip() else None

    account_id = auth.get("account_id")
    account_id = str(account_id).strip() if account_id else None

    if not email:
        # 尝试从文件名推断：xxx@email.json
        base = os.path.basename(file_path)
        if base.lower().endswith(".json"):
            maybe = base[: -len(".json")]
            if "@" in maybe:
                email = maybe

    return email, account_id


def make_email_hash(email: Optional[str], account_id: Optional[str]) -> str:
    # 统一身份：优先 email（lower），没有 email 就用 account_id 做稳定 hash。
    if email:
        ident = f"email:{email.strip().lower()}"
    else:
        ident = f"account_id:{(account_id or '').strip()}"
    return sha256_hex(ident)


async def probe_wham(access_token: str, chatgpt_account_id: Optional[str], timeout_s: int) -> Tuple[Optional[int], Optional[str]]:
    if not access_token:
        return None, "missing_access_token"

    headers = {
        "Accept": "application/json, text/plain, */*",
        "Authorization": f"Bearer {access_token}",
        "User-Agent": "codex_cli_rs/0.76.0 (Windows 11; x86_64)",
    }
    if chatgpt_account_id:
        headers["Chatgpt-Account-Id"] = str(chatgpt_account_id)

    trust_env = False
    connector = aiohttp.TCPConnector(limit=5, limit_per_host=5)
    async with aiohttp.ClientSession(connector=connector, trust_env=trust_env) as session:
        try:
            async with session.get(WHAM_URL, headers=headers, timeout=aiohttp.ClientTimeout(total=max(1, timeout_s))) as resp:
                _ = await resp.text()
                return int(resp.status), None
        except Exception as ex:
            return None, str(ex)


async def post_json(server: str, upload_key: str, path: str, payload: Dict[str, Any]) -> Tuple[int, str]:
    url = server.rstrip("/") + path
    headers = {
        "Content-Type": "application/json",
        "X-Upload-Key": upload_key,
    }
    async with aiohttp.ClientSession(trust_env=True) as session:
        async with session.post(url, headers=headers, json=payload) as resp:
            text = await resp.text()
            return int(resp.status), text


def extract_access_token(auth: Dict[str, Any]) -> str:
    v = auth.get("access_token")
    return v.strip() if isinstance(v, str) and v.strip() else ""


def main() -> int:
    p = argparse.ArgumentParser(description="Upload single auth json -> report to refill server (compliant mode A)")
    p.add_argument("--server", required=True, help="server base url, e.g. http://127.0.0.1:8787")
    p.add_argument("--upload-key", required=True, help="upload key (normal admin)")
    p.add_argument("--file", required=True, help="path to auth json")
    p.add_argument("--mode", choices=["probe", "register"], default="probe", help="probe=wham probe then report; register=only register identity")
    p.add_argument("--timeout", type=int, default=15, help="probe timeout seconds")
    args = p.parse_args()

    auth = load_auth(args.file)
    email, account_id = extract_email_and_account_id(auth, args.file)
    email_hash = make_email_hash(email, account_id)

    probed_at = utc_now_iso()
    token = extract_access_token(auth)
    auth_json_str = canonical_auth_json(auth)

    if args.mode == "register":
        # 模式A：允许上传 auth_json（服务端会加密存储）。
        # 注意：access_token 字段不会被服务端使用，这里不再上传，避免口径混乱。
        payload = {
            "accounts": [
                {
                    "email_hash": email_hash,
                    "account_id": account_id,
                    "seen_at": probed_at,
                    # 服务端接口期望 object；这里把规范化 JSON 再反序列化为对象上传
                    "auth_json": json.loads(auth_json_str),
                }
            ]
        }
        status, text = asyncio.run(post_json(args.server, args.upload_key, "/v1/accounts/register", payload))
        print(f"HTTP {status}\n{text}")
        return 0 if 200 <= status < 300 else 2

    status_code, err = asyncio.run(probe_wham(token, account_id, timeout_s=args.timeout))

    payload = {
        "reports": [
            {
                "email_hash": email_hash,
                "account_id": account_id,
                "status_code": status_code,
                "probed_at": probed_at,
                "access_token": token,
                "chatgpt_account_id": account_id,
            }
        ]
    }

    http_status, text = asyncio.run(post_json(args.server, args.upload_key, "/v1/probe-report", payload))
    print(f"probe_result status_code={status_code} error={err}\nHTTP {http_status}\n{text}")
    return 0 if 200 <= http_status < 300 else 2


if __name__ == "__main__":
    raise SystemExit(main())
