"""Provider Scheduler — dynamic priority-based mailbox provider selection.

This module implements a weighted priority scheduler for choosing which mailbox
provider to use in ``auto`` mode.  Each provider is represented by a
``ProviderSlot`` and the ``ProviderScheduler`` ranks them by *effective
priority* (base priority minus a failure penalty that decays over time).

Slots
-----
+------+-----------+---------------+----------------------------------+
| Prio | Slot name | Provider      | Notes                            |
+------+-----------+---------------+----------------------------------+
| 100  | gptmail_test | GPTMail gpt-test | Free key, daily quota reset   |
|  80  | mailcreate   | MailCreate       | Self-hosted, stable            |
|  60  | gptmail_paid | GPTMail paid     | keys_file, costs money         |
|  40  | mailtm       | Mail.tm          | Public free, fallback          |
+------+-----------+---------------+----------------------------------+

Special rules
~~~~~~~~~~~~~
- **gpt-test daily reset**: The ``gpt-test`` key has a daily quota that resets
  at local midnight (00:00).  When it fails ≥ ``ban_threshold`` times, it is
  banned for the rest of the day.  At midnight the ban is lifted and the
  failure count is reset to zero.

- **General failure decay**: For all other slots, the failure count is halved
  every ``decay_half_life_seconds`` (default 300 s = 5 min).
"""

from __future__ import annotations

import math
import os
import threading
import time
from dataclasses import dataclass, field
from datetime import datetime, date
from typing import List, Optional


# ---------------------------------------------------------------------------
# Configuration from environment (with sensible defaults)
# ---------------------------------------------------------------------------

_FAILURE_PENALTY = int(os.environ.get("SCHED_FAILURE_PENALTY", "15") or "15")
_DECAY_HALF_LIFE = float(os.environ.get("SCHED_DECAY_HALF_LIFE_SECONDS", "300") or "300")
_GPT_TEST_BAN_THRESHOLD = int(os.environ.get("GPT_TEST_BAN_THRESHOLD", "3") or "3")

# Base priorities
_PRIO_GPTMAIL_TEST = int(os.environ.get("SCHED_PRIO_GPTMAIL_TEST", "100") or "100")
_PRIO_MAILCREATE = int(os.environ.get("SCHED_PRIO_MAILCREATE", "80") or "80")
_PRIO_GPTMAIL_PAID = int(os.environ.get("SCHED_PRIO_GPTMAIL_PAID", "60") or "60")
_PRIO_MAILTM = int(os.environ.get("SCHED_PRIO_MAILTM", "40") or "40")


# ---------------------------------------------------------------------------
# ProviderSlot
# ---------------------------------------------------------------------------

@dataclass
class ProviderSlot:
    """Represents a single mailbox provider candidate."""

    name: str
    base_priority: int
    daily_reset: bool = False          # True only for gpt-test

    # Mutable runtime state --------------------------------------------------
    recent_failures: int = field(default=0, repr=False)
    last_failure_ts: float = field(default=0.0, repr=False)
    banned_date: Optional[date] = field(default=None, repr=False)  # gpt-test only
    _enabled: bool = field(default=True, repr=False)

    # --- public helpers ----------------------------------------------------

    @property
    def is_enabled(self) -> bool:
        return self._enabled

    def disable(self) -> None:
        """Permanently disable this slot (e.g. no keys configured)."""
        self._enabled = False

    def enable(self) -> None:
        self._enabled = True

    def effective_priority(self, now: Optional[float] = None) -> float:
        """Compute the effective priority after applying failure penalty + decay.

        The penalty decays exponentially: each ``_DECAY_HALF_LIFE`` seconds the
        effective failure count is halved.
        """
        if not self._enabled:
            return -1e9

        now = now or time.time()

        # --- gpt-test daily ban check ------------------------------------
        if self.daily_reset and self.banned_date is not None:
            today = date.today()
            if self.banned_date >= today:
                return -1e9  # Still banned today
            # New day → auto-unban
            self.banned_date = None
            self.recent_failures = 0
            self.last_failure_ts = 0.0

        if self.recent_failures <= 0 or self.last_failure_ts <= 0:
            return float(self.base_priority)

        elapsed = max(0.0, now - self.last_failure_ts)
        half_lives = elapsed / max(_DECAY_HALF_LIFE, 1.0)
        decayed_failures = self.recent_failures * math.pow(0.5, half_lives)

        penalty = decayed_failures * _FAILURE_PENALTY
        return float(self.base_priority) - penalty


