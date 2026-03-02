from __future__ import annotations

import argparse
import os
import time

# Reuse the existing browser automation + repairer implementation.
# IMPORTANT: importing main.py is safe because its worker loop is guarded by __main__.
import main as codex  # type: ignore


def _ensure_dirs() -> None:
    os.makedirs(codex.DATA_DIR, exist_ok=True)

    # Per-instance dirs
    os.makedirs(codex._results_dir(), exist_ok=True)  # type: ignore
    os.makedirs(codex._data_path(codex.ERROR_DIRNAME, codex.INSTANCE_ID), exist_ok=True)

    # Shared dirs
    os.makedirs(codex._data_path(codex.NEED_FIX_AUTH_DIRNAME), exist_ok=True)
    os.makedirs(codex._data_path(codex.FIXED_SUCCESS_DIRNAME), exist_ok=True)
    os.makedirs(codex._data_path(codex.FIXED_FAIL_DIRNAME), exist_ok=True)


def _run_loop_forever() -> None:
    # Keep REPAIRER_POLL_SECONDS in sync with env even when importing.
    try:
        codex.REPAIRER_POLL_SECONDS = float(os.environ.get("REPAIRER_POLL_SECONDS", str(codex.REPAIRER_POLL_SECONDS)))  # type: ignore
        if codex.REPAIRER_POLL_SECONDS < 0.2:  # type: ignore
            codex.REPAIRER_POLL_SECONDS = 0.2  # type: ignore
    except Exception:
        pass

    # Run the real loop implementation from main.py.
    codex._repairer_loop()  # type: ignore


def _run_once() -> int:
    """Process at most one file and exit.

    This is useful for local debugging (especially with HEADLESS=0).

    Note:
      If the main repairer loop previously claimed a file into
      need_fix_auth/_processing/, we also try to process it directly.
    """

    try:
        codex.REPAIRER_POLL_SECONDS = 0.2  # type: ignore
    except Exception:
        pass

    # Release stales first.
    try:
        stale_seconds = int(os.environ.get("REPAIRER_STALE_SECONDS", "1800"))
    except Exception:
        stale_seconds = 1800

    try:
        codex._repairer_release_stale_processing(stale_seconds=stale_seconds)  # type: ignore
    except Exception:
        pass

    claimed = None
    try:
        claimed = codex._repairer_claim_one_file()  # type: ignore
    except Exception:
        claimed = None

    if not claimed:
        # Try _processing directly
        proc_dir = os.path.join(codex._data_path(codex.NEED_FIX_AUTH_DIRNAME), "_processing")
        try:
            names = [
                os.path.join(proc_dir, n)
                for n in os.listdir(proc_dir)
                if n.lower().endswith(".json") and os.path.isfile(os.path.join(proc_dir, n))
            ]
        except Exception:
            names = []

        if not names:
            print("[codex_fixer] no files in need_fix_auth/ or need_fix_auth/_processing/")
            return 0

        names.sort(key=lambda p: os.path.getmtime(p))
        claimed = names[0]

    name = os.path.basename(claimed)
    proxies = []
    try:
        proxies = codex.load_proxies()  # type: ignore
    except Exception:
        proxies = []

    proxy = None
    if proxies:
        # keep deterministic for debug: use first proxy
        proxy = proxies[0]

    ok = False
    reason = ""
    out_path = None
    try:
        ok, reason, out_path = codex._repair_one_auth_file(claimed, proxy=proxy)  # type: ignore
    except Exception as e:
        ok, reason, out_path = False, f"exception:{e}", None

    if ok:
        try:
            os.remove(claimed)
        except Exception:
            pass
        print(f"[codex_fixer] ok file={name} out={out_path}")
        return 0

    # Failure: follow main.py policy (report + delete)
    try:
        os.remove(claimed)
    except Exception:
        pass
    print(f"[codex_fixer] fail file={name} reason={reason}")
    return 2


def main() -> int:
    ap = argparse.ArgumentParser(description="codex_fixer: repair auth jsons in need_fix_auth/ using browser automation")
    ap.add_argument("--once", action="store_true", help="process at most one file then exit")
    args = ap.parse_args()

    _ensure_dirs()

    # Show key knobs
    headless = os.environ.get("HEADLESS", "1")
    print(f"[codex_fixer] HEADLESS={headless} (set HEADLESS=0 to show Chrome window)")
    print(f"[codex_fixer] need_fix_dir={codex._data_path(codex.NEED_FIX_AUTH_DIRNAME)}")
    print(f"[codex_fixer] fixed_success_dir={codex._data_path(codex.FIXED_SUCCESS_DIRNAME)}")

    if args.once:
        return _run_once()

    _run_loop_forever()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
