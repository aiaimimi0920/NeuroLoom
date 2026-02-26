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


def post(url: str, body: str, header: dict) -> tuple[str,dict]:
    data = body.encode("utf-8")
    req = urllib.request.Request(url, data=data, headers=header, method="POST") 
    with urllib.request.urlopen(req) as resp: 
        resp_text = resp.read().decode("utf-8")
        resp_headers = dict(resp.headers)
        return resp_text, resp_headers

def put(url: str, body: str, header: dict) -> tuple[str,dict]:
    data = body.encode("utf-8")
    req = urllib.request.Request(url, data=data, headers=header, method="PUT") 
    with urllib.request.urlopen(req) as resp: 
        resp_text = resp.read().decode("utf-8")
        resp_headers = dict(resp.headers)
        return resp_text, resp_headers

def get(url: str, headers: dict | None=None) -> tuple[str, dict]:
    # res, header
    try: 
        req = urllib.request.Request(url, headers = headers or {})
        with urllib.request.urlopen(req) as response:
            resp_text = response.read().decode("utf-8")
            resp_headers = dict(response.getheaders())
            return resp_text, resp_headers
    except Exception as e: 
            print(e)
            return -1, {}

def get_email() -> str:
    body, _ = get("https://mail.chatgpt.org.uk/api/generate-email", {"X-API-Key": "gpt-test", "User-Agent": "Mozilla/5.0"})
    data = json.loads(body)
    return data["data"]["email"]

def get_oai_code(email: str) -> str:
    regex = r" (?<!\d)(\d{6})(?!\d)" #r"(?<!\d)\d{6}(?!\d)"
    while 1:
        body,_ = get(f"https://mail.chatgpt.org.uk/api/emails?email={email}", {"X-API-Key": "gpt-test", "User-Agent": "Mozilla/5.0"})
        data = json.loads(body)
        emails = data["data"]["emails"]
        for email in emails:
            if "openai" in email["from_address"]:
                m = re.search(regex, email["subject"])
                if m:
                    return m.group(1)
                m = re.search(regex, email["html_content"])
                return m.group(1)
            else:
                continue
        

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


def _post_form(url: str, data: Dict[str, str], timeout: int = 30) -> Dict[str, Any]:
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
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
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
) -> str:
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

    # Match this repo's codex auth file schema.
    config = {
        "id_token": id_token,
        "access_token": access_token,
        "refresh_token": refresh_token,
        "account_id": account_id,
        "last_refresh": now_rfc3339,
        "email": email,
        "type": "codex",
        "expired": expired_rfc3339,
    }

    return json.dumps(config, ensure_ascii=False, separators=(",", ":"))


def new_driver() :
    options = uc.ChromeOptions()
    options.add_argument('--headless')
    options.add_argument('--no-sandbox')
    options.add_argument('--disable-dev-shm-usage')
    options.add_argument('--disable-gpu')
    options.add_argument('--window-size=1920,1080')
    options.add_argument('--user-agent=Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36')
    
    # 强制与最新安装的 Chrome 145 版本进行匹配
    driver = uc.Chrome(options=options, use_subprocess=True, version_main=145)
    return driver

def generate_name() -> str:
    return "Neo"

def enter_birthday(driver) -> None:
    birthday_input = driver.switch_to.active_element
    birthday_input.send_keys("1")
    birthday_input.send_keys(Keys.TAB)
    birthday_input = driver.switch_to.active_element
    birthday_input.send_keys("1")
    birthday_input.send_keys(Keys.TAB)
    birthday_input = driver.switch_to.active_element
    birthday_input.send_keys("2000")
    birthday_input.send_keys(Keys.ENTER)


def register(driver) -> str:
    email = get_email()
    print(email)
    oauth = generate_oauth_url()
    url = oauth.auth_url
    print(url)
    driver.get(url)
    WebDriverWait(driver, 60).until(EC.url_contains("auth.openai.com"))
    print("Reach oai sign up page")
    sign_up_button = WebDriverWait(driver, 20).until(
        EC.element_to_be_clickable((By.XPATH, "//*[normalize-space()='Sign up']"))
    )
    sign_up_button.click()
    print("Sign up clicked")
    email_input = WebDriverWait(driver, 20).until(
        EC.visibility_of_element_located((By.ID, "_r_f_-email"))
    )
    email_input.clear()
    print("Reach email input")
    for char in email:
        email_input.send_keys(char)
        time.sleep(0.01)
    email_input.send_keys(Keys.ENTER)    
    print("Enter pressed")
    pwd_input = WebDriverWait(driver, 20).until(
        EC.visibility_of_element_located((By.ID, "_r_u_-new-password"))
    )
    print("Reach password input")
    for i in range(12):
        pwd_input.send_keys("a")
        time.sleep(0.005)
    pwd_input.send_keys(Keys.ENTER)
    print("Enter pressed")
    code = get_oai_code(email)
    print(code)
    try:
        code_input = WebDriverWait(driver, 5).until(
            EC.visibility_of_element_located((By.ID, "_r_4_-code"))
        )
        print("Reach code input")
        for char in code:
            code_input.send_keys(char)
            time.sleep(0.02)
        code_input.send_keys(Keys.ENTER)
        print("Enter pressed")
    except TimeoutException:
        #OAI new login page
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
    name = generate_name()
    name_input = WebDriverWait(driver, 20).until(
        EC.visibility_of_element_located((By.ID, "_r_g_-name"))
    )
    for char in name:
        name_input.send_keys(char)
        time.sleep(0.02)
    name_input.send_keys(Keys.TAB)
    print("Reach birthday input")
    birthday_input = driver.switch_to.active_element
    print(birthday_input.get_attribute("id"))
    enter_birthday(driver)
    continue_button = WebDriverWait(driver, 20).until(
        EC.element_to_be_clickable((By.CSS_SELECTOR, 'button[data-dd-action-name="Continue"]')) #EC.element_to_be_clickable((By.XPATH, "//*[normalize-space()='Continue']"))
    )
    print("Waiting for callback URL")
    continue_button.click()
    WebDriverWait(driver, 60).until(EC.url_contains("localhost:1455"))
    callback_url = driver.current_url
    print(callback_url)
    call_back = submit_callback_url(callback_url=callback_url, expected_state=oauth.state, code_verifier=oauth.code_verifier, redirect_uri=oauth.redirect_uri)
    print(call_back)
    return call_back


if __name__ == "__main__":
    driver = new_driver()
    results = []
    # 如果输出文件不存在，或者是首次运行，清空或准备文件
    for i in range(3):
        try:
            res = register(driver)
            results.append(res)
            print(f"[{i+1}/3] Success")
        except Exception as e:
            print(f"[{i+1}/3] Failed: {e}")
            continue
    driver.quit()
    
    # 将包含Token结果存储到容器的挂载目录(此处为工作目录的 results.txt )
    os.makedirs("data", exist_ok=True)
    with open("data/results.txt", "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2)
    
    print("All done. Results saved to data/results.txt.")
