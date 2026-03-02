from __future__ import annotations

import json
import os
import random
import sys
import threading
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from typing import Any


def _utc_now_iso() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def _data_dir() -> str:
    return os.environ.get("DATA_DIR") or os.path.join(os.path.dirname(__file__), "data")


def _wait_update_dir() -> str:
    return os.path.join(_data_dir(), os.environ.get("WAIT_UPDATE_DIRNAME") or "wait_update")


def _fixed_success_dir() -> str:
    return os.path.join(_data_dir(), os.environ.get("FIXED_SUCCESS_DIRNAME") or "fixed_success")


def _queue_dirs() -> list[str]:
    # 上传者同时消费两类产物：
    # 1) 注册成功直接产物 wait_update/
    # 2) 修补成功产物 fixed_success/
    return [_wait_update_dir(), _fixed_success_dir()]


def _instance_id() -> str:
    # 注意：docker compose --scale 时，环境变量 INSTANCE_ID 往往完全相同；
    # HOSTNAME（容器 ID）才是每个副本天然唯一。
    raw = (os.environ.get("HOSTNAME") or os.environ.get("INSTANCE_ID") or "uploader").strip()
    safe = "".join(ch if ch.isalnum() or ch in ("-", "_") else "_" for ch in raw)
    return safe or "uploader"


def _processing_dir(src_dir: str) -> str:
    # 多 uploader 容器共享同一数据卷时，必须隔离 processing 目录，避免互相覆盖/抢占。
    return os.path.join(src_dir, f"_processing_{_instance_id()}")


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
    except urllib.error.URLError as e:
        return 0, str(e)


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


def _build_register_account(*, auth_obj: Any, account_id: str) -> dict[str, Any]:
    """Build single account object for InfiniteRefill `/v1/accounts/register` v2."""

    acc = str(account_id).strip()
    if not acc:
        raise RuntimeError("missing account_id")

    if not isinstance(auth_obj, dict):
        raise RuntimeError("invalid auth json object")

    email = str(auth_obj.get("email") or "").strip()
    password = str(auth_obj.get("password") or "").strip()
    r2_url = str(auth_obj.get("r2_url") or "").strip() or None

    if not email:
        raise RuntimeError("missing email in auth json")
    if not password:
        raise RuntimeError("missing password in auth json")

    owner = str((os.environ.get("REFILL_DEFAULT_OWNER") or "-1")).strip() or "-1"

    account: dict[str, Any] = {
        "account_id": acc,
        "email": email,
        "password": password,
        "owner": owner,
        # 按要求上传完整 auth_json，由 Worker 转存 R2
        "auth_json": auth_obj,
    }
    if r2_url:
        account["r2_url"] = r2_url

    return account


def _build_register_payload(*, accounts: list[dict[str, Any]]) -> dict[str, Any]:
    return {"accounts": accounts}


def _claim_one_file() -> tuple[str, str] | None:
    """Atomically claim one json from either wait_update/ or fixed_success/.

    Returns:
      (claimed_path, source_queue_dir)

    实现说明：
    - 不再每次全量扫描目录（在 8k~10k 文件时会非常慢）
    - 每个队列只抽样一小批候选名，降低目录扫描与 stat 开销
    """

    queue_dirs = _queue_dirs()
    random.shuffle(queue_dirs)

    # 每次认领仅扫描有限个候选，避免全目录遍历导致吞吐掉到 0
    claim_scan_limit = max(20, int(os.environ.get("UPLOAD_CLAIM_SCAN_LIMIT") or "200"))

    for src_dir in queue_dirs:
        if not os.path.isdir(src_dir):
            continue

        pdir = _processing_dir(src_dir)
        os.makedirs(pdir, exist_ok=True)

        names: list[str] = []
        try:
            with os.scandir(src_dir) as it:
                for ent in it:
                    # 跳过目录（特别是 _processing）
                    if not ent.is_file(follow_symlinks=False):
                        continue
                    nm = ent.name
                    if not nm.lower().endswith(".json"):
                        continue
                    names.append(nm)
                    if len(names) >= claim_scan_limit:
                        break
        except Exception:
            continue

        if not names:
            continue

        random.shuffle(names)

        for name in names:
            src = os.path.join(src_dir, name)
            dst = os.path.join(pdir, name)
            try:
                os.replace(src, dst)
                return dst, src_dir
            except FileNotFoundError:
                continue
            except PermissionError:
                continue
            except OSError:
                continue

    return None


