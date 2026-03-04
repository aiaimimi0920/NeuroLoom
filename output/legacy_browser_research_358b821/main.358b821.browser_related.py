# ===== get_email =====
def get_email(proxy: str | None = None) -> tuple[str, str]:
    """Compatibility wrapper.

    Returns:
      (email, address_jwt)

    Note:
      `proxy` is ignored here because the mailbox API call is performed by our
      MailCreate client (direct HTTPS) and does not use the Selenium proxy.
    """

    _ = proxy
    print("[mailbox] mailbox_direct=true action=create_temp_mailbox")
    return create_temp_mailbox()



# ===== get_oai_code =====
def get_oai_code(*, address_jwt: str, timeout_seconds: int = 180, proxy: str | None = None) -> str:
    """Compatibility wrapper.

    Note:
      `proxy` is ignored here for the same reason as [`get_email()`](platformtools/auto_register/codex_register/main.py:1).
    """

    _ = proxy
    print("[mailbox] mailbox_direct=true action=wait_openai_code")
    return wait_openai_code(address_jwt=address_jwt, timeout_seconds=timeout_seconds)


def _proxy_dict_for_requests(proxy: str | None) -> dict[str, str] | None:
    p = str(proxy or "").strip()
    if not p:
        return None
    return {"http": p, "https": p}


def _decode_cookie_json_prefix(raw_cookie: str) -> dict[str, Any]:
    """Decode first JWT-like segment from cookie and parse as JSON."""

    v = str(raw_cookie or "").strip()
    if not v:
        return {}

    head = v.split(".", 1)[0]
    # OpenAI cookies may use standard base64 (not always urlsafe/padded).
    for use_urlsafe in (False, True):
        try:
            pad = "=" * ((4 - (len(head) % 4)) % 4)
            blob = (head + pad).encode("ascii")
            decoded = (
                base64.urlsafe_b64decode(blob)
                if use_urlsafe
                else base64.b64decode(blob)
            )
            obj = json.loads(decoded.decode("utf-8"))
            if isinstance(obj, dict):
                return obj
        except Exception:
            continue

    return {}


def _follow_redirects_for_callback(*, sess, start_url: str, max_hops: int = 8) -> str:
    cur = str(start_url or "").strip()
    if not cur:
        raise RuntimeError("missing continue_url")

    for _ in range(max_hops):
        resp = sess.get(cur, allow_redirects=False, timeout=PROTOCOL_TIMEOUT_SECONDS)
        status = int(getattr(resp, "status_code", 0) or 0)
        loc = str(resp.headers.get("Location") or "").strip()

        if loc and "localhost:1455" in loc:
            return loc

        if status in (301, 302, 303, 307, 308) and loc:
            if loc.startswith("/"):
                pu = urllib.parse.urlparse(cur)
                loc = f"{pu.scheme}://{pu.netloc}{loc}"
            cur = loc
            continue

        # 非重定向且未拿到 callback
        break

    raise RuntimeError("protocol flow did not reach localhost callback")


def register_protocol(proxy: str | None = None) -> tuple[str, str]:
    """Compatibility shim: protocol flow moved to flows/protocol_flow.py."""

    return run_protocol_register(proxy)


AUTH_URL = "https://auth.openai.com/oauth/authorize"
TOKEN_URL = "https://auth.openai.com/oauth/token"
CLIENT_ID = "app_EMoamEEZ73f0CkXaXp7hrann"

DEFAULT_CALLBACK_PORT = 1455
DEFAULT_REDIRECT_URI = f"http://localhost:{DEFAULT_CALLBACK_PORT}/auth/callback"
DEFAULT_SCOPE = "openid email profile offline_access"


def _b64url_no_pad(raw: bytes) -> str:
    return base64.urlsafe_b64encode(raw).decode("ascii").rstrip("=")


def _sha256_b64url_no_pad(s: str) -> str:
    return _b64url_no_pad(hashlib.sha256(s.encode("ascii")).digest())


def _random_state(nbytes: int = 16) -> str:
    return secrets.token_urlsafe(nbytes)


def _pkce_verifier() -> str:
    # RFC 7636 allows 43..128 chars; urlsafe token is fine.
    return secrets.token_urlsafe(64)


def _parse_callback_url(callback_url: str) -> Dict[str, str]:
    candidate = callback_url.strip()
    if not candidate:
        return {
            "code": "",
            "state": "",
            "error": "",
            "error_description": "",
        }

    if "://" not in candidate:
        if candidate.startswith("?"):
            candidate = f"http://localhost{candidate}"
        elif any(ch in candidate for ch in "/?#") or ":" in candidate:
            candidate = f"http://{candidate}"
        elif "=" in candidate:
            candidate = f"http://localhost/?{candidate}"

    parsed = urllib.parse.urlparse(candidate)
    query = urllib.parse.parse_qs(parsed.query, keep_blank_values=True)
    fragment = urllib.parse.parse_qs(parsed.fragment, keep_blank_values=True)

    # Query takes precedence; fragment is a fallback.
    for key, values in fragment.items():
        if key not in query or not query[key] or not (query[key][0] or "").strip():
            query[key] = values

    def get1(k: str) -> str:
        v = query.get(k, [""])
        return (v[0] or "").strip()

    code = get1("code")
    state = get1("state")
    error = get1("error")
    error_description = get1("error_description")

    # Handle malformed callback payloads where state is appended with '#'.
    if code and not state and "#" in code:
        code, state = code.split("#", 1)

    if not error and error_description:
        error, error_description = error_description, ""

    return {
        "code": code,
        "state": state,
        "error": error,
        "error_description": error_description,
    }


def _jwt_claims_no_verify(id_token: str) -> Dict[str, Any]:
    # WARNING: no signature verification; this only decodes claims to extract fields.
    if not id_token or id_token.count(".") < 2:
        return {}
    payload_b64 = id_token.split(".")[1]
    pad = "=" * ((4 - (len(payload_b64) % 4)) % 4)
    try:
        payload = base64.urlsafe_b64decode((payload_b64 + pad).encode("ascii"))
        return json.loads(payload.decode("utf-8"))
    except Exception:
        return {}


def _to_int(v: Any) -> int:
    try:
        return int(v)
    except (TypeError, ValueError):
        return 0


def get_opener(proxy: str | None = None):
    if not proxy:
        return urllib.request.build_opener()
    proxy_handler = urllib.request.ProxyHandler({'http': proxy, 'https': proxy})
    return urllib.request.build_opener(proxy_handler)

def _post_form(url: str, data: Dict[str, str], timeout: int = 30, proxy: str | None = None) -> Dict[str, Any]:
    """POST form data with Chrome TLS/HTTP2 fingerprint via curl_cffi.

    Strategy: try direct (no proxy) first, then with proxy, then urllib fallback.
    The token exchange endpoint doesn't need residential IP masking;
    the proxy is mainly for browser navigation to bypass Cloudflare.
    """
    # Build list of (label, proxies_dict) attempts
    proxy_configs = [("direct", None)]
    if proxy:
        proxy_configs.append(("proxy", {"https": proxy, "http": proxy}))

    for attempt in range(4):
        for label, proxies in proxy_configs:
            # --- curl_cffi with Chrome impersonation ---
            try:
                from curl_cffi import requests as cffi_requests
                resp = cffi_requests.post(
                    url,
                    data=data,
                    headers={
                        "Content-Type": "application/x-www-form-urlencoded",
                        "Accept": "application/json",
                        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36",
                    },
                    impersonate="chrome",
                    timeout=timeout,
                    proxies=proxies,
                    verify=False,
                )
                if resp.status_code != 200:
                    raise RuntimeError(
                        f"token exchange failed: {resp.status_code}: {resp.text[:500]}"
                    )
                print(f"POST token exchange ok via {label} (curl_cffi)")
                return resp.json()
            except ImportError:
                break  # curl_cffi not installed, skip to urllib
            except RuntimeError:
                raise
            except Exception as e:
                print(f"POST Request error (curl_cffi/{label}, attempt {attempt+1}): {e}")
                continue  # try next proxy config

        # --- Fallback: urllib (direct only, proxy already failed above) ---
        try:
            body = urllib.parse.urlencode(data).encode("utf-8")
            req = urllib.request.Request(
                url,
                data=body,
                method="POST",
                headers={
                    "Content-Type": "application/x-www-form-urlencoded",
                    "Accept": "application/json",
                },
            )
            with urllib.request.build_opener().open(req, timeout=timeout) as resp:
                raw = resp.read()
                if resp.status != 200:
                    raise RuntimeError(
                        f"token exchange failed: {resp.status}: {raw.decode('utf-8', 'replace')}"
                    )
                print("POST token exchange ok via urllib (direct)")
                return json.loads(raw.decode("utf-8"))
        except urllib.error.HTTPError as exc:
            raw = exc.read()
            raise RuntimeError(
                f"token exchange failed: {exc.code}: {raw.decode('utf-8', 'replace')}"
            ) from exc
        except Exception as e:
            print(f"POST Request error (urllib, attempt {attempt+1}): {e}")

        time.sleep(2)

    raise RuntimeError("Failed to post form after max retries")


