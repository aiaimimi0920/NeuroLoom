from __future__ import annotations

from typing import Any

from platformtools.auto_register.codex_register.browser_version.main import (
    new_driver as browser_new_driver,
    register as browser_register,
)


def run_browser_register(proxy: str | None) -> tuple[str, str, Any, Any]:
    """Run browser register flow from browser_version implementation.

    Returns:
      (reg_email, auth_json, driver, proxy_dir)
    """

    driver, proxy_dir = browser_new_driver(proxy)
    reg_email, res = browser_register(driver, proxy)
    return reg_email, res, driver, proxy_dir