def _claim_batch_files(*, batch_size: int) -> list[tuple[str, str]]:
    out: list[tuple[str, str]] = []
    for _ in range(max(1, batch_size)):
        one = _claim_one_file()
        if not one:
            break
        out.append(one)
    return out


def _release_stale_processing(*, stale_seconds: int) -> None:
    for queue_dir in _queue_dirs():
        pdir = _processing_dir(queue_dir)
        if not os.path.isdir(pdir):
            continue

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

            # move back for retry (回到各自来源队列)
            try:
                os.replace(path, os.path.join(queue_dir, name))
            except Exception:
                pass


class _Metrics:
    def __init__(self) -> None:
        self.lock = threading.Lock()
        self.start_ts = time.time()
        self.files_ok = 0
        self.files_fail = 0
        self.files_claimed = 0
        self.batches = 0
        self.http_calls = 0
        self.http_ms = 0.0
        self.prepare_ms = 0.0

    def add(self, *, files_ok: int, files_fail: int, files_claimed: int, batches: int, http_calls: int, http_ms: float, prepare_ms: float) -> None:
        with self.lock:
            self.files_ok += files_ok
            self.files_fail += files_fail
            self.files_claimed += files_claimed
            self.batches += batches
            self.http_calls += http_calls
            self.http_ms += http_ms
            self.prepare_ms += prepare_ms

    def snapshot(self) -> dict[str, float]:
        with self.lock:
            elapsed = max(0.001, time.time() - self.start_ts)
            total_done = self.files_ok + self.files_fail
            return {
                "elapsed_s": elapsed,
                "files_ok": float(self.files_ok),
                "files_fail": float(self.files_fail),
                "files_claimed": float(self.files_claimed),
                "files_done": float(total_done),
                "batches": float(self.batches),
                "http_calls": float(self.http_calls),
                "throughput_per_min": float(total_done) * 60.0 / elapsed,
                "ok_per_min": float(self.files_ok) * 60.0 / elapsed,
                "avg_http_ms": self.http_ms / max(1.0, float(self.http_calls)),
                "avg_prepare_ms": self.prepare_ms / max(1.0, float(self.batches)),
            }


