# API 文档（对外开放接口一览）

本文档以当前服务端实现为准（Worker 入口：[`server/src/index.ts`](src/index.ts:726)）。

- **所有请求/响应时间**均为 ISO8601 UTC 字符串（形如 `2026-02-28T17:19:14Z`）。
- **JSON 接口**要求 `Content-Type: application/json`（否则返回 `415 expected application/json`）。
- 除少量鉴权失败返回纯文本外，多数接口返回 JSON：`{ ok: true/false, ... }`。

---

## 0. 基础信息

### 0.1 Base URL

- 本地开发（wrangler dev）：例如 `http://127.0.0.1:8788`
- 线上（自定义域名）：例如 `https://refill.aiaimimi.com`

下文示例以：

```bash
export SERVER_URL="http://127.0.0.1:8788"
```

### 0.2 鉴权方式（最重要）

#### Admin（超级管理员）

- Header：`Authorization: Bearer <ADMIN_TOKEN>`
- Header：`X-Admin-Guard: <ADMIN_GUARD>`
- 且来源 IP 必须命中白名单 `ADMIN_IP_WHITELIST`（IPv4 或 CIDR，逗号分隔）。

强校验逻辑见：[`server.index.requireAdmin()`](src/index.ts:402)

#### Upload（热心群众 / 维修者）

- Header：`X-Upload-Key: <UPLOAD_KEY>`

校验逻辑见：[`server.index.requireUploadKey()`](src/index.ts:516)

> 维修者与热心群众**同凭据体系**（都是 `UPLOAD_KEY`），仅“角色称呼/使用接口”不同。

#### User（普通用户）

- Header：`X-User-Key: <USER_KEY>`

校验逻辑见：[`server.index.requireUserKey()`](src/index.ts:525)

#### 权限继承

- `UPLOAD_KEY` 拥有 `USER_KEY` 的全部权限（即 upload ⊇ user）。见：[`server.index.requireAtLeastUser()`](src/index.ts:501)

---

## 1. Health / UI

### 1.1 `GET /health`

用于探活。

**响应 200**

```json
{ "ok": true, "ts": "2026-02-28T17:19:14Z" }
```

实现入口：[`server.index.fetch()`](src/index.ts:692)

### 1.2 `GET /` / `GET /ui` / `GET /ui/`

返回一个纯前端页面（用于普通管理员上报工具）。

实现入口：[`server.index.fetch()`](src/index.ts:698)

---

## 2. Admin API（/admin/*）

> 这些接口全部要求管理员强校验（见上文）。

### 2.1 `GET /admin/stats`

返回 key/账户/上报统计。

实现入口：[`server.index.fetch()`](src/index.ts:704)

### 2.2 `POST /admin/keys/issue`

批量生成平台密钥（只返回一次明文；DB 仅保存 hash）。

**请求体**：[`IssueKeysBody`](src/index.ts:546)

```json
{ "type": "user", "count": 5, "label": "demo" }
```

- `type`: `user | upload | refill`
- `count`: 1~200
- `label`: 可选
- `bind_pool_size`: 可选（目前仅回显，不强制落库）

**响应 200**（示例）

```json
{
  "ok": true,
  "type": "user",
  "count_requested": 5,
  "count_issued": 5,
  "label": "demo",
  "keys": ["k_...", "k_..."],
  "issued_at": "2026-02-28T17:19:14Z",
  "errors": []
}
```

实现入口：[`server.index.fetch()`](src/index.ts:732)

### 2.3 `POST /admin/packages/issue`

生成“手动分发包”（zip + 清单），并返回 presigned 下载链接。

**请求体**：[`IssuePackagesBody`](src/index.ts:557)

```json
{
  "type": "user",
  "count": 10,
  "label": "demo",
  "server_url": "https://refill.aiaimimi.com",
  "ttl_minutes": 60
}
```

- `type`: `user | upload`
- `count`: 1~200
- `server_url`: 必须 `http(s)://...`
- `ttl_minutes`: 1~1440，默认 60

**响应 200**：返回 `packages[]` 与 `manifest.download_url`。

实现入口：[`server.index.fetch()`](src/index.ts:799)

