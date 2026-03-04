"""GPTMail API key manager

Purpose:
- Load multiple GPTMail API keys from a config file (one per line)
- Rotate keys automatically when quota is exceeded

File format:
- One key per line
- Supports inline comments: everything after `#` is ignored
- Supports marking exhausted keys like: `sk-xxxx # [EXHAUSTED]`

This intentionally does NOT write back to disk by default.
"""

from __future__ import annotations

import calendar
import os
import re
import time
import threading
from dataclasses import dataclass
from pathlib import Path
from typing import List, Optional


# 进程级运行时冷却态（仅 GPTMail 使用）
# key -> {"level": int, "until": int, "day": int}
# - day: 记录最后一次更新时的 UTC 日期 (YYYYMMDD)
_RUNTIME_COOLDOWN_STATE: dict[str, dict[str, int]] = {}
_RUNTIME_COOLDOWN_LOCK = threading.Lock()

# 付费 key 连续非网络失败跟踪
# key -> consecutive non-network failure count
_CONSECUTIVE_FAIL_COUNT: dict[str, int] = {}
_CONSECUTIVE_FAIL_LOCK = threading.Lock()
_PAID_KEY_EXHAUST_THRESHOLD = int(os.environ.get("GPTMAIL_PAID_KEY_EXHAUST_THRESHOLD", "10") or "10")

# 网络错误关键词（命中任一则认为是网络波动，不计入连续失败）
_NETWORK_ERROR_KEYWORDS = (
    "timeout", "timed out", "connection refused", "connection reset",
    "connectionerror", "urlopen error", "name or service not known",
    "temporary failure in name resolution", "network is unreachable",
    "no route to host", "ssl", "eof occurred", "broken pipe",
    "connection aborted", "remotedisconnected", "incompleteread",
)


@dataclass
class GPTMailKey:
    key: str
    exhausted: bool = False
    reset_at_epoch: int | None = None
    cooldown_level: int = 0
    cooldown_until_epoch: int | None = None


