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

Profiles
--------
Some providers may have multiple credential sets (e.g. DNSHE account #1/#2/#3).
We support a simple suffix scheme:
- DNSHE_API_KEY_1=...
- DNSHE_API_SECRET_1=...
- DNSHE_API_BASE_1=...
(and so on for _2, _3)
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


def get_var_profiled(file_vars: Dict[str, str], name: str, *, profile: str = "", default: str = "") -> str:
    """Read a variable with optional profile suffix.

    If profile is provided (e.g. "1"), we try NAME_<profile> first, then NAME.

    Examples:
    - get_var_profiled(vars, "DNSHE_API_KEY", profile="2") reads DNSHE_API_KEY_2 then DNSHE_API_KEY.
    """

    p = str(profile or "").strip()
    if p:
        v = get_var(file_vars, f"{name}_{p}")
        if v:
            return v
    return get_var(file_vars, name, default)