def _upload_batch(claimed_items: list[tuple[str, str]], *, timeout: int) -> tuple[list[bool], dict[str, float]]:
    """Upload a claimed batch in one HTTP request.

    Returns:
      (ok_flags_for_each_claimed_item, metrics)
    """

    t0 = time.perf_counter()
    n = len(claimed_items)
    ok_flags = [False] * n
    accounts: list[dict[str, Any]] = []
    payload_to_claimed_idx: list[int] = []

    for i, (path, _src) in enumerate(claimed_items):
        name = os.path.basename(path)
        try:
            auth_obj = _read_json(path)
            account_id = _infer_account_id(auth_obj)
            if not account_id:
                raise RuntimeError(f"missing account_id in auth json: {name}")
            account = _build_register_account(auth_obj=auth_obj, account_id=account_id)
            accounts.append(account)
            payload_to_claimed_idx.append(i)
        except Exception as e:
            sys.stderr.write(f"[uploader] precheck failed: file={name} err={e}\n")
            ok_flags[i] = False

    prepare_ms = (time.perf_counter() - t0) * 1000.0

    if not accounts:
        return ok_flags, {
            "prepare_ms": prepare_ms,
            "http_ms": 0.0,
            "http_calls": 0.0,
            "sent": 0.0,
        }

    payload = _build_register_payload(accounts=accounts)
    headers = {
        "Content-Type": "application/json",
        "X-Upload-Key": _upload_key(),
        # 避免被 Cloudflare/WAF 以默认 Python UA 拦截（1010）
        "User-Agent": os.environ.get("UPLOADER_USER_AGENT") or "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36",
        "Accept": "application/json, text/plain, */*",
    }

    # 可选：带管理员头（某些站点策略会放行 admin header）
    admin_auth = (os.environ.get("MAILCREATE_ADMIN_AUTH") or os.environ.get("REFILL_ADMIN_AUTH") or "").strip()
    if admin_auth:
        headers["X-Admin-Auth"] = admin_auth

    t_http = time.perf_counter()
    status, text = _post_json(url=_accounts_register_url(), headers=headers, payload=payload, timeout=timeout)
    http_ms = (time.perf_counter() - t_http) * 1000.0

    print(
        f"[uploader] batch post done: files_claimed={n} files_sent={len(accounts)} "
        f"http={status} prepare_ms={prepare_ms:.1f} http_ms={http_ms:.1f}"
    )

    if 200 <= status < 300:
        try:
            obj = json.loads(text) if text else {}
        except Exception:
            obj = {}

        if isinstance(obj, dict) and obj.get("ok") is True:
            errs = obj.get("errors") if isinstance(obj.get("errors"), list) else []
            bad_payload_idx: set[int] = set()
            for e in errs:
                try:
                    idx = int((e or {}).get("idx"))
                    if 0 <= idx < len(accounts):
                        bad_payload_idx.add(idx)
                except Exception:
                    continue

            for payload_idx, claimed_idx in enumerate(payload_to_claimed_idx):
                ok_flags[claimed_idx] = payload_idx not in bad_payload_idx

            if bad_payload_idx:
                sys.stderr.write(
                    f"[uploader] batch logical partial-failed: http={status} "
                    f"bad={len(bad_payload_idx)}/{len(accounts)} errs={json.dumps(errs, ensure_ascii=False)[:1000]}\n"
                )
            return ok_flags, {
                "prepare_ms": prepare_ms,
                "http_ms": http_ms,
                "http_calls": 1.0,
                "sent": float(len(accounts)),
            }

    # 全量失败：回退本批次所有已发送项
    for claimed_idx in payload_to_claimed_idx:
        ok_flags[claimed_idx] = False

    sys.stderr.write(f"[uploader] batch failed: http={status} resp={text[:500]}\n")
    return ok_flags, {
        "prepare_ms": prepare_ms,
        "http_ms": http_ms,
        "http_calls": 1.0,
        "sent": float(len(accounts)),
    }


def _reporter_loop(metrics: _Metrics, *, interval_seconds: int) -> None:
    interval = max(5, interval_seconds)
    while True:
        time.sleep(interval)
        s = metrics.snapshot()
        print(
            "[uploader][metrics] "
            f"elapsed={s['elapsed_s']:.0f}s "
            f"claimed={int(s['files_claimed'])} done={int(s['files_done'])} "
            f"ok={int(s['files_ok'])} fail={int(s['files_fail'])} "
            f"throughput={s['throughput_per_min']:.1f}/min ok_rate={s['ok_per_min']:.1f}/min "
            f"batches={int(s['batches'])} http_calls={int(s['http_calls'])} "
            f"avg_prepare_ms={s['avg_prepare_ms']:.1f} avg_http_ms={s['avg_http_ms']:.1f}"
        )


