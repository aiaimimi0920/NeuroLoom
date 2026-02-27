from __future__ import annotations

import base64
import hashlib
import secrets
import urllib.parse
import urllib.request
from dataclasses import dataclass
from typing import Any, Dict
import undetected_chromedriver as uc
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as EC
from selenium.common.exceptions import TimeoutException
from selenium.webdriver.common.keys import Keys
from selenium.webdriver.common.action_chains import ActionChains
import time
import random
import string
import os
import re
import json
from urllib.parse import urlparse, parse_qs
from urllib import request
import tempfile
import shutil
import concurrent.futures
import threading

write_lock = threading.Lock()
driver_init_lock = threading.Lock()

class APIKeyManager:
    def __init__(self, keys_file="data/api_keys.txt"):
        self.public_key = "gpt-test"
        self.keys_file = keys_file
        self.private_keys = []
        self.current_private_idx = 0
        self.lock = threading.Lock()
        self.public_key_disabled_until = 0

    def load_private_keys(self):
        try:
            if os.path.exists(self.keys_file):
                with open(self.keys_file, "r", encoding="utf-8") as f:
                    keys = []
                    for line in f:
                        if line.strip() and "[exhausted]" not in line.lower():
                            key = line.split()[0].strip()
                            keys.append(key)
                    self.private_keys = keys
        except Exception:
            pass

    def get_key(self):
        while True:
            with self.lock:
                self.load_private_keys()
                now_ts = time.time()
                
                # Option 1: Public key is off cooldown
                if now_ts > self.public_key_disabled_until:
                    return self.public_key
                
                # Option 2: Private keys available
                if self.private_keys:
                    key = self.private_keys[self.current_private_idx % len(self.private_keys)]
                    self.current_private_idx += 1
                    return key
                    
            # Option 3: Everything is exhausted. 
            # Public key is on strict cooldown, and NO valid private keys remain.
            # Block the worker threads gracefully instead of spamming.
            print(f"[*] 所有API Key (包含公共Key) 均已耗尽/被限流。脚本休眠 60 秒后重检查 (可随时在 api_keys.txt 补充新Key)...")
            time.sleep(60)

    def mark_key_failed(self, key):
        with self.lock:
            if key == self.public_key:
                # 遇到429，禁用公共Key 1小时，期待整点或早8点刷新
                self.public_key_disabled_until = time.time() + 3600
                print(f"[*] 公共API Key ({key}) 触发限流，进入1小时深度冷却...")
            else:
                print(f"[*] 私有API Key ({key}) 请求失败，可能已耗尽额度，标注为已用尽。")
                try:
                    if os.path.exists(self.keys_file):
                        with open(self.keys_file, "r", encoding="utf-8") as f:
                            lines = f.readlines()
                        with open(self.keys_file, "w", encoding="utf-8") as f:
                            for line in lines:
                                if line.startswith(key) and "[exhausted]" not in line.lower():
                                    f.write(line.rstrip() + " # [EXHAUSTED]\n")
                                else:
                                    f.write(line)
                except Exception as e:
                    print(f"Error updating api_keys.txt: {e}")
key_manager = APIKeyManager()


def post(url: str, body: str, header: dict, proxy: str | None=None) -> tuple[str,dict]:
    data = body.encode("utf-8")
    req = urllib.request.Request(url, data=data, headers=header, method="POST") 
    with get_opener(proxy).open(req) as resp: 
        resp_text = resp.read().decode("utf-8")
        resp_headers = dict(resp.headers)
        return resp_text, resp_headers

def put(url: str, body: str, header: dict, proxy: str | None=None) -> tuple[str,dict]:
    data = body.encode("utf-8")
    req = urllib.request.Request(url, data=data, headers=header, method="PUT") 
    with get_opener(proxy).open(req) as resp: 
        resp_text = resp.read().decode("utf-8")
        resp_headers = dict(resp.headers)
        return resp_text, resp_headers