### 2.4 `GET /admin/backup/export`

导出“有效数据”的 dump（不含 probes 日志；不含无效库；不含任何 token 字段）。

实现入口：[`server.index.fetch()`](src/index.ts:953)

### 2.5 `POST /admin/backup/import`

导入 export 的 dump（用于迁移/恢复）。

**请求体**：[`BackupImportBody`](src/index.ts:638)

```json
{ "dump": { "version": 1, "exported_at": "...", "upload_keys": [], "user_keys": [], "refill_keys": [], "accounts": [] } }
```

实现入口：[`server.index.fetch()`](src/index.ts:988)

### 2.6 旧接口（兼容保留）

#### `POST /admin/upload-keys`

导入一批 upload keys（明文入参，DB 存 hash）。

请求体：[`UploadKeysBody`](src/index.ts:542)

实现入口：[`server.index.fetch()`](src/index.ts:1085)

#### `POST /admin/user-keys`

导入一批 user keys。

请求体：[`UserKeysBody`](src/index.ts:543)

实现入口：[`server.index.fetch()`](src/index.ts:1110)

#### `POST /admin/refill-keys`

导入一批 refill keys（会 AES-GCM 加密后存储）。

请求体：[`RefillKeysBody`](src/index.ts:544)

实现入口：[`server.index.fetch()`](src/index.ts:1135)

### 2.7 `GET /admin/refill-keys`

查看 refill keys 状态列表。

- 可选 query：`?status=available|claimed|revoked`

实现入口：[`server.index.fetch()`](src/index.ts:1164)

### 2.8 `GET /admin/accounts`

查看账户列表。

- 可选 query：`?invalid=1` 仅看 invalid

实现入口：[`server.index.fetch()`](src/index.ts:1189)

---

## 3. Artworks API（作品投稿/领取/损坏上报）

### 3.1 `POST /v1/artworks/submit`（Upload）

提交作品 JSON（服务端会递归去敏后存入 R2，并写索引到 D1）。

鉴权：`X-Upload-Key`（或管理员）。

**请求体**：[`SubmitArtworksBody`](src/index.ts:576)

- 单篇提交：

```json
{ "artwork": { "account_id": "acc_123", "title": "..." } }
```

- 批量提交：

```json
{ "items": [ {"account_id":"acc_1"}, {"account_id":"acc_2"} ], "label": "demo" }
```

限制：
- 单篇 ≤ 64KB（UTF-8 字节）
- 批量最多 2000 篇
- 批量总大小 ≤ 10MB

**关于 artwork_id（非常重要）**
- 优先使用作品 JSON 内的 `account_id` 作为唯一 id（即 `artwork_id`），见：[`server.index.extractArtworkIdFromJsonArtwork()`](src/index.ts:673)
- 若没有 `account_id`，才使用服务端生成的旧 id（`a_...`）。

**去重 / 墓地规则**
- 若 `artwork_tombstones` 已存在该 `artwork_id`：拒绝（错误 `tombstoned_artwork_id`）。
- 若 `artworks` 已存在该 `artwork_id`：拒绝（错误 `duplicate_artwork_id`）。

实现入口：[`server.index.fetch()`](src/index.ts:1211)

### 3.2 `POST /v1/artworks/claim`（User/Upload）

从公共池领取作品（变更为私有绑定 `claimed`），返回作品正文。

**不活跃自动解绑规则（新）**

- 如果某个 `USER_KEY` / `UPLOAD_KEY` 连续 **7 天**都没有对平台发起任何请求，将视为“已离开平台”。
- 平台会在后续请求时做**懒回收**：把该 key 绑定的 `status='claimed'` 作品取消绑定并回归公共池（`available`）。
- 活跃度记录与回收逻辑见：[`server.index.touchClientActivity()`](src/index.ts:101)、[`server.index.reapInactiveClaimedArtworks()`](src/index.ts:121)（以代码为准）。

鉴权：`X-User-Key` 或 `X-Upload-Key`（权限继承）。

请求体：[`ClaimArtworksBody`](src/index.ts:584)

```json
{ "count": 1 }
```