def _stale_reaper_loop(*, stale_seconds: int) -> None:
    loop_sleep = max(5, min(60, stale_seconds // 2 if stale_seconds > 0 else 30))
    while True:
        try:
            _release_stale_processing(stale_seconds=stale_seconds)
        except Exception as e:
            sys.stderr.write(f"[uploader] stale_reaper exception: {e}\n")
        time.sleep(loop_sleep)


def _worker_loop(*, worker_id: int, poll_seconds: int, sleep_on_error: int, batch_size: int, timeout: int, metrics: _Metrics) -> None:
    while True:
        claimed = _claim_batch_files(batch_size=batch_size)
        if not claimed:
            time.sleep(poll_seconds)
            continue

        ok_flags, m = _upload_batch(claimed, timeout=timeout)

        ok_count = 0
        fail_count = 0
        any_fail = False

        for (claimed_path, src_queue_dir), ok in zip(claimed, ok_flags):
            if ok:
                ok_count += 1
                try:
                    os.remove(claimed_path)
                except Exception:
                    pass
            else:
                fail_count += 1
                any_fail = True
                try:
                    os.replace(claimed_path, os.path.join(src_queue_dir, os.path.basename(claimed_path)))
                except Exception:
                    # worst-case keep it in processing; stale reaper will recover
                    pass

        metrics.add(
            files_ok=ok_count,
            files_fail=fail_count,
            files_claimed=len(claimed),
            batches=1,
            http_calls=int(m.get("http_calls", 0.0)),
            http_ms=float(m.get("http_ms", 0.0)),
            prepare_ms=float(m.get("prepare_ms", 0.0)),
        )

        print(
            f"[uploader][worker-{worker_id}] batch done "
            f"claimed={len(claimed)} ok={ok_count} fail={fail_count}"
        )

        if any_fail:
            time.sleep(sleep_on_error)


def main() -> int:
    poll_seconds = int(os.environ.get("UPLOAD_POLL_SECONDS") or "2")
    stale_seconds = int(os.environ.get("UPLOAD_STALE_SECONDS") or "600")
    sleep_on_error = int(os.environ.get("UPLOAD_SLEEP_ON_ERROR") or "5")

    # 新增：并发与合包参数
    workers = int(os.environ.get("UPLOAD_WORKERS") or "4")
    batch_size = int(os.environ.get("UPLOAD_BATCH_SIZE") or "10")
    timeout = int(os.environ.get("UPLOAD_HTTP_TIMEOUT") or "30")
    metrics_interval = int(os.environ.get("UPLOAD_METRICS_INTERVAL") or "30")

    workers = max(1, workers)
    batch_size = max(1, min(2000, batch_size))

    for q in _queue_dirs():
        os.makedirs(q, exist_ok=True)
        os.makedirs(_processing_dir(q), exist_ok=True)

    print(f"[uploader] wait_update_dir={_wait_update_dir()}")
    print(f"[uploader] fixed_success_dir={_fixed_success_dir()}")
    print(f"[uploader] post_url={_accounts_register_url()}")
    print(
        f"[uploader] config workers={workers} batch_size={batch_size} timeout={timeout}s "
        f"poll={poll_seconds}s stale={stale_seconds}s sleep_on_error={sleep_on_error}s"
    )

    metrics = _Metrics()

    t_reaper = threading.Thread(target=_stale_reaper_loop, kwargs={"stale_seconds": stale_seconds}, name="uploader_stale_reaper", daemon=True)
    t_reaper.start()

    t_reporter = threading.Thread(
        target=_reporter_loop,
        kwargs={"metrics": metrics, "interval_seconds": metrics_interval},
        name="uploader_reporter",
        daemon=True,
    )
    t_reporter.start()

    threads: list[threading.Thread] = []
    for i in range(workers):
        t = threading.Thread(
            target=_worker_loop,
            kwargs={
                "worker_id": i + 1,
                "poll_seconds": poll_seconds,
                "sleep_on_error": sleep_on_error,
                "batch_size": batch_size,
                "timeout": timeout,
                "metrics": metrics,
            },
            name=f"uploader_worker_{i + 1}",
            daemon=True,
        )
        t.start()
        threads.append(t)

    # 主线程阻塞等待（进程守护运行）
    for t in threads:
        t.join()

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