@dataclass(frozen=True)
class OAuthStart:
    auth_url: str
    state: str
    code_verifier: str
    redirect_uri: str



# ===== generate_oauth_url =====
def generate_oauth_url(
    *,
    redirect_uri: str = DEFAULT_REDIRECT_URI,
    scope: str = DEFAULT_SCOPE,
) -> OAuthStart:
    """
    1) Generate oauth URL -> return a URL that can pull up authorization.

    You must keep the returned `state` and `code_verifier` and pass them into
    `submit_callback_url`.
    """
    state = _random_state()
    code_verifier = _pkce_verifier()
    code_challenge = _sha256_b64url_no_pad(code_verifier)

    params = {
        "client_id": CLIENT_ID,
        "response_type": "code",
        "redirect_uri": redirect_uri,
        "scope": scope,
        "state": state,
        "code_challenge": code_challenge,
        "code_challenge_method": "S256",
        "prompt": "login",
        "id_token_add_organizations": "true",
        "codex_cli_simplified_flow": "true",
    }
    auth_url = f"{AUTH_URL}?{urllib.parse.urlencode(params)}"
    return OAuthStart(
        auth_url=auth_url,
        state=state,
        code_verifier=code_verifier,
        redirect_uri=redirect_uri,
    )


def submit_callback_url(
    *,
    callback_url: str,
    expected_state: str,
    code_verifier: str,
    redirect_uri: str = DEFAULT_REDIRECT_URI,
    proxy: str | None = None,
    # mailbox_ref: 用于后续“修缮者”读取邮箱验证码；应为 mailbox_provider.py 约定的编码格式
    #   - mailcreate:<jwt>
    #   - gptmail:<email>
    mailbox_ref: str = "",
    password: str = "",
    first_name: str = "",
    last_name: str = "",
    birthdate: str = "",
) -> tuple[str, str]:
    """
    2) Submit call back url -> takes the full callback URL, exchanges the code for
       tokens, and returns a JSON string "config" payload.
    """
    cb = _parse_callback_url(callback_url)
    if cb["error"]:
        desc = cb["error_description"]
        raise RuntimeError(f"oauth error: {cb['error']}: {desc}".strip())

    if not cb["code"]:
        raise ValueError("callback url missing ?code=")
    if not cb["state"]:
        raise ValueError("callback url missing ?state=")
    if cb["state"] != expected_state:
        raise ValueError("state mismatch")

    token_resp = _post_form(
        TOKEN_URL,
        {
            "grant_type": "authorization_code",
            "client_id": CLIENT_ID,
            "code": cb["code"],
            "redirect_uri": redirect_uri,
            "code_verifier": code_verifier,
        },
        timeout=30,
        proxy=proxy
    )

    access_token = (token_resp.get("access_token") or "").strip()
    refresh_token = (token_resp.get("refresh_token") or "").strip()
    id_token = (token_resp.get("id_token") or "").strip()
    expires_in = _to_int(token_resp.get("expires_in"))

    claims = _jwt_claims_no_verify(id_token)
    email = str(claims.get("email") or "").strip()
    auth_claims = claims.get("https://api.openai.com/auth") or {}
    account_id = str(auth_claims.get("chatgpt_account_id") or "").strip()

    now = int(time.time())
    expired_rfc3339 = time.strftime(
        "%Y-%m-%dT%H:%M:%SZ", time.gmtime(now + max(expires_in, 0))
    )
    now_rfc3339 = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(now))

    # Construct the JSON format exactly as requested by user
    config = dict(claims)
    config.update({
        "type": "codex",
        "email": email,
        "expired": expired_rfc3339,
        "disabled": False,
        "id_token": id_token,
        "access_token": access_token,
        "refresh_token": refresh_token,
        "password": password,
        "birthdate": birthdate,
        "client_id": CLIENT_ID,
        "last_name": last_name,
        "account_id": account_id,
        "first_name": first_name,
        "session_id": claims.get("session_id", ""),
        "last_refresh": now_rfc3339,
        "pwd_auth_time": claims.get("pwd_auth_time", int(time.time() * 1000)),
        "https://api.openai.com/auth": auth_claims,
        "https://api.openai.com/profile": claims.get("https://api.openai.com/profile", {}),
    })

    # 强制 schema 并集：即使上游响应缺字段，也保留关键键。
    schema_defaults = {
        "refresh_token": "",
        "session_id": "",
        "password": "",
        "birthdate": "",
        "first_name": "",
        "last_name": "",
        "mailbox_ref": "",
    }
    for _k, _v in schema_defaults.items():
        if _k not in config:
            config[_k] = _v

    # Optional: persist mailbox ref for future repairer runs.
    # Keep it as-is (opaque ref) to avoid mixing provider logic here.
    if mailbox_ref and str(mailbox_ref).strip():
        config["mailbox_ref"] = str(mailbox_ref).strip()

    return email, json.dumps(config, ensure_ascii=False, separators=(",", ":"))



# ===== create_proxy_extension =====
def create_proxy_extension(proxy: str) -> str | None:
    match = re.search(r"http://([^:]+):([^@]+)@([^:]+):(\d+)", proxy)
    if not match:
        return None
    user, pwd, host, port = match.groups()
    
    manifest_json = """
    {
        "version": "1.0.0",
        "manifest_version": 2,
        "name": "Chrome Proxy",
        "permissions": [
            "proxy",
            "tabs",
            "unlimitedStorage",
            "storage",
            "<all_urls>",
            "webRequest",
            "webRequestBlocking"
        ],
        "background": {
            "scripts": ["background.js"]
        },
        "minimum_chrome_version":"22.0.0"
    }
    """

    background_js = """
    var config = {
            mode: "fixed_servers",
            rules: {
              singleProxy: {
                scheme: "http",
                host: "%s",
                port: parseInt(%s)
              },
              bypassList: ["localhost", "127.0.0.1", "<local>"]
            }
          };

    chrome.proxy.settings.set({value: config, scope: "regular"}, function() {});

    function callbackFn(details) {
        return {
            authCredentials: {
                username: "%s",
                password: "%s"
            }
        };
    }

    chrome.webRequest.onAuthRequired.addListener(
                callbackFn,
                {urls: ["<all_urls>"]},
                ['blocking']
    );
    """ % (host, port, user, pwd)
    
    plugin_dir = tempfile.mkdtemp(prefix="proxy_auth_")
    with open(os.path.join(plugin_dir, "manifest.json"), "w", encoding="utf-8") as f:
        f.write(manifest_json)
    with open(os.path.join(plugin_dir, "background.js"), "w", encoding="utf-8") as f:
        f.write(background_js)
        
    return plugin_dir