错误：
- `409 pool_full`：私有池已满
- `409 no_available_artwork`：公共池无可领取

实现入口：[`server.index.fetch()`](src/index.ts:1338)

### 3.3 `POST /v1/artworks/report-damage`（User/Upload）

上报已领取作品损坏，并“交换领取”一件 replacement。

鉴权：`X-User-Key` 或 `X-Upload-Key`。

请求体：[`DamageReportBody`](src/index.ts:589)

```json
{ "artwork_id": "acc_123", "kind": "full", "note": "optional" }
```

- `kind=partial`：旧作品进入 `quarantine`（7 天后回公共池）
- `kind=full`：旧作品进入 `repair`（维修区，等待维修者确认/处理；不再直接 deleted）

实现入口：[`server.index.fetch()`](src/index.ts:1442)

### 3.4 `GET /v1/artworks/mine`（可选）

返回“我已领取”的作品 presigned 下载链接列表。

- 需要 env 开关 `ENABLE_MINE_API=1`，否则返回 404。

实现入口：[`server.index.fetch()`](src/index.ts:1733)

---

## 4. Repairs API（维修者工作流）

> 维修者使用 `X-Upload-Key`（与热心群众同凭据模型）。

### 4.1 `POST /v1/repairs/claim`

从维修区随机领取待修作品（`repair` → `repair_claimed`），并返回作品正文。

请求体：[`RepairClaimBody`](src/index.ts:595)

```json
{ "count": 1 }
```

错误：
- `409 no_repairable_artwork`

实现入口：[`server.index.fetch()`](src/index.ts:1553)

### 4.2 `POST /v1/repairs/submit-fixed`

提交修复后的作品（覆盖写回同一 `artwork_id` 的 R2 对象），并将状态恢复为 `available`。

请求体：[`RepairSubmitFixedBody`](src/index.ts:600)

```json
{ "artwork_id": "acc_123", "fixed_artwork": { "account_id": "acc_123", "...": "..." } }
```

约束：`fixed_artwork.account_id` 必须存在且等于 `artwork_id`（防错修）。

实现入口：[`server.index.fetch()`](src/index.ts:1602)

### 4.3 `POST /v1/repairs/submit-failed`

提交“无法修复”的结果：

- `repair_fail_count += 1`
- 若累计达到 3：
  - 写入 `artwork_tombstones` 防重复提交
  - 将对象迁移到 `graveyard/artworks/<id>.json`
  - 状态置为 `graveyard`
- 未达到 3：退回 `repair`，等待下一轮维修

请求体：[`RepairSubmitFailedBody`](src/index.ts:605)

```json
{ "artwork_id": "acc_123", "note": "why failed" }
```

实现入口：[`server.index.fetch()`](src/index.ts:1647)

### 4.4 `POST /v1/repairs/submit-misreport`

提交“误报”裁决：将 `repair_claimed` 恢复为 `available`。

请求体（当前为内联类型）：[`server.index.parseJson()`](src/index.ts:534)

```json
{ "artwork_id": "acc_123", "note": "optional" }
```

实现入口：[`server.index.fetch()`](src/index.ts:1713)

---

## 5. Uploader 激励：用 credit 生成 USER_KEY 分发包

### 5.1 `POST /v1/uploader/packages/issue`（Upload）

消耗 uploader 的 `distribution_credits` 生成一批 USER_KEY 分发包（zip + 清单）。

请求体：[`UploaderIssuePackagesBody`](src/index.ts:566)

```json
{ "count": 5, "label": "demo", "server_url": "https://refill.aiaimimi.com", "ttl_minutes": 60 }
```

错误：
- `403 no_distribution_credit`：没有分发资格

实现入口：[`server.index.fetch()`](src/index.ts:1807)

---

## 6. Refill / Topup（旧“续杯账号 JSON”链路）

### 6.1 `POST /v1/refill/topup`（User/Upload）

客户端上报探测结果（401/429 等），服务端根据本地库返回一批 `accounts[]`（每项包含 `{file_name, auth_json}`），供客户端写入 `auth-dir`。

