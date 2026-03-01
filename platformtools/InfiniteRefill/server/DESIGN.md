# 设计文档：无限续杯服务器（Cloudflare Workers + D1）

实现文件入口：[`server/src/index.ts`](src/index.ts:1)

本地客户端入口（curl + 中文命名）：[`客户端/README.md`](../客户端/README.md:1)

---

## 1) 目标（按你的定义）

系统由一个服务器 + 三类客户端构成。

服务器主要职责：

1. **接受 key**：识别调用方身份/权限。
2. **分发 key**：向客户端分发 refill key（或等价资源）。
3. **认证密钥是否合适**：校验 key 是否存在、是否被封禁、权限是否满足。

---

## 2) 三类客户端与权限继承

### 2.1 角色

- **普通用户客户端（User Client）**
  - 只接受并使用 `USER_KEY`
  - 能力：领取 refill key

- **热心群众客户端（Uploader Client）**
  - 使用 `UPLOAD_KEY`
  - 能力：
    - 拥有普通用户客户端全部权限（权限继承）
    - 可执行“上传/上报”类操作（由你自行定义具体上传内容）

- **超级管理员客户端（Admin Client）**
  - 使用 `ADMIN_TOKEN`
  - 能力：
    - 拥有热心群众客户端全部权限（权限继承）
    - 导入/封禁 `USER_KEY`、`UPLOAD_KEY`
    - 查看统计、审计日志

### 2.2 权限继承规则（建议）

- `ADMIN_TOKEN` ⊇ `UPLOAD_KEY` ⊇ `USER_KEY`

落地建议：

- 把“鉴权 + 角色判定 + enabled 校验 + 审计”做成统一层（中间件/函数）。
- 每个接口声明一个 `required_role`，运行时统一做 `role >= required_role`。

### 2.3 特殊设定：上传 key 可当作普通用户 key 使用 + 贡献者扩容策略

你新增的规则可以表述为两部分：

1) **凭据复用**：`UPLOAD_KEY` 在“领取/续杯”场景下等价于 `USER_KEY`。

- 客户端侧：Uploader Client 不需要再额外持有 `USER_KEY`，直接用 `UPLOAD_KEY` 调用用户侧接口。
- 服务端侧：鉴权层将 `UPLOAD_KEY` 解析为 `role=upload`，并允许访问 `required=user` 的接口。

2) **贡献者扩容**：当某个上传者（按 `UPLOAD_KEY` 维度）累计上传“有效账户”超过 100 个后，提高其“本地可用账户池”的目标规模。

建议把这套策略抽象成**服务器下发的 policy**，以便后续可调整阈值：

- 默认用户（含普通用户 / 一般上传者）：
  - `target_pool_size = 10`
  - `refill_trigger_remaining = 2`（等价于“invalid>=8”）
  - 触发后补齐到 10

- 达标贡献者（上传有效账户 > 100 的上传者）：
  - `target_pool_size = 50`
  - `refill_trigger_remaining = 10`
  - 触发后补齐到 50

> “有效账户”的判定由你定义并固化在服务端统计口径里（例如：上传后经某种判定进入有效池、或状态码为 200 的累计数量等）。关键是：**统计维度按 UPLOAD_KEY**。

落地建议：

- D1 新增贡献计数（按 `upload_key_hash`）：
  - `upload_contrib(upload_key_hash PRIMARY KEY, valid_accounts_uploaded INTEGER, updated_at TEXT)`
- 每次上传/上报成功后，服务端按规则递增 `valid_accounts_uploaded`。
- 客户端续杯流程不硬编码 10/50，而是先请求一次 policy（或在 claim/fetch 响应里携带 policy）。

---

## 3) 本地客户端规范（面向小白）

你新增的约束：**所有本地客户端一概使用 curl 调用**，并且**脚本/命令/文件命名一律使用中文**，以降低小白使用门槛。

建议约定：

- 客户端交付形态：
  - Windows：`.bat`（内部调用 `curl.exe`）
  - macOS/Linux：`.sh`（内部调用 `curl`）
- 命名：中文文件名 + 中文参数说明 + 中文输出
- 统一配置：
  - 服务器地址：`服务器地址`（例如 `https://xxx.workers.dev`）
  - 密钥：`用户密钥` / `上传密钥` / `管理员令牌`

### 3.1 curl 调用约定（建议）

- 一律 `Content-Type: application/json`
- key 放在 header：
  - 用户：`X-User-Key: <USER_KEY>`
  - 上传者：`X-Upload-Key: <UPLOAD_KEY>`（也可用于用户接口）
  - 管理员：`Authorization: Bearer <ADMIN_TOKEN>`

### 3.2 示例（仅展示 curl 形态，不绑定实现细节）