def get(url: str, headers: dict | None=None, proxy: str | None=None) -> tuple[str, dict]:
    for i in range(5):
        try: 
            req = urllib.request.Request(url, headers = headers or {})
            with get_opener(proxy).open(req) as response:
                resp_text = response.read().decode("utf-8")
                resp_headers = dict(response.getheaders())
                return resp_text, resp_headers
        except urllib.error.HTTPError as e:
            if e.code in (401, 429):
                raise # immediately bubble up for API key rotation
            delay = random.uniform(5, 10) + (i * 2)
            print(f"GET Request HTTPError: {e.code} for {url} - Retrying in {delay:.1f}s")
            time.sleep(delay)
        except Exception as e: 
            delay = random.uniform(5, 10) + (i * 2)
            print(f"GET Request error: {e} - Retrying in {delay:.1f}s")
            time.sleep(delay)
    raise RuntimeError(f"Failed to GET {url} after retries")

def get_email(proxy: str | None = None) -> str:
    for _ in range(10):
        api_key = key_manager.get_key()
        try:
            body, _ = get("https://mail.chatgpt.org.uk/api/generate-email", {"X-API-Key": api_key, "User-Agent": "Mozilla/5.0"}, proxy)
            data = json.loads(body)
            if not data.get("success"):
                err = data.get("error", "")
                if "quota" in err.lower() or "limit" in err.lower():
                    key_manager.mark_key_failed(api_key)
                    continue
            return data["data"]["email"]
        except urllib.error.HTTPError as e:
            if e.code in (401, 429):
                key_manager.mark_key_failed(api_key)
                continue
            raise
    raise RuntimeError("Failed to generate email after rotating keys")

def get_oai_code(email: str, proxy: str | None = None) -> str:
    regex = r" (?<!\d)(\d{6})(?!\d)" #r"(?<!\d)\d{6}(?!\d)"
    for _ in range(30):
        time.sleep(4) # Prevent 429 Too Many Requests
        api_key = key_manager.get_key()
        try:
            body,_ = get(f"https://mail.chatgpt.org.uk/api/emails?email={email}", {"X-API-Key": api_key, "User-Agent": "Mozilla/5.0"}, proxy)
            data = json.loads(body)
            if not data.get("success"):
                err = data.get("error", "")
                if "quota" in err.lower() or "limit" in err.lower():
                    key_manager.mark_key_failed(api_key)
                continue
                
            emails = data.get("data", {}).get("emails", [])
            for e in emails:
                if "openai" in e.get("from_address", ""):
                    m = re.search(regex, e.get("subject", ""))
                    if m:
                        return m.group(1)
                    m = re.search(regex, e.get("html_content", ""))
                    if m:
                        return m.group(1)
        except urllib.error.HTTPError as err:
            if err.code == 429:
                key_manager.mark_key_failed(api_key)
                time.sleep(2)
            else:
                print(f"Error fetching OAI code: HTTP {err.code}")
        except Exception as err:
            print(f"Error fetching OAI code: {err}")
    raise RuntimeError("Timeout waiting for OAI code email")
        

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
    for _ in range(4):
        try:
            with get_opener(proxy).open(req, timeout=timeout) as resp:
                raw = resp.read()
                if resp.status != 200:
                    raise RuntimeError(
                        f"token exchange failed: {resp.status}: {raw.decode('utf-8', 'replace')}"
                    )
                return json.loads(raw.decode("utf-8"))
        except urllib.error.HTTPError as exc:
            raw = exc.read()
            raise RuntimeError(
                f"token exchange failed: {exc.code}: {raw.decode('utf-8', 'replace')}"
            ) from exc
        except Exception as e:
            print(f"POST Request error: {e}")
            time.sleep(2)
            
    raise RuntimeError("Failed to post form after max retries")


