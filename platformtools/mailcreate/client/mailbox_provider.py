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

import os
import time
from dataclasses import dataclass
from typing import Optional

from mailcreate_client import MailCreateClient, MailCreateConfig, wait_for_6digit_code
from gptmail_client import GPTMailClient, GPTMailConfig, GPTMailError, wait_for_6digit_code_gptmail
from gptmail_key_manager import GPTMailKeyManager


def _mask_key(k: str) -> str:
    s = (k or "").strip()
    if not s:
        return ""
    if len(s) <= 10:
        return s
    return s[:4] + "..." + s[-4:]


def _split_email(email: str) -> tuple[str, str]:
    s = (email or "").strip()
    if "@" not in s:
        return s, ""
    local, domain = s.split("@", 1)
    return local.strip(), domain.strip().lower()


def _is_bad_dotted_hex_domain(domain: str) -> bool:
    """Heuristic filter.

    OpenAI signup sometimes rejects weird disposable domains.

    Example observed from GPTMail: "5.2.5.b.0.d.0.0.1.c"
    (many 1-char hex labels separated by dots).
    """

    d = (domain or "").strip().lower()
    if not d:
        return True

    parts = [p for p in d.split(".") if p]
    if len(parts) < 2:
        return True

    # Many single-char hex labels => suspicious.
    if len(parts) >= 6 and all((len(p) == 1 and p in "0123456789abcdef") for p in parts):
        return True

    return False


