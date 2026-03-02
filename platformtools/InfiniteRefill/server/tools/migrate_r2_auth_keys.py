import argparse
import json
import os
import subprocess
import tempfile
import time
from typing import Any

ROOT = os.path.dirname(os.path.dirname(__file__))
DEFAULT_BUCKET = "infinite-refill"
DEFAULT_DB = "refill_server_v2"


def run(cmd: list[str], *, timeout_s: int) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout_s)


def run_ok(cmd: list[str], *, timeout_s: int, retries: int, retry_sleep_s: float) -> tuple[bool, str, bool]:
    """返回 (ok, output, timed_out)。timed_out=True 表示最后一次失败是超时。"""
    last_out = ""
    last_timeout = False

    for i in range(max(1, retries + 1)):
        try:
            p = run(cmd, timeout_s=timeout_s)
            out = (p.stdout or "") + (p.stderr or "")
            if p.returncode == 0:
                return True, out, False
            last_out = out
            last_timeout = False
        except subprocess.TimeoutExpired as e:
            last_out = f"TIMEOUT {timeout_s}s: {' '.join(cmd)}\n{str(e)}"
            last_timeout = True

        if i < retries:
            time.sleep(retry_sleep_s)

    return False, last_out, last_timeout


def _extract_rows_from_d1_json(d: Any) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []

    if isinstance(d, dict):
        if isinstance(d.get("results"), list):
            rows.extend([x for x in d.get("results", []) if isinstance(x, dict)])

        result = d.get("result")
        if isinstance(result, list):
            for block in result:
                if isinstance(block, dict) and isinstance(block.get("results"), list):
                    rows.extend([x for x in block.get("results", []) if isinstance(x, dict)])
        elif isinstance(result, dict) and isinstance(result.get("results"), list):
            rows.extend([x for x in result.get("results", []) if isinstance(x, dict)])

    elif isinstance(d, list):
        for block in d:
            if isinstance(block, dict):
                if isinstance(block.get("results"), list):
                    rows.extend([x for x in block.get("results", []) if isinstance(x, dict)])
                elif "account_id" in block and "r2_url" in block:
                    rows.append(block)

    return rows


def load_rows(*, db: str, remote: bool, timeout_s: int, retries: int, retry_sleep_s: float) -> list[dict[str, Any]]:
    q = (
        "SELECT account_id,r2_url FROM accounts_v2 "
        "WHERE r2_url LIKE 'auth-json/%' AND r2_url NOT LIKE 'auth-json/%/%';"
    )

    cmd = [
        "npx.cmd",
        "wrangler",
        "d1",
        "execute",
        db,
        "--command=" + q,
        "--json",
    ]
    if remote:
        cmd.append("--remote")

    ok, out, _ = run_ok(cmd, timeout_s=timeout_s, retries=retries, retry_sleep_s=retry_sleep_s)
    if not ok:
        print(out)
        raise SystemExit("d1_query_failed")

    raw = (out or "").strip()

    try:
        parsed = json.loads(raw)
        return _extract_rows_from_d1_json(parsed)
    except Exception:
        pass

    lb = raw.find("[")
    rb = raw.rfind("]")
    lbo = raw.find("{")
    rbo = raw.rfind("}")

    candidates: list[str] = []
    if lb >= 0 and rb > lb:
        candidates.append(raw[lb : rb + 1])
    if lbo >= 0 and rbo > lbo:
        candidates.append(raw[lbo : rbo + 1])

    for c in candidates:
        try:
            parsed = json.loads(c)
            rows = _extract_rows_from_d1_json(parsed)
            if rows:
                return rows
        except Exception:
            continue

    print(raw[:4000])
    raise SystemExit("failed_to_extract_json")


