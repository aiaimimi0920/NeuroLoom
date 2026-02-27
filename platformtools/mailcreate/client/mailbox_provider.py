"""Mailbox Provider Abstraction

This module provides a provider abstraction so automation code can switch
between our self-hosted MailCreate service and the public GPTMail service.

Providers:
- mailcreate: our Cloudflare Worker temp-mail service
- gptmail: https://mail.chatgpt.org.uk (public API)
- auto: prefer GPTMail first; if it cannot create mailbox (quota/errors), fall
        back to MailCreate.

Design:
- `create_mailbox()` returns Mailbox(provider,email,ref)
- `ref` is an opaque string passed into `wait_openai_code()`.

Ref encoding:
- mailcreate: ref = "mailcreate:<address_jwt>"
- gptmail: ref = "gptmail:<email>"

This makes `auto` mode safe because the polling step can infer provider.

NOTE:
- GPTMail public test key `gpt-test` can be quota-limited.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Optional

from mailcreate_client import MailCreateClient, MailCreateConfig, wait_for_6digit_code
from gptmail_client import GPTMailClient, GPTMailConfig, GPTMailError, wait_for_6digit_code_gptmail
from gptmail_key_manager import GPTMailKeyManager


@dataclass(frozen=True)
class Mailbox:
    provider: str
    email: str
    ref: str


def _encode_ref(provider: str, raw_ref: str) -> str:
    p = (provider or "").strip().lower()
    return f"{p}:{raw_ref}"


def _decode_ref(ref: str) -> tuple[Optional[str], str]:
    s = (ref or "").strip()
    if ":" not in s:
        return None, s
    p, raw = s.split(":", 1)
    p = p.strip().lower()
    raw = raw.strip()
    if p in ("mailcreate", "gptmail", "gpt"):
        return ("gptmail" if p in ("gpt",) else p), raw
    return None, s


def _create_mailbox_mailcreate(*, base_url: str, custom_auth: str, domain: str) -> Mailbox:
    if not base_url:
        raise RuntimeError("mailcreate_base_url is required")
    if not custom_auth:
        raise RuntimeError("mailcreate_custom_auth is required")

    client = MailCreateClient(MailCreateConfig(base_url=base_url, custom_auth=custom_auth))
    res = client.new_address(domain=domain or None)
    email = str(res.get("address") or "").strip()
    jwt = str(res.get("jwt") or "").strip()
    if not email or not jwt:
        raise RuntimeError(f"mailcreate new_address returned invalid payload: {res}")
    return Mailbox(provider="mailcreate", email=email, ref=_encode_ref("mailcreate", jwt))


def _create_mailbox_gptmail(
    *,
    base_url: str,
    api_key: str,
    keys_file: str = "",
    prefix: Optional[str] = None,
    domain: Optional[str] = None,
    max_tries: int = 10,
) -> Mailbox:
    # If api_key provided: single-key mode.
    # Else: load from keys_file (multi-key rotate).

    if api_key:
        client = GPTMailClient(GPTMailConfig(base_url=base_url, api_key=api_key))
        email = client.generate_email(prefix=prefix, domain=domain)
        return Mailbox(provider="gptmail", email=email, ref=_encode_ref("gptmail", email))

    if not keys_file:
        raise RuntimeError("GPTMail requires GPTMAIL_API_KEY or GPTMAIL_KEYS_FILE")

    km = GPTMailKeyManager.from_file(keys_file)
    last_err: Optional[Exception] = None

    for _ in range(max_tries):
        if not km.any_available():
            break
        k = km.next_key()
        try:
            client = GPTMailClient(GPTMailConfig(base_url=base_url, api_key=k))
            email = client.generate_email(prefix=prefix, domain=domain)
            return Mailbox(provider="gptmail", email=email, ref=_encode_ref("gptmail", email))
        except GPTMailError as e:
            last_err = e
            # quota or invalid key => mark exhausted
            km.mark_exhausted(k)
            continue
        except Exception as e:
            last_err = e
            # unknown error: don't burn all keys too aggressively; try next key
            km.mark_exhausted(k)
            continue

    if last_err:
        raise last_err
    raise RuntimeError("Failed to create GPTMail mailbox")


def create_mailbox(
    *,
    provider: str,
    # mailcreate
    mailcreate_base_url: str = "",
    mailcreate_custom_auth: str = "",
    mailcreate_domain: str = "",
    # gptmail
    gptmail_base_url: str = "https://mail.chatgpt.org.uk",
    gptmail_api_key: str = "",
    gptmail_keys_file: str = "",
    gptmail_prefix: Optional[str] = None,
    gptmail_domain: Optional[str] = None,
) -> Mailbox:
    p = (provider or "").strip().lower()

    if p in ("auto", "prefer_gptmail", "gptmail_first"):
        try:
            return _create_mailbox_gptmail(
                base_url=gptmail_base_url,
                api_key=gptmail_api_key,
                keys_file=gptmail_keys_file,
                prefix=gptmail_prefix,
                domain=gptmail_domain,
            )
        except Exception:
            return _create_mailbox_mailcreate(
                base_url=mailcreate_base_url,
                custom_auth=mailcreate_custom_auth,
                domain=mailcreate_domain,
            )

    if p in ("mailcreate", "self", "local"):
        return _create_mailbox_mailcreate(
            base_url=mailcreate_base_url,
            custom_auth=mailcreate_custom_auth,
            domain=mailcreate_domain,
        )

    if p in ("gptmail", "gpt"):
        return _create_mailbox_gptmail(
            base_url=gptmail_base_url,
            api_key=gptmail_api_key,
            keys_file=gptmail_keys_file,
            prefix=gptmail_prefix,
            domain=gptmail_domain,
        )

    raise RuntimeError(f"unknown provider: {provider}")


def wait_openai_code(
    *,
    provider: str,
    mailbox_ref: str,
    # mailcreate
    mailcreate_base_url: str = "",
    mailcreate_custom_auth: str = "",
    # gptmail
    gptmail_base_url: str = "https://mail.chatgpt.org.uk",
    gptmail_api_key: str = "",
    gptmail_keys_file: str = "",
    timeout_seconds: int = 180,
) -> str:
    # In auto mode we rely on encoded ref prefix.
    ref_provider, raw_ref = _decode_ref(mailbox_ref)

    p = (provider or "").strip().lower()
    if p in ("auto", "prefer_gptmail", "gptmail_first"):
        if not ref_provider:
            raise RuntimeError("auto provider requires encoded mailbox_ref like 'gptmail:<email>' or 'mailcreate:<jwt>'")
        p = ref_provider
        mailbox_ref = raw_ref

    if p in ("mailcreate", "self", "local"):
        if not mailcreate_base_url:
            raise RuntimeError("mailcreate_base_url is required")
        if not mailcreate_custom_auth:
            raise RuntimeError("mailcreate_custom_auth is required")
        client = MailCreateClient(MailCreateConfig(base_url=mailcreate_base_url, custom_auth=mailcreate_custom_auth))
        return wait_for_6digit_code(
            client,
            jwt=raw_ref if ref_provider else mailbox_ref,
            from_contains="openai",
            timeout_seconds=timeout_seconds,
            poll_seconds=3.0,
        )

    if p in ("gptmail", "gpt"):
        # We only need one working key to poll.
        key = gptmail_api_key
        if not key and gptmail_keys_file:
            km = GPTMailKeyManager.from_file(gptmail_keys_file)
            key = km.next_key()
        if not key:
            raise RuntimeError("gptmail requires GPTMAIL_API_KEY or GPTMAIL_KEYS_FILE")
        client = GPTMailClient(GPTMailConfig(base_url=gptmail_base_url, api_key=key))
        return wait_for_6digit_code_gptmail(
            client,
            email=raw_ref if ref_provider else mailbox_ref,
            from_contains="openai",
            timeout_seconds=timeout_seconds,
            poll_seconds=3.0,
        )

    raise RuntimeError(f"unknown provider: {provider}")
