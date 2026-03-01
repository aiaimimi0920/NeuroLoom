/// <reference types="@cloudflare/workers-types" />

import { AwsClient } from "aws4fetch";
import { BlobWriter, TextReader, ZipWriter } from "@zip.js/zip.js";

export interface Env {
  DB: D1Database;

  // 分发包（R2 Workers API）
  BUCKET: R2Bucket;

  // R2 presigned URL（S3 API）
  R2_ACCOUNT_ID: string;
  R2_BUCKET_NAME: string;
  R2_ACCESS_KEY_ID?: string;
  R2_SECRET_ACCESS_KEY?: string;

  /**
   * 超级管理员：拥有全部管理权限（导入 upload_key/refill_key、查看统计等）
   */
  ADMIN_TOKEN: string;

  /**
   * 超级管理员强校验：第二因子（固定短码/密钥）。
   * 要求：所有使用 ADMIN_TOKEN 的请求都必须同时提供 Header `X-Admin-Guard`。
   */
  ADMIN_GUARD?: string;

  /**
   * 超级管理员强校验：IP 白名单（支持单 IP 与 CIDR，逗号分隔）。
   * 例："1.2.3.4,5.6.7.0/24"
   */
  ADMIN_IP_WHITELIST?: string;

  /**
   * 是否启用 mine API（默认关闭）。
   * 由于它会暴露“已绑定作品”的可下载链接，建议仅在你确认需要时开启。
   */
  ENABLE_MINE_API?: string;

  /**
   * refill key 加密主密钥（base64，解码后 32 bytes）
   */
  REFILL_KEYS_MASTER_KEY_B64: string;

  /**
   * accounts.auth_json 加密主密钥（base64，解码后 32 bytes）。
   * 用途：服务端加密存储客户端上传的 auth_json（token JSON），并在 /v1/refill/topup 下发时解密。
   */
  ACCOUNTS_MASTER_KEY_B64: string;

  /**
   * 本地快速测试用固定 key（不会提交到仓库；放在 server/.dev.vars）。
   * 这些 key 会在 Worker 启动后“懒初始化”写入 D1（INSERT OR IGNORE），从而可直接用于接口鉴权。
   */
  TEST_REPAIRER_UPLOAD_KEY?: string;
  TEST_UPLOADER_UPLOAD_KEY?: string;
  TEST_USER_KEY?: string;

  SERVER_PEPPER?: string;
}

function json(obj: unknown, status = 200): Response {
  return new Response(JSON.stringify(obj, null, 2), {
    status,
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": "no-store",
    },
  });
}

async function ensureTestKeysInDb(env: Env): Promise<void> {
  const now = utcNowIso();

  const insertUpload = async (raw: string, label: string) => {
    const h = await sha256Hex(raw);
    await env.DB.prepare("INSERT OR IGNORE INTO upload_keys(key_hash,label,enabled,created_at) VALUES(?,?,1,?)")
      .bind(h, label, now)
      .run();
  };

  const insertUser = async (raw: string, label: string) => {
    const h = await sha256Hex(raw);
    await env.DB.prepare("INSERT OR IGNORE INTO user_keys(key_hash,label,enabled,created_at) VALUES(?,?,1,?)")
      .bind(h, label, now)
      .run();
  };

  // 维修者 key：复用 upload 权限（与热心群众同凭据模型）
  if (env.TEST_REPAIRER_UPLOAD_KEY && env.TEST_REPAIRER_UPLOAD_KEY.trim()) {
    await insertUpload(env.TEST_REPAIRER_UPLOAD_KEY.trim(), "test:repairer");
  }

  // 热心群众 key
  if (env.TEST_UPLOADER_UPLOAD_KEY && env.TEST_UPLOADER_UPLOAD_KEY.trim()) {
    await insertUpload(env.TEST_UPLOADER_UPLOAD_KEY.trim(), "test:uploader");
  }

  // 普通用户 key
  if (env.TEST_USER_KEY && env.TEST_USER_KEY.trim()) {
    await insertUser(env.TEST_USER_KEY.trim(), "test:user");
  }
}


async function touchClientActivity(env: Env, ctx: ClientCtx): Promise<void> {
  // 规则：只记录 user/upload 的活跃度，用于 7 天不活跃回收。
  if (ctx.role !== "user" && ctx.role !== "upload") return;

  const keyHash = ctx.role === "user" ? ctx.user_key_hash : ctx.upload_key_hash;
  if (!keyHash) return;

  const now = utcNowIso();
  await env.DB.prepare(
    "INSERT INTO client_activity(key_hash,role,last_seen_at) VALUES(?,?,?) ON CONFLICT(key_hash) DO UPDATE SET role=excluded.role, last_seen_at=excluded.last_seen_at",
  )
    .bind(keyHash, ctx.role, now)
    .run();
}

function isoDaysAgo(days: number, base = new Date()): string {
  const d = new Date(base.getTime() - days * 24 * 60 * 60 * 1000);
  return d.toISOString().replace(/\.\d{3}Z$/, "Z");
}

function isoMinutesAgo(minutes: number, base = new Date()): string {
  const d = new Date(base.getTime() - minutes * 60 * 1000);
  return d.toISOString().replace(/\.\d{3}Z$/, "Z");
}

async function reapInactiveClaimedArtworks(env: Env, days = 7): Promise<{ released: number; cutoff: string }> {
  const cutoff = isoDaysAgo(days);

  // 1) user_key 维度回收
  const u = await env.DB.prepare(
    "UPDATE artworks SET status='available', claimed_by_user_key_hash=NULL, claimed_by_upload_key_hash=NULL, claimed_at=NULL, eligible_after=NULL WHERE status='claimed' AND claimed_by_user_key_hash IS NOT NULL AND EXISTS (SELECT 1 FROM client_activity ca WHERE ca.key_hash=artworks.claimed_by_user_key_hash AND ca.last_seen_at < ?)",
  )
    .bind(cutoff)
    .run();

  // 2) upload_key 维度回收（Uploader 也可领取作品）
  const up = await env.DB.prepare(
    "UPDATE artworks SET status='available', claimed_by_user_key_hash=NULL, claimed_by_upload_key_hash=NULL, claimed_at=NULL, eligible_after=NULL WHERE status='claimed' AND claimed_by_upload_key_hash IS NOT NULL AND EXISTS (SELECT 1 FROM client_activity ca WHERE ca.key_hash=artworks.claimed_by_upload_key_hash AND ca.last_seen_at < ?)",
  )
    .bind(cutoff)
    .run();

  return { released: Number(u.meta.changes || 0) + Number(up.meta.changes || 0), cutoff };
}

function html(body: string, status = 200): Response {
  return new Response(body, {
    status,
    headers: {
      "content-type": "text/html; charset=utf-8",
      "cache-control": "no-store",
    },
  });
}