from selenium import webdriver
from selenium.webdriver.chrome.service import Service
from selenium.webdriver.chrome.options import Options
from pathlib import Path

# Updated User-Agent: Chrome 134 (March 2026 stable)
_CHROME_UA = (
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36"
)

# ---------------------------------------------------------------------------
# Stealth evasion scripts (ported from puppeteer-extra-plugin-stealth)
# ---------------------------------------------------------------------------
_STEALTH_SCRIPTS_DIR = os.path.join(os.path.dirname(__file__), "stealth_scripts")
_STEALTH_EVASIONS = [
    "chrome.app",
    "chrome.csi",
    "chrome.loadTimes",
    "chrome.runtime",
    "iframe.contentWindow",
    "media.codecs",
    "navigator.hardwareConcurrency",
    "navigator.languages",
    "navigator.permissions",
    "navigator.plugins",
    "navigator.webdriver",
    "sourceurl",
    "webgl.vendor",
    "window.outerdimensions",
]


def _load_stealth_scripts() -> list[str]:
    """Load all stealth evasion JS scripts from stealth_scripts/ directory."""
    scripts: list[str] = []
    for name in _STEALTH_EVASIONS:
        js_path = os.path.join(_STEALTH_SCRIPTS_DIR, name, "index.js")
        if os.path.isfile(js_path):
            try:
                with open(js_path, "r", encoding="utf-8") as f:
                    scripts.append(f.read())
            except Exception:
                pass
    return scripts


# Pre-load stealth scripts at import time so we don't re-read from disk per driver.
_STEALTH_JS_SOURCES = _load_stealth_scripts()
if _STEALTH_JS_SOURCES:
    print(f"[stealth] loaded {len(_STEALTH_JS_SOURCES)} evasion scripts from {_STEALTH_SCRIPTS_DIR}")
else:
    print(f"[stealth] WARNING: no evasion scripts found in {_STEALTH_SCRIPTS_DIR}")


# ---------------------------------------------------------------------------
# Human-like behavior simulation helpers
# ---------------------------------------------------------------------------
def _human_delay(min_s: float = 0.3, max_s: float = 1.5) -> None:
    """Random sleep to simulate human reaction time."""
    time.sleep(random.uniform(min_s, max_s))


def _human_mouse_jitter(driver, *, attempts: int = 3) -> None:
    """Perform small random mouse movements to simulate human presence."""
    try:
        actions = ActionChains(driver)
        for _ in range(attempts):
            x_off = random.randint(-80, 80)
            y_off = random.randint(-40, 40)
            actions.move_by_offset(x_off, y_off)
            actions.pause(random.uniform(0.05, 0.15))
        actions.perform()
    except Exception:
        pass


def _human_type(element, text: str, *, per_char_delay: tuple[float, float] = (0.03, 0.10)) -> None:
    """Type text character by character with human-like delays.

    Only used for short inputs (email, password, verification code).
    For long inputs (>60 chars), falls back to send_keys.
    """
    if len(text) > 60:
        element.send_keys(text)
        return
    for ch in text:
        element.send_keys(ch)
        time.sleep(random.uniform(*per_char_delay))


def _resolve_chrome_version_main() -> int | None:
    """Resolve Chrome major version for undetected-chromedriver.

    Priority:
    1) CHROME_VERSION_MAIN env (manual override)
    2) Runtime detection from local chrome binary
    3) None (let uc decide)
    """

    raw = (os.environ.get("CHROME_VERSION_MAIN") or "").strip()
    if raw.isdigit():
        try:
            v = int(raw)
            if v > 0:
                return v
        except Exception:
            pass

    for cmd in (
        ["google-chrome", "--product-version"],
        ["google-chrome-stable", "--product-version"],
        ["chromium", "--product-version"],
    ):
        try:
            out = subprocess.check_output(cmd, stderr=subprocess.STDOUT, timeout=3)
            ver = out.decode("utf-8", errors="ignore").strip()
            m = re.match(r"^(\d+)\.", ver)
            if m:
                major = int(m.group(1))
                if major > 0:
                    return major
        except Exception:
            continue

    return None


def generate_name() -> tuple[str, str]:
    first = ["Neo", "John", "Sarah", "Michael", "Emma", "David", "James", "Robert", "Mary", "William", "Richard", "Thomas", "Charles", "Christopher", "Daniel", "Matthew", "Anthony", "Mark", "Donald", "Steven", "Paul", "Andrew", "Joshua", "Kenneth", "Kevin", "Brian", "George", "Edward", "Ronald", "Timothy"]
    last = ["Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis", "Rodriguez", "Martinez", "Hernandez", "Lopez", "Gonzalez", "Wilson", "Anderson", "Thomas", "Taylor", "Moore", "Jackson", "Martin", "Lee", "Perez", "Thompson", "White"]
    return random.choice(first), random.choice(last)


# ===== generate_pwd =====
def generate_pwd(length=12) -> str:
    chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@*&"
    return "".join(random.choice(chars) for _ in range(length)) + "A1@"

def enter_birthday(driver) -> str:
    # We no longer handle birthday here. The JS in the name step handles both name and birthday if they are on the same explicit page.
    # Otherwise, this is just the fallback blind tab entry.
    try:
        # Standard blind tab entry fallback
        birthday_input = driver.switch_to.active_element
        birthday_input.send_keys("1")
        birthday_input.send_keys(Keys.TAB)
        birthday_input = driver.switch_to.active_element
        birthday_input.send_keys("1")
        birthday_input.send_keys(Keys.TAB)
        birthday_input = driver.switch_to.active_element
        birthday_input.send_keys("2000")
        birthday_input.send_keys(Keys.ENTER)
    except Exception:
        pass
    
    return "2000-01-01"


# ===== smart_wait =====
def smart_wait(driver, by, value, timeout=20, *, debug_kind: str = "", debug_message: str = ""):
    """Wait for an element.

    While waiting, it also checks for OpenAI's “Oops, an error occurred!” overlay
    and clicks “Try again” automatically.

    In debug mode, when it fails it will dump the current page body/source and
    raise a RuntimeError (NOT TimeoutException), so logs won't be filled with
    generic timeout errors.
    """

    if debug_kind:
        _dbg("wait", f"{debug_kind} by={by} value={value!r} timeout={timeout}s", driver=driver)

    challenge_hints = [
        "verify you are human",
        "performing security verification",
        "just a moment",
        "cloudflare",
    ]

    end_time = time.time() + timeout
    while time.time() < end_time:
        try:
            # Fail-fast on Cloudflare/verification challenge pages.
            title_text = str(driver.execute_script("return (document && document.title) ? document.title : '';") or "").lower()
            body_text = str(
                driver.execute_script("return (document && document.body) ? (document.body.innerText || '') : '';")
                or ""
            ).lower()
            cur_url = str(getattr(driver, "current_url", "") or "").lower()
            joined = "\n".join([title_text, body_text, cur_url])
            if any(h in joined for h in challenge_hints):
                if debug_kind and "password" in debug_kind.lower():
                    raise RuntimeError("blocked challenge page before password step")
                raise RuntimeError("blocked challenge page")

            # Check for the "Try again" button and click it if it appears
            try_again_btns = driver.find_elements(
                By.XPATH,
                "//button[contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'try again')]",
            )
            if try_again_btns and try_again_btns[0].is_displayed():
                _dbg("overlay", "Detected 'Oops' overlay; clicking 'Try again'", driver=driver)
                _click_with_debug(driver, try_again_btns[0], tag="overlay_try_again", note="smart_wait oops overlay")
                time.sleep(2)  # Wait for page to reload/recover
                continue

            # Check for the actual target element
            el = driver.find_element(by, value)
            if el.is_displayed() and el.is_enabled():
                if debug_kind:
                    _dbg("wait", f"{debug_kind} ok", driver=driver)
                return el
        except Exception as e:
            if isinstance(e, RuntimeError):
                if debug_kind:
                    msg = debug_message or f"wait aborted by fatal ui error for {by}={value}"
                    try:
                        _dump_page_body(driver=driver, kind=debug_kind, message=msg)
                    except Exception:
                        pass
                    try:
                        _save_error_artifacts(driver=driver, kind=debug_kind, message=msg)
                    except Exception:
                        pass
                raise
        time.sleep(0.5)

    if debug_kind:
        msg = debug_message or f"wait failed for {by}={value}"
        try:
            _dump_page_body(driver=driver, kind=debug_kind, message=msg)
        except Exception:
            pass
        try:
            _save_error_artifacts(driver=driver, kind=debug_kind, message=msg)
        except Exception:
            pass
        raise RuntimeError(f"wait failed: {debug_kind}; page dumped")

    raise TimeoutException(f"Timeout waiting for {by}={value}")

