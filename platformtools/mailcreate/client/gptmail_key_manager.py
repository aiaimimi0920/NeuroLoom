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

from dataclasses import dataclass
from pathlib import Path
from typing import List, Optional


@dataclass
class GPTMailKey:
    key: str
    exhausted: bool = False


class GPTMailKeyManager:
    def __init__(self, *, keys: List[GPTMailKey]):
        self._keys = keys
        self._idx = 0

    @staticmethod
    def _parse_line(line: str) -> Optional[GPTMailKey]:
        raw = (line or "").strip()
        if not raw or raw.startswith("#"):
            return None

        exhausted = "[EXHAUSTED]" in raw.upper()

        # Strip comments
        if "#" in raw:
            raw = raw.split("#", 1)[0].strip()

        if not raw:
            return None

        return GPTMailKey(key=raw, exhausted=exhausted)

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

        return cls(keys=keys)

    def mark_exhausted(self, key: str) -> None:
        for k in self._keys:
            if k.key == key:
                k.exhausted = True

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