@dataclass(frozen=True)
class OAuthStart:
    auth_url: str
    state: str
    code_verifier: str
    redirect_uri: str


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
        "password": password,
        "birthdate": birthdate,
        "client_id": CLIENT_ID,
        "last_name": last_name,
        "account_id": account_id,
        "first_name": first_name,
        "session_id": claims.get("session_id", ""),
        "access_token": access_token,
        "last_refresh": now_rfc3339,
        "pwd_auth_time": claims.get("pwd_auth_time", int(time.time() * 1000)),
        "https://api.openai.com/auth": auth_claims,
        "https://api.openai.com/profile": claims.get("https://api.openai.com/profile", {})
    })

    return email, json.dumps(config, ensure_ascii=False, separators=(",", ":"))


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
              bypassList: ["localhost"]
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

def new_driver(proxy: str | None = None):
    options = Options()
    options.add_argument('--headless')
    options.add_argument('--no-sandbox')
    options.add_argument('--disable-dev-shm-usage')
    options.add_argument('--disable-gpu')
    options.add_argument('--window-size=1920,1080')
    options.add_argument('--user-agent=Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36')
    options.add_argument('--enable-features=NetworkService,NetworkServiceInProcess')
    
    # Disable background telemetry and optimization guide to save proxy traffic
    options.add_argument('--disable-features=OptimizationGuideModelDownloading,OptimizationHintsFetching,OptimizationTargetPrediction,OptimizationGuideModelExecution')
    options.add_argument('--disable-background-networking')
    options.add_argument('--disable-sync')
    options.add_argument('--disable-component-update')
    
    # Aggressively blackhole known telemetry and video domains to prevent proxy drain
    host_rules = (
        "MAP optimizationguide-pa.googleapis.com 127.0.0.1, "
        "MAP update.googleapis.com 127.0.0.1, "
        "MAP browser-intake-datadoghq.com 127.0.0.1, "
        "MAP *.gvt1.com 127.0.0.1, "
        "MAP *.cloudflarestream.com 127.0.0.1"
    )
    options.add_argument(f'--host-resolver-rules={host_rules}')
    
    # Anti-detect features for standard selenium
    options.add_argument('--disable-blink-features=AutomationControlled')
    options.add_experimental_option("excludeSwitches", ["enable-automation"])
    options.add_experimental_option('useAutomationExtension', False)
    
    block_images = int(os.environ.get("BLOCK_IMAGES", "1"))
    block_css = int(os.environ.get("BLOCK_CSS", "1"))
    block_fonts = int(os.environ.get("BLOCK_FONTS", "1"))
    
    prefs = {}
    if block_images == 2:
        prefs["profile.managed_default_content_settings.images"] = 2
        options.add_argument('--blink-settings=imagesEnabled=false')
    if block_css == 2:
        prefs["profile.managed_default_content_settings.stylesheet"] = 2
    if block_fonts == 2:
        prefs["profile.managed_default_content_settings.fonts"] = 2

    if prefs:
        print(f"Traffic Saver Mode Active: Images={block_images==2}, CSS={block_css==2}, Fonts={block_fonts==2}")
        options.add_experimental_option("prefs", prefs)
    
    proxy_dir = None
    if proxy and "@" in proxy:
        proxy_dir = create_proxy_extension(proxy)
        if proxy_dir:
            options.add_argument(f"--load-extension={proxy_dir}")
            options.add_argument(f"--disable-extensions-except={proxy_dir}")
    elif proxy:
        options.add_argument(f'--proxy-server={proxy}')
        
    options.add_argument('--log-level=3')
    options.add_argument('--disable-crash-reporter')
    options.add_argument('--disable-in-process-stack-traces')
    options.page_load_strategy = 'eager' # Don't wait for all resources to download
    
    service = Service()
    driver = webdriver.Chrome(service=service, options=options)
    
    # Execute CDP command to hide webdriver property
    driver.execute_cdp_cmd("Page.addScriptToEvaluateOnNewDocument", {
        "source": """
            Object.defineProperty(navigator, 'webdriver', {
                get: () => undefined
            })
        """
    })
    
    return driver, proxy_dir