# -----------------------------------------------------------------------------
# Repairer (修缮者)：消费 need_fix_auth/ 队列，老号登录换新 token
# -----------------------------------------------------------------------------

def _repairer_dirs() -> tuple[str, str, str, str]:
    need = _data_path(NEED_FIX_AUTH_DIRNAME)
    proc = os.path.join(need, "_processing")
    okd = _data_path(FIXED_SUCCESS_DIRNAME)
    bad = _data_path(FIXED_FAIL_DIRNAME)
    return need, proc, okd, bad


def _repairer_results_dir() -> str:
    return _results_dir()


def _append_jsonl(path: str, obj: dict[str, Any]) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    line = json.dumps(obj, ensure_ascii=False, separators=(",", ":"))
    with open(path, "a", encoding="utf-8") as f:
        f.write(line + "\n")


def _read_json_any(path: str) -> Any:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def _write_json_any(path: str, obj: Any) -> None:
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(obj, f, ensure_ascii=False, indent=2)
    os.replace(tmp, path)


def _deep_merge_keep_old_when_missing(old: Any, new: Any) -> Any:
    """Merge dicts recursively.

    Policy:
    - If key exists in `new`, it overwrites `old`.
    - If key does NOT exist in `new`, keep from `old`.
    - For nested dicts: recurse.
    - For lists / scalars: replace.

    Additionally, for `email` / `password`, we treat empty-string from `new` as "missing"
    and keep old values.
    """

    if isinstance(old, dict) and isinstance(new, dict):
        out: dict[str, Any] = dict(old)
        for k, v in new.items():
            if k in out and isinstance(out.get(k), dict) and isinstance(v, dict):
                out[k] = _deep_merge_keep_old_when_missing(out.get(k), v)
            else:
                out[k] = v

        for k in ("email", "password"):
            if k in old and (k not in new or str(new.get(k) or "").strip() == ""):
                out[k] = old.get(k)

        return out

    return new if new is not None else old


def _repairer_claim_one_file() -> str | None:
    need, proc, _okd, _bad = _repairer_dirs()
    if not os.path.isdir(need):
        return None
    os.makedirs(proc, exist_ok=True)

    try:
        names = [
            n
            for n in os.listdir(need)
            if n.lower().endswith(".json") and os.path.isfile(os.path.join(need, n))
        ]
    except Exception:
        return None

    if not names:
        return None

    # oldest first for stability
    try:
        names.sort(key=lambda n: os.path.getmtime(os.path.join(need, n)))
    except Exception:
        pass

    for name in names:
        src = os.path.join(need, name)
        dst = os.path.join(proc, name)
        try:
            os.replace(src, dst)
            return dst
        except FileNotFoundError:
            continue
        except PermissionError:
            continue
        except OSError:
            continue

    return None


def _repairer_release_stale_processing(*, stale_seconds: int = 1800) -> None:
    _need, proc, _okd, _bad = _repairer_dirs()
    if not os.path.isdir(proc):
        return

    now = time.time()
    for name in os.listdir(proc):
        if not name.lower().endswith(".json"):
            continue
        path = os.path.join(proc, name)
        try:
            st = os.stat(path)
        except Exception:
            continue
        if now - st.st_mtime < stale_seconds:
            continue

        # move back for retry
        try:
            os.replace(path, os.path.join(_data_path(NEED_FIX_AUTH_DIRNAME), name))
        except Exception:
            pass



# ===== _find_visible =====
def _find_visible(driver, by, value):
    try:
        els = driver.find_elements(by, value)
    except Exception:
        return None
    for el in els:
        try:
            if el.is_displayed() and el.is_enabled():
                return el
        except Exception:
            continue
    return None



# ===== _click_if_found =====
def _click_if_found(driver, xpath: str) -> bool:
    try:
        el = _find_visible(driver, By.XPATH, xpath)
        if not el:
            return False
        _click_with_debug(driver, el, tag="click_if_found", note=f"xpath={xpath[:120]}")
        return True
    except Exception:
        return False



# ===== _wait_for_any =====
def _wait_for_any(driver, *, timeout_seconds: int, predicates: list[callable]) -> Any:
    end = time.time() + timeout_seconds
    last_exc: Exception | None = None
    while time.time() < end:
        for p in predicates:
            try:
                v = p()
                if v:
                    return v
            except Exception as e:
                last_exc = e
        time.sleep(0.4)
    raise RuntimeError(f"timeout waiting for condition: {last_exc}")


def _wait_code_try_candidates(*, candidates: list[str], timeout_seconds: int) -> tuple[str, str]:
    """Try multiple encoded mailbox_ref until we can fetch a 6-digit code.

    Special policy:
    - If GPTMail is quota-limited (or all keys exhausted), we should NOT mark the
      auth as "unrepairable". Caller can treat it as a transient "no_quota" case.
    """

    last_err: Exception | None = None
    last_err_str = ""

    for ref in candidates:
        r = str(ref or "").strip()
        if not r:
            continue
        try:
            code = wait_openai_code_by_provider(
                provider="auto",
                mailbox_ref=r,
                mailcreate_base_url=MAILCREATE_BASE_URL,
                mailcreate_custom_auth=MAILCREATE_CUSTOM_AUTH,
                gptmail_base_url=GPTMAIL_BASE_URL,
                gptmail_api_key=GPTMAIL_API_KEY,
                gptmail_keys_file=GPTMAIL_KEYS_FILE,
                timeout_seconds=timeout_seconds,
            )
            return str(code), r
        except Exception as e:
            last_err = e
            last_err_str = str(e)

            # Normalize quota-like failures for caller.
            s = last_err_str.lower()
            if "all gptmail keys are exhausted" in s or "quota" in s or "daily quota" in s:
                raise RuntimeError("no_quota_for_otp")

            # If we are using GPTMail and just cannot get a code (deliverability/empty inbox),
            # treat it as a real repair failure.
            if "timeout waiting for 6-digit code" in s:
                raise RuntimeError("otp_timeout")

            continue

    raise RuntimeError(f"failed to fetch openai code from all mailbox_ref candidates: {last_err}")


