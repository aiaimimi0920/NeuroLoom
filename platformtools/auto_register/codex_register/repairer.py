from __future__ import annotations

import os

# NOTE:
# 主逻辑已在 main.py 内完整实现（含：
# - need_fix_auth/_processing 原子认领与 stale 回收
# - OAuth 登录 + callback 换 token
# - OTP 多 mailbox_ref 策略 / no_quota 特殊分支
# - 成功落 fixed_success（供 uploader 消费）
# - 失败上报 / 删除队列文件 / JSONL 日志）
#
# 本文件作为独立入口，复用 main.py 的修缮者实现，避免两份逻辑漂移。

import main as codex  # type: ignore


def _ensure_dirs() -> None:
    os.makedirs(codex._data_path(codex.NEED_FIX_AUTH_DIRNAME), exist_ok=True)  # type: ignore
    os.makedirs(codex._data_path(codex.FIXED_SUCCESS_DIRNAME), exist_ok=True)  # type: ignore
    os.makedirs(codex._data_path(codex.FIXED_FAIL_DIRNAME), exist_ok=True)  # type: ignore
    os.makedirs(codex._results_dir(), exist_ok=True)  # type: ignore


def main() -> int:
    _ensure_dirs()

    need = codex._data_path(codex.NEED_FIX_AUTH_DIRNAME)  # type: ignore
    okd = codex._data_path(codex.FIXED_SUCCESS_DIRNAME)  # type: ignore
    bad = codex._data_path(codex.FIXED_FAIL_DIRNAME)  # type: ignore

    print(f"[repairer] enabled=1 need_fix_dir={need}")
    print(f"[repairer] fixed_success_dir={okd}")
    print(f"[repairer] fixed_fail_dir={bad}")
    print(f"[repairer] poll_seconds={codex.REPAIRER_POLL_SECONDS}")  # type: ignore

    # 直接进入 main.py 的完整修缮循环
    codex._repairer_loop()  # type: ignore
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