class GPTMailKeyManager:
    def __init__(self, *, keys: List[GPTMailKey], path: str = ""):
        self._keys = keys
        self._idx = 0
        self._path = path

    @staticmethod
    def _now_epoch() -> int:
        return int(time.time())

    @staticmethod
    def _utc_day_token(epoch: int) -> int:
        t = time.gmtime(int(epoch))
        return int(f"{t.tm_year:04d}{t.tm_mon:02d}{t.tm_mday:02d}")

    @staticmethod
    def _next_utc_midnight_epoch(*, now_epoch: int) -> int:
        t = time.gmtime(int(now_epoch))
        midnight = calendar.timegm((t.tm_year, t.tm_mon, t.tm_mday, 0, 0, 0, 0, 0, 0))
        return int(midnight + 86400)

    @staticmethod
    def _next_daily_reset_epoch(*, exhausted_at_epoch: int, reset_hour_local: int = 8, tz_offset_hours: int = 8) -> int:
        """Return next reset time (epoch seconds, UTC-based) after exhaustion.

        We implement a "daily at HH:00" reset in a fixed timezone offset (default +8).
        This avoids relying on container tzdata.
        """

        offset = int(tz_offset_hours) * 3600
        ex_local = time.gmtime(exhausted_at_epoch + offset)

        # reset at ex_local date HH:00
        reset_same_local_epoch = calendar.timegm(
            (ex_local.tm_year, ex_local.tm_mon, ex_local.tm_mday, int(reset_hour_local), 0, 0, 0, 0, 0)
        )
        reset_same_epoch = int(reset_same_local_epoch - offset)

        if exhausted_at_epoch < reset_same_epoch:
            return reset_same_epoch
        return reset_same_epoch + 86400

    @staticmethod
    def _extract_reset_at_epoch(raw: str) -> int | None:
        m = re.search(r"\[\s*RESET_AT_EPOCH\s*=\s*(\d+)\s*\]", raw or "", flags=re.IGNORECASE)
        if not m:
            return None
        try:
            return int(m.group(1))
        except Exception:
            return None

    @staticmethod
    def _strip_reset_at_epoch_token(raw: str) -> str:
        return re.sub(r"\[\s*RESET_AT_EPOCH\s*=\s*\d+\s*\]", "", raw or "", flags=re.IGNORECASE).strip()

    @classmethod
    def _mark_exhausted_in_file(cls, *, path: str, key: str, reset_at_epoch: int | None) -> None:
        p = Path(path)
        if not p.exists() or not p.is_file():
            return

        lock_path = str(p) + ".lock"
        fd = None
        t0 = time.time()
        while time.time() - t0 < 8:
            try:
                fd = os.open(lock_path, os.O_CREAT | os.O_EXCL | os.O_RDWR)
                break
            except FileExistsError:
                time.sleep(0.1)

        try:
            lines = p.read_text(encoding="utf-8", errors="ignore").splitlines(True)
            changed = False
            out: list[str] = []

            for line in lines:
                nl = "\n" if line.endswith("\n") else ""
                raw = line[:-1] if nl else line

                stripped = (raw or "").strip()
                if not stripped or stripped.startswith("#"):
                    out.append(line)
                    continue

                base, *rest = raw.split("#", 1)
                base_key = base.strip()
                if base_key != key:
                    out.append(line)
                    continue

                comment = (rest[0] if rest else "").strip()
                comment_upper = comment.upper()

                # remove old RESET token, then re-add if needed
                comment = cls._strip_reset_at_epoch_token(comment)
                comment_upper = comment.upper()

                if "[EXHAUSTED]" not in comment_upper:
                    comment = (comment + " " if comment else "") + "[EXHAUSTED]"

                if reset_at_epoch is not None and int(reset_at_epoch) > 0:
                    comment = (comment + " " if comment else "") + f"[RESET_AT_EPOCH={int(reset_at_epoch)}]"

                new_line = f"{base_key} # {comment}{nl}"
                out.append(new_line)
                changed = True

            if changed:
                tmp = str(p) + ".tmp"
                Path(tmp).write_text("".join(out), encoding="utf-8")
                os.replace(tmp, str(p))
        finally:
            if fd is not None:
                try:
                    os.close(fd)
                except Exception:
                    pass
                try:
                    os.remove(lock_path)
                except Exception:
                    pass

    @staticmethod
    def _parse_line(line: str) -> Optional[GPTMailKey]:
        raw = (line or "").strip()
        if not raw or raw.startswith("#"):
            return None

        exhausted_marker = "[EXHAUSTED]" in raw.upper()
        reset_at_epoch = GPTMailKeyManager._extract_reset_at_epoch(raw)

        # Strip comments to get the actual key
        if "#" in raw:
            raw = raw.split("#", 1)[0].strip()

        if not raw:
            return None

        exhausted = exhausted_marker

        # Special: if key has a RESET_AT_EPOCH token, auto-reactivate after that time
        if exhausted_marker and reset_at_epoch is not None:
            now_epoch = int(time.time())
            if now_epoch >= int(reset_at_epoch):
                exhausted = False

        return GPTMailKey(key=raw, exhausted=exhausted, reset_at_epoch=reset_at_epoch)

    @classmethod
    def from_file(cls, path: str) -> "GPTMailKeyManager":
        p = Path(path)
        if not p.exists() or not p.is_file():
            raise FileNotFoundError(f"GPTMail keys file not found: {path}")

        keys: List[GPTMailKey] = []
        for line in p.read_text(encoding="utf-8", errors="ignore").splitlines():
            k = cls._parse_line(line)
            if k is not None:
                keys.append(k)

        if not keys:
            raise ValueError(f"No keys found in file: {path}")

        km = cls(keys=keys, path=path)
        km._apply_runtime_cooldown_state()
        return km

    def mark_exhausted(
        self,
        key: str,
        *,
        persist: bool = False,
        reason: str = "",
    ) -> None:
        """Mark a key exhausted.

        If persist=True, also write `# [EXHAUSTED]` back to the keys file so future
        processes/containers won't reuse it.

        Special handling:
        - For key == "gpt-test" and quota-like errors, we persist a reset token
          `[RESET_AT_EPOCH=...]` so it automatically becomes available again after
          the daily reset.
        """

        reset_at_epoch: int | None = None

        # gpt-test: quota resets daily at 08:00 (default, configurable)
        if (key or "").strip() == "gpt-test":
            r = (reason or "").lower()
            if "quota" in r or "daily quota" in r or "exceeded" in r:
                reset_hour = int(os.environ.get("GPTMAIL_TEST_RESET_HOUR_LOCAL") or "8")
                tz_off = int(os.environ.get("GPTMAIL_TEST_RESET_TZ_OFFSET_HOURS") or "8")
                reset_at_epoch = self._next_daily_reset_epoch(
                    exhausted_at_epoch=self._now_epoch(),
                    reset_hour_local=reset_hour,
                    tz_offset_hours=tz_off,
                )

        for k in self._keys:
            if k.key == key:
                k.exhausted = True
                k.reset_at_epoch = reset_at_epoch

        if persist and self._path:
            try:
                self._mark_exhausted_in_file(path=self._path, key=key, reset_at_epoch=reset_at_epoch)
            except Exception:
                pass

    @staticmethod
    def _cooldown_schedule_seconds() -> list[int]:
        raw = (os.environ.get("GPTMAIL_KEY_COOLDOWN_SCHEDULE") or "300,1800,3600,10800,21600").strip()
        vals: list[int] = []
        for p in raw.split(","):
            p = p.strip()
            if not p:
                continue
            try:
                v = int(p)
            except Exception:
                continue
            if v > 0:
                vals.append(v)
        return vals or [300, 1800, 3600, 10800, 21600]

    @staticmethod
    def _pick_cooldown_seconds(level: int) -> int:
        sched = GPTMailKeyManager._cooldown_schedule_seconds()
        idx = max(1, int(level)) - 1
        if idx >= len(sched):
            return int(sched[-1])
        return int(sched[idx])

    def _apply_runtime_cooldown_state(self) -> None:
        now = self._now_epoch()
        now_day = self._utc_day_token(now)
        with _RUNTIME_COOLDOWN_LOCK:
            for k in self._keys:
                st = _RUNTIME_COOLDOWN_STATE.get(k.key)
                if not st:
                    continue
                lvl = int(st.get("level") or 0)
                until = int(st.get("until") or 0)
                day = int(st.get("day") or 0)

                # 特例：gpt-test 每天 UTC 0 点刷新额度，冷却与等级按日自动清零
                if k.key == "gpt-test" and day and day != now_day:
                    lvl = 0
                    until = 0
                    _RUNTIME_COOLDOWN_STATE[k.key] = {"level": 0, "until": 0, "day": now_day}

                if until > 0 and now >= until:
                    # 冷却时间到，仅允许重试；连续失败等级保留，直到成功才清零
                    k.cooldown_level = max(0, lvl)
                    k.cooldown_until_epoch = None
                else:
                    k.cooldown_level = max(0, lvl)
                    k.cooldown_until_epoch = until if until > 0 else None

    @staticmethod
    def _is_cooling(k: GPTMailKey, now_epoch: int) -> bool:
        return bool(k.cooldown_until_epoch and now_epoch < int(k.cooldown_until_epoch))

    def mark_failure_cooldown(self, key: str, *, reason: str = "") -> None:
        """对 GPTMail key 施加阶梯冷却（失败递增，成功清零）。"""

        _ = reason  # 预留：后续可按错误类型做不同冷却策略
        now = self._now_epoch()
        now_day = self._utc_day_token(now)
        with _RUNTIME_COOLDOWN_LOCK:
            st = _RUNTIME_COOLDOWN_STATE.get(key) or {"level": 0, "until": 0, "day": now_day}
            lvl = max(0, int(st.get("level") or 0)) + 1
            cd = self._pick_cooldown_seconds(lvl)
            until = now + cd

            # 特例：gpt-test 不跨 UTC 日累积冷却，且冷却上限不超过下一个 UTC 0 点
            if key == "gpt-test":
                prev_day = int(st.get("day") or 0)
                if prev_day and prev_day != now_day:
                    lvl = 1
                    cd = self._pick_cooldown_seconds(lvl)
                    until = now + cd
                next_reset = self._next_utc_midnight_epoch(now_epoch=now)
                if until > next_reset:
                    until = next_reset

            _RUNTIME_COOLDOWN_STATE[key] = {"level": lvl, "until": until, "day": now_day}

        for k in self._keys:
            if k.key == key:
                k.cooldown_level = lvl
                k.cooldown_until_epoch = until
                break

    # ------------------------------------------------------------------
    # 付费 key 连续非网络失败自动废弃
    # ------------------------------------------------------------------

    @staticmethod
    def _is_network_error(reason: str) -> bool:
        """判断错误是否为网络波动（而非 API 业务错误）。

        网络波动不计入连续失败，避免误废弃正常 key。
        """
        r = (reason or "").lower()
        return any(kw in r for kw in _NETWORK_ERROR_KEYWORDS)

    def mark_failure_maybe_exhaust(
        self,
        key: str,
        *,
        reason: str = "",
    ) -> bool:
        """跟踪付费 key 的连续非网络失败次数。

        - 网络错误（超时、DNS、连接拒绝等）→ 不计入，仅走冷却
        - 非网络错误 → 连续计数 +1
        - 连续非网络失败 ≥ 阈值 → 永久废弃（persist=True）

        Returns:
            True if key was auto-exhausted, False otherwise.
        """
        # gpt-test 不走此逻辑（它有自己的每日重置机制）
        if (key or "").strip() == "gpt-test":
            return False

        if self._is_network_error(reason):
            # 网络问题不计入连续失败，仅施加冷却
            self.mark_failure_cooldown(key, reason=reason)
            return False

        # 累加连续非网络失败
        with _CONSECUTIVE_FAIL_LOCK:
            count = _CONSECUTIVE_FAIL_COUNT.get(key, 0) + 1
            _CONSECUTIVE_FAIL_COUNT[key] = count

        self.mark_failure_cooldown(key, reason=reason)

        if count >= _PAID_KEY_EXHAUST_THRESHOLD:
            print(
                f"[gptmail_key_manager] key '{key[:8]}...' auto-exhausted: "
                f"{count} consecutive non-network failures (threshold={_PAID_KEY_EXHAUST_THRESHOLD})"
            )
            self.mark_exhausted(key, persist=True, reason=reason)
            return True

        return False

    @staticmethod
    def _reset_consecutive_fail(key: str) -> None:
        """成功后清零连续非网络失败计数。"""
        with _CONSECUTIVE_FAIL_LOCK:
            _CONSECUTIVE_FAIL_COUNT.pop(key, None)

    def mark_success(self, key: str) -> None:
        """某 key 成功调用一次后，清零冷却等级、冷却时间和连续失败计数。"""

        with _RUNTIME_COOLDOWN_LOCK:
            _RUNTIME_COOLDOWN_STATE[key] = {
                "level": 0,
                "until": 0,
                "day": self._utc_day_token(self._now_epoch()),
            }

        self._reset_consecutive_fail(key)

        for k in self._keys:
            if k.key == key:
                k.cooldown_level = 0
                k.cooldown_until_epoch = None
                break

    def next_key(self) -> str:
        """Return next non-exhausted and non-cooling key, round-robin."""

        if not self._keys:
            raise RuntimeError("No keys loaded")

        self._apply_runtime_cooldown_state()
        now = self._now_epoch()

        n = len(self._keys)
        for _ in range(n):
            k = self._keys[self._idx % n]
            self._idx = (self._idx + 1) % n
            if not k.key or k.exhausted:
                continue
            if self._is_cooling(k, now):
                continue
            return k.key

        raise RuntimeError("All GPTMail keys are exhausted_or_cooling")

    def any_available(self) -> bool:
        self._apply_runtime_cooldown_state()
        now = self._now_epoch()
        return any((k.key and not k.exhausted and not self._is_cooling(k, now)) for k in self._keys)