def _repairer_drive_login_and_get_callback_url(*, driver, oauth: OAuthStart, email: str, password: str, mailbox_ref_candidates: list[str]) -> tuple[str, str]:
    """Drive OpenAI login flow until OAuth redirects to callback URL.

    Returns:
      (callback_url, chosen_mailbox_ref)
    """

    driver.get(oauth.auth_url)

    try:
        WebDriverWait(driver, 60).until(EC.url_contains("auth.openai.com"))
    except Exception:
        raise RuntimeError("did not reach auth.openai.com")

    # Step: fill email
    email_input = None
    try:
        email_input = smart_wait(driver, By.ID, "_r_f_-email", timeout=15)
    except Exception:
        email_input = _find_visible(driver, By.CSS_SELECTOR, 'input[type="email"]')

    if not email_input:
        raise RuntimeError("email input not found")

    try:
        email_input.clear()
    except Exception:
        pass
    email_input.send_keys(str(email))
    email_input.send_keys(Keys.ENTER)

    def _password_input():
        return _find_visible(driver, By.CSS_SELECTOR, 'input[type="password"]')

    # Some flows require clicking "Continue" then "Continue with password".
    for _ in range(60):
        pwd_inp = _password_input()
        if pwd_inp:
            try:
                pwd_inp.clear()
            except Exception:
                pass
            pwd_inp.send_keys(str(password))
            pwd_inp.send_keys(Keys.ENTER)
            break

        if _click_if_found(
            driver,
            "//button[contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'continue with password') or contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'password')]",
        ):
            time.sleep(0.6)
            continue

        _click_if_found(driver, "//button[contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'continue')]")
        time.sleep(0.6)

    def _has_callback() -> bool:
        try:
            return "localhost:1455" in str(getattr(driver, "current_url", "") or "")
        except Exception:
            return False

    def _code_input():
        # 常见单输入框
        selectors = [
            'input[id*="code"]',
            'input[name*="code"]',
            'input[autocomplete="one-time-code"]',
            'input[inputmode="numeric"][maxlength="6"]',
            'input[aria-label*="code" i]',
            'input[placeholder*="code" i]',
        ]

        for sel in selectors:
            el = _find_visible(driver, By.CSS_SELECTOR, sel)
            if el:
                return el

        # 常见分段输入框
        try:
            group = driver.find_elements(By.CSS_SELECTOR, 'div[role="group"] input[inputmode="numeric"][maxlength="1"]')
            if group:
                return group
        except Exception:
            pass

        return None

    def _has_risk_text_hint() -> bool:
        try:
            txt = str(driver.execute_script("return document && document.body ? (document.body.innerText || '') : ''; ") or "").lower()
        except Exception:
            txt = ""

        hints = [
            "verification code",
            "enter code",
            "check your email",
            "email a code",
            "send code",
            "verify it's you",
            "验证码",
            "发送验证码",
            "邮箱验证码",
            "请输入验证码",
        ]
        return any(h in txt for h in hints)

    def _click_send_code_if_needed() -> bool:
        """有风控时可能先出现“发送验证码”按钮，先触发发送，再等待输入框出现。"""

        send_code_xpaths = [
            "//*[self::button or self::a or self::span][contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'send code')]",
            "//*[self::button or self::a or self::span][contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'email me a code')]",
            "//*[self::button or self::a or self::span][contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'send verification')]",
            "//*[self::button or self::a or self::span][contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'get code')]",
            "//*[self::button or self::a or self::span][contains(., '发送验证码') or contains(., '验证码') or contains(., '发送代码')]",
            "//button[contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'continue') and not(contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'with'))]",
        ]

        for xp in send_code_xpaths:
            if _click_if_found(driver, xp):
                time.sleep(1.0)
                return True

        return False

    def _await_callback_or_code_stage() -> Any:
        """分类讨论：
        - 无风控：直接跳 callback
        - 有风控：先触发发送验证码，再等待 code input 出现
        """

        if _has_callback():
            return "CALLBACK"

        ci = _code_input()
        if ci:
            return ci

        # 如果页面存在风控提示，主动触发“发送验证码”按钮
        if _has_risk_text_hint():
            _click_send_code_if_needed()
            ci2 = _code_input()
            if ci2:
                return ci2

        return None

    v = _wait_for_any(driver, timeout_seconds=80, predicates=[_await_callback_or_code_stage])

    chosen_ref = ""

    if v != "CALLBACK":
        code, chosen_ref = _wait_code_try_candidates(candidates=mailbox_ref_candidates, timeout_seconds=180)

        if isinstance(v, list):
            for cur, digit in zip(v, str(code)):
                try:
                    _click_with_debug(driver, cur, tag="repairer_otp_digit_box", note="repairer segmented otp input")
                    cur.clear()
                except Exception:
                    pass
                cur.send_keys(str(digit))
            try:
                driver.switch_to.active_element.send_keys(Keys.ENTER)
            except Exception:
                pass
        else:
            try:
                v.clear()
            except Exception:
                pass
            v.send_keys(str(code))
            v.send_keys(Keys.ENTER)

    try:
        WebDriverWait(driver, 60).until(EC.url_contains("localhost:1455"))
    except Exception:
        raise RuntimeError("timeout waiting for oauth callback")

    return str(getattr(driver, "current_url", "") or ""), chosen_ref