注意：服务端下发的 `auth_json` 来自 `POST /v1/accounts/register`（Upload）上传的 `auth_json`，并以 AES-GCM(base64) 加密存储在 D1（字段 `accounts.auth_json`）。

请求体（当前为内联类型）：[`server.index.parseJson()`](src/index.ts:575)

```json
{
  "target_pool_size": 10,
  "reports": [
    {"file_name":"codex-acc_123.json","email_hash":"<sha256hex>","account_id":"acc_123","status_code":401,"probed_at":"2026-02-28T00:00:00Z"}
  ]
}
```

响应 200（示例）

```json
{
  "ok": true,
  "target_pool_size": 10,
  "accepted_reports": 1,
  "errors": [],
  "accounts": [
    {"file_name":"codex-acc_123.json","auth_json": {"type":"codex","access_token":"..."}}
  ],
  "received_at": "2026-02-28T00:00:00Z"
}
```

实现入口：[`server.index.fetch()`](src/index.ts:2053)

响应字段补充：
- `issue_errors[]`：下发阶段解密/解析失败的账号（不会进入 accounts[]）。

---

## 7. Refill keys

### 7.1 `POST /v1/refill-keys/claim`（User/Upload）

领取一个 refill key（返回明文 refill_key）。

实现入口：[`server.index.fetch()`](src/index.ts:2215)

---

## 8. Accounts / Probe report

### 7.1 `POST /v1/accounts/register`（Upload）

仅注册身份（不探测），并写入 probes 审计（status_code=NULL）。

请求体：[`RegisterAccountsBody`](src/index.ts:654)

```json
{ "accounts": [ { "email_hash": "<sha256 hex>", "account_id": "acc_123", "seen_at": "2026-02-28T17:19:14Z", "auth_json": {"type":"codex","access_token":"..."} } ] }
```

实现入口：[`server.index.fetch()`](src/index.ts:2012)

### 7.2 `POST /v1/probe-report`（Upload）

上报探测结果（客户端探测后上报），并维护：
- `probes` 明细
- `accounts` 聚合状态
- `invalid_accounts` 去重库（401）
- `exhausted_accounts` 暂存库（429，7 天冷却）

请求体：[`ProbeReportBody`](src/index.ts:646)

```json
{ "reports": [ { "email_hash": "<sha256 hex>", "account_id": "acc_123", "status_code": 401, "probed_at": "2026-02-28T17:19:14Z" } ] }
```

实现入口：[`server.index.fetch()`](src/index.ts:2071)

---

## 8. curl 调用示例（最小可用）

### 8.1 使用本地测试 key（server/.dev.vars）

本地测试 key 由 [`server/.dev.vars`](.dev.vars:1) 提供，并在 Worker 启动后自动写入 D1（只存 hash）。写入逻辑：[`server.index.ensureTestKeysInDb()`](src/index.ts:64)

示例：维修者领取维修任务（如果维修区为空会 409）。

```bash
curl -sS \
  -H "X-Upload-Key: $TEST_REPAIRER_UPLOAD_KEY" \
  -H "Content-Type: application/json" \
  "$SERVER_URL/v1/repairs/claim" \
  -d '{"count":1}'
```

### 8.2 投稿一篇作品（account_id 作为唯一 id）

```bash
curl -sS \
  -H "X-Upload-Key: $TEST_UPLOADER_UPLOAD_KEY" \
  -H "Content-Type: application/json" \
  "$SERVER_URL/v1/artworks/submit" \
  -d '{"artwork":{"account_id":"acc_demo_1","title":"hello"}}'
```

---

## 9. 常见错误码/错误语义（约定）

- `401 missing X-Upload-Key` / `401 missing X-User-Key`：缺少凭据 header
- `403 invalid upload key` / `403 invalid user key`：DB 中未启用或不存在
- `404 not_found`：资源不存在或无权限访问（例如不是你 claim 的作品）
- `409 pool_full` / `409 no_available_artwork` / `409 no_repairable_artwork`：资源不足或状态不允许
- `413 item_too_large` / `413 batch_too_large` / `413 too_many_items`：大小/数量限制

具体错误分支以 [`server/src/index.ts`](src/index.ts:684) 为准。
