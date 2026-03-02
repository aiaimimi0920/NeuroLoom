/// <reference types="@cloudflare/workers-types" />

import { AwsClient } from "aws4fetch";
import { BlobReader, BlobWriter, TextReader, ZipWriter } from "@zip.js/zip.js";

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

async function ensureRuntimeSchema(env: Env): Promise<void> {
  await env.DB.prepare("CREATE TABLE IF NOT EXISTS system_settings (k TEXT PRIMARY KEY, v TEXT NOT NULL, updated_at TEXT NOT NULL)").run();
  await env.DB.prepare("CREATE TABLE IF NOT EXISTS user_daily_sync_all_usage (id INTEGER PRIMARY KEY AUTOINCREMENT, user_id TEXT NOT NULL, day TEXT NOT NULL, sync_count INTEGER NOT NULL DEFAULT 0, updated_at TEXT NOT NULL)").run();
  await env.DB.prepare("CREATE UNIQUE INDEX IF NOT EXISTS idx_user_daily_sync_all_usage_user_day_uq ON user_daily_sync_all_usage(user_id, day)").run();
  await env.DB.prepare("CREATE INDEX IF NOT EXISTS idx_user_daily_sync_all_usage_user_day ON user_daily_sync_all_usage(user_id, day)").run();
  await env.DB.prepare("CREATE UNIQUE INDEX IF NOT EXISTS idx_user_daily_refill_usage_user_day_uq ON user_daily_refill_usage(user_id, day)").run();
  await env.DB.prepare("CREATE TABLE IF NOT EXISTS sync_all_risk_events (id INTEGER PRIMARY KEY AUTOINCREMENT, day TEXT NOT NULL, ip TEXT NOT NULL, key_hash TEXT NOT NULL, ua_hash TEXT NOT NULL, fingerprint TEXT NOT NULL, auth_ok INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL)").run();
  await env.DB.prepare("CREATE INDEX IF NOT EXISTS idx_sync_all_risk_events_day_ip ON sync_all_risk_events(day, ip)").run();
  await env.DB.prepare("CREATE INDEX IF NOT EXISTS idx_sync_all_risk_events_day_ua ON sync_all_risk_events(day, ua_hash)").run();
  await env.DB.prepare("CREATE INDEX IF NOT EXISTS idx_sync_all_risk_events_day_fp ON sync_all_risk_events(day, fingerprint)").run();
  await env.DB.prepare("CREATE TABLE IF NOT EXISTS risk_blacklist (subject_type TEXT NOT NULL, subject_value TEXT NOT NULL, reason TEXT NOT NULL, created_at TEXT NOT NULL, expires_at TEXT, PRIMARY KEY(subject_type, subject_value))").run();

  try {
    await env.DB.prepare("ALTER TABLE users_v2 ADD COLUMN account_limit_delta INTEGER NOT NULL DEFAULT 0").run();
  } catch {
    // ignore when column already exists
  }
}

async function getBaseAccountLimit(env: Env): Promise<number> {
  const row = await env.DB.prepare("SELECT v FROM system_settings WHERE k='platform_base_account_limit'").first<{ v: string }>();
  const n = Math.trunc(Number(row?.v || "20"));
  return Math.max(1, Math.min(500, n || 20));
}

async function setBaseAccountLimit(env: Env, limit: number): Promise<number> {
  const v = Math.max(1, Math.min(500, Math.trunc(Number(limit || 0))));
  const now = utcNowIso();
  await env.DB
    .prepare("INSERT INTO system_settings(k,v,updated_at) VALUES('platform_base_account_limit',?,?) ON CONFLICT(k) DO UPDATE SET v=excluded.v, updated_at=excluded.updated_at")
    .bind(String(v), now)
    .run();
  return v;
}

async function getAbuseIssueMultiplier(env: Env): Promise<number> {
  const row = await env.DB.prepare("SELECT v FROM system_settings WHERE k='abuse_issue_multiplier'").first<{ v: string }>();
  const n = Number(row?.v || "5");
  if (!Number.isFinite(n)) return 5;
  return Math.max(1, Math.min(100, n));
}