def main() -> None:
    p = argparse.ArgumentParser(description="迁移 R2 旧路径 auth-json/{account_id}/{suffix} 到新路径 auth-json/{account_id}-{suffix}，并删除旧对象")
    p.add_argument("--bucket", default=DEFAULT_BUCKET)
    p.add_argument("--db", default=DEFAULT_DB)
    p.add_argument("--local", action="store_true", help="使用本地 D1（默认 remote）")
    p.add_argument("--start-index", type=int, default=0, help="从第 N 条开始，便于断点续跑")
    p.add_argument("--max-rows", type=int, default=0, help="最多处理 N 条，0=全部")
    p.add_argument("--timeout", type=int, default=12, help="每个 wrangler 子命令超时秒数")
    p.add_argument("--retries", type=int, default=0, help="失败重试次数（不含首轮）")
    p.add_argument("--retry-sleep", type=float, default=0.4, help="重试间隔秒")
    p.add_argument("--log-every", type=int, default=20, help="每处理多少条打印一次进度")
    p.add_argument("--max-errors", type=int, default=20, help="错误达到阈值后提前终止，避免卡死")
    p.add_argument("--dry-run", action="store_true", help="只核对与打印，不做 put/delete")
    args = p.parse_args()

    bucket = str(args.bucket).strip()
    rows = load_rows(
        db=str(args.db).strip(),
        remote=not bool(args.local),
        timeout_s=args.timeout,
        retries=args.retries,
        retry_sleep_s=args.retry_sleep,
    )

    if args.start_index > 0:
        rows = rows[args.start_index :]
    if args.max_rows > 0:
        rows = rows[: args.max_rows]

    total = len(rows)
    migrated = 0
    cleaned = 0
    skipped = 0
    errors = 0
    timeouts = 0

    print(json.dumps({
        "phase": "start",
        "total_rows": total,
        "bucket": bucket,
        "db": args.db,
        "remote": not bool(args.local),
        "dry_run": bool(args.dry_run),
        "timeout": args.timeout,
        "retries": args.retries,
        "max_errors": args.max_errors,
    }, ensure_ascii=False))

    for idx, row in enumerate(rows, start=1):
        if args.max_errors > 0 and errors >= args.max_errors:
            print(json.dumps({
                "phase": "abort",
                "reason": "too_many_errors",
                "done": idx - 1,
                "total": total,
                "migrated": migrated,
                "cleaned_duplicate_old": cleaned,
                "skipped": skipped,
                "errors": errors,
                "timeouts": timeouts,
            }, ensure_ascii=False))
            break

        account_id = str(row.get("account_id") or "").strip()
        new_key = str(row.get("r2_url") or "").strip()
        if not account_id or not new_key:
            skipped += 1
            continue

        prefix = f"auth-json/{account_id}-"
        if not new_key.startswith(prefix):
            skipped += 1
            continue

        suffix = new_key[len(prefix):]
        if not suffix:
            skipped += 1
            continue

        old_key = f"auth-json/{account_id}/{suffix}"

        # 性能优先：先只检查 old 是否存在。old 不存在直接跳过。
        ok_old_exists, out_old_exists, old_timeout = run_ok(
            ["npx.cmd", "wrangler", "r2", "object", "get", f"{bucket}/{old_key}", "--pipe"],
            timeout_s=args.timeout,
            retries=args.retries,
            retry_sleep_s=args.retry_sleep,
        )
        if old_timeout:
            timeouts += 1

        if not ok_old_exists:
            # old 已不存在：视为已迁移或无需处理，跳过
            skipped += 1
            if "TIMEOUT" in out_old_exists:
                errors += 1
                print(f"[ERR] check old timeout: {old_key}\n{out_old_exists[:240]}")
            continue

        if args.dry_run:
            # old 存在即代表有事可做（迁移或清理重复 old）
            migrated += 1
        else:
            with tempfile.NamedTemporaryFile(delete=False) as tf:
                tmp_path = tf.name

            try:
                ok_get, out_get, t_get = run_ok(
                    ["npx.cmd", "wrangler", "r2", "object", "get", f"{bucket}/{old_key}", "--file", tmp_path],
                    timeout_s=args.timeout,
                    retries=args.retries,
                    retry_sleep_s=args.retry_sleep,
                )
                if t_get:
                    timeouts += 1
                if not ok_get:
                    errors += 1
                    print(f"[ERR] get old failed: {old_key}\n{out_get[:400]}")
                    continue

                ok_put, out_put, t_put = run_ok(
                    [
                        "npx.cmd",
                        "wrangler",
                        "r2",
                        "object",
                        "put",
                        f"{bucket}/{new_key}",
                        "--file",
                        tmp_path,
                        "--content-type",
                        "application/json; charset=utf-8",
                    ],
                    timeout_s=args.timeout,
                    retries=args.retries,
                    retry_sleep_s=args.retry_sleep,
                )
                if t_put:
                    timeouts += 1
                if not ok_put:
                    errors += 1
                    print(f"[ERR] put new failed: {new_key}\n{out_put[:400]}")
                    continue

                ok_del, out_del, t_del = run_ok(
                    ["npx.cmd", "wrangler", "r2", "object", "delete", f"{bucket}/{old_key}"],
                    timeout_s=args.timeout,
                    retries=args.retries,
                    retry_sleep_s=args.retry_sleep,
                )
                if t_del:
                    timeouts += 1
                if not ok_del:
                    errors += 1
                    print(f"[ERR] delete old failed: {old_key}\n{out_del[:400]}")
                    continue

                cleaned += 1
                migrated += 1
            finally:
                try:
                    os.remove(tmp_path)
                except Exception:
                    pass

        if args.log_every > 0 and idx % args.log_every == 0:
            print(json.dumps({
                "phase": "progress",
                "done": idx,
                "total": total,
                "migrated": migrated,
                "cleaned_duplicate_old": cleaned,
                "skipped": skipped,
                "errors": errors,
                "timeouts": timeouts,
            }, ensure_ascii=False))

    print(json.dumps({
        "phase": "done",
        "total_rows": total,
        "migrated": migrated,
        "cleaned_duplicate_old": cleaned,
        "skipped": skipped,
        "errors": errors,
        "timeouts": timeouts,
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
