from __future__ import annotations

import hashlib
import json
import os
import random
import shutil
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from typing import Any


def _utc_now_iso() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def _sha256_hex(s: str) -> str:
    return hashlib.sha256(s.encode("utf-8")).hexdigest()


def _data_dir() -> str:
    return os.environ.get("DATA_DIR") or os.path.join(os.path.dirname(__file__), "data")


def _wait_update_dir() -> str:
    return os.path.join(_data_dir(), os.environ.get("WAIT_UPDATE_DIRNAME") or "wait_update")


def _processing_dir() -> str:
    return os.path.join(_wait_update_dir(), "_processing")


def _infer_account_id(auth_obj: Any) -> str | None:
    """Infer account_id (the primary unique id).

    约定：account_id 是唯一码；我们用它作为主键来源。
    """

    if not isinstance(auth_obj, dict):
        return None

    raw = auth_obj.get("account_id")
    if raw is None:
        return None

    v = str(raw).strip()
    return v or None


def _read_json(path: str) -> Any:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def _post_json(*, url: str, headers: dict[str, str], payload: Any, timeout: int = 30) -> tuple[int, str]:
    data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    req = urllib.request.Request(url, data=data, headers=headers, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            status = int(getattr(resp, "status", 200))
            text = resp.read().decode("utf-8", errors="replace")
            return status, text
    except urllib.error.HTTPError as e:
        try:
            body = e.read().decode("utf-8", errors="replace")
        except Exception:
            body = str(e)
        return int(getattr(e, "code", 0) or 0), body


def _worker_base_url() -> str:
    v = (os.environ.get("REFILL_SERVER_URL") or os.environ.get("INFINITE_REFILL_SERVER_URL") or "").strip()
    if not v:
        raise RuntimeError("Missing REFILL_SERVER_URL (e.g. https://refill.aiaimimi.com)")
    return v.rstrip("/")


def _upload_key() -> str:
    v = (os.environ.get("REFILL_UPLOAD_KEY") or os.environ.get("INFINITE_REFILL_UPLOAD_KEY") or "").strip()
    if not v:
        raise RuntimeError("Missing REFILL_UPLOAD_KEY")
    return v


def _accounts_register_url() -> str:
    return _worker_base_url() + "/v1/accounts/register"


def _build_register_payload(*, auth_obj: Any, account_id: str) -> dict[str, Any]:
    """Build payload for InfiniteRefill `/v1/accounts/register`.

    你已确认：account_id 是唯一码，应作为主 key。

    说明：InfiniteRefill 服务端当前仍要求 `email_hash` 是 sha256 hex。
    为了保持兼容且满足“主键=account_id”的语义，这里采用：

        email_hash = sha256(account_id)

    服务端会按 `email_hash` 去重落库（accounts 表主键维度），从而等价于
    以 account_id 作为主键。
    """

    acc = str(account_id).strip()
    if not acc:
        raise RuntimeError("missing account_id")

    email_hash = _sha256_hex(acc)

    return {
        "accounts": [
            {
                "email_hash": email_hash,
                "account_id": acc,
                "seen_at": _utc_now_iso(),
                "auth_json": auth_obj,
            }
        ]
    }


def _claim_one_file() -> str | None:
    """Atomically move one json file from wait_update/ into _processing/.

    This avoids double-upload when multiple uploader processes/containers share a volume.
    """

    src_dir = _wait_update_dir()
    if not os.path.isdir(src_dir):
        return None

    os.makedirs(_processing_dir(), exist_ok=True)

    # only consider root-level .json files
    try:
        names = [
            n
            for n in os.listdir(src_dir)
            if n.lower().endswith(".json") and os.path.isfile(os.path.join(src_dir, n))
        ]
    except Exception:
        return None

    if not names:
        return None

    random.shuffle(names)

    for name in names:
        src = os.path.join(src_dir, name)
        dst = os.path.join(_processing_dir(), name)
        try:
            os.replace(src, dst)
            return dst
        except FileNotFoundError:
            continue
        except PermissionError:
            continue
        except OSError:
            continue

    return None


def _release_stale_processing(*, stale_seconds: int) -> None:
    pdir = _processing_dir()
    if not os.path.isdir(pdir):
        return

    now = time.time()
    for name in os.listdir(pdir):
        if not name.lower().endswith(".json"):
            continue
        path = os.path.join(pdir, name)
        try:
            st = os.stat(path)
        except Exception:
            continue
        if now - st.st_mtime < stale_seconds:
            continue

        # move back for retry
        try:
            os.replace(path, os.path.join(_wait_update_dir(), name))
        except Exception:
            pass


def _upload_one(path: str) -> bool:
    name = os.path.basename(path)
    auth_obj = _read_json(path)

    account_id = _infer_account_id(auth_obj)
    if not account_id:
        # 不上传：放回队列等待人工处理（或未来 producer 补齐 account_id）
        raise RuntimeError(f"missing account_id in auth json: {name}")

    payload = _build_register_payload(auth_obj=auth_obj, account_id=account_id)

    headers = {
        "Content-Type": "application/json",
        "X-Upload-Key": _upload_key(),
    }

    status, text = _post_json(url=_accounts_register_url(), headers=headers, payload=payload, timeout=30)

    # success condition: HTTP 2xx and ok=true
    if 200 <= status < 300:
        try:
            obj = json.loads(text) if text else {}
        except Exception:
            obj = {}
        if obj.get("ok") is True:
            return True

    # print minimal diagnostics; keep file for retry
    sys.stderr.write(f"[uploader] upload failed: file={name} http={status} resp={text[:500]}\n")
    return False


def main() -> int:
    poll_seconds = int(os.environ.get("UPLOAD_POLL_SECONDS") or "2")
    stale_seconds = int(os.environ.get("UPLOAD_STALE_SECONDS") or "600")
    sleep_on_error = int(os.environ.get("UPLOAD_SLEEP_ON_ERROR") or "5")

    os.makedirs(_wait_update_dir(), exist_ok=True)
    os.makedirs(_processing_dir(), exist_ok=True)

    print(f"[uploader] wait_update_dir={_wait_update_dir()}")
    print(f"[uploader] post_url={_accounts_register_url()}")

    while True:
        # requeue stale processing files (crash-safe)
        _release_stale_processing(stale_seconds=stale_seconds)

        claimed = _claim_one_file()
        if not claimed:
            time.sleep(poll_seconds)
            continue

        ok = False
        try:
            ok = _upload_one(claimed)
        except Exception as e:
            sys.stderr.write(f"[uploader] exception: {e}\n")

        if ok:
            # delete the processing file ONLY on success
            try:
                os.remove(claimed)
            except Exception:
                pass
            continue

        # failure: move back for retry
        try:
            os.replace(claimed, os.path.join(_wait_update_dir(), os.path.basename(claimed)))
        except Exception:
            # worst-case keep it in processing; stale reaper will recover
            pass

        time.sleep(sleep_on_error)


if __name__ == "__main__":
    raise SystemExit(main())