- 领取 refill key（普通用户 / 上传者都可调用）：

```bash
curl -sS -X POST "${服务器地址}/v1/refill-keys/claim" \
  -H "X-User-Key: ${用户密钥}" \
  -H "Content-Type: application/json" \
  --data "{}"
```

- 获取策略（让客户端知道目标池大小/触发阈值）：

```bash
curl -sS "${服务器地址}/v1/policy" \
  -H "X-User-Key: ${用户密钥}" \
  -H "Content-Type: application/json"
```

> 实际落地时，可以让“上传者客户端”把 header 从 `X-User-Key` 换成 `X-Upload-Key`，达到“上传 key 复用为用户 key”的效果。

---

## 4) 数据模型（D1）

基于当前仓库的表结构（你可自行调整）：[`server/schema.sql`](schema.sql:1)

建议维持两张 key 表 + enabled 字段：

- `user_keys(key_hash, enabled, created_at, label)`
- `upload_keys(key_hash, enabled, created_at, label)`

建议新增/强化审计表（便于追踪封禁与滥用）：

- `audit_log(id, actor_type, actor_key_hash, action, target_type, target_id, ts, meta_json)`

refill key 池：

- `refill_keys(key_hash, key_enc_b64, status, claimed_by_*, claimed_at, created_at)`

---

## 5) 封禁（Ban）模型（按你的需求：封禁“用户密钥”）

- 封禁对象：`USER_KEY`
- 封禁效果：
  - 被封禁的 `USER_KEY` 不允许调用任何需要该 key 的接口（返回 403）
- 存储方式：
  - 最简单：`user_keys.enabled = 0`
- 审计：
  - 管理端每次封禁/解封都写 `audit_log`

> 你也可以同样支持封禁 `UPLOAD_KEY`（同理用 `upload_keys.enabled=0`）。



## 6) API 分层建议（不含具体实现细节）

- `GET  /v1/policy`（required: user）——返回调用方的 `target_pool_size/refill_trigger_remaining` 等策略参数
- `POST /v1/refill-keys/claim`（required: user）
- `POST /v1/upload/...`（required: upload）——上传/上报类接口（你自行定义 payload）

- `POST /admin/keys/issue`（required: admin）——服务端生成平台密钥（user/upload/refill），返回一次明文
- `GET  /admin/backup/export`（required: admin）——导出有效数据（不含日志/无效库/任何 token 字段）
- `POST /admin/backup/import`（required: admin）——导入有效数据（从 dump 恢复）

- `POST /admin/user-keys/ban`（required: admin）——封禁 USER_KEY
- `GET  /admin/stats`（required: admin）——统计/审计

（旧接口，兼容保留但新流程不推荐）：

- `POST /admin/user-keys`（required: admin）——导入/管理 USER_KEY
- `POST /admin/upload-keys`（required: admin）——导入/管理 UPLOAD_KEY

---

## 7) 密钥发放与“手动分发”支持（仅分发平台密钥/配置包）

你新增的诉求是：除了“客户端在线向服务端请求分发”外，还要支持管理员生成一批**平台侧有效密钥**，并以“可离线转交”的方式分发给特殊人群。

这里必须强调边界：

- 本项目的“分发包”仅用于分发 **平台密钥（USER_KEY / UPLOAD_KEY）与客户端配置示例**。
- 不建议、也不在本设计中支持把任何第三方服务的认证 token/凭据打包进压缩包并分发。

### 7.1 服务端生成平台密钥（建议：仅管理员可调用）

建议新增管理员接口：

- `POST /admin/keys/issue`

请求体建议：

```json
{
  "type": "user",
  "count": 30,
  "label": "2026-02 special",
  "bind_pool_size": 10
}
```

语义：

- 服务端生成 `count` 个新平台密钥（`type=user|upload`）。
- “bind_pool_size=10” 仅表示平台侧为该 key 预留/绑定的**配额/槽位**（例如默认 10 个），用于后续策略下发或配额控制；并不意味着服务端会把第三方凭据随 key 一起打包下发。

响应建议（仅展示一次明文 key，服务端仅保存 hash）：

```json
{
  "batch_id": "issue_20260228_xxx",
  "keys": ["plain_key_1", "plain_key_2"],
  "policy": {"target_pool_size": 10, "refill_trigger_remaining": 2}
}
```

### 7.2 导出“手动分发包”（压缩包密码=对应平台密钥）

你提出的形式是：

- 每个密钥对应一个压缩包
- 压缩包密码 = 该密钥本身
- 支持一次申请 N 个时返回 N 个压缩包（数字命名），并附带一个映射文本：`压缩包名：对应密钥`

在不分发第三方凭据的前提下，压缩包内容建议只包含：