def _is_acceptable_openai_email(email: str) -> bool:
    local, domain = _split_email(email)
    if not local or not domain:
        return False
    if " " in email or "\n" in email or "\r" in email or "\t" in email:
        return False
    if ".." in email:
        return False
    if _is_bad_dotted_hex_domain(domain):
        return False

    # Require a "normal-ish" TLD (OpenAI tends to be strict here).
    parts = [p for p in domain.split(".") if p]
    if len(parts) < 2:
        return False
    tld = parts[-1]
    if len(tld) < 2:
        return False

    return True


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

    # `custom_auth` can be empty when the Worker has DISABLE_CUSTOM_AUTH_CHECK=true.
    client = MailCreateClient(MailCreateConfig(base_url=base_url, custom_auth=custom_auth))

    last_res = None
    for _ in range(5):
        res = client.new_address(domain=domain or None)
        last_res = res
        email = str(res.get("address") or "").strip()
        jwt = str(res.get("jwt") or "").strip()
        if not email or not jwt:
            continue
        if not _is_acceptable_openai_email(email):
            print(f"[mailcreate] generated suspicious email={email!r}, retrying")
            continue
        return Mailbox(provider="mailcreate", email=email, ref=_encode_ref("mailcreate", jwt))

    raise RuntimeError(f"mailcreate new_address returned invalid payload: {last_res}")


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

    def _accept(email: str) -> bool:
        if _is_acceptable_openai_email(email):
            return True
        print(f"[gptmail] generated suspicious email={email!r}, retrying")
        return False

    if api_key:
        client = GPTMailClient(GPTMailConfig(base_url=base_url, api_key=api_key))
        last_err2: Optional[Exception] = None
        for _ in range(5):
            try:
                email = client.generate_email(prefix=prefix, domain=domain)
                if _accept(email):
                    return Mailbox(provider="gptmail", email=email, ref=_encode_ref("gptmail", email))
            except Exception as e:
                last_err2 = e
                raise
        raise RuntimeError(f"GPTMail generate_email returned suspicious email too many times: {last_err2}")

    if not keys_file:
        raise RuntimeError("GPTMail requires GPTMAIL_API_KEY or GPTMAIL_KEYS_FILE")

    km = GPTMailKeyManager.from_file(keys_file)
    last_err: Optional[Exception] = None
    last_key = ""

    for _ in range(max_tries):
        if not km.any_available():
            break
        k = km.next_key()
        last_key = k
        try:
            client = GPTMailClient(GPTMailConfig(base_url=base_url, api_key=k))

            # Retry a few times on "weird but success" emails without burning the key.
            for _j in range(3):
                email = client.generate_email(prefix=prefix, domain=domain)
                if _accept(email):
                    return Mailbox(provider="gptmail", email=email, ref=_encode_ref("gptmail", email))

            # Still suspicious => rotate key (but don't mark exhausted).
            continue
        except GPTMailError as e:
            last_err = e
            # quota or invalid key => mark exhausted
            print(f"[gptmail] generate_email failed: key={_mask_key(k)} err={e}")
            km.mark_exhausted(k, persist=True, reason=str(e))
            continue
        except Exception as e:
            last_err = e
            # unknown error: don't burn all keys too aggressively; try next key
            print(f"[gptmail] generate_email exception: key={_mask_key(k)} err={e}")
            km.mark_exhausted(k)
            continue

    if last_err:
        raise RuntimeError(f"GPTMail generate_email failed (last_key={_mask_key(last_key)}): {last_err}")
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
    def _poll_seconds() -> float:
        # Speed knob for code polling. Lower means faster, but higher request rate.
        # Keep conservative defaults to avoid hammering the mailbox API.
        raw = (os.environ.get("OPENAI_CODE_POLL_SECONDS", "2.0") or "2.0").strip()
        try:
            v = float(raw)
        except Exception:
            v = 2.0
        if v < 0.5:
            v = 0.5
        if v > 10.0:
            v = 10.0
        return v
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
            poll_seconds=_poll_seconds(),
        )

    if p in ("gptmail", "gpt"):
        """Poll GPTMail inbox for OpenAI code.

        Why this exists:
        - In our register flow, mailbox provider may be `auto` which often chooses GPTMail.
        - GPTMail keys can be quota-limited or invalid. Previously we only tried ONE key
          during polling, leading to repeated `Invalid API key` failures.

        Policy:
        - If `gptmail_api_key` is provided, use it as-is (single-key mode).
        - Else, load keys from `gptmail_keys_file` and rotate keys ONLY on auth/quota errors.
        - Do NOT rotate keys on "timeout waiting for 6-digit code" because that is usually
          deliverability/flow-related, not a key issue.
        """

        email = raw_ref if ref_provider else mailbox_ref

        if gptmail_api_key:
            client = GPTMailClient(GPTMailConfig(base_url=gptmail_base_url, api_key=gptmail_api_key))
            return wait_for_6digit_code_gptmail(
                client,
                email=email,
                from_contains="openai",
                timeout_seconds=timeout_seconds,
                poll_seconds=_poll_seconds(),
            )

        if not gptmail_keys_file:
            raise RuntimeError("gptmail requires GPTMAIL_API_KEY or GPTMAIL_KEYS_FILE")

        km = GPTMailKeyManager.from_file(gptmail_keys_file)
        last_err: Optional[Exception] = None

        deadline = time.time() + max(1, int(timeout_seconds))

        def _should_rotate_gptmail_key(err: GPTMailError) -> bool:
            msg = str(err or "").lower()
            st = getattr(err, "status", None)
            if st in (401, 403, 429):
                return True
            if "invalid api key" in msg:
                return True
            if "quota" in msg:
                return True
            if "timeout waiting for 6-digit code" in msg:
                return False
            return False

        # Try multiple keys (round-robin) until success or timeout.
        # We only burn a key when it is clearly auth/quota-related.
        for _ in range(50):
            remaining = int(deadline - time.time())
            if remaining <= 0:
                break

            if not km.any_available():
                break

            k = km.next_key()
            try:
                client = GPTMailClient(GPTMailConfig(base_url=gptmail_base_url, api_key=k))
                return wait_for_6digit_code_gptmail(
                    client,
                    email=email,
                    from_contains="openai",
                    timeout_seconds=remaining,
                    poll_seconds=_poll_seconds(),
                )
            except GPTMailError as e:
                last_err = e
                if _should_rotate_gptmail_key(e):
                    print(f"[gptmail] poll failed: key={_mask_key(k)} err={e}")
                    km.mark_exhausted(k, persist=True, reason=str(e))
                    continue
                raise
            except Exception as e:
                last_err = e
                raise

        if last_err:
            raise RuntimeError(f"GPTMail poll failed (last_key={_mask_key(k)}): {last_err}")
        raise RuntimeError("Failed to poll GPTMail mailbox")

    raise RuntimeError(f"unknown provider: {provider}")
