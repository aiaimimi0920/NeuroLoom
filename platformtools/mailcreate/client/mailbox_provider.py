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
from mailtm_client import MailTmClient, MailTmConfig, MailTmError, wait_for_6digit_code_mailtm


MAILBOX_VERBOSE = (os.environ.get("MAILBOX_VERBOSE", "0") or "0").strip().lower() not in (
    "0",
    "false",
    "no",
    "off",
    "",
)


def _mb_log(msg: str) -> None:
    if MAILBOX_VERBOSE:
        print(msg)


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
    if p in ("mailcreate", "gptmail", "gpt", "mailtm"):
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
            generated_ok = False
            for _j in range(3):
                email = client.generate_email(prefix=prefix, domain=domain)
                if _accept(email):
                    generated_ok = True
                    km.mark_success(k)
                    return Mailbox(provider="gptmail", email=email, ref=_encode_ref("gptmail", email))

            # Still suspicious => rotate key (but don't mark exhausted).
            if not generated_ok:
                km.mark_failure_maybe_exhaust(k, reason="suspicious_email")
            continue
        except GPTMailError as e:
            last_err = e
            # quota/invalid 仍维持原 exhausted 语义，同时施加冷却（避免短时间抖动误判）
            print(f"[gptmail] generate_email failed: key={_mask_key(k)} err={e}")
            km.mark_exhausted(k, persist=True, reason=str(e))
            km.mark_failure_maybe_exhaust(k, reason=str(e))
            continue
        except Exception as e:
            last_err = e
            # unknown error: 不直接判死，进入冷却后再试下一个 key
            print(f"[gptmail] generate_email exception: key={_mask_key(k)} err={e}")
            km.mark_failure_maybe_exhaust(k, reason=str(e))
            continue

    if last_err:
        raise RuntimeError(f"GPTMail generate_email failed (last_key={_mask_key(last_key)}): {last_err}")


def _create_mailbox_mailtm(
    *, api_base: str,
) -> Mailbox:
    """Create a temporary mailbox via Mail.tm public API."""
    cfg = MailTmConfig(api_base=api_base)
    client = MailTmClient(cfg)
    email, token, password = client.create_mailbox()
    # ref encodes both token and password so we can poll later
    raw_ref = f"{token}||{password}"
    return Mailbox(provider="mailtm", email=email, ref=_encode_ref("mailtm", raw_ref))
from provider_scheduler import ProviderScheduler, ProviderSlot, create_default_scheduler


# ---------------------------------------------------------------------------
# Global scheduler (lazy-initialised on first auto call)
# ---------------------------------------------------------------------------

_SCHEDULER: Optional[ProviderScheduler] = None
_SCHEDULER_LOCK = __import__("threading").Lock()

_GPT_TEST_KEY = "gpt-test"


def _get_scheduler(
    *,
    gptmail_api_key: str,
    gptmail_keys_file: str,
    mailcreate_base_url: str,
    mailcreate_custom_auth: str,
) -> ProviderScheduler:
    """Return (and lazily create) the global provider scheduler."""
    global _SCHEDULER
    if _SCHEDULER is not None:
        return _SCHEDULER

    with _SCHEDULER_LOCK:
        if _SCHEDULER is not None:
            return _SCHEDULER

        # Determine which backends are available
        has_gptmail_test = (gptmail_api_key == _GPT_TEST_KEY)

        has_gptmail_paid = False
        if gptmail_api_key and gptmail_api_key != _GPT_TEST_KEY:
            has_gptmail_paid = True
        else:
            try:
                km = GPTMailKeyManager.from_file(gptmail_keys_file)
                has_gptmail_paid = km.any_available()
            except Exception:
                pass

        has_mailcreate = bool(mailcreate_base_url)

        _SCHEDULER = create_default_scheduler(
            has_gptmail_test_key=has_gptmail_test,
            has_mailcreate=has_mailcreate,
            has_gptmail_paid=has_gptmail_paid,
            has_mailtm=True,  # always available as fallback
        )

        _mb_log(f"[scheduler] initialised: {_SCHEDULER.status_summary()}")
        return _SCHEDULER