def _repair_one_auth_file(path: str, *, proxy: str | None) -> tuple[bool, str, str | None]:
    """Repair one auth json file.

    Returns:
      (ok, reason, out_path)
    """

    name = os.path.basename(path)
    auth_obj = _read_json_any(path)

    if not isinstance(auth_obj, dict):
        return False, "invalid_json_not_object", None

    email = str(auth_obj.get("email") or "").strip()
    password = str(auth_obj.get("password") or "").strip()
    account_id = str(auth_obj.get("account_id") or "").strip()

    if not account_id:
        # fallback from nested claims field
        try:
            account_id = str((auth_obj.get("https://api.openai.com/auth") or {}).get("chatgpt_account_id") or "").strip()
        except Exception:
            account_id = ""

    if not email:
        return False, "missing_email", None
    if not password:
        return False, "missing_password", None

    # Prepare mailbox_ref candidates (encoded refs for mailbox_provider)
    candidates: list[str] = []

    # 1) previously persisted mailbox_ref (from our own submit_callback_url)
    mr0 = str(auth_obj.get("mailbox_ref") or "").strip()
    if mr0:
        candidates.append(mr0)

    # 2) best-effort: mailcreate jwt mint for existing address (preferred for repair)
    # If admin creds exist, we can poll MailCreate for OpenAI OTP reliably.

    # 3) best-effort: if user provides mailcreate admin creds, they can mint jwt for existing address
    # (Optional; errors ignored.)
    try:
        mc_custom = (
            os.environ.get("MAILCREATE_CUSTOM_AUTH")
            or _PLATFORMTOOLS_DEV_VARS.get("MAILCREATE_CUSTOM_AUTH")
            or str(_MAILCREATE_CFG.get("MAILCREATE_CUSTOM_AUTH") or "")
            or ""
        ).strip()
        mc_admin = (
            os.environ.get("MAILCREATE_ADMIN_AUTH")
            or _PLATFORMTOOLS_DEV_VARS.get("MAILCREATE_ADMIN_AUTH")
            or str(_MAILCREATE_CFG.get("MAILCREATE_ADMIN_AUTH") or "")
            or ""
        ).strip()
        if mc_custom and mc_admin and str(MAILCREATE_BASE_URL or "").strip():
            # admin endpoints
            base = str(MAILCREATE_BASE_URL).strip().rstrip("/")

            def _http_json(*, url: str, method: str = "GET", headers: dict[str, str] | None = None, payload: Any | None = None, timeout: int = 30) -> tuple[int, str, Any]:
                hdr = dict(headers or {})
                data = None
                if payload is not None:
                    data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
                    hdr.setdefault("Content-Type", "application/json")
                req = urllib.request.Request(url, data=data, headers=hdr, method=method)
                try:
                    with urllib.request.urlopen(req, timeout=timeout) as resp:
                        st = int(getattr(resp, "status", 200))
                        text = resp.read().decode("utf-8", errors="replace")
                        try:
                            obj = json.loads(text) if text else {}
                        except Exception:
                            obj = {}
                        return st, text, obj
                except urllib.error.HTTPError as e:
                    text = e.read().decode("utf-8", errors="replace")
                    try:
                        obj = json.loads(text) if text else {}
                    except Exception:
                        obj = {}
                    return int(getattr(e, "code", 0) or 0), text, obj

            admin_headers = {"x-custom-auth": mc_custom, "x-admin-auth": mc_admin, "Accept": "application/json"}
            q = urllib.parse.urlencode({"limit": "50", "offset": "0", "query": email})
            st1, _tx1, obj1 = _http_json(url=f"{base}/admin/address?{q}", method="GET", headers=admin_headers)
            addr_id = None
            if 200 <= st1 < 300 and isinstance(obj1, dict) and isinstance(obj1.get("results"), list):
                target = email.strip().lower()
                for it in obj1.get("results"):
                    if isinstance(it, dict) and str(it.get("name") or "").strip().lower() == target:
                        try:
                            addr_id = int(it.get("id"))
                        except Exception:
                            addr_id = None
                        break

            if addr_id:
                st2, _tx2, obj2 = _http_json(url=f"{base}/admin/show_password/{int(addr_id)}", method="GET", headers=admin_headers)
                if 200 <= st2 < 300 and isinstance(obj2, dict):
                    jwt = str(obj2.get("jwt") or "").strip()
                    if jwt:
                        candidates.append(f"mailcreate:{jwt}")
    except Exception:
        pass

    # 4) last resort: try gptmail by email
    candidates.append(f"gptmail:{email}")

    # de-dup candidates, keep order
    seen: set[str] = set()
    candidates = [c for c in candidates if c and (c not in seen and not seen.add(c))]

    # probe for log (optional)
    try:
        result = _probe_wham_one(auth_obj=auth_obj, proxy=None)
    except Exception:
        result = ProbeResult(status_code=None, note="probe_failed", category="probe_failed")

    _append_jsonl(
        os.path.join(_repairer_results_dir(), "repairer_probe.jsonl"),
        {
            "ts": _utc_now_iso(),
            "file": name,
            "account_id": account_id,
            "email": email,
            "status_code": result.status_code,
            "note": result.note,
            "probe_category": result.category,
            "retry_after_seconds": result.retry_after_seconds,
            "upstream_status": result.http_status,
        },
    )

    driver = None
    proxy_dir = None
    try:
        with driver_init_lock:
            driver, proxy_dir = browser_new_driver(proxy)

        oauth = generate_oauth_url()
        callback_url, chosen_ref = _repairer_drive_login_and_get_callback_url(
            driver=driver,
            oauth=oauth,
            email=email,
            password=password,
            mailbox_ref_candidates=candidates,
        )

        # exchange callback -> new token json
        reg_email, config_json = submit_callback_url(
            callback_url=callback_url,
            expected_state=oauth.state,
            code_verifier=oauth.code_verifier,
            redirect_uri=oauth.redirect_uri,
            proxy=proxy,
            mailbox_ref=(chosen_ref or (mr0 or "")),
            password=password,
            first_name=str(auth_obj.get("first_name") or ""),
            last_name=str(auth_obj.get("last_name") or ""),
            birthdate=str(auth_obj.get("birthdate") or ""),
        )

        try:
            new_obj = json.loads(config_json)
        except Exception:
            new_obj = {}

        merged = _deep_merge_keep_old_when_missing(auth_obj, new_obj)

        # Write outputs
        # 约定：修缮成功后不写入本地 token 池（禁止进入 codex_auth）。
        # 成功产物只进入 fixed_success，后续由 uploader 负责上传 fixed_success 目录。
        ts_ms = int(time.time() * 1000)
        rand = secrets.token_hex(3)
        safe_acc = re.sub(r"[^a-zA-Z0-9_.-]+", "_", (account_id or "unknown"))[:64] or "unknown"
        out_name = f"codex-repaired-{safe_acc}-{INSTANCE_ID}-{ts_ms}-{rand}.json"

        fixed_success_path = os.path.join(_data_path(FIXED_SUCCESS_DIRNAME), out_name)

        with write_lock:
            os.makedirs(_data_path(FIXED_SUCCESS_DIRNAME), exist_ok=True)
            _write_json_any(fixed_success_path, merged)

        _append_jsonl(
            os.path.join(_repairer_results_dir(), "repairer_success.jsonl"),
            {"ts": _utc_now_iso(), "file": name, "account_id": account_id, "email": reg_email, "out": out_name},
        )

        return True, "ok", fixed_success_path

    finally:
        if driver:
            try:
                driver.quit()
            except Exception:
                pass
        if proxy_dir and os.path.exists(proxy_dir):
            try:
                shutil.rmtree(proxy_dir, ignore_errors=True)
            except Exception:
                pass


def _repairer_restore_claimed_for_test(*, claimed: str, name: str) -> None:
    """测试模式：将 _processing 中的输入文件放回 need_fix_auth，方便重复测试。"""

    try:
        dst = os.path.join(_data_path(NEED_FIX_AUTH_DIRNAME), name)
        os.replace(claimed, dst)
    except Exception:
        # 回退：若原子替换失败，尽量复制保留样本
        try:
            shutil.copy2(claimed, os.path.join(_data_path(NEED_FIX_AUTH_DIRNAME), name))
        except Exception:
            pass
        try:
            os.remove(claimed)
        except Exception:
            pass



def _repairer_loop() -> None:
    need, proc, okd, bad = _repairer_dirs()
    os.makedirs(need, exist_ok=True)
    os.makedirs(proc, exist_ok=True)
    os.makedirs(okd, exist_ok=True)
    os.makedirs(bad, exist_ok=True)
    os.makedirs(_repairer_results_dir(), exist_ok=True)

    print(f"[repairer] enabled=1 need_fix_dir={need}")
    print(f"[repairer] poll_seconds={REPAIRER_POLL_SECONDS}")
    print(f"[repairer] test_keep_input={REPAIRER_TEST_KEEP_INPUT}")

    stale_seconds = int(os.environ.get("REPAIRER_STALE_SECONDS", "1800"))
    processed_once_in_test: set[str] = set()

    while True:
        try:
            _repairer_release_stale_processing(stale_seconds=stale_seconds)

            claimed = _repairer_claim_one_file()
            if not claimed:
                time.sleep(REPAIRER_POLL_SECONDS)
                continue

            name = os.path.basename(claimed)

            # 测试模式下：每个文件每次进程仅处理一次，避免无限循环刷同一文件。
            if REPAIRER_TEST_KEEP_INPUT == 1 and name in processed_once_in_test:
                _repairer_restore_claimed_for_test(claimed=claimed, name=name)
                time.sleep(REPAIRER_POLL_SECONDS)
                continue

            proxies = load_proxies()
            proxy = random.choice(proxies) if proxies else None

            ok = False
            out_path = None
            reason = ""
            try:
                ok, reason, out_path = _repair_one_auth_file(claimed, proxy=proxy)
            except Exception as e:
                ok = False
                reason = f"exception:{e}"

            if ok:
                # 正常模式：成功后删除；测试模式：回放到 need_fix_auth 便于重复测试。
                try:
                    if REPAIRER_TEST_KEEP_INPUT == 1:
                        _repairer_restore_claimed_for_test(claimed=claimed, name=name)
                        processed_once_in_test.add(name)
                    else:
                        os.remove(claimed)
                except Exception:
                    pass
                print(f"[repairer] ok file={name} out={out_path}")
                continue

            # Special: OTP quota exhausted. This is NOT an unrecoverable repair failure.
            if "no_quota_for_otp" in (reason or ""):
                _append_jsonl(
                    os.path.join(_repairer_results_dir(), "repairer_no_quota.jsonl"),
                    {"ts": _utc_now_iso(), "file": name, "reason": reason},
                )
                # 正常模式：删除；测试模式：回放输入样本，且不重复处理。
                try:
                    if REPAIRER_TEST_KEEP_INPUT == 1:
                        _repairer_restore_claimed_for_test(claimed=claimed, name=name)
                        processed_once_in_test.add(name)
                    else:
                        os.remove(claimed)
                except Exception:
                    pass
                print(f"[repairer] skip(no_quota) file={name}")
                continue

            # If we had quota to access mailbox API, but still couldn't fetch OTP (timeout),
            # this is considered a real "unrepairable" attempt and should be reported.
            # (The code path sets reason="exception:otp_timeout".)

            # failure policy:
            # - 向服务端上报“该 account_id 无法修缮”（服务端累计 3 次进墓地）
            # - 本地不归档 fixed_fail；直接删除队列文件（避免无限重试）
            try:
                auth_obj = _read_json_any(claimed)
            except Exception:
                auth_obj = {}

            acc = ""
            try:
                acc = str(auth_obj.get("account_id") or "").strip() if isinstance(auth_obj, dict) else ""
            except Exception:
                acc = ""
            if not acc:
                try:
                    acc = str((auth_obj.get("https://api.openai.com/auth") or {}).get("chatgpt_account_id") or "").strip() if isinstance(auth_obj, dict) else ""
                except Exception:
                    acc = ""

            report_ok = False
            report_http = 0
            report_resp = ""
            if acc:
                try:
                    report_ok, report_http, report_resp = _report_auth_repair_failed(account_id=acc, note=reason[:1000])
                except Exception as e:
                    report_ok, report_http, report_resp = False, 0, f"exception:{e}"

            _append_jsonl(
                os.path.join(_repairer_results_dir(), "repairer_failed.jsonl"),
                {
                    "ts": _utc_now_iso(),
                    "file": name,
                    "account_id": acc,
                    "reason": reason,
                    "report_ok": report_ok,
                    "http": report_http,
                    "resp": str(report_resp or "")[:800],
                },
            )

            try:
                if REPAIRER_TEST_KEEP_INPUT == 1:
                    _repairer_restore_claimed_for_test(claimed=claimed, name=name)
                    processed_once_in_test.add(name)
                else:
                    os.remove(claimed)
            except Exception:
                pass

            print(f"[repairer] fail file={name} reason={reason} report_ok={report_ok} http={report_http}")

        except Exception as e:
            print(f"[repairer] loop error: {e}")

        time.sleep(0.2)