def generate_name() -> tuple[str, str]:
    first = ["Neo", "John", "Sarah", "Michael", "Emma", "David", "James", "Robert", "Mary", "William", "Richard", "Thomas", "Charles", "Christopher", "Daniel", "Matthew", "Anthony", "Mark", "Donald", "Steven", "Paul", "Andrew", "Joshua", "Kenneth", "Kevin", "Brian", "George", "Edward", "Ronald", "Timothy"]
    last = ["Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis", "Rodriguez", "Martinez", "Hernandez", "Lopez", "Gonzalez", "Wilson", "Anderson", "Thomas", "Taylor", "Moore", "Jackson", "Martin", "Lee", "Perez", "Thompson", "White"]
    return random.choice(first), random.choice(last)

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

def smart_wait(driver, by, value, timeout=20):
    """
    Waits for an element while simultaneously checking for OpenAI's 
    'Oops, an error occurred!' -> 'Try again' button overlay.
    """
    end_time = time.time() + timeout
    while time.time() < end_time:
        try:
            # Check for the "Try again" button and click it if it appears
            try_again_btns = driver.find_elements(By.XPATH, "//button[contains(translate(., 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'try again')]")
            if try_again_btns and try_again_btns[0].is_displayed():
                print("[*] Detected 'Oops, an error occurred!' page. Clicking 'Try again'...")
                driver.execute_script("arguments[0].click();", try_again_btns[0])
                time.sleep(2) # Wait for page to reload/recover
                continue
                
            # Check for the actual target element
            el = driver.find_element(by, value)
            if el.is_displayed() and el.is_enabled():
                return el
        except Exception:
            pass
        time.sleep(0.5)
    raise TimeoutException(f"Timeout waiting for {by}={value}")

