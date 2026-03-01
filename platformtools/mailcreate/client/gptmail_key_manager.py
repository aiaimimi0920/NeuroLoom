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
from dataclasses import dataclass
from pathlib import Path
from typing import List, Optional


@dataclass
class GPTMailKey:
    key: str
    exhausted: bool = False
    reset_at_epoch: int | None = None


class GPTMailKeyManager:
    def __init__(self, *, keys: List[GPTMailKey], path: str = ""):
        self._keys = keys
        self._idx = 0
        self._path = path

    @staticmethod
    def _now_epoch() -> int:
        return int(time.time())

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

        return cls(keys=keys, path=path)

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

    def next_key(self) -> str:
        """Return next non-exhausted key, round-robin.

        Raises:
          RuntimeError if all keys are exhausted.
        """

        if not self._keys:
            raise RuntimeError("No keys loaded")

        # try at most N keys
        n = len(self._keys)
        for _ in range(n):
            k = self._keys[self._idx % n]
            self._idx = (self._idx + 1) % n
            if not k.exhausted and k.key:
                return k.key

        raise RuntimeError("All GPTMail keys are exhausted")

    def any_available(self) -> bool:
        return any((k.key and not k.exhausted) for k in self._keys)