async function setAbuseIssueMultiplier(env: Env, multiplier: number): Promise<number> {
  const v = Math.max(1, Math.min(100, Number(multiplier || 0)));
  const now = utcNowIso();
  await env.DB
    .prepare("INSERT INTO system_settings(k,v,updated_at) VALUES('abuse_issue_multiplier',?,?) ON CONFLICT(k) DO UPDATE SET v=excluded.v, updated_at=excluded.updated_at")
    .bind(String(v), now)
    .run();
  return v;
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

type BlacklistSubjectType = "ip" | "ua_hash" | "fingerprint";

async function isSubjectBlacklisted(
  env: Env,
  subjectType: BlacklistSubjectType,
  subjectValue: string,
  nowIso: string,
): Promise<{ blocked: boolean; reason?: string }> {
  const row = await env.DB
    .prepare("SELECT reason FROM risk_blacklist WHERE subject_type=? AND subject_value=? AND (expires_at IS NULL OR expires_at='' OR expires_at>?) LIMIT 1")
    .bind(subjectType, subjectValue, nowIso)
    .first<{ reason: string }>();
  if (!row) return { blocked: false };
  return { blocked: true, reason: String(row.reason || "") || `${subjectType}_blacklisted` };
}

async function upsertBlacklist(
  env: Env,
  subjectType: BlacklistSubjectType,
  subjectValue: string,
  reason: string,
  createdAt: string,
  expiresAt: string,
): Promise<void> {
  await env.DB
    .prepare("INSERT INTO risk_blacklist(subject_type,subject_value,reason,created_at,expires_at) VALUES(?,?,?,?,?) ON CONFLICT(subject_type,subject_value) DO UPDATE SET reason=excluded.reason, created_at=excluded.created_at, expires_at=excluded.expires_at")
    .bind(subjectType, subjectValue, reason, createdAt, expiresAt)
    .run();
}

async function recordSyncAllRiskEvent(
  env: Env,
  day: string,
  ip: string,
  keyHash: string,
  uaHash: string,
  fingerprint: string,
  authOk: boolean,
  createdAt: string,
): Promise<void> {
  await env.DB
    .prepare("INSERT INTO sync_all_risk_events(day,ip,key_hash,ua_hash,fingerprint,auth_ok,created_at) VALUES(?,?,?,?,?,?,?)")
    .bind(day, ip, keyHash, uaHash, fingerprint, authOk ? 1 : 0, createdAt)
    .run();
}

async function evaluateSyncAllRiskAndMaybeBlacklist(
  env: Env,
  day: string,
  ip: string,
  uaHash: string,
  fingerprint: string,
  nowIso: string,
): Promise<{ blockedNow: boolean; reason: string }> {
  const ipStats = await env.DB
    .prepare("SELECT COUNT(1) AS total, COUNT(DISTINCT key_hash) AS dk, SUM(CASE WHEN auth_ok=0 THEN 1 ELSE 0 END) AS failed FROM sync_all_risk_events WHERE day=? AND ip=?")
    .bind(day, ip)
    .first<{ total: number; dk: number; failed: number }>();
  const ipDistinctKeys = Number(ipStats?.dk || 0);
  const ipFailed = Number(ipStats?.failed || 0);

  const fpStats = await env.DB
    .prepare("SELECT COUNT(1) AS total, COUNT(DISTINCT key_hash) AS dk, SUM(CASE WHEN auth_ok=0 THEN 1 ELSE 0 END) AS failed FROM sync_all_risk_events WHERE day=? AND fingerprint=?")
    .bind(day, fingerprint)
    .first<{ total: number; dk: number; failed: number }>();
  const fpDistinctKeys = Number(fpStats?.dk || 0);
  const fpFailed = Number(fpStats?.failed || 0);

  const uaStats = await env.DB
    .prepare("SELECT COUNT(1) AS total, COUNT(DISTINCT key_hash) AS dk, SUM(CASE WHEN auth_ok=0 THEN 1 ELSE 0 END) AS failed FROM sync_all_risk_events WHERE day=? AND ua_hash=?")
    .bind(day, uaHash)
    .first<{ total: number; dk: number; failed: number }>();
  const uaDistinctKeys = Number(uaStats?.dk || 0);
  const uaFailed = Number(uaStats?.failed || 0);

  const expiresAt = new Date(Date.now() + 24 * 60 * 60 * 1000).toISOString().replace(/\.\d{3}Z$/, "Z");

  if (ipDistinctKeys >= 6 && ipFailed >= 10) {
    await upsertBlacklist(env, "ip", ip, `sync_all_key_spray_ip: distinct_keys=${ipDistinctKeys}, failed=${ipFailed}`, nowIso, expiresAt);
    await upsertBlacklist(env, "fingerprint", fingerprint, `sync_all_key_spray_fingerprint: distinct_keys=${fpDistinctKeys}, failed=${fpFailed}`, nowIso, expiresAt);
    return { blockedNow: true, reason: "sync_all_ip_key_spray" };
  }

  if (fpDistinctKeys >= 4 && fpFailed >= 8) {
    await upsertBlacklist(env, "fingerprint", fingerprint, `sync_all_key_spray_fingerprint: distinct_keys=${fpDistinctKeys}, failed=${fpFailed}`, nowIso, expiresAt);
    return { blockedNow: true, reason: "sync_all_fingerprint_key_spray" };
  }

  if (uaDistinctKeys >= 20 && uaFailed >= 30) {
    await upsertBlacklist(env, "ua_hash", uaHash, `sync_all_key_spray_ua: distinct_keys=${uaDistinctKeys}, failed=${uaFailed}`, nowIso, expiresAt);
    return { blockedNow: true, reason: "sync_all_ua_key_spray" };
  }

  return { blockedNow: false, reason: "" };
}

function extractR2KeyFromInput(raw: string | null | undefined, bucketName: string): string | null {
  const s = String(raw || "").trim();
  if (!s) return null;

  if (s.startsWith("r2://")) {
    const rest = s.slice(5);
    const slash = rest.indexOf("/");
    if (slash < 0) return null;
    const bucket = rest.slice(0, slash);
    const key = rest.slice(slash + 1);
    if (bucket !== bucketName) return null;
    return key || null;
  }

  if (/^https?:\/\//i.test(s)) {
    try {
      const u = new URL(s);
      const p = u.pathname.replace(/^\/+/, "");
      if (!p) return null;
      if (p.startsWith(`${bucketName}/`)) return p.slice(bucketName.length + 1);
      return null;
    } catch {
      return null;
    }
  }

  return s;
}

async function parseJson<T>(req: Request): Promise<T> {
  const ct = req.headers.get("content-type") || "";
  if (!ct.toLowerCase().includes("application/json")) {
    throw new Response("expected application/json", { status: 415 });
  }
  return (await req.json()) as T;
}

function inferAccountIdFromAuthObj(authObj: unknown): string | null {
  if (!authObj || typeof authObj !== "object") return null;
  const o = authObj as any;
  const direct = String(o.account_id || "").trim();
  if (direct) return direct;
  const nested = String(o?.["https://api.openai.com/auth"]?.chatgpt_account_id || "").trim();
  return nested || null;
}

function inferAccessTokenFromAuthObj(authObj: unknown): string | null {
  if (!authObj || typeof authObj !== "object") return null;
  const o = authObj as any;
  const direct = String(o.access_token || "").trim();
  if (direct) return direct;
  const nested = String(o?.tokens?.access_token || "").trim();
  return nested || null;
}

function whamUsageIsQuota0(obj: unknown): boolean {
  if (!obj || typeof obj !== "object") return false;
  const o = obj as any;
  const rl = o?.rate_limit;
  if (rl && typeof rl === "object") {
    if (rl.allowed === false) return true;
    if (rl.limit_reached === true) return true;
    const usedPercent = Number(rl?.primary_window?.used_percent);
    if (Number.isFinite(usedPercent) && usedPercent >= 100) return true;
  }

  for (const k of ["allowed", "limit_reached", "is_available"]) {
    const v = o?.[k];
    if (v === false || v === 0) return true;
  }
  return false;
}

async function probeWhamStatusFromR2(env: Env, accountId: string, r2Url: string | null | undefined): Promise<{ status: number | null; note: string }> {
  const key = extractR2KeyFromInput(String(r2Url || "").trim(), env.R2_BUCKET_NAME);
  if (!key) return { status: null, note: "missing_r2_key" };

  const obj = await env.BUCKET.get(key);
  if (!obj) return { status: null, note: "r2_object_not_found" };

  const authRaw = await obj.text();
  let authObj: unknown = {};
  try {
    authObj = authRaw ? JSON.parse(authRaw) : {};
  } catch {
    return { status: null, note: "bad_auth_json" };
  }

  const token = inferAccessTokenFromAuthObj(authObj);
  const accountFromAuth = inferAccountIdFromAuthObj(authObj) || accountId;
  if (!token || !accountFromAuth) return { status: null, note: "missing_account_id_or_access_token" };

  const resp = await fetch("https://chatgpt.com/backend-api/wham/usage", {
    method: "GET",
    headers: {
      Authorization: `Bearer ${token}`,
      "chatgpt-account-id": accountFromAuth,
      Accept: "application/json",
      originator: "codex_cli_rs",
    },
  });

  if (resp.status === 401) return { status: 401, note: "http401" };
  if (resp.status === 429) return { status: 429, note: "http429" };
  if (!resp.ok) return { status: null, note: `http${resp.status}` };

  try {
    const body = await resp.json();
    if (whamUsageIsQuota0(body)) return { status: 429, note: "quota0" };
  } catch {
    // ignore body parse error on 2xx
  }

  return { status: 200, note: "ok" };
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
  /** @deprecated 使用 max_accounts_per_user */
  bind_pool_size?: number;
  /** 每个新用户分配的最大账号数（用户可同时持有上限） */
  max_accounts_per_user?: number;
  /** 单个用户最少分配账号数；低于该值则该用户生成失败 */
  min_accounts_required?: number;
  /** 本批次用户统一账户上限加值（可负数） */
  account_limit_delta?: number;
  server_url: string;
  ttl_minutes?: number;
  /** 可选：再打一个“总包.zip”（里面包含本批次所有数字包 + key_manifest.json） */
  return_bundle_zip?: boolean;
  /**
   * ZIP 加密格式：
   * - zipcrypto: 传统 ZipCrypto（Windows 资源管理器兼容更好）
   * - aes: AES（安全更强，但资源管理器兼容性较差）
   */
  zip_encryption?: "zipcrypto" | "aes";
};

type UploaderIssuePackagesBody = {
  // uploader 只能发 user 包（给别人用的 USER_KEY）
  count: number;
  label?: string;
  server_url: string;
  ttl_minutes?: number;
};

type AdminUsersRebindBody = {
  /** 目标用户 key_hash 列表（最终每个 key 分到相同数量账号） */
  key_hashes: string[];
  /** 每个用户分配账号数，默认 50 */
  accounts_per_user?: number;
  /** 候选来源 owner 列表；为空时默认取“全部私有 owner” */
  source_owner_hashes?: string[];
  /** 是否把公有池(-1)也作为候选来源；默认 false */
  include_public_pool?: boolean;
  /** 仅预演不落库；默认 false */
  dry_run?: boolean;
  /** 兼容写入 user_keys 的标签前缀 */
  label_prefix?: string;
};

type AdminUsersCreateBody = {
  /** 目标用户 key_hash 列表（明文 key 先在客户端自行 sha256） */
  key_hashes: string[];
  /** 用户显示名前缀 */
  label_prefix?: string;
  /** 账户上限加值 */
  account_limit_delta?: number;
  /** 仅预演不落库 */
  dry_run?: boolean;
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

type AuthRepairSubmitFailedBody = {
  account_id: string;
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

// --- Refill v2（明文账号模型） ---

type RefillReportItemV2 = {
  account_id: string;
  status_code?: number;
  probed_at?: string;
  owner?: string | number;
  note?: string;
};

type RefillTopupBody = {
  target_pool_size: number;
  reports?: RefillReportItemV2[];
  /** 客户端建议需要服务端复核的账户 id 列表（服务端会二次 wham 快速校验） */
  account_ids?: string[];
};

type RegisterAccountItem = {
  account_id: string;
  email: string;
  password: string;
  r2_url?: string;
  /**
   * owner:
   * -1 公有池
   * -2 维修池
   * -3 墓地
   * -4 维修中
   * 其它字符串：私有池持有者 key_hash
   */
  owner?: string | number;
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
    await ensureRuntimeSchema(env);
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

        const accountsLegacyTotal = await env.DB.prepare("SELECT COUNT(1) as c FROM accounts").first<{ c: number }>();
        const accountsLegacyInvalid = await env.DB.prepare("SELECT COUNT(1) as c FROM accounts WHERE invalid=1").first<{ c: number }>();
        const accountsLegacyExhausted = await env.DB.prepare("SELECT COUNT(1) as c FROM exhausted_accounts").first<{ c: number }>();

        const accountsV2Total = await env.DB.prepare("SELECT COUNT(1) as c FROM accounts_v2").first<{ c: number }>();
        const accountsV2Public = await env.DB.prepare("SELECT COUNT(1) as c FROM accounts_v2 WHERE owner='-1'").first<{ c: number }>();
        const accountsV2Repair = await env.DB.prepare("SELECT COUNT(1) as c FROM accounts_v2 WHERE owner='-2'").first<{ c: number }>();
        const accountsV2Grave = await env.DB.prepare("SELECT COUNT(1) as c FROM accounts_v2 WHERE owner='-3'").first<{ c: number }>();
        const accountsV2Repairing = await env.DB.prepare("SELECT COUNT(1) as c FROM accounts_v2 WHERE owner='-4'").first<{ c: number }>();
        const usersV2Total = await env.DB.prepare("SELECT COUNT(1) as c FROM users_v2").first<{ c: number }>();
        const userKeysV2Total = await env.DB.prepare("SELECT COUNT(1) as c FROM user_keys_v2").first<{ c: number }>();
        const baseAccountLimit = await getBaseAccountLimit(env);
        const abuseIssueMultiplier = await getAbuseIssueMultiplier(env);

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

          accounts_legacy_total: Number(accountsLegacyTotal?.c || 0),
          accounts_legacy_invalid: Number(accountsLegacyInvalid?.c || 0),
          accounts_legacy_exhausted: Number(accountsLegacyExhausted?.c || 0),

          accounts_v2_total: Number(accountsV2Total?.c || 0),
          accounts_v2_public: Number(accountsV2Public?.c || 0),
          accounts_v2_repair: Number(accountsV2Repair?.c || 0),
          accounts_v2_grave: Number(accountsV2Grave?.c || 0),
          accounts_v2_repairing: Number(accountsV2Repairing?.c || 0),
          users_v2_total: Number(usersV2Total?.c || 0),
          user_keys_v2_total: Number(userKeysV2Total?.c || 0),
          platform_base_account_limit: baseAccountLimit,
          abuse_issue_multiplier: abuseIssueMultiplier,

          probes_last_24h: Number(probes24h?.c || 0),
          topup_issued_last_24h: Number(topupIssued24h?.c || 0),
          ts: utcNowIso(),
        });
      }

      if (path === "/admin/limits/base" && req.method === "GET") {
        await requireAdmin(req, env);
        const base = await getBaseAccountLimit(env);
        return json({ ok: true, platform_base_account_limit: base });
      }

      if (path === "/admin/limits/base" && req.method === "POST") {
        await requireAdmin(req, env);
        const body = await parseJson<{ platform_base_account_limit: number }>(req);
        const base = await setBaseAccountLimit(env, Number(body?.platform_base_account_limit || 20));
        return json({ ok: true, platform_base_account_limit: base });
      }

      if (path === "/admin/risk/abuse-multiplier" && req.method === "GET") {
        await requireAdmin(req, env);
        const multiplier = await getAbuseIssueMultiplier(env);
        return json({ ok: true, abuse_issue_multiplier: multiplier });
      }

      if (path === "/admin/risk/abuse-multiplier" && req.method === "POST") {
        await requireAdmin(req, env);
        const body = await parseJson<{ abuse_issue_multiplier: number }>(req);
        const multiplier = await setAbuseIssueMultiplier(env, Number(body?.abuse_issue_multiplier || 5));
        return json({ ok: true, abuse_issue_multiplier: multiplier });
      }

      if (path === "/admin/users/limit" && req.method === "POST") {
        await requireAdmin(req, env);
        const body = await parseJson<{ user_id?: string; key_hash?: string; limit_delta?: number; effective_limit?: number }>(req);

        const userIdRaw = String(body?.user_id || "").trim();
        const keyHashRaw = String(body?.key_hash || "").trim();

        let userId = userIdRaw;
        if (!userId && keyHashRaw) {
          const byKey = await env.DB.prepare("SELECT user_id FROM user_keys_v2 WHERE key_hash=?").bind(keyHashRaw).first<{ user_id: string }>();
          userId = String(byKey?.user_id || "").trim();
        }
        if (!userId) return json({ ok: false, error: "missing user_id_or_key_hash" }, 400);

        const base = await getBaseAccountLimit(env);
        let delta: number;
        if (body?.effective_limit !== undefined && body?.effective_limit !== null) {
          delta = Math.trunc(Number(body.effective_limit || 0)) - base;
        } else if (body?.limit_delta !== undefined && body?.limit_delta !== null) {
          delta = Math.trunc(Number(body.limit_delta || 0));
        } else {
          return json({ ok: false, error: "missing limit_delta_or_effective_limit" }, 400);
        }

        delta = Math.max(-500, Math.min(500, delta));
        const now = utcNowIso();
        const upd = await env.DB.prepare("UPDATE users_v2 SET account_limit_delta=?, updated_at=? WHERE id=?").bind(delta, now, userId).run();
        if ((upd.meta?.changes || 0) !== 1) return json({ ok: false, error: "user_not_found" }, 404);

        const effective = Math.max(1, Math.min(500, base + delta));
        return json({ ok: true, user_id: userId, platform_base_account_limit: base, account_limit_delta: delta, effective_account_limit: effective });
      }

      if (path === "/admin/users/unban" && req.method === "POST") {
        await requireAdmin(req, env);
        const body = await parseJson<{ user_id?: string; key_hash?: string }>(req);

        const userIdRaw = String(body?.user_id || "").trim();
        const keyHashRaw = String(body?.key_hash || "").trim();

        let userId = userIdRaw;
        if (!userId && keyHashRaw) {
          const byKey = await env.DB.prepare("SELECT user_id FROM user_keys_v2 WHERE key_hash=?").bind(keyHashRaw).first<{ user_id: string }>();
          userId = String(byKey?.user_id || "").trim();
        }
        if (!userId) return json({ ok: false, error: "missing user_id_or_key_hash" }, 400);

        const now = utcNowIso();
        const updUser = await env.DB.prepare("UPDATE users_v2 SET disabled=0, updated_at=? WHERE id=?").bind(now, userId).run();
        if ((updUser.meta?.changes || 0) !== 1) return json({ ok: false, error: "user_not_found" }, 404);

        await env.DB.prepare("UPDATE user_keys_v2 SET enabled=1, updated_at=? WHERE user_id=?").bind(now, userId).run();

        const keyStats = await env.DB.prepare("SELECT COUNT(1) AS c FROM user_keys_v2 WHERE user_id=? AND enabled=1").bind(userId).first<{ c: number }>();
        return json({ ok: true, user_id: userId, disabled: 0, enabled_keys: Number(keyStats?.c || 0) });
      }

      if (path === "/admin/users/rebind" && req.method === "POST") {
        await requireAdmin(req, env);
        const body = await parseJson<AdminUsersRebindBody>(req);

        const now = utcNowIso();
        const accountsPerUser = Math.max(1, Math.min(500, Math.trunc(Number(body?.accounts_per_user || 50))));
        const dryRun = body?.dry_run === true;
        const includePublicPool = body?.include_public_pool === true;
        const labelPrefix = String(body?.label_prefix || "rebind:user").trim() || "rebind:user";

        const targetHashes = [...new Set(
          (Array.isArray(body?.key_hashes) ? body.key_hashes : [])
            .map((x) => String(x || "").trim().toLowerCase())
            .filter((x) => isHexSha256(x)),
        )];
        if (targetHashes.length === 0) return json({ ok: false, error: "missing_key_hashes" }, 400);

        const reqTotal = targetHashes.length * accountsPerUser;

        let sourceOwners = [...new Set(
          (Array.isArray(body?.source_owner_hashes) ? body.source_owner_hashes : [])
            .map((x) => String(x || "").trim().toLowerCase())
            .filter((x) => isHexSha256(x)),
        )];

        if (sourceOwners.length === 0) {
          const allOwners = await env.DB
            .prepare("SELECT owner FROM accounts_v2 WHERE owner NOT IN ('-1','-2','-3','-4') GROUP BY owner ORDER BY owner")
            .all<{ owner: string }>();
          sourceOwners = (allOwners.results || []).map((r) => String(r.owner || "").trim().toLowerCase()).filter((x) => isHexSha256(x));
        }
        if (includePublicPool) sourceOwners = [...new Set([...sourceOwners, "-1"])];
        if (sourceOwners.length === 0) return json({ ok: false, error: "no_source_owners" }, 400);

        const srcPlaceholders = sourceOwners.map(() => "?").join(",");
        const srcRows = await env.DB
          .prepare(`SELECT account_id, owner FROM accounts_v2 WHERE owner IN (${srcPlaceholders}) ORDER BY updated_at DESC, account_id ASC`)
          .bind(...sourceOwners)
          .all<{ account_id: string; owner: string }>();

        const pool = (srcRows.results || []).map((r) => ({ account_id: String(r.account_id || "").trim(), owner: String(r.owner || "").trim() }))
          .filter((r) => r.account_id);

        if (pool.length < reqTotal) {
          return json({
            ok: false,
            error: "insufficient_source_accounts",
            requested_total: reqTotal,
            source_available: pool.length,
            target_users: targetHashes.length,
            accounts_per_user: accountsPerUser,
          }, 409);
        }

        const picked = pool.slice(0, reqTotal);
        const assignByKey = new Map<string, string[]>();
        const movePlan: Array<{ account_id: string; from_owner: string; to_owner: string }> = [];

        for (let i = 0; i < targetHashes.length; i++) assignByKey.set(targetHashes[i], []);

        for (let i = 0; i < picked.length; i++) {
          const keyHash = targetHashes[Math.floor(i / accountsPerUser)] || targetHashes[targetHashes.length - 1];
          const row = picked[i];
          assignByKey.get(keyHash)!.push(row.account_id);
          movePlan.push({ account_id: row.account_id, from_owner: row.owner, to_owner: keyHash });
        }

        if (dryRun) {
          return json({
            ok: true,
            dry_run: true,
            target_users: targetHashes.length,
            accounts_per_user: accountsPerUser,
            requested_total: reqTotal,
            source_owners: sourceOwners,
            source_available: pool.length,
            planned_moves: movePlan.length,
            sample_moves: movePlan.slice(0, 10),
            per_user_counts: targetHashes.map((h) => ({ key_hash: h, count: (assignByKey.get(h) || []).length })),
          });
        }

        const userIdByKey = new Map<string, string>();
        for (const h of targetHashes) {
          await env.DB.prepare("INSERT OR IGNORE INTO user_keys(key_hash,label,enabled,created_at) VALUES(?,?,1,?)")
            .bind(h, `${labelPrefix}:${h.slice(0, 8)}`, now)
            .run();
          await env.DB.prepare("UPDATE user_keys SET enabled=1 WHERE key_hash=?").bind(h).run();

          let uk = await env.DB.prepare("SELECT user_id FROM user_keys_v2 WHERE key_hash=? LIMIT 1").bind(h).first<{ user_id: string }>();
          if (!uk) {
            const userId = `u_${crypto.randomUUID()}`;
            await env.DB
              .prepare("INSERT INTO users_v2(id,display_name,roles,current_account_ids,daily_refill_limit,account_limit_delta,disabled,created_at,updated_at) VALUES(?,?,?,?,200,0,0,?,?)")
              .bind(userId, `${labelPrefix}:${h.slice(0, 8)}`, "user", "[]", now, now)
              .run();
            await env.DB
              .prepare("INSERT INTO user_keys_v2(key_hash,user_id,role,enabled,created_at,updated_at) VALUES(?,?,?,1,?,?)")
              .bind(h, userId, "user", now, now)
              .run();
            uk = { user_id: userId };
          } else {
            await env.DB.prepare("UPDATE user_keys_v2 SET enabled=1, updated_at=? WHERE key_hash=?").bind(now, h).run();
            await env.DB.prepare("UPDATE users_v2 SET disabled=0, updated_at=? WHERE id=?").bind(now, uk.user_id).run();
          }
          userIdByKey.set(h, uk.user_id);
        }

        for (const mv of movePlan) {
          await env.DB.prepare("UPDATE accounts_v2 SET owner=?, updated_at=? WHERE account_id=?")
            .bind(mv.to_owner, now, mv.account_id)
            .run();
        }

        for (const h of targetHashes) {
          const ids = assignByKey.get(h) || [];
          const uid = userIdByKey.get(h);
          if (!uid) continue;
          await env.DB.prepare("UPDATE users_v2 SET current_account_ids=?, updated_at=? WHERE id=?")
            .bind(JSON.stringify(ids), now, uid)
            .run();
        }

        const verifyRows = await env.DB
          .prepare(`SELECT owner, COUNT(1) AS c FROM accounts_v2 WHERE owner IN (${targetHashes.map(() => "?").join(",")}) GROUP BY owner ORDER BY owner`)
          .bind(...targetHashes)
          .all<{ owner: string; c: number }>();

        return json({
          ok: true,
          target_users: targetHashes.length,
          accounts_per_user: accountsPerUser,
          requested_total: reqTotal,
          moved_total: movePlan.length,
          source_available: pool.length,
          source_owners: sourceOwners,
          per_user_counts: (verifyRows.results || []).map((r) => ({ key_hash: String(r.owner || ""), count: Number(r.c || 0) })),
        });
      }

      if (path === "/admin/users/create" && req.method === "POST") {
        await requireAdmin(req, env);
        const body = await parseJson<AdminUsersCreateBody>(req);

        const now = utcNowIso();
        const labelPrefix = String(body?.label_prefix || "admin:user").trim() || "admin:user";
        const delta = Math.max(-500, Math.min(500, Math.trunc(Number(body?.account_limit_delta || 0))));
        const dryRun = body?.dry_run === true;

        const hashes = [...new Set(
          (Array.isArray(body?.key_hashes) ? body.key_hashes : [])
            .map((x) => String(x || "").trim().toLowerCase())
            .filter((x) => isHexSha256(x)),
        )];
        if (hashes.length === 0) return json({ ok: false, error: "missing_key_hashes" }, 400);

        const existingRows = await env.DB
          .prepare(`SELECT key_hash, user_id, enabled FROM user_keys_v2 WHERE key_hash IN (${hashes.map(() => "?").join(",")})`)
          .bind(...hashes)
          .all<{ key_hash: string; user_id: string; enabled: number }>();
        const existingByHash = new Map<string, { user_id: string; enabled: number }>();
        for (const r of existingRows.results || []) existingByHash.set(String(r.key_hash || "").toLowerCase(), { user_id: r.user_id, enabled: Number(r.enabled || 0) });

        if (dryRun) {
          const exists = hashes.filter((h) => existingByHash.has(h));
          const willCreate = hashes.filter((h) => !existingByHash.has(h));
          return json({
            ok: true,
            dry_run: true,
            total: hashes.length,
            exists_count: exists.length,
            create_count: willCreate.length,
            exists,
            will_create: willCreate,
            account_limit_delta: delta,
            label_prefix: labelPrefix,
          });
        }

        const created: Array<{ key_hash: string; user_id: string }> = [];
        const enabledExisting: Array<{ key_hash: string; user_id: string }> = [];

        for (const h of hashes) {
          await env.DB.prepare("INSERT OR IGNORE INTO user_keys(key_hash,label,enabled,created_at) VALUES(?,?,1,?)")
            .bind(h, `${labelPrefix}:${h.slice(0, 8)}`, now)
            .run();
          await env.DB.prepare("UPDATE user_keys SET enabled=1 WHERE key_hash=?").bind(h).run();

          const ex = existingByHash.get(h);
          if (ex) {
            await env.DB.prepare("UPDATE user_keys_v2 SET enabled=1, updated_at=? WHERE key_hash=?").bind(now, h).run();
            await env.DB.prepare("UPDATE users_v2 SET disabled=0, updated_at=? WHERE id=?").bind(now, ex.user_id).run();
            enabledExisting.push({ key_hash: h, user_id: ex.user_id });
            continue;
          }

          const userId = `u_${crypto.randomUUID()}`;
          await env.DB
            .prepare("INSERT INTO users_v2(id,display_name,roles,current_account_ids,daily_refill_limit,account_limit_delta,disabled,created_at,updated_at) VALUES(?,?,?,?,200,?,0,?,?)")
            .bind(userId, `${labelPrefix}:${h.slice(0, 8)}`, "user", "[]", delta, now, now)
            .run();
          await env.DB
            .prepare("INSERT INTO user_keys_v2(key_hash,user_id,role,enabled,created_at,updated_at) VALUES(?,?,?,1,?,?)")
            .bind(h, userId, "user", now, now)
            .run();
          created.push({ key_hash: h, user_id: userId });
        }

        return json({
          ok: true,
          total: hashes.length,
          created_count: created.length,
          enabled_existing_count: enabledExisting.length,
          created,
          enabled_existing: enabledExisting,
          account_limit_delta: delta,
          label_prefix: labelPrefix,
        });
      }

      if (path === "/admin/users/reset" && req.method === "POST") {
        await requireAdmin(req, env);

        const now = utcNowIso();
        const released = await env.DB
          .prepare("UPDATE accounts_v2 SET owner='-1', updated_at=? WHERE owner NOT IN ('-1','-2','-3','-4')")
          .bind(now)
          .run();

        const deletedUsage = await env.DB.prepare("DELETE FROM user_daily_refill_usage").run();
        const deletedUserKeys = await env.DB.prepare("DELETE FROM user_keys_v2").run();
        const deletedUsers = await env.DB.prepare("DELETE FROM users_v2").run();

        return json({
          ok: true,
          released_accounts: Number(released.meta?.changes || 0),
          deleted_usage_rows: Number(deletedUsage.meta?.changes || 0),
          deleted_user_keys: Number(deletedUserKeys.meta?.changes || 0),
          deleted_users: Number(deletedUsers.meta?.changes || 0),
          ts: now,
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
        const baseAccountLimit = await getBaseAccountLimit(env);
        let batchAccountLimitDelta = Math.trunc(Number(body?.account_limit_delta ?? 0) || 0);
        batchAccountLimitDelta = Math.max(-500, Math.min(500, batchAccountLimitDelta));

        let bindPoolSize = Math.max(1, Math.min(200, baseAccountLimit + batchAccountLimitDelta));
        if (body?.max_accounts_per_user !== undefined || body?.bind_pool_size !== undefined) {
          const explicit = Math.max(1, Math.min(200, Math.trunc(Number((body?.max_accounts_per_user ?? body?.bind_pool_size) || 0)) || 1));
          bindPoolSize = explicit;
          batchAccountLimitDelta = explicit - baseAccountLimit;
        }

        const minAccountsDefault = type === "user" ? bindPoolSize : 1;
        const minAccountsRequired = Math.max(
          1,
          Math.min(bindPoolSize, Math.trunc(Number(body?.min_accounts_required ?? minAccountsDefault) || minAccountsDefault)),
        );
        const serverUrl = String(body?.server_url || "").trim();
        const ttlMinutes = body?.ttl_minutes ? Math.max(1, Math.min(24 * 60, Math.trunc(Number(body.ttl_minutes)))) : 60;
        const returnBundleZip = body?.return_bundle_zip === true;
        const zipEncryption = String(body?.zip_encryption || "zipcrypto").trim().toLowerCase() === "aes" ? "aes" : "zipcrypto";
        const useZipCrypto = zipEncryption === "zipcrypto";

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

        await env.DB.prepare(
          "INSERT INTO package_batches(batch_id,key_type,count,label,bind_pool_size,server_url,created_at,expires_at) VALUES(?,?,?,?,?,?,?,?)",
        )
          .bind(batchId, type, count, label, bindPoolSize, serverUrl, now, expiresAt)
          .run();

        const packages: Array<{
          name: string;
          no: number;
          key: string;
          password: string;
          key_hash: string;
          user_id: string | null;
          assigned_count: number;
          download_url: string;
        }> = [];
        const errors: Array<{ idx: number; error: string }> = [];
        const manifestMap: Record<string, string> = {};
        const zipArtifacts: Array<{ name: string; blob: Blob }> = [];

        for (let i = 1; i <= count; i++) {
          const k = genKey();
          const h = await sha256Hex(k);
          const zipName = `${i}.zip`;
          const r2Key = `batches/${batchId}/${zipName}`;

          try {
            let assigned: Array<{ account_id: string; email: string; password: string; r2_url: string | null; auth_json_text: string | null }> = [];
            let userId: string | null = null;

            if (type === "user") {
              userId = `u_${crypto.randomUUID().replace(/-/g, "")}`;

              // 先尝试“独占抢占账号”，满足最小要求后再创建 user/user_key。
              const candidateRows = await env.DB
                .prepare(
                  "SELECT account_id,email,password,r2_url FROM accounts_v2 WHERE owner='-1' ORDER BY updated_at DESC LIMIT ?",
                )
                .bind(bindPoolSize * 4)
                .all<{ account_id: string; email: string; password: string; r2_url: string | null }>();

              for (const row of candidateRows.results || []) {
                if (assigned.length >= bindPoolSize) break;
                const claim = await env.DB
                  .prepare("UPDATE accounts_v2 SET owner=?, updated_at=?, last_refilled_at=? WHERE account_id=? AND owner='-1'")
                  .bind(h, now, now, row.account_id)
                  .run();
                if ((claim.meta?.changes || 0) === 1) {
                  let authJsonText: string | null = null;
                  const r2KeyForAuth = extractR2KeyFromInput(String(row.r2_url || "").trim(), env.R2_BUCKET_NAME);
                  if (r2KeyForAuth) {
                    const authObj = await env.BUCKET.get(r2KeyForAuth);
                    if (authObj) {
                      authJsonText = await authObj.text();
                    }
                  }

                  assigned.push({
                    account_id: String(row.account_id || ""),
                    email: String(row.email || ""),
                    password: String(row.password || ""),
                    r2_url: row.r2_url || null,
                    auth_json_text: authJsonText,
                  });
                }
              }

              if (assigned.length < minAccountsRequired) {
                // 不满足最小可用账号数：回滚抢占，且该用户不落库；按你的规则直接停止后续生成。
                for (const a of assigned) {
                  await env.DB.prepare("UPDATE accounts_v2 SET owner='-1', updated_at=? WHERE account_id=? AND owner=?")
                    .bind(now, a.account_id, h)
                    .run();
                }
                errors.push({ idx: i, error: `insufficient_accounts: need>=${minAccountsRequired}, got=${assigned.length}` });
                break;
              }

              await env.DB
                .prepare(
                  "INSERT OR IGNORE INTO users_v2(id,display_name,roles,current_account_ids,daily_refill_limit,account_limit_delta,disabled,created_at,updated_at) VALUES(?,?,?,?,200,?,0,?,?)",
                )
                .bind(userId, label, "user", "[]", batchAccountLimitDelta, now, now)
                .run();

              await env.DB
                .prepare(
                  "INSERT OR IGNORE INTO user_keys_v2(key_hash,user_id,role,enabled,created_at,updated_at) VALUES(?,?,?,1,?,?)",
                )
                .bind(h, userId, "user", now, now)
                .run();

              // 兼容 v1 鉴权：普通用户脚本使用 X-User-Key 命中 user_keys
              await env.DB
                .prepare("INSERT OR IGNORE INTO user_keys(key_hash,label,enabled,created_at) VALUES(?,?,1,?)")
                .bind(h, `pkg:${batchId}:${zipName}`, now)
                .run();

              const accountIdList = assigned.map((x) => x.account_id);
              await env.DB.prepare("UPDATE users_v2 SET current_account_ids=?, updated_at=? WHERE id=?")
                .bind(JSON.stringify(accountIdList), now, userId)
                .run();
            } else {
              await env.DB.prepare("INSERT OR IGNORE INTO upload_keys(key_hash,label,enabled,created_at) VALUES(?,?,1,?)")
                .bind(h, label, now)
                .run();
            }

            const zw = new ZipWriter(new BlobWriter("application/zip"));

            const envText = [
              `SERVER_URL=${serverUrl}`,
              type === "user" ? `USER_KEY=${k}` : `UPLOAD_KEY=${k}`,
              "ACCOUNTS_DIR=",
              "TARGET_POOL_SIZE=10",
              "TOTAL_HOLD_LIMIT=50",
              "INTERVAL_MINUTES=30",
              "SYNC_TARGET_DIR=",
            ].join("\n") + "\n";

            const readmeText = type === "user"
              ? [
                  "这是管理员手动分发包（用户专属）。",
                  `批次: ${batchId}`,
                  `包序号: ${i}`,
                  `绑定账户数: ${assigned.length}`,
                  "说明：本包中的账号 JSON 仅分配给当前用户 key，对应账号已在服务端标记 owner=该用户 key_hash。",
                ].join("\n")
              : [
                  "这是管理员手动分发包（upload）。",
                  `批次: ${batchId}`,
                  `包序号: ${i}`,
                ].join("\n");

            // 按要求：子包解压密码与包内 USER_KEY 保持一致（即用户 key 明文）
            const zipPassword = k;
            const encOpt = useZipCrypto ? { password: zipPassword, zipCrypto: true } : { password: zipPassword, encryptionStrength: 3 as const };

            await zw.add("无限续杯配置.env", new TextReader(envText), encOpt);
            await zw.add("README.txt", new TextReader(readmeText), encOpt);

            if (type === "user") {
              for (let j = 0; j < assigned.length; j++) {
                const a = assigned[j];
                const accountFileName = `accounts/${String(j + 1).padStart(3, "0")}-${a.account_id}.json`;
                if (a.auth_json_text && a.auth_json_text.trim()) {
                  await zw.add(accountFileName, new TextReader(a.auth_json_text), encOpt);
                } else {
                  const fallbackObj = {
                    account_id: a.account_id,
                    email: a.email,
                    password: a.password,
                  };
                  await zw.add(accountFileName, new TextReader(JSON.stringify(fallbackObj, null, 2)), encOpt);
                }
              }
            }

            const zipBlob = await zw.close();

            await env.BUCKET.put(r2Key, zipBlob, {
              httpMetadata: { contentType: "application/zip" },
              customMetadata: {
                batch_id: batchId,
                name: zipName,
                kind: "zip",
                type,
                user_id: userId || "",
                assigned_count: String(assigned.length),
              },
            });

            await env.DB.prepare(
              "INSERT OR IGNORE INTO package_objects(batch_id,name,kind,r2_key,created_at) VALUES(?,?,?,?,?)",
            )
              .bind(batchId, zipName, "zip", r2Key, now)
              .run();

            if (returnBundleZip) {
              zipArtifacts.push({ name: zipName, blob: zipBlob });
            }

            const downloadUrl = await makePresignedGet(r2Key);
            packages.push({
              name: zipName,
              no: i,
              key: k,
              password: zipPassword,
              key_hash: h,
              user_id: userId,
              assigned_count: assigned.length,
              download_url: downloadUrl,
            });
            manifestMap[zipName] = zipPassword;
          } catch (e: any) {
            errors.push({ idx: i, error: String(e?.message || e) });
          }
        }

        const manifestName = "key_manifest.json";
        const manifestKey = `batches/${batchId}/${manifestName}`;
        const manifestPayload = {
          batch_id: batchId,
          type,
          server_url: serverUrl,
          bind_pool_size: bindPoolSize,
          mapping: manifestMap,
        };

        await env.BUCKET.put(manifestKey, JSON.stringify(manifestPayload, null, 2), {
          httpMetadata: { contentType: "application/json; charset=utf-8" },
          customMetadata: { batch_id: batchId, name: manifestName, kind: "manifest", type },
        });
        await env.DB.prepare(
          "INSERT OR IGNORE INTO package_objects(batch_id,name,kind,r2_key,created_at) VALUES(?,?,?,?,?)",
        )
          .bind(batchId, manifestName, "manifest", manifestKey, now)
          .run();

        const manifestUrl = await makePresignedGet(manifestKey);

        let bundle: { name: string; download_url: string } | null = null;
        if (returnBundleZip && zipArtifacts.length > 0) {
          const bundleName = "packages.bundle.zip";
          const bundleKey = `batches/${batchId}/${bundleName}`;
          const bundleZip = new ZipWriter(new BlobWriter("application/zip"));

          for (const z of zipArtifacts) {
            await bundleZip.add(z.name, new BlobReader(z.blob));
          }
          await bundleZip.add(manifestName, new TextReader(JSON.stringify(manifestPayload, null, 2)));

          const bundleBlob = await bundleZip.close();
          await env.BUCKET.put(bundleKey, bundleBlob, {
            httpMetadata: { contentType: "application/zip" },
            customMetadata: { batch_id: batchId, name: bundleName, kind: "bundle", type },
          });
          await env.DB.prepare(
            "INSERT OR IGNORE INTO package_objects(batch_id,name,kind,r2_key,created_at) VALUES(?,?,?,?,?)",
          )
            .bind(batchId, bundleName, "bundle", bundleKey, now)
            .run();

          bundle = { name: bundleName, download_url: await makePresignedGet(bundleKey) };
        }

        return json({
          ok: true,
          batch_id: batchId,
          type,
          count_requested: count,
          count_issued: packages.length,
          label,
          bind_pool_size: bindPoolSize,
          max_accounts_per_user: bindPoolSize,
          min_accounts_required: minAccountsRequired,
          account_limit_delta: batchAccountLimitDelta,
          platform_base_account_limit: baseAccountLimit,
          server_url: serverUrl,
          ttl_minutes: ttlMinutes,
          expires_at: expiresAt,
          zip_encryption: zipEncryption,
          packages,
          manifest: {
            name: manifestName,
            download_url: manifestUrl,
            mapping: manifestMap,
            lines: Object.entries(manifestMap).map(([pkgName, passwd]) => `${pkgName}：${passwd}`),
          },
          bundle,
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
              "ACCOUNTS_DIR=",
              "TARGET_POOL_SIZE=10",
              "TOTAL_HOLD_LIMIT=50",
              "INTERVAL_MINUTES=30",
              "SYNC_TARGET_DIR=",
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
        const requestedAccountIds = Array.isArray(body?.account_ids)
          ? [...new Set(body.account_ids.map((x) => String(x || "").trim()).filter((x) => /^[A-Za-z0-9_-]{3,128}$/.test(x)))].slice(0, 200)
          : [];
        if (reports.length > 2000) return json({ ok: false, error: "too_many_items" }, 413);

        const receivedAt = utcNowIso();
        const day = receivedAt.slice(0, 10);
        const ownerKey = (caller.user_key_hash || caller.upload_key_hash || caller.audit_key_hash || "").trim();
        if (!ownerKey) return json({ ok: false, error: "missing_owner_key" }, 401);

        // 1) 绑定 caller 到 users_v2 / user_keys_v2，并执行禁用校验
        const now = receivedAt;
        const roleForKey = caller.role === "admin" ? "super_admin" : caller.role === "upload" ? "uploader" : "user";

        let userKeyRow = await env.DB
          .prepare("SELECT user_id, role, enabled FROM user_keys_v2 WHERE key_hash=?")
          .bind(ownerKey)
          .first<{ user_id: string; role: string; enabled: number }>();

        if (!userKeyRow) {
          const userId = `u_${crypto.randomUUID()}`;
          const defaultRoles = roleForKey;
          await env.DB
            .prepare(
              "INSERT INTO users_v2(id,display_name,roles,current_account_ids,daily_refill_limit,disabled,created_at,updated_at) VALUES(?,?,?,?,200,0,?,?)",
            )
            .bind(userId, null, defaultRoles, "[]", now, now)
            .run();

          await env.DB
            .prepare(
              "INSERT INTO user_keys_v2(key_hash,user_id,role,enabled,created_at,updated_at) VALUES(?,?,?,1,?,?)",
            )
            .bind(ownerKey, userId, roleForKey, now, now)
            .run();

          userKeyRow = { user_id: userId, role: roleForKey, enabled: 1 };
        }

        if (Number(userKeyRow.enabled) !== 1) {
          return json({ ok: false, error: "user_key_disabled" }, 403);
        }

        const userRow = await env.DB
          .prepare("SELECT disabled,daily_refill_limit,account_limit_delta FROM users_v2 WHERE id=?")
          .bind(userKeyRow.user_id)
          .first<{ disabled: number; daily_refill_limit: number; account_limit_delta: number }>();

        if (!userRow) return json({ ok: false, error: "user_not_found" }, 403);
        if (Number(userRow.disabled) === 1) return json({ ok: false, error: "user_disabled" }, 403);

        // 2) 消费 report：根据探测状态更新 owner
        let acceptedReports = 0;
        const reportErrors: Array<{ idx: number; error: string }> = [];

        for (let i = 0; i < reports.length; i++) {
          const it = reports[i] as RefillReportItemV2;
          const accountId = String(it?.account_id || "").trim();
          const statusCode = typeof it?.status_code === "number" ? Math.trunc(it.status_code) : null;

          if (!accountId) {
            reportErrors.push({ idx: i, error: "missing account_id" });
            continue;
          }

          let ownerNext: string | null = null;
          if (statusCode === 401) ownerNext = "-2";
          else if (statusCode === 429) ownerNext = "-4";
          else if (statusCode !== null && statusCode >= 200 && statusCode < 300) ownerNext = "-1";

          if (ownerNext !== null) {
            await env.DB
              .prepare("UPDATE accounts_v2 SET owner=?, updated_at=?, last_seen_at=? WHERE account_id=?")
              .bind(ownerNext, now, now, accountId)
              .run();
          }

          acceptedReports++;
        }

        // 3) 取号：仅发放公有池(-1)和本人私有池(owner==ownerKey)，墓地(-3)永不下发
        const baseAccountLimit = await getBaseAccountLimit(env);
        const delta = Math.trunc(Number(userRow.account_limit_delta || 0));
        const effectiveAccountLimit = Math.max(1, Math.min(500, baseAccountLimit + delta));
        const abuseIssueMultiplier = await getAbuseIssueMultiplier(env);
        const abuseIssueThreshold = Math.max(1, Math.floor(abuseIssueMultiplier * effectiveAccountLimit));

        const ownedRow = await env.DB.prepare("SELECT COUNT(1) AS c FROM accounts_v2 WHERE owner=?").bind(ownerKey).first<{ c: number }>();
        const currentOwned = Number(ownedRow?.c || 0);

        const rawWant = Math.max(1, Math.min(500, target));

        // 4) 服务端二次复核客户端提交的 account_ids：判定是否需要续杯（401/429）
        // 额外规则：若 note=r2_object_not_found，直接硬删除该“JSON凭证账户”（accounts_v2），并记审计。
        const replacedFromRequested: Array<{ old_account_id: string; reason: string }> = [];
        const deletedMissingR2FromRequested: Array<{ old_account_id: string; reason: string }> = [];
        if (requestedAccountIds.length > 0) {
          for (const accountId of requestedAccountIds) {
            const owned = await env.DB
              .prepare("SELECT account_id,r2_url FROM accounts_v2 WHERE account_id=? AND owner=?")
              .bind(accountId, ownerKey)
              .first<{ account_id: string; r2_url: string | null }>();
            if (!owned) continue;

            let status: number | null = null;
            let note = "";
            try {
              const p = await probeWhamStatusFromR2(env, owned.account_id, owned.r2_url);
              status = p.status;
              note = p.note;
            } catch (e: any) {
              status = null;
              note = `probe_error:${String(e?.message || e)}`;
            }

            if (note === "r2_object_not_found") {
              await env.DB.prepare("DELETE FROM accounts_v2 WHERE account_id=?")
                .bind(owned.account_id)
                .run();
              deletedMissingR2FromRequested.push({ old_account_id: owned.account_id, reason: note });
              continue;
            }

            if (status === 401 || status === 429) {
              const repairOwner = status === 401 ? "-2" : "-4";
              await env.DB.prepare("UPDATE accounts_v2 SET owner=?, updated_at=?, last_seen_at=? WHERE account_id=? AND owner=?")
                .bind(repairOwner, now, now, owned.account_id, ownerKey)
                .run();
              replacedFromRequested.push({ old_account_id: owned.account_id, reason: `${status}:${note || "need_refill"}` });
            }
          }
        }

        // 复核后重新计算持有量与容量（支持“同量替换”）
        const ownedRowNow = await env.DB.prepare("SELECT COUNT(1) AS c FROM accounts_v2 WHERE owner=?").bind(ownerKey).first<{ c: number }>();
        const currentOwnedNow = Number(ownedRowNow?.c || 0);
        const remainingCapacityNow = Math.max(0, effectiveAccountLimit - currentOwnedNow);

        // 分发策略：
        // - 未携带 account_ids：直接补到当前可持有上限
        // - 携带 account_ids：先剔除坏号，再按“剔除后持有量”补到当前可持有上限
        //   （若上限被下调且仍超持，则仅删除不补发）
        const desiredToLimit = Math.max(0, effectiveAccountLimit - currentOwnedNow);
        const wantFinal = Math.max(0, Math.min(500, desiredToLimit));

        if (wantFinal <= 0) {
          const overHeld = currentOwnedNow > effectiveAccountLimit;
          return json({
            ok: true,
            target_pool_size: target,
            total_hold_limit: effectiveAccountLimit,
            accepted_reports: acceptedReports,
            requested_account_ids: requestedAccountIds,
            replaced_from_requested: replacedFromRequested,
            issued_count: 0,
            used_today: Number((await env.DB.prepare("SELECT refilled_count FROM user_daily_refill_usage WHERE user_id=? AND day=?").bind(userKeyRow.user_id, day).first<{ refilled_count: number }>())?.refilled_count || 0),
            daily_limit: Math.max(1, Number(userRow.daily_refill_limit || 200)),
            auto_disabled: false,
            errors: reportErrors,
            issue_errors: [{
              account_id: "",
              error: overHeld
                ? `account_limit_overheld_no_refill: owned=${currentOwnedNow}, limit=${effectiveAccountLimit}`
                : `account_limit_reached: owned=${currentOwnedNow}, limit=${effectiveAccountLimit}`,
            }],
            account_limit: {
              platform_base_account_limit: baseAccountLimit,
              account_limit_delta: delta,
              effective_account_limit: effectiveAccountLimit,
              current_owned: currentOwnedNow,
              abuse_issue_multiplier: abuseIssueMultiplier,
              abuse_issue_threshold: abuseIssueThreshold,
            },
            accounts: [],
            received_at: receivedAt,
          });
        }

        const rows = wantFinal > 0
          ? await env.DB
              .prepare(
                "SELECT account_id,email,password,r2_url,owner FROM accounts_v2 WHERE owner='-1' AND owner <> '-3' ORDER BY RANDOM() LIMIT ?",
              )
              .bind(wantFinal * 8)
              .all<{ account_id: string; email: string; password: string; r2_url: string | null; owner: string }>()
          : { results: [] as Array<{ account_id: string; email: string; password: string; r2_url: string | null; owner: string }> };

        if (!env.R2_ACCESS_KEY_ID || !env.R2_SECRET_ACCESS_KEY) {
          return json({ ok: false, error: "missing_r2_s3_credentials" }, 500);
        }
        if (!env.R2_ACCOUNT_ID || env.R2_ACCOUNT_ID === "REPLACE_ME") {
          return json({ ok: false, error: "missing_r2_account_id" }, 500);
        }

        const r2 = new AwsClient({
          service: "s3",
          region: "auto",
          accessKeyId: env.R2_ACCESS_KEY_ID,
          secretAccessKey: env.R2_SECRET_ACCESS_KEY,
        });
        const ttlMinutes = 10;
        const expiresAt = new Date(Date.now() + ttlMinutes * 60 * 1000).toISOString().replace(/\.\d{3}Z$/, "Z");

        const extractR2Key = (raw: string): string | null => {
          const s = String(raw || "").trim();
          if (!s) return null;

          if (s.startsWith("r2://")) {
            const rest = s.slice(5);
            const slash = rest.indexOf("/");
            if (slash < 0) return null;
            const bucket = rest.slice(0, slash);
            const key = rest.slice(slash + 1);
            if (bucket !== env.R2_BUCKET_NAME) return null;
            return key || null;
          }

          if (/^https?:\/\//i.test(s)) {
            try {
              const u = new URL(s);
              const p = u.pathname.replace(/^\/+/, "");
              if (!p) return null;
              if (p.startsWith(`${env.R2_BUCKET_NAME}/`)) return p.slice(env.R2_BUCKET_NAME.length + 1);
              return null;
            } catch {
              return null;
            }
          }

          return s;
        };

        const makePresignedGet = async (r2Key: string): Promise<string> => {
          const u = new URL(`https://${env.R2_ACCOUNT_ID}.r2.cloudflarestorage.com/${env.R2_BUCKET_NAME}/${r2Key}`);
          u.searchParams.set("X-Amz-Expires", String(ttlMinutes * 60));
          const signed = await r2.sign(new Request(u.toString(), { method: "GET" }), { aws: { signQuery: true } });
          return signed.url.toString();
        };

        const accountsOut: Array<{
          file_name: string;
          account_id: string;
          owner: string;
          download_url: string;
        }> = [];
        const issueErrors: Array<{ account_id: string; error: string }> = [];
        for (const d of deletedMissingR2FromRequested) {
          issueErrors.push({ account_id: d.old_account_id, error: `missing_r2_object_deleted:${d.reason}` });
        }
        const stamp = receivedAt.replace(/[-:TZ]/g, "");

        for (let i = 0; i < (rows.results || []).length && accountsOut.length < wantFinal; i++) {
          const r = (rows.results || [])[i];

          // 仅从公有池抢占，并发安全
          const claim = await env.DB
            .prepare("UPDATE accounts_v2 SET owner=?, updated_at=?, last_refilled_at=? WHERE account_id=? AND owner='-1'")
            .bind(ownerKey, now, now, r.account_id)
            .run();
          if ((claim.meta?.changes || 0) !== 1) continue;
          r.owner = ownerKey;

          // 一重快速校验：公有池候选若 401/429，直接移入修补池并继续找
          let pickedStatus: number | null = null;
          let pickedNote = "";
          try {
            const p = await probeWhamStatusFromR2(env, r.account_id, r.r2_url);
            pickedStatus = p.status;
            pickedNote = p.note;
          } catch (e: any) {
            pickedStatus = null;
            pickedNote = `probe_error:${String(e?.message || e)}`;
          }

          if (pickedNote === "r2_object_not_found") {
            await env.DB.prepare("DELETE FROM accounts_v2 WHERE account_id=? AND owner=?")
              .bind(r.account_id, ownerKey)
              .run();
            issueErrors.push({ account_id: r.account_id, error: `missing_r2_object_deleted:${pickedNote}` });
            continue;
          }

          if (pickedStatus === 401 || pickedStatus === 429) {
            const repairOwner = pickedStatus === 401 ? "-2" : "-4";
            await env.DB.prepare("UPDATE accounts_v2 SET owner=?, updated_at=?, last_seen_at=? WHERE account_id=? AND owner=?")
              .bind(repairOwner, now, now, r.account_id, ownerKey)
              .run();
            issueErrors.push({ account_id: r.account_id, error: `public_candidate_rejected:${pickedStatus}:${pickedNote || "need_refill"}` });
            continue;
          }

          const fileName = `codex-${String(r.account_id || "").trim()}.json`;

          let r2Key = extractR2Key(String(r.r2_url || ""));
          if (!r2Key) {
            r2Key = `refill/topup-generated/${stamp}/${r.account_id}.json`;
            const fallbackObj = {
              account_id: String(r.account_id || ""),
              email: String(r.email || ""),
              password: String(r.password || ""),
              owner: String(r.owner || ownerKey),
              generated_at: now,
              generated_by: "v1/refill/topup",
            };
            await env.BUCKET.put(r2Key, JSON.stringify(fallbackObj, null, 2), {
              httpMetadata: { contentType: "application/json; charset=utf-8" },
            });
            await env.DB.prepare("UPDATE accounts_v2 SET r2_url=?, updated_at=? WHERE account_id=?").bind(r2Key, now, r.account_id).run();
          }

          const downloadUrl = await makePresignedGet(r2Key);
          accountsOut.push({
            file_name: fileName,
            account_id: String(r.account_id || ""),
            owner: String(r.owner || ownerKey),
            download_url: downloadUrl,
          });
        }

        // 4) 当日续杯计数 + 超限自动禁用
        const issuedCount = accountsOut.length;
        await env.DB
          .prepare(
            "INSERT INTO user_daily_refill_usage(user_id,day,refilled_count,updated_at) VALUES(?,?,?,?) ON CONFLICT(user_id,day) DO UPDATE SET refilled_count=refilled_count+excluded.refilled_count, updated_at=excluded.updated_at",
          )
          .bind(userKeyRow.user_id, day, issuedCount, now)
          .run();

        const usageRow = await env.DB
          .prepare("SELECT refilled_count FROM user_daily_refill_usage WHERE user_id=? AND day=?")
          .bind(userKeyRow.user_id, day)
          .first<{ refilled_count: number }>();

        const usedToday = Number(usageRow?.refilled_count || 0);
        const dailyLimit = Math.max(1, Number(userRow.daily_refill_limit || 200));
        let autoDisabled = false;
        let abuse_auto_banned = false;

        if (usedToday > abuseIssueThreshold) {
          autoDisabled = true;
          abuse_auto_banned = true;
          await env.DB.prepare("UPDATE users_v2 SET disabled=1, updated_at=? WHERE id=?").bind(now, userKeyRow.user_id).run();
          await env.DB.prepare("UPDATE user_keys_v2 SET enabled=0, updated_at=? WHERE user_id=?").bind(now, userKeyRow.user_id).run();
          issueErrors.push({
            account_id: "",
            error: `abuse_issue_limit_exceeded: used=${usedToday}, threshold=${abuseIssueThreshold}, multiplier=${abuseIssueMultiplier}, effective_limit=${effectiveAccountLimit}`,
          });
        } else if (usedToday > dailyLimit) {
          autoDisabled = true;
          await env.DB.prepare("UPDATE users_v2 SET disabled=1, updated_at=? WHERE id=?").bind(now, userKeyRow.user_id).run();
          await env.DB.prepare("UPDATE user_keys_v2 SET enabled=0, updated_at=? WHERE user_id=?").bind(now, userKeyRow.user_id).run();
          issueErrors.push({ account_id: "", error: `daily_refill_limit_exceeded: used=${usedToday}, limit=${dailyLimit}` });
        }

        return json({
          ok: true,
          target_pool_size: target,
          total_hold_limit: effectiveAccountLimit,
          accepted_reports: acceptedReports,
          requested_account_ids: requestedAccountIds,
          replaced_from_requested: replacedFromRequested,
          issued_count: issuedCount,
          used_today: usedToday,
          daily_limit: dailyLimit,
          auto_disabled: autoDisabled,
          abuse_auto_banned,
          errors: reportErrors,
          issue_errors: issueErrors,
          account_limit: {
            platform_base_account_limit: baseAccountLimit,
            account_limit_delta: delta,
            effective_account_limit: effectiveAccountLimit,
            current_owned: currentOwnedNow,
            abuse_issue_multiplier: abuseIssueMultiplier,
            abuse_issue_threshold: abuseIssueThreshold,
          },
          ttl_minutes: ttlMinutes,
          expires_at: expiresAt,
          accounts: accountsOut,
          received_at: receivedAt,
        });
      }

      if (path === "/v1/refill/sync-all" && req.method === "POST") {
        const receivedAt = utcNowIso();
        const day = receivedAt.slice(0, 10);
        const clientIp = (getClientIp(req) || "unknown").trim() || "unknown";
        const uaRaw = (req.headers.get("user-agent") || req.headers.get("User-Agent") || "").trim();
        const uaHash = await sha256Hex(uaRaw || "ua:empty");
        const fingerprint = await sha256Hex(`${clientIp}|${uaHash}`);

        const ipBlk = await isSubjectBlacklisted(env, "ip", clientIp, receivedAt);
        if (ipBlk.blocked) {
          return json({ ok: false, error: "ip_blacklisted", reason: ipBlk.reason, received_at: receivedAt }, 403);
        }
        const fpBlk = await isSubjectBlacklisted(env, "fingerprint", fingerprint, receivedAt);
        if (fpBlk.blocked) {
          return json({ ok: false, error: "fingerprint_blacklisted", reason: fpBlk.reason, received_at: receivedAt }, 403);
        }
        const uaBlk = await isSubjectBlacklisted(env, "ua_hash", uaHash, receivedAt);
        if (uaBlk.blocked) {
          return json({ ok: false, error: "ua_blacklisted", reason: uaBlk.reason, received_at: receivedAt }, 403);
        }

        const presentedRaw = (readUserKey(req) || readUploadKey(req) || readBearer(req) || "").trim();
        const presentedKeyHash = presentedRaw ? await sha256Hex(presentedRaw) : "missing";

        let caller: ClientCtx;
        try {
          caller = await requireAtLeastUser(req, env);
        } catch (e: any) {
          await recordSyncAllRiskEvent(env, day, clientIp, presentedKeyHash, uaHash, fingerprint, false, receivedAt);
          const risk = await evaluateSyncAllRiskAndMaybeBlacklist(env, day, clientIp, uaHash, fingerprint, receivedAt);
          if (risk.blockedNow) {
            return json({ ok: false, error: risk.reason, received_at: receivedAt }, 429);
          }
          if (e instanceof Response) {
            return json({ ok: false, error: String(await e.text().catch(() => "unauthorized") || "unauthorized"), received_at: receivedAt }, e.status || 403);
          }
          return json({ ok: false, error: "unauthorized", received_at: receivedAt }, 403);
        }

        const ownerKey = (caller.user_key_hash || caller.upload_key_hash || caller.audit_key_hash || "").trim();
        if (!ownerKey) return json({ ok: false, error: "missing_owner_key" }, 401);

        const effectivePresented = (caller.user_key_hash || caller.upload_key_hash || caller.admin_token_hash || presentedKeyHash || "missing").trim() || "missing";
        await recordSyncAllRiskEvent(env, day, clientIp, effectivePresented, uaHash, fingerprint, true, receivedAt);
        const riskAfterOk = await evaluateSyncAllRiskAndMaybeBlacklist(env, day, clientIp, uaHash, fingerprint, receivedAt);
        if (riskAfterOk.blockedNow) {
          return json({ ok: false, error: riskAfterOk.reason, received_at: receivedAt }, 429);
        }

        const roleForKey = caller.role === "admin" ? "super_admin" : caller.role === "upload" ? "uploader" : "user";
        let userKeyRow = await env.DB
          .prepare("SELECT user_id, role, enabled FROM user_keys_v2 WHERE key_hash=?")
          .bind(ownerKey)
          .first<{ user_id: string; role: string; enabled: number }>();

        if (!userKeyRow) {
          const userId = `u_${crypto.randomUUID()}`;
          await env.DB
            .prepare("INSERT INTO users_v2(id,display_name,roles,current_account_ids,daily_refill_limit,disabled,created_at,updated_at) VALUES(?,?,?,?,200,0,?,?)")
            .bind(userId, null, roleForKey, "[]", receivedAt, receivedAt)
            .run();
          await env.DB
            .prepare("INSERT INTO user_keys_v2(key_hash,user_id,role,enabled,created_at,updated_at) VALUES(?,?,?,1,?,?)")
            .bind(ownerKey, userId, roleForKey, receivedAt, receivedAt)
            .run();
          userKeyRow = { user_id: userId, role: roleForKey, enabled: 1 };
        }

        if (Number(userKeyRow.enabled) !== 1) return json({ ok: false, error: "user_key_disabled" }, 403);
        const userRow = await env.DB
          .prepare("SELECT disabled FROM users_v2 WHERE id=?")
          .bind(userKeyRow.user_id)
          .first<{ disabled: number }>();
        if (!userRow) return json({ ok: false, error: "user_not_found" }, 403);
        if (Number(userRow.disabled) === 1) return json({ ok: false, error: "user_disabled" }, 403);

        await env.DB
          .prepare("INSERT INTO user_daily_sync_all_usage(user_id,day,sync_count,updated_at) VALUES(?,?,1,?) ON CONFLICT(user_id,day) DO UPDATE SET sync_count=sync_count+1, updated_at=excluded.updated_at")
          .bind(userKeyRow.user_id, day, receivedAt)
          .run();

        const syncRow = await env.DB
          .prepare("SELECT sync_count FROM user_daily_sync_all_usage WHERE user_id=? AND day=?")
          .bind(userKeyRow.user_id, day)
          .first<{ sync_count: number }>();
        const syncAllUsedToday = Number(syncRow?.sync_count || 0);
        const syncAllLimit = 10;

        if (syncAllUsedToday >= syncAllLimit) {
          await env.DB.prepare("UPDATE users_v2 SET disabled=1, updated_at=? WHERE id=?").bind(receivedAt, userKeyRow.user_id).run();
          await env.DB.prepare("UPDATE user_keys_v2 SET enabled=0, updated_at=? WHERE user_id=?").bind(receivedAt, userKeyRow.user_id).run();
          return json({
            ok: false,
            error: "sync_all_limit_exceeded",
            sync_all_used_today: syncAllUsedToday,
            sync_all_limit: syncAllLimit,
            auto_disabled: true,
            received_at: receivedAt,
          }, 429);
        }

        const rows = await env.DB
          .prepare("SELECT account_id,email,password,r2_url,owner FROM accounts_v2 WHERE owner=? AND owner <> '-3' ORDER BY updated_at DESC LIMIT 500")
          .bind(ownerKey)
          .all<{ account_id: string; email: string; password: string; r2_url: string | null; owner: string }>();

        if (!env.R2_ACCESS_KEY_ID || !env.R2_SECRET_ACCESS_KEY) {
          return json({ ok: false, error: "missing_r2_s3_credentials" }, 500);
        }
        if (!env.R2_ACCOUNT_ID || env.R2_ACCOUNT_ID === "REPLACE_ME") {
          return json({ ok: false, error: "missing_r2_account_id" }, 500);
        }

        const r2 = new AwsClient({
          service: "s3",
          region: "auto",
          accessKeyId: env.R2_ACCESS_KEY_ID,
          secretAccessKey: env.R2_SECRET_ACCESS_KEY,
        });
        const ttlMinutes = 10;
        const expiresAt = new Date(Date.now() + ttlMinutes * 60 * 1000).toISOString().replace(/\.\d{3}Z$/, "Z");

        const extractR2Key = (raw: string): string | null => {
          const s = String(raw || "").trim();
          if (!s) return null;
          if (s.startsWith("r2://")) {
            const rest = s.slice(5);
            const slash = rest.indexOf("/");
            if (slash < 0) return null;
            const bucket = rest.slice(0, slash);
            const key = rest.slice(slash + 1);
            if (bucket !== env.R2_BUCKET_NAME) return null;
            return key || null;
          }
          if (/^https?:\/\//i.test(s)) {
            try {
              const u = new URL(s);
              const p = u.pathname.replace(/^\/+/, "");
              if (!p) return null;
              if (p.startsWith(`${env.R2_BUCKET_NAME}/`)) return p.slice(env.R2_BUCKET_NAME.length + 1);
              return null;
            } catch {
              return null;
            }
          }
          return s;
        };

        const makePresignedGet = async (r2Key: string): Promise<string> => {
          const u = new URL(`https://${env.R2_ACCOUNT_ID}.r2.cloudflarestorage.com/${env.R2_BUCKET_NAME}/${r2Key}`);
          u.searchParams.set("X-Amz-Expires", String(ttlMinutes * 60));
          const signed = await r2.sign(new Request(u.toString(), { method: "GET" }), { aws: { signQuery: true } });
          return signed.url.toString();
        };

        const accountsOut: Array<{ file_name: string; account_id: string; owner: string; download_url: string }> = [];
        for (let i = 0; i < (rows.results || []).length; i++) {
          const r = (rows.results || [])[i];
          let r2Key = extractR2Key(String(r.r2_url || ""));
          if (!r2Key) {
            const stamp = receivedAt.replace(/[-:TZ]/g, "");
            r2Key = `refill/sync-all-generated/${stamp}/${r.account_id}.json`;
            const fallbackObj = {
              account_id: String(r.account_id || ""),
              email: String(r.email || ""),
              password: String(r.password || ""),
              owner: String(r.owner || ownerKey),
              generated_at: receivedAt,
              generated_by: "v1/refill/sync-all",
            };
            await env.BUCKET.put(r2Key, JSON.stringify(fallbackObj, null, 2), {
              httpMetadata: { contentType: "application/json; charset=utf-8" },
            });
            await env.DB.prepare("UPDATE accounts_v2 SET r2_url=?, updated_at=? WHERE account_id=?").bind(r2Key, receivedAt, r.account_id).run();
          }
          const downloadUrl = await makePresignedGet(r2Key);
          accountsOut.push({
            file_name: `codex-${String(r.account_id || "").trim()}.json`,
            account_id: String(r.account_id || ""),
            owner: String(r.owner || ownerKey),
            download_url: downloadUrl,
          });
        }

        return json({
          ok: true,
          mode: "sync_all",
          sync_all_used_today: syncAllUsedToday,
          sync_all_limit: syncAllLimit,
          ttl_minutes: ttlMinutes,
          expires_at: expiresAt,
          accounts: accountsOut,
          issued_count: 0,
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
        const errors: Array<{ idx: number; error: string }> = [];

        for (let i = 0; i < items.length; i++) {
          const it = items[i] as any;

          // 仅接受 auth_json：禁止客户端直接提交 r2_url，R2 key 由服务端生成。
          const legacyAuth = it?.auth_json && typeof it.auth_json === "object" ? it.auth_json : null;
          if (!legacyAuth) {
            errors.push({ idx: i, error: "missing auth_json" });
            continue;
          }
          if (it?.r2_url !== null && it?.r2_url !== undefined && String(it.r2_url).trim()) {
            errors.push({ idx: i, error: "r2_url_not_allowed" });
            continue;
          }

          const accountId = String(it?.account_id || legacyAuth?.account_id || "").trim();
          const email = String(it?.email || legacyAuth?.email || "").trim();
          const password = String(it?.password || legacyAuth?.password || "").trim();
          let r2Url: string | null = null;

          let owner = "-1";
          if (it?.owner !== null && it?.owner !== undefined) {
            owner = String(it.owner).trim();
          }
          if (!owner) owner = "-1";

          // owner 允许：-1/-2/-3/-4 或任意非空字符串（私有池 key_hash）
          if (!["-1", "-2", "-3", "-4"].includes(owner)) {
            if (!/^[A-Za-z0-9:_-]{8,128}$/.test(owner)) {
              errors.push({ idx: i, error: "invalid owner" });
              continue;
            }
          }

          if (!accountId) {
            errors.push({ idx: i, error: "missing account_id" });
            continue;
          }
          if (!/^[A-Za-z0-9_-]{3,128}$/.test(accountId)) {
            errors.push({ idx: i, error: "invalid account_id" });
            continue;
          }
          // 兼容多来源数据：仅要求非空，避免误拦截合法但非常规格式邮箱
          if (!email) {
            errors.push({ idx: i, error: "missing email" });
            continue;
          }
          if (!password) {
            errors.push({ idx: i, error: "missing password" });
            continue;
          }
          const now = utcNowIso();

          // 服务端统一生成并写入 R2 key，避免 DB 出现“客户端伪造/失效 r2_url 引用”。
          const seed = crypto.randomUUID().replace(/-/g, "");
          const key = `auth-json/${accountId}-${now.replace(/[-:TZ]/g, "")}-${seed.slice(0, 12)}.json`;
          const authToStore = { ...(legacyAuth as Record<string, unknown>) };
          delete (authToStore as Record<string, unknown>).r2_url;
          await env.BUCKET.put(key, JSON.stringify(authToStore, null, 2), {
            httpMetadata: { contentType: "application/json; charset=utf-8" },
          });
          r2Url = key;
          const existing = await env.DB
            .prepare("SELECT id,r2_url FROM accounts_v2 WHERE account_id=?")
            .bind(accountId)
            .first<{ id: number; r2_url: string | null }>();

          if (!existing) {
            await env.DB
              .prepare(
                "INSERT INTO accounts_v2(account_id,email,password,r2_url,owner,created_at,updated_at,last_seen_at,last_refilled_at) VALUES(?,?,?,?,?,?,?,?,NULL)",
              )
              .bind(accountId, email, password, r2Url, owner, now, now, now)
              .run();
          } else {
            const oldR2Raw = existing.r2_url ? String(existing.r2_url).trim() : "";
            const oldR2Key = extractR2KeyFromInput(oldR2Raw, env.R2_BUCKET_NAME);
            const newR2Key = extractR2KeyFromInput(r2Url, env.R2_BUCKET_NAME);
            const nextR2Url = r2Url || oldR2Raw || null;

            await env.DB
              .prepare(
                "UPDATE accounts_v2 SET email=?, password=?, r2_url=?, owner=?, updated_at=?, last_seen_at=? WHERE account_id=?",
              )
              .bind(email, password, nextR2Url, owner, now, now, accountId)
              .run();

            if (newR2Key && oldR2Key && newR2Key !== oldR2Key) {
              await env.BUCKET.delete(oldR2Key);
            }
          }

          // 兼容保留审计：register 行为写 probes（email_hash 用 account_id 的 sha256）
          const pseudoEmailHash = await sha256Hex(accountId);
          await env.DB
            .prepare(
              "INSERT INTO probes(upload_key_hash,email_hash,account_id,status_code,probed_at,received_at) VALUES(?,?,?,?,?,?)",
            )
            .bind(caller.audit_key_hash, pseudoEmailHash, accountId, null, now, receivedAt)
            .run();

          accepted++;
        }

        return json({ ok: true, accepted, errors, received_at: receivedAt });
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
