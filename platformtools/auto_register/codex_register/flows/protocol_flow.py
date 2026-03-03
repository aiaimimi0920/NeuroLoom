from __future__ import annotations

from platformtools.auto_register.codex_register.protocol_version.main import (
    register_protocol as register_protocol_impl,
)


def run_protocol_register(proxy: str | None = None) -> tuple[str, str]:
    """Run protocol register flow via protocol_version implementation."""

    return register_protocol_impl(proxy)