# ---------------------------------------------------------------------------
# ProviderScheduler
# ---------------------------------------------------------------------------

class ProviderScheduler:
    """Thread-safe scheduler that picks the best mailbox provider."""

    def __init__(self, slots: Optional[List[ProviderSlot]] = None):
        self._lock = threading.Lock()
        self._slots: List[ProviderSlot] = list(slots or [])

    # --- slot management ---------------------------------------------------

    def add_slot(self, slot: ProviderSlot) -> None:
        with self._lock:
            # Replace existing slot with same name
            self._slots = [s for s in self._slots if s.name != slot.name]
            self._slots.append(slot)

    def get_slot(self, name: str) -> Optional[ProviderSlot]:
        with self._lock:
            for s in self._slots:
                if s.name == name:
                    return s
        return None

    # --- scheduling --------------------------------------------------------

    def pick(self) -> List[ProviderSlot]:
        """Return a list of enabled slots sorted by effective priority (desc).

        The caller should iterate through the list and try each slot in order
        until one succeeds.
        """
        now = time.time()
        with self._lock:
            enabled = [s for s in self._slots if s.is_enabled]
            # Sort by effective priority descending, then by base priority as tiebreaker
            enabled.sort(key=lambda s: (s.effective_priority(now), s.base_priority), reverse=True)
            # Filter out slots with very negative priority (banned)
            return [s for s in enabled if s.effective_priority(now) > -1e8]

    def mark_success(self, slot_name: str) -> None:
        """Mark a successful use of the provider.  Resets failure count."""
        with self._lock:
            for s in self._slots:
                if s.name == slot_name:
                    s.recent_failures = 0
                    s.last_failure_ts = 0.0
                    break

    def mark_failure(self, slot_name: str) -> None:
        """Mark a failure.  Increments failure count and potentially bans gpt-test."""
        now = time.time()
        with self._lock:
            for s in self._slots:
                if s.name == slot_name:
                    s.recent_failures += 1
                    s.last_failure_ts = now

                    # gpt-test daily ban check
                    if s.daily_reset and s.recent_failures >= _GPT_TEST_BAN_THRESHOLD:
                        s.banned_date = date.today()
                        print(
                            f"[scheduler] gpt-test banned for today "
                            f"(failures={s.recent_failures}, threshold={_GPT_TEST_BAN_THRESHOLD})"
                        )
                    break

    def status_summary(self) -> str:
        """Return a human-readable status line for logging."""
        now = time.time()
        parts = []
        with self._lock:
            for s in self._slots:
                ep = s.effective_priority(now)
                flag = ""
                if not s.is_enabled:
                    flag = " [OFF]"
                elif s.banned_date is not None and s.banned_date >= date.today():
                    flag = " [BANNED]"
                elif s.recent_failures > 0:
                    flag = f" [fail={s.recent_failures}]"
                parts.append(f"{s.name}={ep:.0f}{flag}")
        return "  ".join(parts)


# ---------------------------------------------------------------------------
# Default scheduler factory
# ---------------------------------------------------------------------------

def create_default_scheduler(
    *,
    has_gptmail_test_key: bool = False,
    has_mailcreate: bool = False,
    has_gptmail_paid: bool = False,
    has_mailtm: bool = True,
) -> ProviderScheduler:
    """Create a scheduler with the standard 4-slot configuration.

    Slots are only enabled if the corresponding backend is configured.
    """
    slots = [
        ProviderSlot(
            name="gptmail_test",
            base_priority=_PRIO_GPTMAIL_TEST,
            daily_reset=True,
            _enabled=has_gptmail_test_key,
        ),
        ProviderSlot(
            name="mailcreate",
            base_priority=_PRIO_MAILCREATE,
            _enabled=has_mailcreate,
        ),
        ProviderSlot(
            name="gptmail_paid",
            base_priority=_PRIO_GPTMAIL_PAID,
            _enabled=has_gptmail_paid,
        ),
        ProviderSlot(
            name="mailtm",
            base_priority=_PRIO_MAILTM,
            _enabled=has_mailtm,
        ),
    ]
    return ProviderScheduler(slots)