def _create_by_slot_name(
    slot_name: str,
    *,
    mailcreate_base_url: str,
    mailcreate_custom_auth: str,
    mailcreate_domain: str,
    gptmail_base_url: str,
    gptmail_api_key: str,
    gptmail_keys_file: str,
    gptmail_prefix: Optional[str],
    gptmail_domain: Optional[str],
    mailtm_api_base: str,
) -> Mailbox:
    """Dispatch mailbox creation by scheduler slot name."""

    if slot_name == "gptmail_test":
        return _create_mailbox_gptmail(
            base_url=gptmail_base_url,
            api_key=_GPT_TEST_KEY,
            keys_file="",
            prefix=gptmail_prefix,
            domain=gptmail_domain,
        )

    if slot_name == "mailcreate":
        return _create_mailbox_mailcreate(
            base_url=mailcreate_base_url,
            custom_auth=mailcreate_custom_auth,
            domain=mailcreate_domain,
        )

    if slot_name == "gptmail_paid":
        # Use explicit key if it's not gpt-test, else fall back to keys_file
        api_key = gptmail_api_key if (gptmail_api_key and gptmail_api_key != _GPT_TEST_KEY) else ""
        return _create_mailbox_gptmail(
            base_url=gptmail_base_url,
            api_key=api_key,
            keys_file=gptmail_keys_file,
            prefix=gptmail_prefix,
            domain=gptmail_domain,
        )

    if slot_name == "mailtm":
        return _create_mailbox_mailtm(api_base=mailtm_api_base)

    raise RuntimeError(f"unknown scheduler slot: {slot_name}")


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
    # mailtm
    mailtm_api_base: str = "https://api.mail.tm",
) -> Mailbox:
    p = (provider or "").strip().lower()

    if p in ("auto", "prefer_gptmail", "gptmail_first"):
        sched = _get_scheduler(
            gptmail_api_key=gptmail_api_key,
            gptmail_keys_file=gptmail_keys_file,
            mailcreate_base_url=mailcreate_base_url,
            mailcreate_custom_auth=mailcreate_custom_auth,
        )

        candidates = sched.pick()
        if not candidates:
            raise RuntimeError("auto: no available mailbox providers")

        _mb_log(f"[scheduler] pick order: {[s.name for s in candidates]}  ({sched.status_summary()})")

        last_err: Optional[Exception] = None
        for slot in candidates:
            try:
                mb = _create_by_slot_name(
                    slot.name,
                    mailcreate_base_url=mailcreate_base_url,
                    mailcreate_custom_auth=mailcreate_custom_auth,
                    mailcreate_domain=mailcreate_domain,
                    gptmail_base_url=gptmail_base_url,
                    gptmail_api_key=gptmail_api_key,
                    gptmail_keys_file=gptmail_keys_file,
                    gptmail_prefix=gptmail_prefix,
                    gptmail_domain=gptmail_domain,
                    mailtm_api_base=mailtm_api_base,
                )
                sched.mark_success(slot.name)
                _mb_log(f"[scheduler] {slot.name} success  email={mb.email}")
                return mb
            except Exception as e:
                last_err = e
                sched.mark_failure(slot.name)
                _mb_log(f"[scheduler] {slot.name} failed: {e}")
                continue

        raise RuntimeError(f"auto: all providers failed. last error: {last_err}")

    # --- Specified provider (no fallback) ---------------------------------

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

    if p in ("mailtm", "mail.tm", "mail_tm"):
        return _create_mailbox_mailtm(api_base=mailtm_api_base)

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
    # mailtm
    mailtm_api_base: str = "https://api.mail.tm",
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

    ref_preview = str(mailbox_ref or "").strip()
    if len(ref_preview) > 18:
        ref_preview = ref_preview[:18] + "..."
    _mb_log(
        f"[mailbox-provider] wait_openai_code provider_req={provider} provider_resolved={p} "
        f"ref_provider={ref_provider or 'none'} ref_preview={ref_preview} timeout={timeout_seconds}s"
    )

    if p in ("mailcreate", "self", "local"):
        if not mailcreate_base_url:
            raise RuntimeError("mailcreate_base_url is required")
        # NOTE:
        # - custom_auth can be empty when Worker sets DISABLE_CUSTOM_AUTH_CHECK=true.
        # - keep this path permissive; if auth is actually required, server will return
        #   a clear 401 and we bubble it up for diagnosis.

        # MailCreate is self-hosted and (almost) free to poll, so we allow
        # configuring a higher polling frequency via env.
        # NOTE: GPTMail polling stays at 3.0s to avoid burning quota.
        poll_seconds = 3.0
        try:
            v = (os.environ.get("MAILCREATE_POLL_SECONDS") or "").strip()
            if v:
                poll_seconds = float(v)
        except Exception:
            poll_seconds = 3.0

        # guard rails: too aggressive polling can DoS our worker or amplify transient errors
        if poll_seconds < 0.2:
            poll_seconds = 0.2
        if poll_seconds > 30.0:
            poll_seconds = 30.0

        client = MailCreateClient(MailCreateConfig(base_url=mailcreate_base_url, custom_auth=mailcreate_custom_auth))
        target_jwt = raw_ref if ref_provider else mailbox_ref
        _mb_log(f"[mailbox-provider] mailcreate poll start poll_seconds={poll_seconds} timeout={timeout_seconds}s")
        try:
            code = wait_for_6digit_code(
                client,
                jwt=target_jwt,
                from_contains="openai",
                timeout_seconds=timeout_seconds,
                poll_seconds=poll_seconds,
            )
            _mb_log(f"[mailbox-provider] mailcreate poll ok code_len={len(str(code or ''))}")
            return code
        except Exception as e:
            _mb_log(f"[mailbox-provider] mailcreate poll fail err_type={type(e).__name__} err={e}")
            raise

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
            _mb_log(f"[mailbox-provider] gptmail(single-key) poll start timeout={timeout_seconds}s")
            try:
                code = wait_for_6digit_code_gptmail(
                    client,
                    email=email,
                    from_contains="openai",
                    timeout_seconds=timeout_seconds,
                    poll_seconds=3.0,
                )
                _mb_log(f"[mailbox-provider] gptmail(single-key) poll ok code_len={len(str(code or ''))}")
                return code
            except Exception as e:
                _mb_log(f"[mailbox-provider] gptmail(single-key) poll fail err_type={type(e).__name__} err={e}")
                raise

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
                code = wait_for_6digit_code_gptmail(
                    client,
                    email=email,
                    from_contains="openai",
                    timeout_seconds=remaining,
                    poll_seconds=3.0,
                )
                km.mark_success(k)
                return code
            except GPTMailError as e:
                last_err = e
                if _should_rotate_gptmail_key(e):
                    _mb_log(f"[gptmail] poll failed: key={_mask_key(k)} err={e}")
                    km.mark_exhausted(k, persist=True, reason=str(e))
                    km.mark_failure_maybe_exhaust(k, reason=str(e))
                    continue
                # 非认证/配额类错误也进入冷却，但不判 exhausted
                km.mark_failure_maybe_exhaust(k, reason=str(e))
                raise
            except Exception as e:
                last_err = e
                km.mark_failure_maybe_exhaust(k, reason=str(e))
                raise

        if last_err:
            raise RuntimeError(f"GPTMail poll failed (last_key={_mask_key(k)}): {last_err}")
        raise RuntimeError("Failed to poll GPTMail mailbox")

    if p in ("mailtm", "mail.tm", "mail_tm"):
        # ref = "mailtm:<token>||<password>"
        raw = raw_ref if ref_provider else mailbox_ref
        parts = raw.split("||", 1)
        if len(parts) != 2:
            raise RuntimeError(f"mailtm ref must be 'token||password', got ref_preview={raw[:30]}")
        mailtm_token, _ = parts

        # Reconstruct email from ref to pass into the poller
        # The email isn't stored in ref, so we use mailbox_ref itself if needed.
        # Actually ref_provider decoding strips it. We need to poll by token only.
        cfg = MailTmConfig(api_base=mailtm_api_base)
        client = MailTmClient(cfg)

        _mb_log(f"[mailbox-provider] mailtm poll start timeout={timeout_seconds}s")
        try:
            code = wait_for_6digit_code_mailtm(
                client,
                token=mailtm_token,
                email="(mailtm)",
                from_contains="openai",
                timeout_seconds=timeout_seconds,
                poll_seconds=3.0,
            )
            _mb_log(f"[mailbox-provider] mailtm poll ok code_len={len(str(code or ''))}")
            return code
        except Exception as e:
            _mb_log(f"[mailbox-provider] mailtm poll fail err_type={type(e).__name__} err={e}")
            raise

    raise RuntimeError(f"unknown provider: {provider}")