def register(driver, proxy=None) -> tuple[str, str]:
    email = get_email(proxy)
    print("Email obtained:", email)
    oauth = generate_oauth_url()
    url = oauth.auth_url
    print("OAuth URL:", url)
    driver.get(url)
    
    WebDriverWait(driver, 60).until(EC.url_contains("auth.openai.com"))
    print("Reach oai sign up page")
    try:
        sign_up_button = smart_wait(driver, By.XPATH, "//*[normalize-space()='Sign up']", timeout=20)
        sign_up_button.click()
        print("Sign up clicked")
    except TimeoutException:
        print("Timeout waiting for sign up button. Capturing screenshot...")
        driver.save_screenshot(f"data/error_signup_{int(time.time())}.png")
        raise
    
    email_input = smart_wait(driver, By.ID, "_r_f_-email", timeout=20)
    email_input.clear()
    print("Reach email input")
    for char in email:
        email_input.send_keys(char)
        time.sleep(0.01)
    email_input.send_keys(Keys.ENTER)    
    print("Enter pressed")
    
    try:
        pwd_input = smart_wait(driver, By.ID, "_r_u_-new-password", timeout=30)
        print("Reach password input")
        pwd = generate_pwd()
        for char in pwd:
            pwd_input.send_keys(char)
            time.sleep(0.005)
        pwd_input.send_keys(Keys.ENTER)
        print("Enter pressed")
    except TimeoutException:
        print("Timeout waiting for password input. This is usually due to a Captcha or blocked email. Capturing screenshot...")
        driver.save_screenshot(f"data/error_pwd_{int(time.time())}.png")
        raise RuntimeError("Blocked: Password field did not appear (Captcha/Risk Flag).")
    
    code = get_oai_code(email, proxy)
    print("Verification Code:", code)
    try:
        code_input = smart_wait(driver, By.ID, "_r_4_-code", timeout=10)
        print("Reach code input")
        for char in code:
            code_input.send_keys(char)
            time.sleep(0.02)
        code_input.send_keys(Keys.ENTER)
        print("Enter pressed")
    except TimeoutException:
        print("Reach new code input")
        code_inputs = WebDriverWait(driver, 10).until(
            lambda d: d.find_elements(
                By.CSS_SELECTOR,
                'div[role="group"] input[inputmode="numeric"][maxlength="1"]'
            )
        )
        for current, digit in zip(code_inputs, code):
            WebDriverWait(driver, 1).until(EC.element_to_be_clickable(current))
            current.click()
            current.clear()
            current.send_keys(digit)
        driver.switch_to.active_element().send_keys(Keys.ENTER)    
        
    first_name, last_name = generate_name()
    full_name_str = first_name + " " + last_name
    
    print("Trying to fill Name and Birthday via explicit focus-loop v3")
    explicit_form_detected = False
    try:
        # Initial settle wait
        time.sleep(10)
        
        visible_inputs = []
        for _ in range(5): # Re-try loop to find BOTH inputs
            visible_inputs = driver.execute_script("""
                var inputs = document.querySelectorAll('input:not([type="hidden"])');
                var visible = [];
                for(var i=0; i<inputs.length; i++) {
                    var rect = inputs[i].getBoundingClientRect();
                    if(rect.width > 0 && rect.height > 0 && window.getComputedStyle(inputs[i]).visibility !== 'hidden') {
                        visible.push(inputs[i]);
                    }
                }
                return visible;
            """)
            if visible_inputs and len(visible_inputs) >= 2:
                break
            time.sleep(2)
        
        if visible_inputs and len(visible_inputs) >= 2:
            explicit_form_detected = True
            print(f"Detected explicit form with {len(visible_inputs)} inputs.")
            
            # 1. Name
            name_input = visible_inputs[0]
            driver.execute_script("arguments[0].scrollIntoView({block: 'center'});", name_input)
            time.sleep(0.5)
            name_input.click()
            time.sleep(0.5)
            
            # Use JS to robustly set value, bypassing React input loss
            driver.execute_script('''
                var input = arguments[0];
                var val = arguments[1];
                var nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set;
                if(nativeInputValueSetter) { nativeInputValueSetter.call(input, val); }
                input.value = val;
                input.dispatchEvent(new Event('input', { bubbles: true }));
            ''', name_input, full_name_str)
            
            time.sleep(0.5)
            name_input.send_keys(Keys.TAB)
            time.sleep(1.0) # Wait for potential side-effects
            
            # 2. Birthday
            # Re-fetch because DOM might refresh after name blur
            visible_inputs = driver.execute_script("return Array.from(document.querySelectorAll('input:not([type=\\\"hidden\\\"])')).filter(i => {var r=i.getBoundingClientRect(); return r.width > 0 && r.height > 0;});")
            if len(visible_inputs) >= 2:
                bday_input = visible_inputs[1]
                driver.execute_script("arguments[0].scrollIntoView({block: 'center'});", bday_input)
                time.sleep(0.5)
                bday_input.click()
                time.sleep(0.5)
                
                bday_input.send_keys(Keys.CONTROL + "a")
                bday_input.send_keys(Keys.BACKSPACE)
                for _ in range(5): bday_input.send_keys(Keys.DELETE)
                time.sleep(0.3)
                
                # Type slower for masked date
                for char in "01151995":
                    bday_input.send_keys(char)
                    time.sleep(0.12)
                time.sleep(0.5)
                
                # Check if it was filled correctly. If not, use JS setter.
                current_val = bday_input.get_attribute('value')
                if not current_val or len(current_val) < 8 or "202" in current_val:
                    print("Birthday input did not format correctly, using JS React setter.")
                    driver.execute_script('''
                        var input = arguments[0];
                        var nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set;
                        if(nativeInputValueSetter) { nativeInputValueSetter.call(input, "01/15/1995"); }
                        input.value = "01/15/1995";
                        input.dispatchEvent(new Event('input', { bubbles: true }));
                    ''', bday_input)
                    time.sleep(0.5)
                
                bday_input.send_keys(Keys.TAB)
                time.sleep(0.5)
                
        elif visible_inputs and len(visible_inputs) == 1:
            print("Only one visible input found, treating as normal Name field.")
            name_input = visible_inputs[0]
            name_input.click()
            active = driver.switch_to.active_element
            active.send_keys(Keys.CONTROL + "a")
            active.send_keys(Keys.BACKSPACE)
            for char in full_name_str:
                active.send_keys(char)
                time.sleep(0.01)
            active.send_keys(Keys.TAB)
    except Exception as e:
        print(f"Name/Birthday filling error: {e}")
        
    print("Reach birthday input")
    
    birthdate = "1995-01-15" if explicit_form_detected else "2000-01-01"
    if explicit_form_detected:
        try:
            # Aggressive error-checking loop for React validation failures
            for attempt in range(3):
                time.sleep(1.5)
                page_src = driver.page_source
                error_detected = False
                
                # Check for "Name" validation error
                if "Please enter name" in page_src or "doesn't look right" in page_src:
                    print(f"[Retry {attempt+1}] Name validation error detected, attempting re-fill...")
                    driver.execute_script("document.querySelectorAll('input:not([type=\"hidden\"])')[0].focus();")
                    active = driver.switch_to.active_element
                    active.send_keys(Keys.CONTROL + "a")
                    active.send_keys(Keys.BACKSPACE)
                    for char in "John Smith": 
                        active.send_keys(char)
                        time.sleep(0.05)
                    active.send_keys(Keys.TAB)
                    error_detected = True
                    
                # Check for "Birthday" validation error
                if "We can't create an account with that info" in page_src or "Please enter a valid date" in page_src:
                    print(f"[Retry {attempt+1}] Birthday validation error detected, attempting manual slow re-fill...")
                    inputs = driver.execute_script("return document.querySelectorAll('input:not([type=\"hidden\"])');")
                    if len(inputs) >= 2:
                        driver.execute_script("arguments[0].focus();", inputs[1])
                        active = driver.switch_to.active_element
                        active.send_keys(Keys.HOME)
                        active.send_keys(Keys.CONTROL + "a")
                        active.send_keys(Keys.BACKSPACE)
                        # Completely clear the mask
                        for _ in range(15): 
                            active.send_keys(Keys.BACKSPACE)
                            active.send_keys(Keys.DELETE)
                        time.sleep(0.5)
                        # Super slow physical typing
                        for char in "01151995":
                            active.send_keys(char)
                            time.sleep(0.2)
                        active.send_keys(Keys.TAB)
                        error_detected = True
                
                if not error_detected:
                    break # Form is clean, proceed to click Continue
                    
        except Exception as e:
            print(f"Error during validation retry loop: {e}")
    else:
        birthdate = enter_birthday(driver)
    
    try:
        # Final confirmation page click
        continue_button = smart_wait(driver, By.XPATH, '//button[contains(., "Agree") or contains(translate(., "ABCDEFGHIJKLMNOPQRSTUVWXYZ", "abcdefghijklmnopqrstuvwxyz"), "continue")]', timeout=10)
        print("Clicking continue/agree button")
        try:
            continue_button.click()
            time.sleep(1)
        except Exception:
            driver.execute_script("arguments[0].click();", continue_button)
    except TimeoutException:
        print("Continue button missing, hitting ENTER")
        try:
            driver.switch_to.active_element.send_keys(Keys.ENTER)
        except Exception:
            pass
        
    try:
        WebDriverWait(driver, 60).until(EC.url_contains("localhost:1455"))
    except TimeoutException:
        print("Timeout waiting for callback URL. Capturing screenshot...")
        driver.save_screenshot(f"data/error_callback_{int(time.time())}.png")
        raise RuntimeError("Blocked: Timeout waiting for callback URL to localhost.")
        
    callback_url = driver.current_url
    print("Success Callback URL Captured.")
    
    reg_email, call_back = submit_callback_url(
        callback_url=callback_url, 
        expected_state=oauth.state, 
        code_verifier=oauth.code_verifier, 
        redirect_uri=oauth.redirect_uri,
        proxy=proxy,
        password=pwd,
        first_name=first_name,
        last_name=last_name,
        birthdate=birthdate
    )
    return reg_email, call_back

