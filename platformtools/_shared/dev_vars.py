from __future__ import annotations

"""Shared .dev.vars loader for platformtools.

Design goals
------------
- A single secrets file at `platformtools/.dev.vars` should be discoverable by any tool
  under `platformtools/`.
- Tools may also support a local sibling `.dev.vars` (e.g. `platformtools/dnshe/.dev.vars`)
  for overrides, but the global file is the recommended default.
- `.dev.vars` must never be committed.

Parsing rules
-------------
- Supports simple KEY=VALUE lines
- Ignores blank lines and comments (#...)
- Trims surrounding single/double quotes
"""

import os
from typing import Dict, Optional


def _strip_quotes(v: str) -> str:
    v = v.strip()
    if len(v) >= 2 and ((v[0] == v[-1]) and v[0] in ("'", '"')):
        return v[1:-1]
    return v


def _load_dotenv_file(path: str) -> Dict[str, str]:
    out: Dict[str, str] = {}
    try:
        with open(path, "r", encoding="utf-8") as f:
            for raw in f:
                line = raw.strip()
                if not line or line.startswith("#"):
                    continue
                if "=" not in line:
                    continue
                k, v = line.split("=", 1)
                k = k.strip()
                if not k:
                    continue
                out[k] = _strip_quotes(v)
    except FileNotFoundError:
        return {}
    return out


def find_platformtools_root(start_dir: str) -> Optional[str]:
    d = os.path.abspath(start_dir)
    while True:
        if os.path.basename(d).lower() == "platformtools":
            return d
        parent = os.path.dirname(d)
        if parent == d:
            return None
        d = parent


def load_platformtools_dev_vars(*, start_dir: str) -> Dict[str, str]:
    """Load merged dev vars.

    Priority (later wins):
    1) platformtools/.dev.vars
    2) <tool_dir>/.dev.vars

    This keeps a single global secrets file working for all tools, while allowing
    tool-local overrides when needed.
    """

    merged: Dict[str, str] = {}

    plat_root = find_platformtools_root(start_dir)
    if plat_root:
        merged.update(_load_dotenv_file(os.path.join(plat_root, ".dev.vars")))

    # local override
    merged.update(_load_dotenv_file(os.path.join(os.path.abspath(start_dir), ".dev.vars")))

    return merged


def get_var(file_vars: Dict[str, str], name: str, default: str = "") -> str:
    return ((file_vars.get(name) or os.environ.get(name) or default) or "").strip()
