# 无限续杯服务器（Cloudflare Workers 方案，合规版 / 模式A）

这个服务端只做**你自己的 key 管理/分发 + 客户端健康上报 + 去重与统计**，并明确三种权限：

- 超级管理员：`ADMIN_TOKEN`（/admin/* 全部管理功能）
- 热心群众：`UPLOAD_KEY`（上传/上报；并可按“权限继承”访问用户接口）
- 普通用户：`USER_KEY`

关键变更：`USER_KEY / UPLOAD_KEY` 推荐由服务端生成（管理员申请批量生成），而不是管理员手工导入明文 key。

- 服务端**不存储/不分发**任何第三方服务的认证 token（如 `access_token` / `refresh_token` / `id_token`）。
- “健康探测”在客户端完成：客户端用本地逻辑探测（例如 `HTTP 401` 判无效），然后只把 **`email_hash/account_id/status_code/timestamp`** 上报给服务端。
- 热心群众只拥有一个 `UPLOAD_KEY`：可上报/注册，且可按权限继承调用用户接口；但不能访问任何 `/admin/*`。

## 能力对应（按你的 5 条需求映射）

1. 普通管理员上传密钥（UPLOAD_KEY）
   - 使用 `X-Upload-Key` 作为调用凭据。
   - 目前**不支持无密钥上传**（服务端会拒绝）。

2. 普通用户请求密钥（USER_KEY）
   - 使用 `X-User-Key` 作为调用凭据。
   - 只能领取 refill key，不允许上报/注册。

3. 超级管理员上传一组续杯 key（REFILL_KEY_POOL）
   - 超级管理员用 `ADMIN_TOKEN` 调用管理接口批量导入。
   - 服务端对 refill key **加密存储**（AES-GCM），并按“未分发/已分发”追踪。

3. 服务端验证健康度
   - 合规版不直接请求第三方接口；健康探测由客户端完成。
   - 服务端提供 `probe-report` 接口接收上报并维护账户状态/失效库。

5. 超级管理员观察分发与状态
   - `/admin/stats`：key 分发统计、账户统计、上报统计。
   - `/admin/refill-keys`：查看每个 refill key 的分发状态（不返回明文 key）。

6. 自动删除无效认证 + 基础识别码去重
   - 因为服务端不保存 token，所谓“删除认证文件”在服务端等价为：将账户标记为 invalid 并进入 `invalid_accounts` 去重库。
   - 去重 key：优先 `email_hash`（也可带 `account_id` 作为辅助）。

---

## 目录结构

- [`infinite_refill/server/wrangler.toml`](wrangler.toml:1)
- [`infinite_refill/server/schema.sql`](schema.sql:1)
- [`infinite_refill/server/src/index.ts`](src/index.ts:1)

## 部署（wrangler）

1) 安装 wrangler（示例）

```bash
npm i -g wrangler
```

2) 创建 D1

```bash
wrangler d1 create refill_server_v2
```

把输出的 `database_id` 写进 [`wrangler.toml`](wrangler.toml:1)

3) 初始化表

```bash
wrangler d1 execute refill_server_v2 --file ./migrations/0001_init.sql
```

4) 本地 secrets 约定：`.dev.vars` / `.dev.vars.example`

- 所有敏感信息（token/密钥/guard）**只写到** [`server/.dev.vars`](.dev.vars:1)（已在 gitignore 中忽略，不会提交）。
- 仓库仅保留参考模板：[`server/.dev.vars.example`](.dev.vars.example:1)
- 约定初始化流程：如果本地不存在 `server/.dev.vars`，则从 `server/.dev.vars.example` 复制一份并重命名为 `.dev.vars`，再填写真实值。
  - Windows 一键初始化：[`server/tools/初始化本地.dev.vars.cmd`](tools/初始化本地.dev.vars.cmd:1)
  - macOS/Linux 一键初始化：[`server/tools/初始化本地.dev.vars.sh`](tools/初始化本地.dev.vars.sh:1)

5) 设置线上 Worker secrets（生产环境）

- `ADMIN_TOKEN`：超级管理员 Bearer token
- `ADMIN_GUARD`：超级管理员第二因子（Header `X-Admin-Guard`）
- `ADMIN_IP_WHITELIST`：超级管理员 IP 白名单（支持 IPv4/CIDR，逗号分隔；家庭宽带可能变化）
- `REFILL_KEYS_MASTER_KEY_B64`：用于加密保存 refill key 的主密钥（base64，解码后 32 bytes）
- `R2_ACCESS_KEY_ID` / `R2_SECRET_ACCESS_KEY`：R2 S3 API token（用于生成 presigned URL，给 `/admin/packages/issue` 返回可下载链接）

### 4.1 去哪里找 R2_ACCESS_KEY_ID / R2_SECRET_ACCESS_KEY？

这两个值来自 **Cloudflare R2 的 API token（S3 API credentials）**，需要你在 Cloudflare 控制台创建：

1) 打开 Cloudflare Dashboard → **R2 Object Storage**
2) 右侧或页面内找到 **Manage R2 API tokens**
3) 点击 **Create API token**
4) 权限建议：
   - 最小可用：`Object Read & Write`（并尽量 scope 到 `infinite-refill-packages` 这个 bucket）
   - 如果你还要让服务端创建/删除 bucket：才需要更高权限（一般不需要）
5) 创建完成后，页面会显示：
   - **Access Key ID** → 对应 `R2_ACCESS_KEY_ID`
   - **Secret Access Key** → 对应 `R2_SECRET_ACCESS_KEY`

注意：`Secret Access Key` **只显示一次**，Cloudflare 不提供“再次查看明文”的能力；丢了就只能新建 token。

参考 Cloudflare 文档：

- R2 API token 创建与说明：[`cloudflare.r2.api.tokens`](https://developers.cloudflare.com/r2/api/tokens/:1)

### 4.2 为什么我不能“直接把这两个返回给你”？

- 技术上：Cloudflare 的 `Secret Access Key` 创建后不可再次读取；即便我能看到，也不应该在聊天里回传任何密钥材料。
- 安全上：任何拿到这两个值的人，都可以以你的身份操作 R2（读取/写入对象），等同于把对象存储钥匙公开。

因此本仓库统一通过 `wrangler secret put` 交互写入，避免出现在代码/配置/git/聊天记录中。

```bash
wrangler secret put ADMIN_TOKEN
wrangler secret put REFILL_KEYS_MASTER_KEY_B64
wrangler secret put R2_ACCESS_KEY_ID
wrangler secret put R2_SECRET_ACCESS_KEY
```

为了避免把密钥写进 [`server/wrangler.toml`](wrangler.toml:1) 或 git，已提供“一键部署”脚本（仍通过交互输入 secrets）：

- Windows：[`server/tools/一键部署_线上.cmd`](tools/一键部署_线上.cmd:1)
- macOS/Linux：[`server/tools/一键部署_线上.sh`](tools/一键部署_线上.sh:1)

6) 本地调试 / 发布

```bash
# 本地：确保 server/.dev.vars 存在（没有就先运行初始化脚本）
wrangler dev

# 生产：先 wrangler secret put，再 deploy
wrangler deploy
```

---

## 家庭宽带 IP 变化时怎么处理（管理员强校验相关）

如果你启用了管理员强校验（IP 白名单 + `X-Admin-Guard`），家庭宽带公网出口 IP 变化会导致 `/admin/*` 全部返回 `admin_ip_not_allowed`。

处理步骤：

1) 先获取当前公网 IP（任选其一）：
   - Windows：
     - `powershell -NoProfile -Command "$ip=(Invoke-RestMethod -UseBasicParsing 'https://api.ipify.org'); Write-Output $ip"`
   - macOS/Linux：
     - `curl -sS https://api.ipify.org`

2) 更新线上白名单 secret（用最新 IP 覆盖，例如 `x.x.x.x/32`）：

```bash
cd server
(echo x.x.x.x/32) | npx wrangler secret put ADMIN_IP_WHITELIST
npx wrangler deploy
```

3) 同步更新本地文件（可选，但建议）：
- [`server/.dev.vars`](.dev.vars:1) 中的 `ADMIN_IP_WHITELIST=`

说明：当前服务端白名单实现仅支持 IPv4/CIDR；如后续你需要 IPv6 白名单再扩展。

---

## 源码透明（给用户看的）

- 本地客户端（curl + 中文命名）：
  - 入口说明：[`客户端/README.md`](../客户端/README.md:1)
- 本仓库所有“会删除/替换本地文件”的脚本，都在这里集中索引：[`TRANSPARENCY.md`](../TRANSPARENCY.md:1)

## API 文档

API（完整版）已抽到独立文件，便于后续维护与对外开放：

- [`server/API.md`](API.md:1)

该文档包含：

- 鉴权方式（Admin 强校验 / Upload / User + 权限继承）
- 全部开放路由清单（含 repairs/repairer、artworks、uploader 发包、backup 等）
- 请求体/响应体字段与错误语义
- 可直接复制的 curl 调用示例

## 本地客户端（curl + 中文命名，跨平台）

按设计约束：本仓库本地客户端统一使用 `curl` 调用，并采用中文文件名/中文提示，面向小白。

入口：[`客户端/README.md`](../客户端/README.md:1)

说明：
- 普通用户：领取 refill key
- 热心群众：提交 JSON 请求体（register / probe-report）
- 超级管理员：查看统计、导入密钥等