def load_proxies() -> list[str]:
    if DISABLE_PROXY:
        return []
    proxy_file = _data_path("proxies.txt")
    if os.path.exists(proxy_file):
        with open(proxy_file, "r", encoding="utf-8") as f:
            proxies = [line.strip() for line in f if line.strip() and not line.startswith("#")]
        return proxies
    return []


class ProxyPool:
    """Thread-safe + cross-instance mutex proxy allocator.

    Ensures no two workers (within or across container instances) use the
    same proxy entry simultaneously.

    Intra-instance:  threading.Lock protects the in-memory set.
    Inter-instance:  file-based locks in /data/.proxy_locks/ (shared volume).
    Lock files auto-expire after LOCK_TTL_SECONDS to handle crashes.
    """

    LOCK_TTL_SECONDS = 600  # 10 min auto-expire for stale locks

    def __init__(self):
        self._lock = threading.Lock()
        self._in_use: set[str] = set()  # proxies checked out by THIS instance
        self._lock_dir = _data_path(".proxy_locks")
        os.makedirs(self._lock_dir, exist_ok=True)

    def _lock_path(self, proxy: str) -> str:
        """Deterministic lock file path for a proxy string."""
        import hashlib
        h = hashlib.md5(proxy.encode()).hexdigest()[:12]
        return os.path.join(self._lock_dir, f"proxy_{h}.lock")

    def _is_locked_by_other(self, proxy: str) -> bool:
        """Check if another instance holds the lock (via file)."""
        lp = self._lock_path(proxy)
        if not os.path.exists(lp):
            return False
        try:
            mtime = os.path.getmtime(lp)
            if time.time() - mtime > self.LOCK_TTL_SECONDS:
                # Stale lock — remove it
                try:
                    os.remove(lp)
                except OSError:
                    pass
                return False
            with open(lp, "r") as f:
                owner = f.read().strip()
            # If we own it, it's not "locked by other"
            return owner != INSTANCE_ID
        except Exception:
            return False

    def _write_lock_file(self, proxy: str):
        lp = self._lock_path(proxy)
        try:
            with open(lp, "w") as f:
                f.write(INSTANCE_ID)
        except Exception:
            pass

    def _remove_lock_file(self, proxy: str):
        lp = self._lock_path(proxy)
        try:
            if os.path.exists(lp):
                with open(lp, "r") as f:
                    owner = f.read().strip()
                if owner == INSTANCE_ID:
                    os.remove(lp)
        except Exception:
            pass

    def acquire(self, worker_id: int, timeout: float = 60) -> str | None:
        """Acquire an exclusive proxy. Returns proxy string or None if timed out."""
        deadline = time.time() + timeout
        attempt = 0
        while time.time() < deadline:
            proxies = load_proxies()
            if not proxies:
                return None
            random.shuffle(proxies)

            with self._lock:
                for p in proxies:
                    if p not in self._in_use and not self._is_locked_by_other(p):
                        self._in_use.add(p)
                        self._write_lock_file(p)
                        return p

            # All proxies busy — wait with backoff
            attempt += 1
            wait = min(2.0 * attempt, 10.0)
            print(f"[Worker {worker_id}] 所有代理占用中，等待 {wait:.0f}s...")
            time.sleep(wait)

        print(f"[Worker {worker_id}] 获取代理超时")
        return None

    def release(self, proxy: str):
        """Release a proxy back to the pool."""
        with self._lock:
            self._in_use.discard(proxy)
            self._remove_lock_file(proxy)


# Global proxy pool (one per container instance)
_proxy_pool = ProxyPool()