- 一个最小配置文件（例如 `.env` 或 `config.txt`）：写入 `SERVER_URL` 与该 `USER_KEY/UPLOAD_KEY`
- 一个 `README.txt`：指向 [`客户端/README.md`](../客户端/README.md:1) 的使用说明

服务端实现路径建议（二选一）：

1) **离线工具生成**：管理员在本地运行打包脚本（7zip/zip），服务端只提供批量发 key 的接口。
2) **服务端返回下载链接（你已确认采用这条）**：服务端将加密压缩包写入对象存储（R2），接口返回一个批次下载链接（或多个对象 URL），管理员据此下载后再离线分发。

### 7.3 服务端生成并返回“分发压缩包”（R2 + 下载链接）

你坚持要“服务端直接返回压缩包（写入 R2 并返回下载链接）”，建议把链路定义为**一个原子操作**，避免 key 明文泄露多次：

- `POST /admin/packages/issue`（required: admin）

请求体示例：

```json
{
  "type": "user",
  "count": 30,
  "label": "2026-02 special",
  "bind_pool_size": 10,
  "server_url": "https://xxx.workers.dev",
  "zip_name_style": "number",
  "zip_password_style": "same_as_key",
  "ttl_minutes": 60
}
```

语义：

- 服务端生成 `count` 个平台密钥（`type=user|upload`），并立即为每个 key 生成一个加密压缩包。
- 压缩包命名：`1.zip`...`N.zip`（数字命名）。
- 压缩包密码：等于该压缩包对应的**平台密钥明文**。
- 生成映射文本：`分发清单.txt`，每行：`压缩包名：对应密钥`。
- 产物写入 R2：
  - `batches/<batch_id>/1.zip`
  - `batches/<batch_id>/2.zip`
  - ...
  - `batches/<batch_id>/分发清单.txt`
- 返回“批次下载链接”（建议为短期有效的 signed URL）。

压缩包内容建议（最小可用）：

- `无限续杯配置.env`
  - 写入 `SERVER_URL=<server_url>`
  - 写入 `USER_KEY=<该包对应的密钥>`（或 `UPLOAD_KEY=<...>`）
  - 其余字段给默认值（`TARGET_POOL_SIZE=10` 等）
- `README.txt`
  - 引导用户去看 [`客户端/README.md`](../客户端/README.md:1)

返回体示例（key 明文仅出现一次）：

```json
{
  "ok": true,
  "batch_id": "issue_20260228_xxx",
  "type": "user",
  "count": 30,
  "packages": [
    {
      "name": "1.zip",
      "key": "k_xxx",
      "download_url": "https://...signed..."
    }
  ],
  "manifest": {
    "name": "分发清单.txt",
    "download_url": "https://...signed..."
  },
  "expires_at": "2026-02-28T01:00:00Z"
}
```

> 备注：由于你要求“压缩包密码=密钥”，返回体本身就包含密钥；务必仅管理员可调用，并为下载链接设置短有效期。

### 7.4 Workers 端实现清单（供实现模式执行）

- 存储：为 Worker 绑定 R2（建议变量名 `BUCKET`），并实现“写对象 + 生成短期 signed URL”。
- 打包：实现“生成 zip + 设置密码”。（需要选定库；若最终库无法在 Workers 上实现真正的 zip 密码，则需你确认替代加密方案。）
- 输出：按批次写入 `分发清单.txt`，并在响应里同时返回。
- 安全：
  - `/admin/*` 通过 Cloudflare Zero Trust Access 或 IP 白名单
  - 管理接口本身再做限速（避免滥用生成大量包）

> 注意：本设计只规定“包里装什么/如何命名/如何映射”，不涉及任何第三方 token 的采集或分发细节。

---

## 8) 防撞库与访问控制（Cloudflare WAF/Rate Limiting 为主）

你提出的风险点是：平台密钥如果是“唯一凭据”，会不会被人撞库一直试。

你的决策是：

- 优先使用 Cloudflare WAF/Rate Limiting 对 `/v1/*` 做 IP 限速
- 对连续 401/403 的来源触发更严格限速
- 对 `/admin/*` 再叠加 Cloudflare Zero Trust Access 或 IP 白名单
- Worker 只做最小兜底（不重复造轮子）

建议落地要点：

1) `/admin/*`：只允许白名单出口 IP 或通过 Access 鉴权，再加较低的速率限制。
2) `/v1/*`：基础限速 + 失败增强限速（401/403 触发更严格）。
3) Worker 兜底：
   - 先做 key 格式校验（减少无意义请求）
   - 错误响应保持你确认的语义：missing=401、invalid=403，但尽量不要在响应体暴露更多信息。

相关实现入口：[`server/src/index.ts`](src/index.ts:1)。