const UI_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>无限续杯 - 普通管理员上报工具</title>
  <style>
    body{font-family:system-ui,-apple-system,Segoe UI,Roboto,Helvetica,Arial; margin:16px;}
    .row{margin:10px 0;}
    input,select,button,textarea{font-size:16px; padding:8px; max-width:100%;}
    .box{border:1px solid #ddd; border-radius:8px; padding:12px;}
    .muted{color:#666; font-size:13px;}
    .ok{color:#0a7;}
    .err{color:#c00;}
    code{background:#f6f6f6; padding:2px 6px; border-radius:6px;}
  </style>
</head>
<body>
  <h2>普通管理员（UPLOAD_KEY）上报工具</h2>
  <div class="muted">
    合规版：页面只会把 <code>email_hash/account_id/status_code/time</code> 上报到本 Worker；不会把 token 上传到服务器。
  </div>

  <div class="box">
    <div class="row">
      <label>UPLOAD_KEY：<br/><input id="uploadKey" placeholder="输入 UPLOAD_KEY" style="width:420px"/></label>
    </div>

    <div class="row">
      <label>模式：
        <select id="mode">
          <option value="register">仅注册身份（不探测）</option>
          <option value="report">仅上报状态（不探测，手动填 status_code）</option>
        </select>
      </label>
    </div>

    <div class="row" id="statusRow" style="display:none;">
      <label>status_code（应用于全部文件）：
        <select id="statusCode">
          <option value="200">200 (OK)</option>
          <option value="401" selected>401 (Invalid)</option>
          <option value="429">429 (Exhausted)</option>
        </select>
      </label>
      <div class="muted">说明：浏览器无法直接请求 chatgpt.com 做探测（CORS），所以这里用“手动填入探测结果”的方式。</div>
    </div>

    <div class="row">
      <label>选择认证 JSON 文件（可多选）：<br/>
        <input id="files" type="file" multiple accept="application/json,.json" />
      </label>
    </div>

    <div class="row">
      <button id="runBtn">开始上报</button>
    </div>

    <div class="row">
      <div id="result" class="muted"></div>
      <textarea id="log" rows="10" style="width:100%;" readonly></textarea>
    </div>
  </div>

<script>
  const $ = (id)=>document.getElementById(id);

  function utcNowIso(){
    return new Date().toISOString().replace(/\.\d{3}Z$/, 'Z');
  }

  async function sha256Hex(str){
    const buf = new TextEncoder().encode(str);
    const dig = await crypto.subtle.digest('SHA-256', buf);
    const bytes = new Uint8Array(dig);
    return Array.from(bytes).map(b=>b.toString(16).padStart(2,'0')).join('');
  }

  function inferEmailFromFilename(name){
    const base = name.toLowerCase().endsWith('.json') ? name.slice(0,-5) : name;
    return base.includes('@') ? base : '';
  }

  async function postJson(path, uploadKey, payload){
    const res = await fetch(path, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Upload-Key': uploadKey,
      },
      body: JSON.stringify(payload),
    });
    const text = await res.text();
    return { status: res.status, text };
  }

  $('mode').addEventListener('change', ()=>{
    $('statusRow').style.display = $('mode').value === 'report' ? '' : 'none';
  });

  $('runBtn').addEventListener('click', async ()=>{
    const uploadKey = $('uploadKey').value.trim();
    if(!uploadKey){
      alert('请先输入 UPLOAD_KEY');
      return;
    }

    const files = Array.from($('files').files || []);
    if(files.length === 0){
      alert('请选择至少一个 JSON 文件');
      return;
    }

    $('log').value = '';
    const mode = $('mode').value;
    const statusCode = parseInt($('statusCode').value, 10);

    let ok = 0, bad = 0;

    for(const f of files){
      try{
        const text = await f.text();
        let obj = {};
        try{ obj = JSON.parse(text); }catch{ obj = {}; }
        const email = (typeof obj.email === 'string' && obj.email.trim()) ? obj.email.trim() : inferEmailFromFilename(f.name);
        const accountId = (typeof obj.account_id === 'string' && obj.account_id.trim()) ? obj.account_id.trim() : '';

        const ident = email ? ('email:' + email.toLowerCase()) : ('account_id:' + accountId);
        const emailHash = await sha256Hex(ident);

        if(mode === 'register'){
          const payload = { accounts: [ { email_hash: emailHash, account_id: accountId || undefined, seen_at: utcNowIso() } ] };
          const r = await postJson('/v1/accounts/register', uploadKey, payload);
          $('log').value += '[register] ' + f.name + ' -> HTTP ' + r.status + '\\n' + r.text + '\\n\\n';
          if(r.status >= 200 && r.status < 300) ok++; else bad++;
        }else{
          const payload = { reports: [ { email_hash: emailHash, account_id: accountId || undefined, status_code: statusCode, probed_at: utcNowIso() } ] };
          const r = await postJson('/v1/probe-report', uploadKey, payload);
          $('log').value += '[report] ' + f.name + ' status=' + statusCode + ' -> HTTP ' + r.status + '\\n' + r.text + '\\n\\n';
          if(r.status >= 200 && r.status < 300) ok++; else bad++;
        }
      }catch(e){
        bad++;
        $('log').value += '[error] ' + f.name + ': ' + String(e) + '\\n\\n';
      }
    }

    $('result').innerHTML = '完成：<span class="ok">ok=' + ok + '</span> <span class="err">bad=' + bad + '</span> total=' + (ok+bad);
  });
</script>
</body>
</html>`;

function utcNowIso(): string {
  return new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
}

function strByteLenUtf8(s: string): number {
  return new TextEncoder().encode(s).byteLength;
}

function isTruthyEnv(v: unknown): boolean {
  const s = String(v ?? "").trim().toLowerCase();
  return s === "1" || s === "true" || s === "yes" || s === "on";
}

function genArtworkId(): string {
  const bytes = new Uint8Array(12);
  crypto.getRandomValues(bytes);
  const b64 = bytesToBase64(bytes).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
  return `a_${b64}`;
}

function scrubSensitiveKeysDeep(input: unknown): unknown {
  // 递归删除敏感 key（大小写不敏感）。
  const banned = new Set(["access_token", "refresh_token", "id_token", "password"]);

  const walk = (v: any): any => {
    if (v === null || v === undefined) return v;

    if (Array.isArray(v)) {
      return v.map(walk);
    }

    if (typeof v === "object") {
      const out: any = {};
      for (const [k, val] of Object.entries(v)) {
        const keyLc = String(k).toLowerCase();
        if (banned.has(keyLc)) continue;
        out[k] = walk(val);
      }
      return out;
    }

    return v;
  };

  return walk(input);
}

function readBearer(req: Request): string | null {
  const v = req.headers.get("authorization") || req.headers.get("Authorization");
  if (!v) return null;
  const m = v.match(/^Bearer\s+(.+)$/i);
  return m ? m[1].trim() : null;
}

function readUploadKey(req: Request): string | null {
  const v = req.headers.get("x-upload-key") || req.headers.get("X-Upload-Key");
  return v?.trim() || null;
}

function readUserKey(req: Request): string | null {
  const v = req.headers.get("x-user-key") || req.headers.get("X-User-Key");
  return v?.trim() || null;
}

function readAdminGuard(req: Request): string | null {
  const v = req.headers.get("x-admin-guard") || req.headers.get("X-Admin-Guard");
  return v?.trim() || null;
}

function getClientIp(req: Request): string | null {
  // Cloudflare 线上：CF-Connecting-IP
  const cf = req.headers.get("cf-connecting-ip") || req.headers.get("CF-Connecting-IP");
  if (cf && cf.trim()) return cf.trim();

  // 兜底：X-Forwarded-For（取第一个）
  const xff = req.headers.get("x-forwarded-for") || req.headers.get("X-Forwarded-For");
  if (xff && xff.trim()) return xff.split(",")[0].trim() || null;

  return null;
}

function isLocalhostIp(ip: string): boolean {
  return ip === "127.0.0.1" || ip === "::1";
}

function ipv4ToInt(ip: string): number | null {
  const m = ip.trim().match(/^\s*(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})\s*$/);
  if (!m) return null;
  const parts = m.slice(1).map((x) => Number(x));
  if (parts.some((n) => !Number.isFinite(n) || n < 0 || n > 255)) return null;
  // 32-bit unsigned
  return ((parts[0] << 24) | (parts[1] << 16) | (parts[2] << 8) | parts[3]) >>> 0;
}

function ipv4InCidr(ipInt: number, netInt: number, prefix: number): boolean {
  if (prefix <= 0) return true;
  if (prefix >= 32) return ipInt === netInt;
  const mask = (0xffffffff << (32 - prefix)) >>> 0;
  return (ipInt & mask) === (netInt & mask);
}

function isAdminIpAllowed(clientIp: string, whitelist: string): boolean {
  const wl = String(whitelist || "").trim();
  if (!wl) return false;

  // 仅实现 IPv4（当前需求给的也是 IPv4/CIDR）；IPv6 需要时再加
  const ipInt = ipv4ToInt(clientIp);
  if (ipInt === null) return false;

  const tokens = wl.split(",").map((s) => s.trim()).filter(Boolean);
  for (const t of tokens) {
    const cidr = t.match(/^(.+?)\/(\d{1,2})$/);
    if (!cidr) {
      const one = ipv4ToInt(t);
      if (one !== null && one === ipInt) return true;
      continue;
    }

    const base = ipv4ToInt(cidr[1]);
    const prefix = Number(cidr[2]);
    if (base === null || !Number.isFinite(prefix) || prefix < 0 || prefix > 32) continue;
    if (ipv4InCidr(ipInt, base, prefix)) return true;
  }

  return false;
}

function base64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

function toArrayBufferView(bytes: Uint8Array): ArrayBuffer {
  // 规避 TS 对 SharedArrayBuffer/ArrayBufferLike 的不兼容推断
  return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer;
}

function bytesToBase64(bytes: Uint8Array): string {
  let s = "";
  for (let i = 0; i < bytes.length; i++) s += String.fromCharCode(bytes[i]);
  return btoa(s);
}

async function sha256Hex(s: string): Promise<string> {
  const data = new TextEncoder().encode(s);
  const digest = await crypto.subtle.digest("SHA-256", data);
  const bytes = new Uint8Array(digest);
  return [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("");
}

async function importAesKey(masterKeyB64: string): Promise<CryptoKey> {
  const raw = base64ToBytes(masterKeyB64);
  if (raw.byteLength !== 32) throw new Error("REFILL_KEYS_MASTER_KEY_B64 must decode to 32 bytes");
  return crypto.subtle.importKey("raw", toArrayBufferView(raw), { name: "AES-GCM" }, false, ["encrypt", "decrypt"]);
}

async function aesGcmEncryptToB64(masterKeyB64: string, plaintext: string): Promise<string> {
  const key = await importAesKey(masterKeyB64);
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const pt = new TextEncoder().encode(plaintext);
  const ct = new Uint8Array(await crypto.subtle.encrypt({ name: "AES-GCM", iv }, key, pt));
  // payload = iv || ct
  const payload = new Uint8Array(iv.length + ct.length);
  payload.set(iv, 0);
  payload.set(ct, iv.length);
  return bytesToBase64(payload);
}

async function aesGcmDecryptFromB64(masterKeyB64: string, payloadB64: string): Promise<string> {
  const key = await importAesKey(masterKeyB64);
  const payload = base64ToBytes(payloadB64);
  if (payload.byteLength < 13) throw new Error("bad encrypted payload");
  const iv = payload.slice(0, 12);
  const ct = payload.slice(12);
  const pt = new Uint8Array(await crypto.subtle.decrypt({ name: "AES-GCM", iv }, key, ct));
  return new TextDecoder().decode(pt);
}

function normalizeAuthJsonToString(input: unknown): string {
  if (input === null || input === undefined) return "";
  if (typeof input === "string") return input;
  return JSON.stringify(input);
}

async function accountsAuthJsonEncrypt(env: Env, authJsonPlain: string): Promise<string> {
  if (!env.ACCOUNTS_MASTER_KEY_B64 || !env.ACCOUNTS_MASTER_KEY_B64.trim()) {
    throw new Error("ACCOUNTS_MASTER_KEY_B64 not configured");
  }
  return aesGcmEncryptToB64(env.ACCOUNTS_MASTER_KEY_B64, authJsonPlain);
}

async function accountsAuthJsonDecrypt(env: Env, authJsonEncB64: string): Promise<string> {
  if (!env.ACCOUNTS_MASTER_KEY_B64 || !env.ACCOUNTS_MASTER_KEY_B64.trim()) {
    throw new Error("ACCOUNTS_MASTER_KEY_B64 not configured");
  }
  return aesGcmDecryptFromB64(env.ACCOUNTS_MASTER_KEY_B64, authJsonEncB64);
}

async function requireAdmin(req: Request, env: Env): Promise<void> {
  // 1) 必须匹配 ADMIN_TOKEN
  const tok = readBearer(req);
  if (!tok || tok !== env.ADMIN_TOKEN) throw new Response("unauthorized", { status: 401 });

  // 2) 第二因子：X-Admin-Guard
  if (!env.ADMIN_GUARD) throw new Response("admin_guard_not_configured", { status: 500 });
  const guard = readAdminGuard(req);
  if (!guard || guard !== env.ADMIN_GUARD) throw new Response("admin_guard_failed", { status: 403 });

  // 3) IP 白名单：必须命中（本地 127.0.0.1/::1 放行）
  const ip = getClientIp(req);
  if (!ip) throw new Response("missing_client_ip", { status: 403 });
  if (isLocalhostIp(ip)) return;

  const wl = env.ADMIN_IP_WHITELIST || "";
  if (!wl.trim()) throw new Response("admin_ip_whitelist_not_configured", { status: 500 });
  if (!isAdminIpAllowed(ip, wl)) throw new Response("admin_ip_not_allowed", { status: 403 });
}

type ClientRole = "user" | "upload" | "admin";

type ClientCtx = {
  role: ClientRole;
  /**
   * 用于审计/归因的哈希：
   * - user/upload: 对应 key_hash
   * - admin: admin:<sha256(ADMIN_TOKEN)>
   */
  audit_key_hash: string;
  user_key_hash?: string;
  upload_key_hash?: string;
  admin_token_hash?: string;
};

async function tryAdminCtx(req: Request, env: Env): Promise<ClientCtx | null> {
  const tok = readBearer(req);
  if (!tok || tok !== env.ADMIN_TOKEN) return null;

  // admin 视为高危凭据：必须通过强校验
  try {
    await requireAdmin(req, env);
  } catch {
    return null;
  }

  const h = await sha256Hex(tok);
  return { role: "admin", audit_key_hash: `admin:${h}`, admin_token_hash: h };
}

async function requireAtLeastUpload(req: Request, env: Env): Promise<ClientCtx> {
  const admin = await tryAdminCtx(req, env);
  if (admin) return admin;

  const u = await requireUploadKey(req, env);
  return { role: "upload", audit_key_hash: u.hash, upload_key_hash: u.hash };
}

async function requireAtLeastUser(req: Request, env: Env): Promise<ClientCtx> {
  const admin = await tryAdminCtx(req, env);
  if (admin) return admin;

  // 权限继承：UPLOAD_KEY 也拥有 USER_KEY 的全部权限
  const maybeUpload = readUploadKey(req);
  if (maybeUpload) {
    const u = await requireUploadKey(req, env);
    return { role: "upload", audit_key_hash: u.hash, upload_key_hash: u.hash };
  }

  const user = await requireUserKey(req, env);
  return { role: "user", audit_key_hash: user.hash, user_key_hash: user.hash };
}

async function requireUploadKey(req: Request, env: Env): Promise<{ raw: string; hash: string } | never> {
  const raw = readUploadKey(req);
  if (!raw) throw new Response("missing X-Upload-Key", { status: 401 });
  const h = await sha256Hex(raw);
  const row = await env.DB.prepare("SELECT enabled FROM upload_keys WHERE key_hash=?").bind(h).first<{ enabled: number }>();
  if (!row || Number(row.enabled) !== 1) throw new Response("invalid upload key", { status: 403 });
  return { raw, hash: h };
}

async function requireUserKey(req: Request, env: Env): Promise<{ raw: string; hash: string } | never> {
  const raw = readUserKey(req);
  if (!raw) throw new Response("missing X-User-Key", { status: 401 });
  const h = await sha256Hex(raw);
  const row = await env.DB.prepare("SELECT enabled FROM user_keys WHERE key_hash=?").bind(h).first<{ enabled: number }>();
  if (!row || Number(row.enabled) !== 1) throw new Response("invalid user key", { status: 403 });
  return { raw, hash: h };
}

async function parseJson<T>(req: Request): Promise<T> {
  const ct = req.headers.get("content-type") || "";
  if (!ct.toLowerCase().includes("application/json")) {
    throw new Response("expected application/json", { status: 415 });
  }
  return (await req.json()) as T;
}

type UploadKeysBody = { keys: string[]; label?: string };
type UserKeysBody = { keys: string[]; label?: string };
type RefillKeysBody = { keys: string[]; label?: string };

type IssueKeysBody = {
  type: "user" | "upload" | "refill";
  count: number;
  label?: string;
  /**
   * 仅用于策略/配额预留（不涉及任何第三方 token）。
   * 当前服务端实现会原样回显，但不强制落库。
   */
  bind_pool_size?: number;
};

type IssuePackagesBody = {
  type: "user" | "upload";
  count: number;
  label?: string;
  bind_pool_size?: number;
  server_url: string;
  ttl_minutes?: number;
};

type UploaderIssuePackagesBody = {
  // uploader 只能发 user 包（给别人用的 USER_KEY）
  count: number;
  label?: string;
  server_url: string;
  ttl_minutes?: number;
};

// --- Artworks (艺术品) ---

type SubmitArtworksBody = {
  // 单篇：直接提交一个 JSON（字段名/结构不做限制；服务端会递归删敏感 key）
  artwork?: unknown;
  // 批量：items[]
  items?: unknown[];
  label?: string;
};

type ClaimArtworksBody = {
  // 未来可扩展筛选条件；当前随机领取即可
  count?: number;
};

type DamageReportBody = {
  artwork_id: string;
  kind: "full" | "partial";
  note?: string;
};

type RepairClaimBody = {
  // 未来可扩展筛选条件；当前随机领取即可
  count?: number;
};

type RepairSubmitFixedBody = {
  artwork_id: string;
  fixed_artwork: unknown;
};

type RepairSubmitFailedBody = {
  artwork_id: string;
  note?: string;
};

type BackupDumpV1 = {
  version: 1;
  exported_at: string;
  upload_keys: Array<{ key_hash: string; label: string | null; enabled: number; created_at: string }>;
  user_keys: Array<{ key_hash: string; label: string | null; enabled: number; created_at: string }>;
  refill_keys: Array<{
    key_hash: string;
    key_enc_b64: string;
    status: string;
    claimed_by_upload_key_hash: string | null;
    claimed_by_user_key_hash: string | null;
    claimed_at: string | null;
    created_at: string;
  }>;
  /**
   * 仅导出“有效账户”（invalid=0）且不包含任何 token 字段。
   */
  accounts: Array<{
    email_hash: string;
    account_id: string | null;
    first_seen_at: string;
    last_seen_at: string;
    last_status_code: number | null;
    last_probed_at: string | null;
  }>;
};

type BackupImportBody = { dump: BackupDumpV1 };

type ProbeReportItem = {
  email_hash: string;
  account_id?: string;
  status_code?: number;
  probed_at: string;
};

type ProbeReportBody = { reports: ProbeReportItem[] };

// --- Refill (旧“账号续杯”链路：客户端探测后上报，并由服务端下发替换账号 JSON) ---

type RefillTopupReportItem = {
  file_name?: string;
  email_hash: string;
  account_id?: string;
  status_code?: number;
  probed_at: string;
};

type RefillTopupBody = {
  target_pool_size: number;
  reports: RefillTopupReportItem[];
};

type RegisterAccountItem = {
  email_hash: string;
  account_id?: string;
  seen_at: string;
  /**
   * 可选：账号 auth JSON（包含 token 等敏感字段）。
   * 仅允许 Upload/Admin 通过 /v1/accounts/register 上传；服务端会加密存储。
   */
  auth_json?: unknown;
};

type RegisterAccountsBody = { accounts: RegisterAccountItem[] };

function isHexSha256(s: string): boolean {
  return /^[a-f0-9]{64}$/i.test(s);
}

function isTruthySqliteBool(v: unknown): boolean {
  return Number(v || 0) === 1;
}

function isValidArtworkId(s: string): boolean {
  const v = String(s || "").trim();
  // 兼容两种 id：
  // 1) 旧版：genArtworkId(): `a_` + base64url(12 bytes) => 16 chars
  // 2) 新版：来自作品 JSON 的 account_id（要求只含安全字符，避免 R2 key 注入）
  if (/^a_[A-Za-z0-9_-]{16}$/.test(v)) return true;
  return /^[A-Za-z0-9_-]{3,128}$/.test(v);
}

function extractArtworkIdFromJsonArtwork(scrubbed: unknown): string | null {
  // 约定：artwork JSON 内的 account_id 作为唯一 id（即 artwork_id）
  if (!scrubbed || typeof scrubbed !== "object") return null;
  const raw = (scrubbed as any).account_id;
  if (raw === null || raw === undefined) return null;
  const v = String(raw).trim();
  if (!v) return null;
  if (!/^[A-Za-z0-9_-]{3,128}$/.test(v)) return null;
  return v;
}

export default {
  async fetch(req: Request, env: Env): Promise<Response> {
    const url = new URL(req.url);
    const path = url.pathname;

    // 本地测试 key 懒初始化：把 server/.dev.vars 里的固定 key 写入 D1（只写 hash，不存明文）。
    await ensureTestKeysInDb(env);

    // 新规则：7 天不活跃自动解绑 claimed 艺术品（懒回收）。
    // - 对所有“成功鉴权为 user/upload”的请求做 last_seen 记录
    // - 在 artworks API 访问时触发一次回收（可被任意请求触发，不要求同一个 key）

    try {
      const activityCtx = await requireAtLeastUser(req, env);
      await touchClientActivity(env, activityCtx);
    } catch {
      // 未携带可用 user/upload/admin 凭据：不记录活跃度。
    }

    if (path.startsWith("/v1/artworks")) {
      await reapInactiveClaimedArtworks(env, 7);
    }

    try {
      // health
      if (req.method === "GET" && path === "/health") {
        return json({ ok: true, ts: utcNowIso() });
      }

      // UI (PWA/纯前端)：给手机/桌面浏览器用
      if (req.method === "GET" && (path === "/" || path === "/ui" || path === "/ui/")) {
        return html(UI_HTML);
      }

      // --- Admin API ---
      if (path === "/admin/stats" && req.method === "GET") {
        await requireAdmin(req, env);

        const uploadKeys = await env.DB.prepare("SELECT COUNT(1) as c FROM upload_keys").first<{ c: number }>();
        const userKeys = await env.DB.prepare("SELECT COUNT(1) as c FROM user_keys").first<{ c: number }>();
        const refillAvail = await env.DB.prepare("SELECT COUNT(1) as c FROM refill_keys WHERE status='available'").first<{ c: number }>();
        const refillClaimed = await env.DB.prepare("SELECT COUNT(1) as c FROM refill_keys WHERE status='claimed'").first<{ c: number }>();
        const accountsTotal = await env.DB.prepare("SELECT COUNT(1) as c FROM accounts").first<{ c: number }>();
        const accountsInvalid = await env.DB.prepare("SELECT COUNT(1) as c FROM accounts WHERE invalid=1").first<{ c: number }>();
        const accountsExhausted = await env.DB.prepare("SELECT COUNT(1) as c FROM exhausted_accounts").first<{ c: number }>();
        const accountsWithAuth = await env.DB.prepare("SELECT COUNT(1) as c FROM accounts WHERE has_auth_json=1").first<{ c: number }>();
        const accountsWithAuthValid = await env.DB.prepare("SELECT COUNT(1) as c FROM accounts WHERE invalid=0 AND has_auth_json=1").first<{ c: number }>();

        const cutoff24h = isoDaysAgo(1);
        const probes24h = await env.DB.prepare("SELECT COUNT(1) as c FROM probes WHERE received_at >= ?")
          .bind(cutoff24h)
          .first<{ c: number }>();
        const topupIssued24h = await env.DB.prepare("SELECT COUNT(1) as c FROM topup_issues WHERE issued_at >= ?")
          .bind(cutoff24h)
          .first<{ c: number }>();

        return json({
          ok: true,
          upload_keys_total: Number(uploadKeys?.c || 0),
          user_keys_total: Number(userKeys?.c || 0),
          refill_keys_available: Number(refillAvail?.c || 0),
          refill_keys_claimed: Number(refillClaimed?.c || 0),
          accounts_total: Number(accountsTotal?.c || 0),
          accounts_invalid: Number(accountsInvalid?.c || 0),
          accounts_exhausted: Number(accountsExhausted?.c || 0),
          accounts_with_auth_json_total: Number(accountsWithAuth?.c || 0),
          accounts_with_auth_json_valid: Number(accountsWithAuthValid?.c || 0),
          probes_last_24h: Number(probes24h?.c || 0),
          topup_issued_last_24h: Number(topupIssued24h?.c || 0),
          ts: utcNowIso(),
        });
      }

      if (path === "/admin/keys/issue" && req.method === "POST") {
        await requireAdmin(req, env);
        const body = await parseJson<IssueKeysBody>(req);

        const type = body?.type;
        const count = Math.max(1, Math.min(200, Math.trunc(Number(body?.count || 0))));
        const label = body?.label ? String(body.label).trim() : null;
        const bindPoolSize = body?.bind_pool_size ? Math.trunc(Number(body.bind_pool_size)) : null;

        if (type !== "user" && type !== "upload" && type !== "refill") {
          return json({ ok: false, error: "invalid_type" }, 400);
        }

        const now = utcNowIso();

        // 生成一个“可分发的明文 key”。服务端只保存 hash。
        const genKey = (): string => {
          const bytes = new Uint8Array(24);
          crypto.getRandomValues(bytes);
          // url-safe base64
          const b64 = bytesToBase64(bytes).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
          return `k_${b64}`;
        };

        const keys: string[] = [];
        const errors: Array<{ idx: number; error: string }> = [];

        for (let i = 0; i < count; i++) {
          const k = genKey();
          const h = await sha256Hex(k);

          try {
            if (type === "upload") {
              await env.DB.prepare("INSERT OR IGNORE INTO upload_keys(key_hash,label,enabled,created_at) VALUES(?,?,1,?)")
                .bind(h, label, now)
                .run();
            } else if (type === "user") {
              await env.DB.prepare("INSERT OR IGNORE INTO user_keys(key_hash,label,enabled,created_at) VALUES(?,?,1,?)")
                .bind(h, label, now)
                .run();
            } else {
              // refill key 仍然走加密存储
              const enc = await aesGcmEncryptToB64(env.REFILL_KEYS_MASTER_KEY_B64, k);
              await env.DB.prepare("INSERT OR IGNORE INTO refill_keys(key_hash,key_enc_b64,status,created_at) VALUES(?,?,'available',?)")
                .bind(h, enc, now)
                .run();
            }

            keys.push(k);
          } catch (e: any) {
            errors.push({ idx: i, error: String(e?.message || e) });
          }
        }

        return json({
          ok: true,
          type,
          count_requested: count,
          count_issued: keys.length,
          label,
          bind_pool_size: bindPoolSize,
          keys,
          issued_at: now,
          errors,
        });
      }

      if (path === "/admin/packages/issue" && req.method === "POST") {
        await requireAdmin(req, env);
        const body = await parseJson<IssuePackagesBody>(req);

        const type = body?.type;
        const count = Math.max(1, Math.min(200, Math.trunc(Number(body?.count || 0))));
        const label = body?.label ? String(body.label).trim() : null;
        const bindPoolSize = body?.bind_pool_size ? Math.trunc(Number(body.bind_pool_size)) : null;
        const serverUrl = String(body?.server_url || "").trim();
        const ttlMinutes = body?.ttl_minutes ? Math.max(1, Math.min(24 * 60, Math.trunc(Number(body.ttl_minutes)))) : 60;

        if (type !== "user" && type !== "upload") {
          return json({ ok: false, error: "invalid_type" }, 400);
        }
        if (!serverUrl.startsWith("http://") && !serverUrl.startsWith("https://")) {
          return json({ ok: false, error: "invalid_server_url" }, 400);
        }
        if (!env.R2_ACCESS_KEY_ID || !env.R2_SECRET_ACCESS_KEY) {
          return json({ ok: false, error: "missing_r2_s3_credentials" }, 500);
        }
        if (!env.R2_ACCOUNT_ID || env.R2_ACCOUNT_ID === "REPLACE_ME") {
          return json({ ok: false, error: "missing_r2_account_id" }, 500);
        }

        const now = utcNowIso();
        const expiresAt = new Date(Date.now() + ttlMinutes * 60 * 1000).toISOString().replace(/\.\d{3}Z$/, "Z");

        const batchId = `pkg_${now.replace(/[-:TZ]/g, "")}_${Math.random().toString(16).slice(2, 10)}`;

        const genKey = (): string => {
          const bytes = new Uint8Array(24);
          crypto.getRandomValues(bytes);
          const b64 = bytesToBase64(bytes).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
          return `k_${b64}`;
        };

        const r2 = new AwsClient({
          service: "s3",
          region: "auto",
          accessKeyId: env.R2_ACCESS_KEY_ID,
          secretAccessKey: env.R2_SECRET_ACCESS_KEY,
        });

        const makePresignedGet = async (r2Key: string): Promise<string> => {
          const u = new URL(`https://${env.R2_ACCOUNT_ID}.r2.cloudflarestorage.com/${env.R2_BUCKET_NAME}/${r2Key}`);
          u.searchParams.set("X-Amz-Expires", String(ttlMinutes * 60));
          const signed = await r2.sign(new Request(u.toString(), { method: "GET" }), { aws: { signQuery: true } });
          return signed.url.toString();
        };

        const buildZipBlob = async (password: string, envText: string, readmeText: string): Promise<Blob> => {
          const zw = new ZipWriter(new BlobWriter("application/zip"));
          await zw.add("无限续杯配置.env", new TextReader(envText), { password, encryptionStrength: 3 });
          await zw.add("README.txt", new TextReader(readmeText), { password, encryptionStrength: 3 });
          return zw.close();
        };

        const packages: Array<{ name: string; key: string; download_url: string }> = [];
        const errors: Array<{ idx: number; error: string }> = [];

        // 写入 batch 元数据（不存明文 key）
        await env.DB.prepare(
          "INSERT INTO package_batches(batch_id,key_type,count,label,bind_pool_size,server_url,created_at,expires_at) VALUES(?,?,?,?,?,?,?,?)",
        )
          .bind(batchId, type, count, label, bindPoolSize, serverUrl, now, expiresAt)
          .run();

        const manifestLines: string[] = [];

        for (let i = 1; i <= count; i++) {
          const k = genKey();
          const h = await sha256Hex(k);
          const zipName = `${i}.zip`;
          const r2Key = `batches/${batchId}/${zipName}`;

          try {
            if (type === "user") {
              await env.DB.prepare("INSERT OR IGNORE INTO user_keys(key_hash,label,enabled,created_at) VALUES(?,?,1,?)")
                .bind(h, label, now)
                .run();
            } else {
              await env.DB.prepare("INSERT OR IGNORE INTO upload_keys(key_hash,label,enabled,created_at) VALUES(?,?,1,?)")
                .bind(h, label, now)
                .run();
            }

            const envText = [
              `SERVER_URL=${serverUrl}`,
              type === "user" ? `USER_KEY=${k}` : `UPLOAD_KEY=${k}`,
              "TARGET_POOL_SIZE=10",
              "TRIGGER_REMAINING=2",
            ].join("\n") + "\n";

            const readmeText = [
              "这是无限续杯的分发包。",
              "\n",
              "1) 解压后，把本文件夹复制到仓库的 客户端/普通用户/状态/ 或按客户端文档提示放置。",
              "2) 入口说明：客户端/README.md",
              "\n",
              "注意：此包只包含平台密钥与配置，不包含任何第三方 token。",
            ].join("\n");

            const zipBlob = await buildZipBlob(k, envText, readmeText);
            await env.BUCKET.put(r2Key, zipBlob, {
              httpMetadata: { contentType: "application/zip" },
              customMetadata: { batch_id: batchId, name: zipName, kind: "zip" },
            });

            await env.DB.prepare(
              "INSERT OR IGNORE INTO package_objects(batch_id,name,kind,r2_key,created_at) VALUES(?,?,?,?,?)",
            )
              .bind(batchId, zipName, "zip", r2Key, now)
              .run();

            const downloadUrl = await makePresignedGet(r2Key);
            packages.push({ name: zipName, key: k, download_url: downloadUrl });
            manifestLines.push(`${zipName}：${k}`);
          } catch (e: any) {
            errors.push({ idx: i, error: String(e?.message || e) });
          }
        }

        const manifestName = "分发清单.txt";
        const manifestKey = `batches/${batchId}/${manifestName}`;
        const manifestText = manifestLines.join("\r\n") + "\r\n";
        await env.BUCKET.put(manifestKey, manifestText, {
          httpMetadata: { contentType: "text/plain; charset=utf-8" },
          customMetadata: { batch_id: batchId, name: manifestName, kind: "manifest" },
        });
        await env.DB.prepare(
          "INSERT OR IGNORE INTO package_objects(batch_id,name,kind,r2_key,created_at) VALUES(?,?,?,?,?)",
        )
          .bind(batchId, manifestName, "manifest", manifestKey, now)
          .run();

        const manifestUrl = await makePresignedGet(manifestKey);

        return json({
          ok: true,
          batch_id: batchId,
          type,
          count_requested: count,
          count_issued: packages.length,
          label,
          bind_pool_size: bindPoolSize,
          server_url: serverUrl,
          ttl_minutes: ttlMinutes,
          expires_at: expiresAt,
          packages,
          manifest: { name: manifestName, download_url: manifestUrl },
          errors,
        });
      }

      if (path === "/admin/backup/export" && req.method === "GET") {
        await requireAdmin(req, env);

        // 仅导出“有效数据”：不导出 probes 日志；不导出 invalid/exhausted 等无效库。
        const exportedAt = utcNowIso();

        const uploadKeys = await env.DB
          .prepare("SELECT key_hash,label,enabled,created_at FROM upload_keys WHERE enabled=1 ORDER BY id ASC")
          .all<{ key_hash: string; label: string | null; enabled: number; created_at: string }>();
        const userKeys = await env.DB
          .prepare("SELECT key_hash,label,enabled,created_at FROM user_keys WHERE enabled=1 ORDER BY id ASC")
          .all<{ key_hash: string; label: string | null; enabled: number; created_at: string }>();
        const refillKeys = await env.DB
          .prepare(
            "SELECT key_hash,key_enc_b64,status,claimed_by_upload_key_hash,claimed_by_user_key_hash,claimed_at,created_at FROM refill_keys WHERE status!='revoked' ORDER BY id ASC",
          )
          .all<any>();
        const accounts = await env.DB
          .prepare(
            "SELECT email_hash,account_id,first_seen_at,last_seen_at,last_status_code,last_probed_at FROM accounts WHERE invalid=0 ORDER BY last_seen_at DESC",
          )
          .all<any>();

        const dump: BackupDumpV1 = {
          version: 1,
          exported_at: exportedAt,
          upload_keys: uploadKeys.results || [],
          user_keys: userKeys.results || [],
          refill_keys: refillKeys.results || [],
          accounts: accounts.results || [],
        };

        return json({ ok: true, dump });
      }

      if (path === "/admin/backup/import" && req.method === "POST") {
        await requireAdmin(req, env);
        const body = await parseJson<BackupImportBody>(req);
        const dump = body?.dump;

        if (!dump || dump.version !== 1) {
          return json({ ok: false, error: "invalid_dump" }, 400);
        }

        let insertedUpload = 0;
        let insertedUser = 0;
        let insertedRefill = 0;
        let upsertedAccounts = 0;

        for (const row of Array.isArray(dump.upload_keys) ? dump.upload_keys : []) {
          const h = String(row?.key_hash || "").trim();
          if (!h) continue;
          await env.DB.prepare("INSERT OR IGNORE INTO upload_keys(key_hash,label,enabled,created_at) VALUES(?,?,1,?)")
            .bind(h, row.label || null, row.created_at || utcNowIso())
            .run();
          insertedUpload++;
        }

        for (const row of Array.isArray(dump.user_keys) ? dump.user_keys : []) {
          const h = String(row?.key_hash || "").trim();
          if (!h) continue;
          await env.DB.prepare("INSERT OR IGNORE INTO user_keys(key_hash,label,enabled,created_at) VALUES(?,?,1,?)")
            .bind(h, row.label || null, row.created_at || utcNowIso())
            .run();
          insertedUser++;
        }

        for (const row of Array.isArray(dump.refill_keys) ? dump.refill_keys : []) {
          const h = String(row?.key_hash || "").trim();
          const enc = String(row?.key_enc_b64 || "").trim();
          if (!h || !enc) continue;

          // 仅恢复“有效状态”
          const status = row?.status ? String(row.status) : "available";
          const createdAt = row?.created_at ? String(row.created_at) : utcNowIso();

          await env.DB.prepare(
            "INSERT OR IGNORE INTO refill_keys(key_hash,key_enc_b64,status,claimed_by_upload_key_hash,claimed_by_user_key_hash,claimed_at,created_at) VALUES(?,?,?,?,?,?,?)",
          )
            .bind(
              h,
              enc,
              status,
              row?.claimed_by_upload_key_hash || null,
              row?.claimed_by_user_key_hash || null,
              row?.claimed_at || null,
              createdAt,
            )
            .run();

          insertedRefill++;
        }

        for (const row of Array.isArray(dump.accounts) ? dump.accounts : []) {
          const emailHash = String(row?.email_hash || "").trim().toLowerCase();
          if (!isHexSha256(emailHash)) continue;

          const accountId = row?.account_id ? String(row.account_id).trim() : null;
          const firstSeenAt = String(row?.first_seen_at || "").trim() || utcNowIso();
          const lastSeenAt = String(row?.last_seen_at || "").trim() || firstSeenAt;
          const lastStatusCode = typeof row?.last_status_code === "number" ? Math.trunc(row.last_status_code) : null;
          const lastProbedAt = row?.last_probed_at ? String(row.last_probed_at).trim() : null;

          const existing = await env.DB.prepare("SELECT 1 as ok FROM accounts WHERE email_hash=?")
            .bind(emailHash)
            .first<{ ok: number }>();

          if (!existing) {
            await env.DB.prepare(
              "INSERT INTO accounts(email_hash,account_id,first_seen_at,last_seen_at,last_status_code,last_probed_at,invalid,invalid_at) VALUES(?,?,?,?,?,?,0,NULL)",
            )
              .bind(emailHash, accountId, firstSeenAt, lastSeenAt, lastStatusCode, lastProbedAt)
              .run();
          } else {
            await env.DB.prepare(
              "UPDATE accounts SET account_id=COALESCE(?,account_id), first_seen_at=MIN(first_seen_at, ?), last_seen_at=MAX(last_seen_at, ?), last_status_code=COALESCE(?,last_status_code), last_probed_at=COALESCE(?,last_probed_at) WHERE email_hash=?",
            )
              .bind(accountId, firstSeenAt, lastSeenAt, lastStatusCode, lastProbedAt, emailHash)
              .run();
          }
          upsertedAccounts++;
        }

        return json({
          ok: true,
          inserted_upload_keys: insertedUpload,
          inserted_user_keys: insertedUser,
          inserted_refill_keys: insertedRefill,
          upserted_accounts: upsertedAccounts,
        });
      }

      if (path === "/admin/upload-keys" && req.method === "POST") {
        await requireAdmin(req, env);
        const body = await parseJson<UploadKeysBody>(req);
        const keys = Array.isArray(body.keys) ? body.keys : [];
        const now = utcNowIso();

        let inserted = 0;
        const errors: Array<{ key: string; error: string }> = [];
        for (const raw of keys) {
          const k = String(raw || "").trim();
          if (!k) continue;
          const h = await sha256Hex(k);
          try {
            await env.DB.prepare("INSERT OR IGNORE INTO upload_keys(key_hash,label,enabled,created_at) VALUES(?,?,1,?)")
              .bind(h, body.label || null, now)
              .run();
            inserted++;
          } catch (e: any) {
            errors.push({ key: k, error: String(e?.message || e) });
          }
        }

        return json({ ok: true, inserted, errors });
      }

      if (path === "/admin/user-keys" && req.method === "POST") {
        await requireAdmin(req, env);
        const body = await parseJson<UserKeysBody>(req);
        const keys = Array.isArray(body.keys) ? body.keys : [];
        const now = utcNowIso();

        let inserted = 0;
        const errors: Array<{ key: string; error: string }> = [];
        for (const raw of keys) {
          const k = String(raw || "").trim();
          if (!k) continue;
          const h = await sha256Hex(k);
          try {
            await env.DB.prepare("INSERT OR IGNORE INTO user_keys(key_hash,label,enabled,created_at) VALUES(?,?,1,?)")
              .bind(h, body.label || null, now)
              .run();
            inserted++;
          } catch (e: any) {
            errors.push({ key: k, error: String(e?.message || e) });
          }
        }

        return json({ ok: true, inserted, errors });
      }

      if (path === "/admin/refill-keys" && req.method === "POST") {
        await requireAdmin(req, env);
        const body = await parseJson<RefillKeysBody>(req);
        const keys = Array.isArray(body.keys) ? body.keys : [];
        const now = utcNowIso();

        let inserted = 0;
        const errors: Array<{ error: string }> = [];

        for (const raw of keys) {
          const k = String(raw || "").trim();
          if (!k) continue;
          const h = await sha256Hex(k);
          const enc = await aesGcmEncryptToB64(env.REFILL_KEYS_MASTER_KEY_B64, k);
          try {
            await env.DB.prepare(
              "INSERT OR IGNORE INTO refill_keys(key_hash,key_enc_b64,status,created_at) VALUES(?,?,'available',?)",
            )
              .bind(h, enc, now)
              .run();
            inserted++;
          } catch (e: any) {
            errors.push({ error: String(e?.message || e) });
          }
        }

        return json({ ok: true, inserted, errors });
      }

      if (path === "/admin/refill-keys" && req.method === "GET") {
        await requireAdmin(req, env);
        const status = url.searchParams.get("status");
        const q = status
          ? env.DB
              .prepare(
                "SELECT id,key_hash,status,claimed_by_upload_key_hash,claimed_by_user_key_hash,claimed_at,created_at FROM refill_keys WHERE status=? ORDER BY id DESC",
              )
              .bind(status)
          : env.DB.prepare(
              "SELECT id,key_hash,status,claimed_by_upload_key_hash,claimed_by_user_key_hash,claimed_at,created_at FROM refill_keys ORDER BY id DESC",
            );

        const rows = await q.all<{
          id: number;
          key_hash: string;
          status: string;
          claimed_by_upload_key_hash: string | null;
          claimed_at: string | null;
          created_at: string;
        }>();

        return json({ ok: true, rows: rows.results });
      }

      if (path === "/admin/accounts" && req.method === "GET") {
        await requireAdmin(req, env);
        const invalid = url.searchParams.get("invalid");
        const q = invalid === "1"
          ? env.DB.prepare(
              "SELECT email_hash,account_id,last_status_code,last_probed_at,invalid,invalid_at,first_seen_at,last_seen_at FROM accounts WHERE invalid=1 ORDER BY invalid_at DESC LIMIT 500",
            )
          : env.DB.prepare(
              "SELECT email_hash,account_id,last_status_code,last_probed_at,invalid,invalid_at,first_seen_at,last_seen_at FROM accounts ORDER BY last_seen_at DESC LIMIT 500",
            );

        const rows = await q.all<any>();
        return json({ ok: true, rows: rows.results });
      }

      // --- Client API ---
      // 角色拆分：
      // - 普通管理员（UPLOAD_KEY）：只允许上报/注册
      // - 普通用户（USER_KEY）：只允许 claim refill_key

      // --- Artworks API（艺术品） ---

      if (path === "/v1/artworks/submit" && req.method === "POST") {
        const caller = await requireAtLeastUpload(req, env);
        const body = await parseJson<SubmitArtworksBody>(req);

        const rawItems: unknown[] = [];
        if (typeof body === "object" && body && (body as any).artwork !== undefined) rawItems.push((body as any).artwork);
        if (typeof body === "object" && body && Array.isArray((body as any).items)) rawItems.push(...((body as any).items as unknown[]));

        if (rawItems.length === 0) return json({ ok: false, error: "missing_items" }, 400);
        if (rawItems.length > 2000) return json({ ok: false, error: "too_many_items" }, 413);

        const now = utcNowIso();
        const maxPerItem = 64 * 1024;
        const maxTotal = 10 * 1024 * 1024;

        let totalBytes = 0;
        let accepted = 0;
        const items: Array<{ artwork_id: string; size_bytes: number }> = [];
        const errors: Array<{ idx: number; error: string }> = [];

        for (let i = 0; i < rawItems.length; i++) {
          const original = rawItems[i];

          try {
            const scrubbed = scrubSensitiveKeysDeep(original);
            const text = JSON.stringify(scrubbed);
            const size = strByteLenUtf8(text);

            if (size > maxPerItem) {
              errors.push({ idx: i, error: "item_too_large" });
              continue;
            }
            if (totalBytes + size > maxTotal) {
              errors.push({ idx: i, error: "batch_too_large" });
              break;
            }

            // 优先使用 artwork JSON 内的 account_id 作为唯一 id（新规则）
            const fromAccountId = extractArtworkIdFromJsonArtwork(scrubbed);
            const artworkId = fromAccountId || genArtworkId();

            // 墓地 token：若已 tombstone，则拒绝重复提交
            const tomb = await env.DB.prepare("SELECT 1 as ok FROM artwork_tombstones WHERE artwork_id=?")
              .bind(artworkId)
              .first<{ ok: number }>();
            if (tomb) {
              errors.push({ idx: i, error: "tombstoned_artwork_id" });
              continue;
            }

            // 去重：同 artwork_id 已存在则拒绝（避免覆盖）
            const exists = await env.DB.prepare("SELECT 1 as ok FROM artworks WHERE artwork_id=?")
              .bind(artworkId)
              .first<{ ok: number }>();
            if (exists) {
              errors.push({ idx: i, error: "duplicate_artwork_id" });
              continue;
            }

            const r2Key = `artworks/${artworkId}.json`;

            await env.BUCKET.put(r2Key, text + "\n", {
              httpMetadata: { contentType: "application/json; charset=utf-8" },
              customMetadata: { artwork_id: artworkId },
            });

            await env.DB.prepare(
              "INSERT INTO artworks(artwork_id,upload_key_hash,status,created_at) VALUES(?,?,\"available\",?)",
            )
              .bind(artworkId, caller.audit_key_hash, now)
              .run();

            await env.DB.prepare(
              "INSERT INTO artwork_objects(artwork_id,r2_key,size_bytes,created_at) VALUES(?,?,?,?)",
            )
              .bind(artworkId, r2Key, size, now)
              .run();

            totalBytes += size;
            accepted++;
            items.push({ artwork_id: artworkId, size_bytes: size });
          } catch (e: any) {
            errors.push({ idx: i, error: String(e?.message || e) });
          }
        }

        // uploader 激励：仅对 upload 角色生效
        // - valid_artworks_submitted += accepted
        // - 每跨越 100 的段位 +1 distribution_credit（累计）
        let uploaderStats: { valid_artworks_submitted: number; distribution_credits: number } | null = null;
        if (accepted > 0 && caller.role === "upload" && caller.upload_key_hash) {
          await env.DB.prepare(
            "INSERT OR IGNORE INTO uploader_stats(upload_key_hash,valid_artworks_submitted,distribution_credits,updated_at) VALUES(?,?,?,?)",
          )
            .bind(caller.upload_key_hash, 0, 0, now)
            .run();

          // 用单条 UPDATE 做“段位跨越”计算，避免并发下丢增量
          await env.DB.prepare(
            "UPDATE uploader_stats SET distribution_credits = distribution_credits + (CAST((valid_artworks_submitted + ?)/100 AS INTEGER) - CAST(valid_artworks_submitted/100 AS INTEGER)), valid_artworks_submitted = valid_artworks_submitted + ?, updated_at=? WHERE upload_key_hash=?",
          )
            .bind(accepted, accepted, now, caller.upload_key_hash)
            .run();

          const st = await env.DB.prepare(
            "SELECT valid_artworks_submitted, distribution_credits FROM uploader_stats WHERE upload_key_hash=?",
          )
            .bind(caller.upload_key_hash)
            .first<{ valid_artworks_submitted: number; distribution_credits: number }>();

          uploaderStats = {
            valid_artworks_submitted: Number(st?.valid_artworks_submitted || 0),
            distribution_credits: Number(st?.distribution_credits || 0),
          };
        }

        return json({
          ok: true,
          accepted,
          total_bytes: totalBytes,
          items,
          errors,
          received_at: now,
          uploader_stats: uploaderStats,
        });
      }

      if (path === "/v1/artworks/claim" && req.method === "POST") {
        const caller = await requireAtLeastUser(req, env);
        const body = await parseJson<ClaimArtworksBody>(req);

        const now = utcNowIso();

        // 懒回收：温养到期的 quarantine 回公共池
        await env.DB.prepare(
          "UPDATE artworks SET status='available', eligible_after=NULL WHERE status='quarantine' AND eligible_after IS NOT NULL AND eligible_after <= ?",
        )
          .bind(now)
          .run();

        const isUser = caller.role === "user";

        // 私有池容量：
        // - 普通用户：固定 10
        // - 热心群众：默认 10；有效投稿 >= 1000 时提升到 50
        // - admin：不走激励，仍按 10（通常不使用该接口）
        let cap = 10;
        if (!isUser && caller.role === "upload") {
          await env.DB.prepare(
            "INSERT OR IGNORE INTO uploader_stats(upload_key_hash,valid_artworks_submitted,distribution_credits,updated_at) VALUES(?,?,?,?)",
          )
            .bind(caller.upload_key_hash, 0, 0, now)
            .run();

          const st = await env.DB
            .prepare("SELECT valid_artworks_submitted FROM uploader_stats WHERE upload_key_hash=?")
            .bind(caller.upload_key_hash)
            .first<{ valid_artworks_submitted: number }>();
          cap = Number(st?.valid_artworks_submitted || 0) >= 1000 ? 50 : 10;
        }

        const requested = Math.max(1, Math.min(cap, Math.trunc(Number(body?.count || 1))));

        const claimedByUser = isUser ? caller.user_key_hash || null : null;
        const claimedByUpload = !isUser
          ? caller.role === "upload"
            ? caller.upload_key_hash || null
            : caller.audit_key_hash
          : null;

        const poolRow = isUser
          ? await env.DB
              .prepare("SELECT COUNT(1) as c FROM artworks WHERE status='claimed' AND claimed_by_user_key_hash=?")
              .bind(caller.user_key_hash)
              .first<{ c: number }>()
          : await env.DB
              .prepare("SELECT COUNT(1) as c FROM artworks WHERE status='claimed' AND claimed_by_upload_key_hash=?")
              .bind(caller.role === "upload" ? caller.upload_key_hash : caller.audit_key_hash)
              .first<{ c: number }>();

        const poolSize = Number(poolRow?.c || 0);
        if (poolSize >= cap) return json({ ok: false, error: "pool_full" }, 409);

        const remaining = cap - poolSize;
        const want = Math.min(requested, remaining);

        const out: Array<{ artwork_id: string; claimed_at: string; artwork: unknown }> = [];

        for (let n = 0; n < want; n++) {
          let claimed = false;

          for (let attempt = 0; attempt < 8; attempt++) {
            const pick = await env.DB
              .prepare(
                "SELECT a.artwork_id as artwork_id, o.r2_key as r2_key FROM artworks a JOIN artwork_objects o ON a.artwork_id=o.artwork_id WHERE a.status='available' ORDER BY RANDOM() LIMIT 1",
              )
              .first<{ artwork_id: string; r2_key: string }>();

            if (!pick) break;

            const up = await env.DB
              .prepare(
                "UPDATE artworks SET status='claimed', claimed_by_user_key_hash=?, claimed_by_upload_key_hash=?, claimed_at=?, eligible_after=NULL WHERE artwork_id=? AND status='available'",
              )
              .bind(claimedByUser, claimedByUpload, now, pick.artwork_id)
              .run();

            if (up.meta.changes !== 1) continue;

            const obj = await env.BUCKET.get(pick.r2_key);
            if (!obj) throw new Error("missing_r2_object");
            const text = await obj.text();
            let artwork: unknown = null;
            try {
              artwork = JSON.parse(text);
            } catch {
              artwork = text;
            }

            out.push({ artwork_id: pick.artwork_id, claimed_at: now, artwork });
            claimed = true;
            break;
          }

          if (!claimed) break;
        }

        if (out.length === 0) return json({ ok: false, error: "no_available_artwork" }, 409);
        return json({ ok: true, claimed: out.length, items: out });
      }

      if (path === "/v1/artworks/report-damage" && req.method === "POST") {
        const caller = await requireAtLeastUser(req, env);
        const body = await parseJson<DamageReportBody>(req);

        const artworkId = String(body?.artwork_id || "").trim();
        const kind = body?.kind;
        const note = body?.note ? String(body.note).slice(0, 1000) : null;

        if (!isValidArtworkId(artworkId)) return json({ ok: false, error: "invalid_artwork_id" }, 400);
        if (kind !== "full" && kind !== "partial") return json({ ok: false, error: "invalid_kind" }, 400);

        const now = utcNowIso();

        const isUser = caller.role === "user";
        const ownerHash = isUser ? caller.user_key_hash : caller.role === "upload" ? caller.upload_key_hash : caller.audit_key_hash;

        const owned = isUser
          ? await env.DB
              .prepare(
                "SELECT artwork_id FROM artworks WHERE artwork_id=? AND status='claimed' AND claimed_by_user_key_hash=? LIMIT 1",
              )
              .bind(artworkId, ownerHash)
              .first<{ artwork_id: string }>()
          : await env.DB
              .prepare(
                "SELECT artwork_id FROM artworks WHERE artwork_id=? AND status='claimed' AND claimed_by_upload_key_hash=? LIMIT 1",
              )
              .bind(artworkId, ownerHash)
              .first<{ artwork_id: string }>();

        if (!owned) return json({ ok: false, error: "not_found" }, 404);

        // 1) 先信任客户端：写入损坏上报与待校验队列
        await env.DB.prepare(
          "INSERT INTO artwork_damage_reports(artwork_id,reporter_key_hash,reporter_role,kind,reported_at,note) VALUES(?,?,?,?,?,?)",
        )
          .bind(artworkId, caller.audit_key_hash, isUser ? "user" : "upload", kind, now, note)
          .run();

        await env.DB.prepare(
          "INSERT INTO pending_verification_artworks(artwork_id,last_report_kind,last_reported_at) VALUES(?,?,?) ON CONFLICT(artwork_id) DO UPDATE SET last_report_kind=excluded.last_report_kind, last_reported_at=excluded.last_reported_at",
        )
          .bind(artworkId, kind, now)
          .run();

        // 2) 领取一件替换作品（不受 pool_full 限制：这是一次交换）
        let replacement: { artwork_id: string; r2_key: string } | null = null;

        const claimedByUser = isUser ? ownerHash || null : null;
        const claimedByUpload = !isUser ? ownerHash || null : null;

        for (let attempt = 0; attempt < 12; attempt++) {
          const pick = await env.DB
            .prepare(
              "SELECT a.artwork_id as artwork_id, o.r2_key as r2_key FROM artworks a JOIN artwork_objects o ON a.artwork_id=o.artwork_id WHERE a.status='available' ORDER BY RANDOM() LIMIT 1",
            )
            .first<{ artwork_id: string; r2_key: string }>();

          if (!pick) break;

          const up = await env.DB
            .prepare(
              "UPDATE artworks SET status='claimed', claimed_by_user_key_hash=?, claimed_by_upload_key_hash=?, claimed_at=?, eligible_after=NULL WHERE artwork_id=? AND status='available'",
            )
            .bind(claimedByUser, claimedByUpload, now, pick.artwork_id)
            .run();

          if (up.meta.changes !== 1) continue;
          replacement = { artwork_id: pick.artwork_id, r2_key: pick.r2_key };
          break;
        }

        if (!replacement) return json({ ok: false, error: "no_available_artwork" }, 409);

        // 3) 流转旧作品状态（维修区/温养），并解绑
        if (kind === "full") {
          // 新逻辑：客户端声称完全损坏，不再直接 deleted，而是进入“维修区”等待维修者处理。
          await env.DB.prepare(
            "UPDATE artworks SET status='repair', repair_fail_count=0, repair_claimed_by_upload_key_hash=NULL, repair_claimed_at=NULL, repair_last_fail_note=NULL, repair_last_failed_at=NULL, claimed_by_user_key_hash=NULL, claimed_by_upload_key_hash=NULL, claimed_at=NULL, eligible_after=NULL, deleted_reason=NULL, deleted_at=NULL WHERE artwork_id=?",
          )
            .bind(artworkId)
            .run();
        } else {
          const base = Date.parse(now);
          const eligibleAfter = new Date((isNaN(base) ? Date.now() : base) + 7 * 24 * 60 * 60 * 1000)
            .toISOString()
            .replace(/\.\d{3}Z$/, "Z");

          await env.DB.prepare(
            "UPDATE artworks SET status='quarantine', eligible_after=?, claimed_by_user_key_hash=NULL, claimed_by_upload_key_hash=NULL, claimed_at=NULL, deleted_reason=NULL, deleted_at=NULL WHERE artwork_id=?",
          )
            .bind(eligibleAfter, artworkId)
            .run();
        }

        // 4) 返回替换作品正文
        const obj = await env.BUCKET.get(replacement.r2_key);
        if (!obj) throw new Error("missing_r2_object");
        const text = await obj.text();
        let artwork: unknown = null;
        try {
          artwork = JSON.parse(text);
        } catch {
          artwork = text;
        }

        return json({ ok: true, replaced_artwork_id: artworkId, replacement: { artwork_id: replacement.artwork_id, claimed_at: now, artwork } });
      }

      // --- Repairer（维修者）: 使用 UPLOAD_KEY 进入维修区处理作品 ---

      if (path === "/v1/repairs/claim" && req.method === "POST") {
        const caller = await requireAtLeastUpload(req, env);
        const body = await parseJson<RepairClaimBody>(req);
        const now = utcNowIso();

        const want = Math.max(1, Math.min(10, Math.trunc(Number(body?.count || 1))));
        const out: Array<{ artwork_id: string; claimed_at: string; artwork: unknown }> = [];

        for (let n = 0; n < want; n++) {
          let claimed = false;

          for (let attempt = 0; attempt < 10; attempt++) {
            const pick = await env.DB.prepare(
              "SELECT a.artwork_id as artwork_id, o.r2_key as r2_key FROM artworks a JOIN artwork_objects o ON a.artwork_id=o.artwork_id WHERE a.status='repair' ORDER BY RANDOM() LIMIT 1",
            )
              .first<{ artwork_id: string; r2_key: string }>();

            if (!pick) break;

            const up = await env.DB.prepare(
              "UPDATE artworks SET status='repair_claimed', repair_claimed_by_upload_key_hash=?, repair_claimed_at=? WHERE artwork_id=? AND status='repair'",
            )
              .bind(caller.audit_key_hash, now, pick.artwork_id)
              .run();

            if (up.meta.changes !== 1) continue;

            const obj = await env.BUCKET.get(pick.r2_key);
            if (!obj) throw new Error("missing_r2_object");
            const text = await obj.text();
            let artwork: unknown = null;
            try {
              artwork = JSON.parse(text);
            } catch {
              artwork = text;
            }

            out.push({ artwork_id: pick.artwork_id, claimed_at: now, artwork });
            claimed = true;
            break;
          }

          if (!claimed) break;
        }

        if (out.length === 0) return json({ ok: false, error: "no_repairable_artwork" }, 409);
        return json({ ok: true, claimed: out.length, items: out });
      }

      if (path === "/v1/repairs/submit-fixed" && req.method === "POST") {
        const caller = await requireAtLeastUpload(req, env);
        const body = await parseJson<RepairSubmitFixedBody>(req);

        const artworkId = String(body?.artwork_id || "").trim();
        if (!isValidArtworkId(artworkId)) return json({ ok: false, error: "invalid_artwork_id" }, 400);

        const now = utcNowIso();

        // 必须是自己领取的 repair_claimed
        const row = await env.DB.prepare(
          "SELECT a.artwork_id as artwork_id, o.r2_key as r2_key FROM artworks a JOIN artwork_objects o ON a.artwork_id=o.artwork_id WHERE a.artwork_id=? AND a.status='repair_claimed' AND a.repair_claimed_by_upload_key_hash=? LIMIT 1",
        )
          .bind(artworkId, caller.audit_key_hash)
          .first<{ artwork_id: string; r2_key: string }>();
        if (!row) return json({ ok: false, error: "not_found" }, 404);

        const scrubbed = scrubSensitiveKeysDeep((body as any).fixed_artwork);
        const fromAccountId = extractArtworkIdFromJsonArtwork(scrubbed);
        if (!fromAccountId) return json({ ok: false, error: "missing_account_id" }, 400);
        if (fromAccountId !== artworkId) return json({ ok: false, error: "account_id_mismatch" }, 400);

        const text = JSON.stringify(scrubbed) + "\n";
        const size = strByteLenUtf8(text);
        if (size > 64 * 1024) return json({ ok: false, error: "item_too_large" }, 413);

        // 覆盖写回同一个对象（让修缮后的作品“以相同 account_id/唯一id”重新可领取）
        await env.BUCKET.put(row.r2_key, text, {
          httpMetadata: { contentType: "application/json; charset=utf-8" },
          customMetadata: { artwork_id: artworkId, repaired: "1" },
        });

        await env.DB.prepare(
          "UPDATE artworks SET status='available', claimed_by_user_key_hash=NULL, claimed_by_upload_key_hash=NULL, claimed_at=NULL, eligible_after=NULL, repair_fail_count=0, repair_claimed_by_upload_key_hash=NULL, repair_claimed_at=NULL, repair_last_fail_note=NULL, repair_last_failed_at=NULL, deleted_reason=NULL, deleted_at=NULL WHERE artwork_id=?",
        )
          .bind(artworkId)
          .run();

        await env.DB.prepare("UPDATE artwork_objects SET size_bytes=?, created_at=? WHERE artwork_id=?")
          .bind(size, now, artworkId)
          .run();

        return json({ ok: true, artwork_id: artworkId, status: "available", repaired_at: now });
      }

      if (path === "/v1/repairs/submit-failed" && req.method === "POST") {
        const caller = await requireAtLeastUpload(req, env);
        const body = await parseJson<RepairSubmitFailedBody>(req);

        const artworkId = String(body?.artwork_id || "").trim();
        if (!isValidArtworkId(artworkId)) return json({ ok: false, error: "invalid_artwork_id" }, 400);

        const note = body?.note ? String(body.note).slice(0, 1000) : null;
        const now = utcNowIso();

        const cur = await env.DB.prepare(
          "SELECT repair_fail_count FROM artworks WHERE artwork_id=? AND status='repair_claimed' AND repair_claimed_by_upload_key_hash=? LIMIT 1",
        )
          .bind(artworkId, caller.audit_key_hash)
          .first<{ repair_fail_count: number }>();
        if (!cur) return json({ ok: false, error: "not_found" }, 404);

        const next = Number(cur.repair_fail_count || 0) + 1;

        if (next >= 3) {
          // 进入墓地区：写 tombstone，作品本体可移入 graveyard 前缀（保留但不再分发）
          await env.DB.prepare(
            "INSERT OR IGNORE INTO artwork_tombstones(artwork_id,reason,note,created_at) VALUES(?,?,?,?)",
          )
            .bind(artworkId, "repair_failed_3", note, now)
            .run();

          const objRow = await env.DB.prepare("SELECT r2_key,size_bytes FROM artwork_objects WHERE artwork_id=?")
            .bind(artworkId)
            .first<{ r2_key: string; size_bytes: number }>();
          if (objRow?.r2_key) {
            const src = await env.BUCKET.get(objRow.r2_key);
            if (src) {
              const graveKey = `graveyard/artworks/${artworkId}.json`;
              const buf = await src.arrayBuffer();
              await env.BUCKET.put(graveKey, buf, {
                httpMetadata: { contentType: "application/json; charset=utf-8" },
                customMetadata: { artwork_id: artworkId, graveyard: "1" },
              });
              // 删除原位置，避免误发
              await env.BUCKET.delete(objRow.r2_key);
              await env.DB.prepare("UPDATE artwork_objects SET r2_key=? WHERE artwork_id=?")
                .bind(graveKey, artworkId)
                .run();
            }
          }

          await env.DB.prepare(
            "UPDATE artworks SET status='graveyard', deleted_reason='repair_failed_3', deleted_at=?, claimed_by_user_key_hash=NULL, claimed_by_upload_key_hash=NULL, claimed_at=NULL, eligible_after=NULL, repair_claimed_by_upload_key_hash=NULL, repair_claimed_at=NULL, repair_last_fail_note=?, repair_last_failed_at=?, repair_fail_count=? WHERE artwork_id=?",
          )
            .bind(now, note, now, next, artworkId)
            .run();

          return json({ ok: true, artwork_id: artworkId, status: "graveyard", repair_fail_count: next });
        }

        // 仍可继续维修：退回 repair 队列
        await env.DB.prepare(
          "UPDATE artworks SET status='repair', repair_fail_count=?, repair_claimed_by_upload_key_hash=NULL, repair_claimed_at=NULL, repair_last_fail_note=?, repair_last_failed_at=? WHERE artwork_id=?",
        )
          .bind(next, note, now, artworkId)
          .run();

        return json({ ok: true, artwork_id: artworkId, status: "repair", repair_fail_count: next });
      }

      if (path === "/v1/repairs/submit-misreport" && req.method === "POST") {
        const caller = await requireAtLeastUpload(req, env);
        const body = await parseJson<{ artwork_id: string; note?: string }>(req);

        const artworkId = String((body as any)?.artwork_id || "").trim();
        if (!isValidArtworkId(artworkId)) return json({ ok: false, error: "invalid_artwork_id" }, 400);

        const now = utcNowIso();
        const note = (body as any)?.note ? String((body as any).note).slice(0, 1000) : null;

        const up = await env.DB.prepare(
          "UPDATE artworks SET status='available', claimed_by_user_key_hash=NULL, claimed_by_upload_key_hash=NULL, claimed_at=NULL, eligible_after=NULL, repair_claimed_by_upload_key_hash=NULL, repair_claimed_at=NULL, repair_last_fail_note=COALESCE(repair_last_fail_note, ?), repair_last_failed_at=COALESCE(repair_last_failed_at, ?) WHERE artwork_id=? AND status='repair_claimed' AND repair_claimed_by_upload_key_hash=?",
        )
          .bind(note, now, artworkId, caller.audit_key_hash)
          .run();

        if (up.meta.changes !== 1) return json({ ok: false, error: "not_found" }, 404);
        return json({ ok: true, artwork_id: artworkId, status: "available", noted_at: now });
      }

      if (path === "/v1/artworks/mine" && req.method === "GET") {
        if (!isTruthyEnv(env.ENABLE_MINE_API)) return json({ ok: false, error: "not_found" }, 404);

        const caller = await requireAtLeastUser(req, env);

        if (!env.R2_ACCESS_KEY_ID || !env.R2_SECRET_ACCESS_KEY) {
          return json({ ok: false, error: "missing_r2_s3_credentials" }, 500);
        }
        if (!env.R2_ACCOUNT_ID || env.R2_ACCOUNT_ID === "REPLACE_ME") {
          return json({ ok: false, error: "missing_r2_account_id" }, 500);
        }

        const isUser = caller.role === "user";
        const ownerHash = isUser ? caller.user_key_hash : caller.role === "upload" ? caller.upload_key_hash : caller.audit_key_hash;

        // mine 返回已绑定作品列表：
        // - 普通用户：最多 10
        // - 热心群众：默认 10；有效投稿 >= 1000 时最多 50
        let cap = 10;
        if (!isUser && caller.role === "upload") {
          await env.DB.prepare(
            "INSERT OR IGNORE INTO uploader_stats(upload_key_hash,valid_artworks_submitted,distribution_credits,updated_at) VALUES(?,?,?,?)",
          )
            .bind(caller.upload_key_hash, 0, 0, utcNowIso())
            .run();

          const st = await env.DB
            .prepare("SELECT valid_artworks_submitted FROM uploader_stats WHERE upload_key_hash=?")
            .bind(caller.upload_key_hash)
            .first<{ valid_artworks_submitted: number }>();
          cap = Number(st?.valid_artworks_submitted || 0) >= 1000 ? 50 : 10;
        }

        const q = isUser
          ? env.DB
              .prepare(
                "SELECT a.artwork_id as artwork_id, a.claimed_at as claimed_at, o.r2_key as r2_key, o.size_bytes as size_bytes FROM artworks a JOIN artwork_objects o ON a.artwork_id=o.artwork_id WHERE a.status='claimed' AND a.claimed_by_user_key_hash=? ORDER BY a.claimed_at DESC LIMIT ?",
              )
              .bind(ownerHash, cap)
          : env.DB
              .prepare(
                "SELECT a.artwork_id as artwork_id, a.claimed_at as claimed_at, o.r2_key as r2_key, o.size_bytes as size_bytes FROM artworks a JOIN artwork_objects o ON a.artwork_id=o.artwork_id WHERE a.status='claimed' AND a.claimed_by_upload_key_hash=? ORDER BY a.claimed_at DESC LIMIT ?",
              )
              .bind(ownerHash, cap);

        const rows = await q.all<{ artwork_id: string; claimed_at: string; r2_key: string; size_bytes: number }>();

        const r2 = new AwsClient({
          service: "s3",
          region: "auto",
          accessKeyId: env.R2_ACCESS_KEY_ID,
          secretAccessKey: env.R2_SECRET_ACCESS_KEY,
        });

        const ttlSeconds = 15 * 60;
        const items = [] as Array<{ artwork_id: string; claimed_at: string; size_bytes: number; download_url: string }>;

        for (const row of rows.results || []) {
          const u = new URL(`https://${env.R2_ACCOUNT_ID}.r2.cloudflarestorage.com/${env.R2_BUCKET_NAME}/${row.r2_key}`);
          u.searchParams.set("X-Amz-Expires", String(ttlSeconds));
          const signed = await r2.sign(new Request(u.toString(), { method: "GET" }), { aws: { signQuery: true } });
          items.push({
            artwork_id: row.artwork_id,
            claimed_at: row.claimed_at,
            size_bytes: Number(row.size_bytes || 0),
            download_url: signed.url.toString(),
          });
        }

        return json({ ok: true, items });
      }

      // --- Uploader 激励：用分发资格生成 USER_KEY 分发包（消耗制） ---

      if (path === "/v1/uploader/packages/issue" && req.method === "POST") {
        const u = await requireUploadKey(req, env);
        const body = await parseJson<UploaderIssuePackagesBody>(req);

        const count = Math.max(1, Math.min(50, Math.trunc(Number(body?.count || 0))));
        const label = body?.label ? String(body.label).trim() : null;
        const serverUrl = String(body?.server_url || "").trim();
        const ttlMinutes = body?.ttl_minutes ? Math.max(1, Math.min(24 * 60, Math.trunc(Number(body.ttl_minutes)))) : 60;

        if (!serverUrl.startsWith("http://") && !serverUrl.startsWith("https://")) {
          return json({ ok: false, error: "invalid_server_url" }, 400);
        }
        if (!env.R2_ACCESS_KEY_ID || !env.R2_SECRET_ACCESS_KEY) {
          return json({ ok: false, error: "missing_r2_s3_credentials" }, 500);
        }
        if (!env.R2_ACCOUNT_ID || env.R2_ACCOUNT_ID === "REPLACE_ME") {
          return json({ ok: false, error: "missing_r2_account_id" }, 500);
        }

        const now = utcNowIso();

        // 确保 stats 行存在
        await env.DB.prepare(
          "INSERT OR IGNORE INTO uploader_stats(upload_key_hash,valid_artworks_submitted,distribution_credits,updated_at) VALUES(?,?,?,?)",
        )
          .bind(u.hash, 0, 0, now)
          .run();

        // 消费 1 个分发资格（必须 >=1）
        const dec = await env.DB.prepare(
          "UPDATE uploader_stats SET distribution_credits=distribution_credits-1, updated_at=? WHERE upload_key_hash=? AND distribution_credits>=1",
        )
          .bind(now, u.hash)
          .run();
        if (dec.meta.changes !== 1) return json({ ok: false, error: "no_distribution_credit" }, 403);

        const creditsRow = await env.DB.prepare("SELECT distribution_credits, valid_artworks_submitted FROM uploader_stats WHERE upload_key_hash=?")
          .bind(u.hash)
          .first<{ distribution_credits: number; valid_artworks_submitted: number }>();

        const batchId = `upkg_${now.replace(/[-:TZ]/g, "")}_${Math.random().toString(16).slice(2, 10)}`;
        const expiresAt = new Date(Date.now() + ttlMinutes * 60 * 1000).toISOString().replace(/\.\d{3}Z$/, "Z");

        // 写入 batch 元数据（不存明文 key）
        await env.DB.prepare(
          "INSERT INTO package_batches(batch_id,key_type,count,label,bind_pool_size,server_url,created_at,expires_at) VALUES(?,?,?,?,?,?,?,?)",
        )
          .bind(batchId, "user", count, label, null, serverUrl, now, expiresAt)
          .run();

        // 记录 issuer
        await env.DB.prepare("INSERT OR REPLACE INTO package_batch_issuers(batch_id,issuer_key_hash,issuer_role,created_at) VALUES(?,?,?,?)")
          .bind(batchId, u.hash, "upload", now)
          .run();

        const r2 = new AwsClient({
          service: "s3",
          region: "auto",
          accessKeyId: env.R2_ACCESS_KEY_ID,
          secretAccessKey: env.R2_SECRET_ACCESS_KEY,
        });

        const makePresignedGet = async (r2Key: string): Promise<string> => {
          const u2 = new URL(`https://${env.R2_ACCOUNT_ID}.r2.cloudflarestorage.com/${env.R2_BUCKET_NAME}/${r2Key}`);
          u2.searchParams.set("X-Amz-Expires", String(ttlMinutes * 60));
          const signed = await r2.sign(new Request(u2.toString(), { method: "GET" }), { aws: { signQuery: true } });
          return signed.url.toString();
        };

        const buildZipBlob = async (password: string, envText: string, readmeText: string): Promise<Blob> => {
          const zw = new ZipWriter(new BlobWriter("application/zip"));
          await zw.add("无限续杯配置.env", new TextReader(envText), { password, encryptionStrength: 3 });
          await zw.add("README.txt", new TextReader(readmeText), { password, encryptionStrength: 3 });
          return zw.close();
        };

        const genKey = (): string => {
          const bytes = new Uint8Array(24);
          crypto.getRandomValues(bytes);
          const b64 = bytesToBase64(bytes).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
          return `k_${b64}`;
        };

        const packages: Array<{ name: string; key: string; download_url: string }> = [];
        const errors: Array<{ idx: number; error: string }> = [];
        const manifestLines: string[] = [];

        for (let i = 1; i <= count; i++) {
          const k = genKey();
          const h = await sha256Hex(k);
          const zipName = `${i}.zip`;
          const r2Key = `batches/${batchId}/${zipName}`;

          try {
            await env.DB.prepare("INSERT OR IGNORE INTO user_keys(key_hash,label,enabled,created_at) VALUES(?,?,1,?)")
              .bind(h, label, now)
              .run();

            const envText = [
              `SERVER_URL=${serverUrl}`,
              `USER_KEY=${k}`,
              "TARGET_POOL_SIZE=10",
              "TRIGGER_REMAINING=2",
            ].join("\n") + "\n";

            const readmeText = [
              "这是无限续杯的分发包（由热心群众通过分发资格生成）。",
              "\n",
              "1) 解压后，把本文件夹复制到仓库的 客户端/普通用户/状态/ 或按客户端文档提示放置。",
              "2) 入口说明：客户端/README.md",
              "\n",
              "注意：此包只包含平台密钥与配置，不包含任何第三方 token。",
            ].join("\n");

            const zipBlob = await buildZipBlob(k, envText, readmeText);
            await env.BUCKET.put(r2Key, zipBlob, {
              httpMetadata: { contentType: "application/zip" },
              customMetadata: { batch_id: batchId, name: zipName, kind: "zip", issuer: "upload" },
            });

            await env.DB.prepare("INSERT OR IGNORE INTO package_objects(batch_id,name,kind,r2_key,created_at) VALUES(?,?,?,?,?)")
              .bind(batchId, zipName, "zip", r2Key, now)
              .run();

            const downloadUrl = await makePresignedGet(r2Key);
            packages.push({ name: zipName, key: k, download_url: downloadUrl });
            manifestLines.push(`${zipName}：${k}`);
          } catch (e: any) {
            errors.push({ idx: i, error: String(e?.message || e) });
          }
        }

        if (packages.length === 0) {
          // 全失败：退回分发资格
          await env.DB.prepare("UPDATE uploader_stats SET distribution_credits=distribution_credits+1, updated_at=? WHERE upload_key_hash=?")
            .bind(now, u.hash)
            .run();
          return json({ ok: false, error: "issue_failed" }, 500);
        }

        const manifestName = "分发清单.txt";
        const manifestKey = `batches/${batchId}/${manifestName}`;
        const manifestText = manifestLines.join("\r\n") + "\r\n";
        await env.BUCKET.put(manifestKey, manifestText, {
          httpMetadata: { contentType: "text/plain; charset=utf-8" },
          customMetadata: { batch_id: batchId, name: manifestName, kind: "manifest", issuer: "upload" },
        });
        await env.DB.prepare("INSERT OR IGNORE INTO package_objects(batch_id,name,kind,r2_key,created_at) VALUES(?,?,?,?,?)")
          .bind(batchId, manifestName, "manifest", manifestKey, now)
          .run();

        const manifestUrl = await makePresignedGet(manifestKey);

        const creditsAfter = await env.DB.prepare("SELECT distribution_credits FROM uploader_stats WHERE upload_key_hash=?")
          .bind(u.hash)
          .first<{ distribution_credits: number }>();

        return json({
          ok: true,
          batch_id: batchId,
          type: "user",
          count_requested: count,
          count_issued: packages.length,
          label,
          server_url: serverUrl,
          ttl_minutes: ttlMinutes,
          expires_at: expiresAt,
          packages,
          manifest: { name: manifestName, download_url: manifestUrl },
          errors,
          credits_remaining: Number(creditsAfter?.distribution_credits || 0),
          valid_artworks_submitted: Number(creditsRow?.valid_artworks_submitted || 0),
        });
      }

      if (path === "/v1/refill/topup" && req.method === "POST") {
        const caller = await requireAtLeastUser(req, env);
        const body = await parseJson<RefillTopupBody>(req);

        const target = Math.max(1, Math.min(200, Math.trunc(Number(body?.target_pool_size || 0))));
        const reports = Array.isArray(body?.reports) ? body.reports : [];
        if (reports.length > 2000) return json({ ok: false, error: "too_many_items" }, 413);

        const receivedAt = utcNowIso();

        // 0) 先把“冷却到期”的 exhausted_accounts 释放回可用（invalid=0）
        // 注意：这是旧链路逻辑，直接以 eligible_after 判断即可。
        await env.DB.prepare(
          "UPDATE accounts SET invalid=0, invalid_at=NULL WHERE invalid=1 AND email_hash IN (SELECT email_hash FROM exhausted_accounts WHERE eligible_after <= ?)",
        )
          .bind(receivedAt)
          .run();
        await env.DB.prepare("DELETE FROM exhausted_accounts WHERE eligible_after <= ?").bind(receivedAt).run();

        // 1) 写入 probes 审计 + accounts 聚合（复用 probe-report 的逻辑形态，只是 caller 可能是 user/upload）
        let accepted = 0;
        const errors: Array<{ idx: number; error: string }> = [];

        for (let i = 0; i < reports.length; i++) {
          const it = reports[i];
          const emailHash = String(it?.email_hash || "").trim().toLowerCase();
          const accountId = it?.account_id ? String(it.account_id).trim() : null;
          const probedAt = String(it?.probed_at || "").trim();
          const fileName = it?.file_name ? String(it.file_name).slice(0, 200) : null;
          let statusCode = typeof it?.status_code === "number" ? Math.trunc(it.status_code) : null;

          if (!isHexSha256(emailHash)) {
            errors.push({ idx: i, error: "invalid email_hash (must be sha256 hex)" });
            continue;
          }
          if (!probedAt) {
            errors.push({ idx: i, error: "missing probed_at" });
            continue;
          }
          if (statusCode !== null && !Number.isFinite(statusCode)) statusCode = null;

          // probes 明细（保留 file_name 仅用于排障：写入 account_id 字段后面拼接）
          const accountIdForProbe = fileName ? `${accountId || ""}#${fileName}`.slice(0, 200) : accountId;
          await env.DB
            .prepare(
              "INSERT INTO probes(upload_key_hash,email_hash,account_id,status_code,probed_at,received_at) VALUES(?,?,?,?,?,?)",
            )
            .bind(caller.audit_key_hash, emailHash, accountIdForProbe, statusCode, probedAt, receivedAt)
            .run();

          // accounts 聚合（以 email_hash 去重）
          const existing = await env.DB.prepare("SELECT invalid FROM accounts WHERE email_hash=?")
            .bind(emailHash)
            .first<{ invalid: number }>();

          if (!existing) {
            await env.DB
              .prepare(
                "INSERT INTO accounts(email_hash,account_id,first_seen_at,last_seen_at,last_status_code,last_probed_at,invalid,invalid_at) VALUES(?,?,?,?,?,?,0,NULL)",
              )
              .bind(emailHash, accountId, receivedAt, receivedAt, statusCode, probedAt)
              .run();
          } else {
            await env.DB
              .prepare(
                "UPDATE accounts SET account_id=COALESCE(?,account_id), last_seen_at=?, last_status_code=?, last_probed_at=? WHERE email_hash=?",
              )
              .bind(accountId, receivedAt, statusCode, probedAt, emailHash)
              .run();
          }

          // invalid / exhausted
          if (statusCode === 401) {
            const first = await env.DB.prepare("SELECT first_invalid_at FROM invalid_accounts WHERE email_hash=?")
              .bind(emailHash)
              .first<{ first_invalid_at: string }>();

            if (!first) {
              await env.DB
                .prepare(
                  "INSERT INTO invalid_accounts(email_hash,account_id,first_invalid_at,last_invalid_at,last_status_code) VALUES(?,?,?,?,?)",
                )
                .bind(emailHash, accountId, probedAt, probedAt, statusCode)
                .run();
            } else {
              await env.DB
                .prepare(
                  "UPDATE invalid_accounts SET account_id=COALESCE(?,account_id), last_invalid_at=?, last_status_code=? WHERE email_hash=?",
                )
                .bind(accountId, probedAt, statusCode, emailHash)
                .run();
            }

            await env.DB
              .prepare("UPDATE accounts SET invalid=1, invalid_at=COALESCE(invalid_at, ?) WHERE email_hash=?")
              .bind(probedAt, emailHash)
              .run();
          }

          if (statusCode === 429) {
            const d = new Date(probedAt);
            if (!isNaN(d.getTime())) {
              const eligibleAfter = new Date(d.getTime() + 7 * 24 * 60 * 60 * 1000).toISOString().replace(/\.\d{3}Z$/, "Z");

              const ext = await env.DB.prepare("SELECT id FROM exhausted_accounts WHERE email_hash=?")
                .bind(emailHash)
                .first<{ id: number }>();

              if (!ext) {
                await env.DB
                  .prepare(
                    "INSERT INTO exhausted_accounts(email_hash,account_id,exhausted_at,last_status_code,last_probed_at,eligible_after) VALUES(?,?,?,?,?,?)",
                  )
                  .bind(emailHash, accountId, probedAt, statusCode, probedAt, eligibleAfter)
                  .run();
              } else {
                await env.DB
                  .prepare(
                    "UPDATE exhausted_accounts SET account_id=COALESCE(?,account_id), exhausted_at=?, last_status_code=?, last_probed_at=?, eligible_after=? WHERE email_hash=?",
                  )
                  .bind(accountId, probedAt, statusCode, probedAt, eligibleAfter, emailHash)
                  .run();
              }

              await env.DB
                .prepare("UPDATE accounts SET invalid=1, invalid_at=COALESCE(invalid_at, ?) WHERE email_hash=?")
                .bind(probedAt, emailHash)
                .run();
            }
          }

          accepted++;
        }

        // 2) 下发替换账号（从 accounts 表中挑选：invalid=0 且 auth_json 非空，且不在 exhausted 冷却期）
        const want = Math.max(1, Math.min(50, target));

        const issuedCutoff = isoMinutesAgo(10, new Date(Date.parse(receivedAt)));

        const rows = await env.DB
          .prepare(
            "SELECT email_hash, account_id, auth_json FROM accounts WHERE invalid=0 AND has_auth_json=1 AND auth_json IS NOT NULL AND NOT EXISTS (SELECT 1 FROM exhausted_accounts ea WHERE ea.email_hash=accounts.email_hash AND ea.eligible_after > ?) AND NOT EXISTS (SELECT 1 FROM topup_issues ti WHERE ti.email_hash=accounts.email_hash AND ti.issued_at > ?) ORDER BY last_seen_at DESC LIMIT ?",
          )
          .bind(receivedAt, issuedCutoff, want)
          .all<{ email_hash: string; account_id: string | null; auth_json: string }>();

        const accountsOut: Array<{ file_name: string; auth_json: unknown }> = [];
        const issueErrors: Array<{ email_hash: string; error: string }> = [];
        const stamp = receivedAt.replace(/[-:TZ]/g, "");

        for (let i = 0; i < (rows.results || []).length; i++) {
          const r = (rows.results || [])[i];
          const fileName = `无限续杯-${stamp}-${String(i + 1).padStart(3, "0")}.json`;

          try {
            const plain = await accountsAuthJsonDecrypt(env, String(r.auth_json));
            const auth = JSON.parse(plain);

            // 下发审计（soft lease：用于短时间内避免重复下发同一账号）
            await env.DB.prepare(
              "INSERT INTO topup_issues(email_hash,issued_at,issued_to_key_hash,issued_to_role,request_received_at) VALUES(?,?,?,?,?)",
            )
              .bind(String(r.email_hash || ""), receivedAt, caller.audit_key_hash, caller.role, receivedAt)
              .run();

            accountsOut.push({ file_name: fileName, auth_json: auth });
          } catch (e: any) {
            issueErrors.push({ email_hash: String(r.email_hash || ""), error: String(e?.message || e) });
          }
        }

        return json({
          ok: true,
          target_pool_size: target,
          accepted_reports: accepted,
          errors,
          issue_errors: issueErrors,
          accounts: accountsOut,
          received_at: receivedAt,
        });
      }

      if (path === "/v1/refill-keys/claim" && req.method === "POST") {
        const caller = await requireAtLeastUser(req, env);
        const now = utcNowIso();

        const row = await env.DB.prepare("SELECT id,key_enc_b64 FROM refill_keys WHERE status='available' ORDER BY id ASC LIMIT 1").first<{
          id: number;
          key_enc_b64: string;
        }>();

        if (!row) return json({ ok: false, error: "no_available_refill_key" }, 409);

        // 尝试 claim（乐观并发：用 status 条件防止重复 claim）
        const claimedByUser = caller.role === "user" ? caller.user_key_hash || null : null;
        const claimedByUpload = caller.role === "upload" ? caller.upload_key_hash || null : caller.role === "admin" ? caller.audit_key_hash : null;

        const res = await env.DB
          .prepare(
            "UPDATE refill_keys SET status='claimed', claimed_by_user_key_hash=?, claimed_by_upload_key_hash=?, claimed_at=? WHERE id=? AND status='available'",
          )
          .bind(claimedByUser, claimedByUpload, now, row.id)
          .run();

        if (res.meta.changes !== 1) {
          return json({ ok: false, error: "race_lost_try_again" }, 409);
        }

        const refillKey = await aesGcmDecryptFromB64(env.REFILL_KEYS_MASTER_KEY_B64, row.key_enc_b64);
        return json({ ok: true, refill_key: refillKey, claimed_at: now });
      }

      if (path === "/v1/accounts/register" && req.method === "POST") {
        const caller = await requireAtLeastUpload(req, env);
        const body = await parseJson<RegisterAccountsBody>(req);
        const items = Array.isArray(body.accounts) ? body.accounts : [];

        if (items.length > 2000) return json({ ok: false, error: "too_many_items" }, 413);

        const receivedAt = utcNowIso();
        let accepted = 0;
        let storedAuthJson = 0;
        const errors: Array<{ idx: number; error: string }> = [];

        for (let i = 0; i < items.length; i++) {
          const it = items[i];
          const emailHash = String(it?.email_hash || "").trim().toLowerCase();
          const accountId = it?.account_id ? String(it.account_id).trim() : null;
          const seenAt = String(it?.seen_at || "").trim();
          const authJsonRaw = (it as any)?.auth_json;

          if (!isHexSha256(emailHash)) {
            errors.push({ idx: i, error: "invalid email_hash (must be sha256 hex)" });
            continue;
          }
          if (!seenAt) {
            errors.push({ idx: i, error: "missing seen_at" });
            continue;
          }

          // 可选：auth_json（敏感）。仅用于旧 topup 链路下发。
          // - 支持 string 或 object
          // - 限制体积，防止 D1 被塞爆
          let authJsonEnc: string | null = null;
          if (authJsonRaw !== null && authJsonRaw !== undefined) {
            const plain = normalizeAuthJsonToString(authJsonRaw);
            if (plain.length > 64 * 1024) {
              errors.push({ idx: i, error: "auth_json too large (max 64KB)" });
              continue;
            }
            try {
              authJsonEnc = await accountsAuthJsonEncrypt(env, plain);
              storedAuthJson++;
            } catch (e: any) {
              errors.push({ idx: i, error: `auth_json encrypt failed: ${String(e?.message || e)}` });
              continue;
            }
          }

          const now = utcNowIso();
          const existing = await env.DB.prepare("SELECT 1 as ok FROM accounts WHERE email_hash=?")
            .bind(emailHash)
            .first<{ ok: number }>();

          if (!existing) {
            await env.DB
              .prepare(
                "INSERT INTO accounts(email_hash,account_id,first_seen_at,last_seen_at,last_status_code,last_probed_at,invalid,invalid_at,has_auth_json,auth_json) VALUES(?,?,?,?,NULL,NULL,0,NULL,?,?)",
              )
              .bind(emailHash, accountId, now, now, authJsonEnc ? 1 : 0, authJsonEnc)
              .run();
          } else {
            await env.DB
              .prepare(
                "UPDATE accounts SET account_id=COALESCE(?,account_id), last_seen_at=?, has_auth_json=CASE WHEN ? IS NOT NULL THEN 1 ELSE has_auth_json END, auth_json=COALESCE(?,auth_json) WHERE email_hash=?",
              )
              .bind(accountId, now, authJsonEnc, authJsonEnc, emailHash)
              .run();
          }

          // 审计：register 也写入 probes 表（status_code=NULL）便于统计“上传行为”
          await env.DB
            .prepare(
              "INSERT INTO probes(upload_key_hash,email_hash,account_id,status_code,probed_at,received_at) VALUES(?,?,?,?,?,?)",
            )
            .bind(caller.audit_key_hash, emailHash, accountId, null, seenAt, receivedAt)
            .run();

          accepted++;
        }

        return json({ ok: true, accepted, stored_auth_json: storedAuthJson, errors, received_at: receivedAt });
      }

      if (path === "/v1/probe-report" && req.method === "POST") {
        const caller = await requireAtLeastUpload(req, env);
        const body = await parseJson<ProbeReportBody>(req);
        const items = Array.isArray(body.reports) ? body.reports : [];

        if (items.length > 2000) return json({ ok: false, error: "too_many_items" }, 413);

        const receivedAt = utcNowIso();
        let accepted = 0;
        const errors: Array<{ idx: number; error: string }> = [];

        for (let i = 0; i < items.length; i++) {
          const it = items[i];
          const emailHash = String(it?.email_hash || "").trim().toLowerCase();
          const accountId = it?.account_id ? String(it.account_id).trim() : null;
          let statusCode = typeof it?.status_code === "number" ? Math.trunc(it.status_code) : null;
          const probedAt = String(it?.probed_at || "").trim();

          if (!isHexSha256(emailHash)) {
            errors.push({ idx: i, error: "invalid email_hash (must be sha256 hex)" });
            continue;
          }
          if (!probedAt) {
            errors.push({ idx: i, error: "missing probed_at" });
            continue;
          }


          // 1) 插入 probes 明细
          await env.DB
            .prepare(
              "INSERT INTO probes(upload_key_hash,email_hash,account_id,status_code,probed_at,received_at) VALUES(?,?,?,?,?,?)",
            )
            .bind(caller.audit_key_hash, emailHash, accountId, statusCode, probedAt, receivedAt)
            .run();

          // 2) upsert accounts（以 email_hash 去重）
          const now = utcNowIso();
          const existing = await env.DB
            .prepare("SELECT invalid FROM accounts WHERE email_hash=?")
            .bind(emailHash)
            .first<{ invalid: number }>();

          if (!existing) {
            await env.DB
              .prepare(
                "INSERT INTO accounts(email_hash,account_id,first_seen_at,last_seen_at,last_status_code,last_probed_at,invalid,invalid_at) VALUES(?,?,?,?,?,?,0,NULL)",
              )
              .bind(emailHash, accountId, now, now, statusCode, probedAt)
              .run();
          } else {
            await env.DB
              .prepare(
                "UPDATE accounts SET account_id=COALESCE(?,account_id), last_seen_at=?, last_status_code=?, last_probed_at=? WHERE email_hash=?",
              )
              .bind(accountId, now, statusCode, probedAt, emailHash)
              .run();
          }

          // 3) invalid 库
          if (statusCode === 401) {
            const first = await env.DB.prepare("SELECT first_invalid_at FROM invalid_accounts WHERE email_hash=?")
              .bind(emailHash)
              .first<{ first_invalid_at: string }>();

            if (!first) {
              await env.DB
                .prepare(
                  "INSERT INTO invalid_accounts(email_hash,account_id,first_invalid_at,last_invalid_at,last_status_code) VALUES(?,?,?,?,?)",
                )
                .bind(emailHash, accountId, probedAt, probedAt, statusCode)
                .run();
            } else {
              await env.DB
                .prepare("UPDATE invalid_accounts SET account_id=COALESCE(?,account_id), last_invalid_at=?, last_status_code=? WHERE email_hash=?")
                .bind(accountId, probedAt, statusCode, emailHash)
                .run();
            }

            await env.DB
              .prepare("UPDATE accounts SET invalid=1, invalid_at=COALESCE(invalid_at, ?) WHERE email_hash=?")
              .bind(probedAt, emailHash)
              .run();
          }

          // 4) exhausted 库（周限额暂存）
          if (statusCode === 429) {
            // 计算 7 天后的时间：eligible_after
            const probedDate = new Date(probedAt);
            if (!isNaN(probedDate.getTime())) {
              const eligibleAfter = new Date(probedDate.getTime() + 7 * 24 * 60 * 60 * 1000).toISOString().replace(/\.\d{3}Z$/, "Z");
              
              const ext = await env.DB.prepare("SELECT id FROM exhausted_accounts WHERE email_hash=?")
                .bind(emailHash)
                .first<{ id: number }>();

              if (!ext) {
                await env.DB
                  .prepare(
                    "INSERT INTO exhausted_accounts(email_hash,account_id,exhausted_at,last_status_code,last_probed_at,eligible_after) VALUES(?,?,?,?,?,?)",
                  )
                  .bind(emailHash, accountId, probedAt, statusCode, probedAt, eligibleAfter)
                  .run();
              } else {
                await env.DB
                  .prepare("UPDATE exhausted_accounts SET account_id=COALESCE(?,account_id), exhausted_at=?, last_status_code=?, last_probed_at=?, eligible_after=? WHERE email_hash=?")
                  .bind(accountId, probedAt, statusCode, probedAt, eligibleAfter, emailHash)
                  .run();
              }

              // 在 accounts 表中将其标记为 invalid，以便它被客户端和普通选取逻辑视为无效
              await env.DB
                .prepare("UPDATE accounts SET invalid=1, invalid_at=COALESCE(invalid_at, ?) WHERE email_hash=?")
                .bind(probedAt, emailHash)
                .run();
            }
          }


          accepted++;
        }

        return json({ ok: true, accepted, errors, received_at: receivedAt });
      }

      return json({ ok: false, error: "not_found" }, 404);
    } catch (e: any) {
      if (e instanceof Response) return e;
      return json({ ok: false, error: String(e?.message || e) }, 500);
    }
  },
};