def worker(worker_id: int):
    fatal_driver_errors = 0
    fatal_restart_threshold = max(1, int(os.environ.get("FATAL_DRIVER_RESTART_THRESHOLD", "3") or "3"))

    while True:
        proxy = _proxy_pool.acquire(worker_id)

        if proxy:
            print(f"[Worker {worker_id}] ---> 使用代理: {proxy} <---")
        elif REQUIRE_PROXY:
            print(
                f"[Worker {worker_id}] [x] register_proxy_required no_proxy_available "
                f"flow={REGISTER_FLOW_MODE} headless={int(os.environ.get('HEADLESS', '1') or '1')}"
            )
            time.sleep(1.0)
            continue
        else:
            print(f"[Worker {worker_id}] ---> 未配置可用代理，使用本地网络直连 <---")

        driver = None
        proxy_dir = None
        keep_browser_for_debug = False
        need_wait_for_debug = False

        try:
            if REGISTER_FLOW_MODE == "protocol":
                print(f"[Worker {worker_id}] ---> 协议流模式（REGISTER_FLOW_MODE=protocol） <---")
                reg_email, res = run_protocol_register(proxy)
            else:
                reg_email, res, driver, proxy_dir = run_browser_register(proxy)

            # Write outputs (sharded results + per-account json)
            with write_lock:
                _append_result_line(res)

                # Write per-account auth json file into codex_auth/
                codex_auth_dir = _data_path(CODEX_AUTH_DIRNAME)
                os.makedirs(codex_auth_dir, exist_ok=True)
                # Use a unique filename to avoid collisions when multiple containers
                # share the same data volume.
                ts_ms = int(time.time() * 1000)
                rand = secrets.token_hex(3)
                auth_path = os.path.join(
                    codex_auth_dir,
                    f"codex-{reg_email}-free-{INSTANCE_ID}-{ts_ms}-{rand}.json",
                )
                with open(auth_path, "w", encoding="utf-8") as f:
                    f.write(json.dumps(json.loads(res), indent=2, ensure_ascii=False))

                # 可选：配置同步（把 codex_auth 写入同步目录）
                try:
                    _sync_codex_auth_copy(src_path=auth_path)
                except Exception:
                    pass

                # Also copy into wait_update/ for downstream pickup
                wait_update_dir = _data_path(WAIT_UPDATE_DIRNAME)
                os.makedirs(wait_update_dir, exist_ok=True)
                try:
                    shutil.copy2(auth_path, os.path.join(wait_update_dir, os.path.basename(auth_path)))
                except Exception:
                    pass

            print(
                f"[Worker {worker_id}] [✓] 注册成功，Token 已保存在 {CODEX_AUTH_DIRNAME} 并复制到 {WAIT_UPDATE_DIRNAME}，并追加到 results 分片！"
            )

        except RuntimeError as e:
            # Expected blocks, no stack trace needed
            fatal_driver_errors = 0
            keep_browser_for_debug = (DEBUG_KEEP_BROWSER_ON_FAIL == 1 and driver is not None)
            need_wait_for_debug = (DEBUG_WAIT_ON_FAIL == 1 and driver is not None)
            # Capture screenshot for debugging
            if driver is not None:
                try:
                    _save_error_artifacts(driver=driver, kind="runtime_error", message=f"Worker {worker_id}: {e}")
                except Exception:
                    pass
            print(f"[Worker {worker_id}] [x] {e} (准备换IP重试)")
        except TimeoutException as e:
            fatal_driver_errors = 0
            keep_browser_for_debug = (DEBUG_KEEP_BROWSER_ON_FAIL == 1 and driver is not None)
            need_wait_for_debug = (DEBUG_WAIT_ON_FAIL == 1 and driver is not None)
            # Capture screenshot for debugging
            if driver is not None:
                try:
                    _save_error_artifacts(driver=driver, kind="timeout", message=f"Worker {worker_id}: {e}")
                except Exception:
                    pass
            print(f"[Worker {worker_id}] [x] 页面加载超时，可能遇到风控盾拦截。 (准备换IP重试)")
        except Exception as e:
            err_str = str(e)
            is_proxy_eof = (
                "RemoteDisconnected" in err_str
                or "Connection aborted" in err_str
                or "Max retries exceeded" in err_str
                or "UNEXPECTED_EOF_WHILE_READING" in err_str
                or "UNEXPECTED_MESSAGE" in err_str
            )
            is_fatal_driver = (
                "SessionNotCreatedException" in err_str
                or "failed to start a thread for the new session" in err_str
                or "chromedriver unexpectedly exited" in err_str
                or "DevToolsActivePort" in err_str
                or "Chrome instance exited" in err_str
            )

            if is_proxy_eof:
                fatal_driver_errors = 0
                keep_browser_for_debug = (DEBUG_KEEP_BROWSER_ON_FAIL == 1 and driver is not None)
                need_wait_for_debug = (DEBUG_WAIT_ON_FAIL == 1 and driver is not None)
                # Capture screenshot for proxy EOF errors too
                if driver is not None:
                    try:
                        _save_error_artifacts(driver=driver, kind="proxy_eof", message=f"Worker {worker_id}: {err_str}")
                    except Exception:
                        pass
                print(f"[Worker {worker_id}] [x] 代理连接强制中断 (SSL/EOF断流)，准备换IP重试")
            else:
                import traceback
                trace_str = traceback.format_exc()
                keep_browser_for_debug = (DEBUG_KEEP_BROWSER_ON_FAIL == 1 and driver is not None)
                need_wait_for_debug = (DEBUG_WAIT_ON_FAIL == 1 and driver is not None)
                # Capture screenshot for unexpected errors
                if driver is not None:
                    try:
                        _save_error_artifacts(driver=driver, kind="unexpected_error", message=f"Worker {worker_id}: {trace_str[-500:]}")
                    except Exception:
                        pass
                print(f"[Worker {worker_id}] [x] 本次注册流程意外中止:\\n{trace_str}")

                if is_fatal_driver:
                    fatal_driver_errors += 1
                    print(
                        f"[Worker {worker_id}] [x] 致命浏览器错误累计={fatal_driver_errors}/"
                        f"{fatal_restart_threshold}，达到阈值将触发容器重启"
                    )
                    if fatal_driver_errors >= fatal_restart_threshold:
                        print(f"[Worker {worker_id}] [x] 触发进程退出，交给容器 restart=always 自动拉起")
                        os._exit(66)
                else:
                    fatal_driver_errors = 0

        finally:
            if need_wait_for_debug and driver is not None:
                print(
                    f"[Worker {worker_id}] [debug] 检测到失败，按 DEBUG_WAIT_ON_FAIL=1 进入现场等待。"
                    f" 浏览器将保持打开，按 Ctrl+C 继续。"
                )
                try:
                    while True:
                        time.sleep(1.0)
                except KeyboardInterrupt:
                    print(f"[Worker {worker_id}] [debug] 收到 Ctrl+C，结束现场等待。")

            if driver and not keep_browser_for_debug:
                try:
                    driver.quit()
                except Exception:
                    pass
            if proxy_dir and os.path.exists(proxy_dir):
                shutil.rmtree(proxy_dir, ignore_errors=True)
            # Release proxy back to pool
            if proxy:
                _proxy_pool.release(proxy)

        # 自由调整休眠时间
        sleep_min = int(os.environ.get("SLEEP_MIN", "5"))
        sleep_max = int(os.environ.get("SLEEP_MAX", "20"))
        sleep_time = random.randint(sleep_min, sleep_max) if sleep_max >= sleep_min else sleep_min
        print(f"[Worker {worker_id}] 任务结束。挂起 {sleep_time} 秒后开启下一轮尝试...")
        time.sleep(sleep_time)

if __name__ == "__main__":
    os.makedirs(DATA_DIR, exist_ok=True)

    # Per-instance dirs (safe for multi-container shared volume)
    os.makedirs(_results_dir(), exist_ok=True)
    os.makedirs(_data_path(ERROR_DIRNAME, INSTANCE_ID), exist_ok=True)

    # Shared dirs
    os.makedirs(_data_path(CODEX_AUTH_DIRNAME), exist_ok=True)
    os.makedirs(_data_path(WAIT_UPDATE_DIRNAME), exist_ok=True)
    os.makedirs(_data_path(NEED_FIX_AUTH_DIRNAME), exist_ok=True)
    os.makedirs(_data_path(FIXED_SUCCESS_DIRNAME), exist_ok=True)
    os.makedirs(_data_path(FIXED_FAIL_DIRNAME), exist_ok=True)

    # Move any legacy root results shards/state into this instance dir.
    _migrate_legacy_results_layout()

    proxy_file = _data_path("proxies.txt")
    
    if not os.path.exists(proxy_file):
        with open(proxy_file, "w", encoding="utf-8") as f:
            f.write("# 在此文件中添加您的代理IP池，每行一个\n")
            f.write("# 格式示例: http://192.168.1.100:8080\n")
            
    concurrency = int(os.environ.get("CONCURRENCY", "1"))
    if concurrency < 0:
        concurrency = 0

    # 方案A：同一容器同时做生产 + 探测/续杯（可通过 env 关闭）
    if ENABLE_PROBE == 1:
        try:
            t = threading.Thread(target=_probe_loop, name="probe_loop", daemon=True)
            t.start()
        except Exception as e:
            print(f"[probe] failed to start probe thread: {e}")

    # 修缮者：同一进程内后台跑 need_fix_auth 修复循环
    if ENABLE_REPAIRER == 1:
        try:
            t2 = threading.Thread(target=_repairer_loop, name="repairer_loop", daemon=True)
            t2.start()
        except Exception as e:
            print(f"[repairer] failed to start repairer thread: {e}")

    print(f"==== 守护进程启动: 无限循环多线程生成器 (并发数: {concurrency}) ====")
    print(f"INSTANCE_ID={INSTANCE_ID}")
    print(f"results 分片将写入 {_results_dir()} (每 {RESULTS_SHARD_SIZE} 条一片)")
    print(f"账号 JSON 将写入 {_data_path(CODEX_AUTH_DIRNAME)} 并复制到 {_data_path(WAIT_UPDATE_DIRNAME)}")
    print(f"代理池请直接写入 {proxy_file}")
    
    if concurrency > 0:
        with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as executor:
            for i in range(concurrency):
                executor.submit(worker, i + 1)
                # 错开启动时间，避免瞬间打满并发
                time.sleep(random.randint(2, 5))
    else:
        # Allow running repairer/probe-only mode without starting register workers.
        while True:
            time.sleep(3600)