def load_proxies() -> list[str]:
    proxy_file = "data/proxies.txt"
    if os.path.exists(proxy_file):
        with open(proxy_file, "r", encoding="utf-8") as f:
            proxies = [line.strip() for line in f if line.strip() and not line.startswith("#")]
        return proxies
    return []

def worker(worker_id: int):
    results_file = "data/results.jsonl"
    while True:
        proxies = load_proxies()
        proxy = random.choice(proxies) if proxies else None
        
        if proxy:
            print(f"[Worker {worker_id}] ---> 使用代理: {proxy} <---")
        else:
            print(f"[Worker {worker_id}] ---> 未配置可用代理，使用本地网络直连 <---")
            
        driver = None
        proxy_dir = None
        try:
            with driver_init_lock:
                driver, proxy_dir = new_driver(proxy)
            reg_email, res = register(driver, proxy)
            
            # Write to overall file
            with write_lock:
                with open(results_file, "a", encoding="utf-8") as f:
                    f.write(res + "\n")
                
                # Write to split folders
                os.makedirs("data/cli_codex", exist_ok=True)
                with open(f"data/cli_codex/codex-{reg_email}-free.json", "w", encoding="utf-8") as f:
                    f.write(json.dumps(json.loads(res), indent=2, ensure_ascii=False))
                    
            print(f"[Worker {worker_id}] [✓] 注册成功，Token 已保存在 cli_codex 中并追加到记录文件！")
            
        except RuntimeError as e:
            # Expected blocks, no stack trace needed
            print(f"[Worker {worker_id}] [x] {e} (准备换IP重试)")
        except TimeoutException as e:
            print(f"[Worker {worker_id}] [x] 页面加载超时，可能遇到风控盾拦截。 (准备换IP重试)")
        except Exception as e:
            err_str = str(e)
            if "RemoteDisconnected" in err_str or "Connection aborted" in err_str or "Max retries exceeded" in err_str or "UNEXPECTED_EOF_WHILE_READING" in err_str or "UNEXPECTED_MESSAGE" in err_str:
                print(f"[Worker {worker_id}] [x] 代理连接强制中断 (SSL/EOF断流)，准备换IP重试")
            else:
                import traceback
                trace_str = traceback.format_exc()
                print(f"[Worker {worker_id}] [x] 本次注册流程意外中止:\\n{trace_str}")
            
        finally:
            if driver:
                try:
                    driver.quit()
                except Exception:
                    pass
            if proxy_dir and os.path.exists(proxy_dir):
                shutil.rmtree(proxy_dir, ignore_errors=True)
        
        # 自由调整休眠时间
        sleep_min = int(os.environ.get("SLEEP_MIN", "5"))
        sleep_max = int(os.environ.get("SLEEP_MAX", "20"))
        sleep_time = random.randint(sleep_min, sleep_max) if sleep_max >= sleep_min else sleep_min
        print(f"[Worker {worker_id}] 任务结束。挂起 {sleep_time} 秒后开启下一轮尝试...")
        time.sleep(sleep_time)

if __name__ == "__main__":
    os.makedirs("data", exist_ok=True)
    results_file = "data/results.jsonl"
    proxy_file = "data/proxies.txt"
    
    if not os.path.exists(proxy_file):
        with open(proxy_file, "w", encoding="utf-8") as f:
            f.write("# 在此文件中添加您的代理IP池，每行一个\n")
            f.write("# 格式示例: http://192.168.1.100:8080\n")
            
    concurrency = int(os.environ.get("CONCURRENCY", "1"))
    
    print(f"==== 守护进程启动: 无限循环多线程生成器 (并发数: {concurrency}) ====")
    print(f"数据将实时追加保存在 {results_file}")
    print(f"代理池请直接写入 {proxy_file}")
    
    with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as executor:
        for i in range(concurrency):
            executor.submit(worker, i+1)
            # 错开启动时间，避免瞬间打满并发
            time.sleep(random.randint(2, 5))

