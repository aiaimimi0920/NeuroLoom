# InfiniteRefill：拒绝/跳过逻辑 + 服务端取字段逻辑（含：已移除敏感字段检查后的现状）

> 目标：把 InfiniteRefill 中“会拒绝/跳过提交”的逻辑、以及服务端在 `/v1/accounts/register` 与 `/v1/refill/topup` 中“从请求体取字段/校验/下发字段”的逻辑完整摘录出来，便于你后续对接 `codex_register/wait_update`。
>
> 备注：根据你的最新指令，仓库内 **客户端侧的敏感字段检查（access_token/refresh_token/id_token）已被禁用**，允许提交/上传包含这些字段的 JSON。本文件相应更新为“禁用后的现状与索引”。

---

## 1) 客户端侧：热心群众提交 JSON 的拒绝/跳过逻辑

### 1.1 Windows：敏感字段检测已禁用（原先为 findstr 拦截）

文件：[`platformtools/InfiniteRefill/客户端/热心群众/提交JSON.bat`](../../InfiniteRefill/客户端/热心群众/提交JSON.bat:1)

现状：
- 已按你的指令禁用敏感字段检测；不再因为出现 `access_token/refresh_token/id_token` 而 `[SKIP]`。

当前对应位置（已禁用提示）：[`提交JSON.bat`](../../InfiniteRefill/客户端/热心群众/提交JSON.bat:69)

### 1.2 Windows：JSON/结构不合规跳过（PowerShell 结构校验）

文件：[`platformtools/InfiniteRefill/客户端/热心群众/提交JSON.bat`](../../InfiniteRefill/客户端/热心群众/提交JSON.bat:1)

关键拒绝逻辑：
- 解析失败或字段结构不合规（缺少 `accounts[]/reports[]`、`email_hash` 非 64 hex、缺少 `seen_at/probed_at/status_code`）会 `[SKIP]`。

来源位置：[`提交JSON.bat`](../../InfiniteRefill/客户端/热心群众/提交JSON.bat:77)

### 1.3 macOS/Linux：提交前统一走 `json_check_payload_file()` 校验，不通过则跳过

文件：[`platformtools/InfiniteRefill/客户端/热心群众/提交JSON.sh`](../../InfiniteRefill/客户端/热心群众/提交JSON.sh:1)

关键拒绝逻辑：
- 对每个文件先调用 [`json_check_payload_file()`](../../InfiniteRefill/客户端/_lib/json.sh:158)，失败则 `[SKIP]`。

摘录（原样）：

```bash
  local msg
  if ! msg="$(check_json "$f" 2>&1)"; then
    echo "[SKIP] $base：$msg"
    return 0
  fi
```

来源位置：[`提交JSON.sh`](../../InfiniteRefill/客户端/热心群众/提交JSON.sh:60)

---

## 2) 客户端库：敏感字段检查已禁用（`json_check_no_sensitive_keys()` 现状）

文件：[`platformtools/InfiniteRefill/客户端/_lib/json.sh`](../../InfiniteRefill/客户端/_lib/json.sh:1)

### 2.1 `json_check_no_sensitive_keys()`（现状：直接放行）

入口函数：[`json_check_no_sensitive_keys()`](../../InfiniteRefill/客户端/_lib/json.sh:85)

现状：
- 该函数已被改为直接 `return 0`，不再递归拒绝 `access_token/refresh_token/id_token`。
- `json_check_payload_file()` 中“先做敏感字段检查”的步骤已被移除/注释为禁用。

对应位置：[`json_check_payload_file()`](../../InfiniteRefill/客户端/_lib/json.sh:93)

### 2.2 `json_check_payload_file()` 的结构拒绝（register/report 两种模式）

入口函数：[`json_check_payload_file()`](../../InfiniteRefill/客户端/_lib/json.sh:158)

拒绝规则（语义层面）：
- `register`：必须存在 `accounts` 数组；每一项必须至少有：
  - `email_hash` 为 64 hex
  - `seen_at` 非空
- `report`：必须存在 `reports` 数组；每一项必须至少有：
  - `email_hash` 为 64 hex
  - `probed_at` 非空
  - `status_code` 存在且为 number/string

实现位置（python/osascript/jq 三套分支）：
- Python 分支：[`json_check_payload_file()`](../../InfiniteRefill/客户端/_lib/json.sh:170)
- JXA 分支：[`json_check_payload_file()`](../../InfiniteRefill/客户端/_lib/json.sh:228)
- jq 分支：[`json_check_payload_file()`](../../InfiniteRefill/客户端/_lib/json.sh:277)

---

## 3) 服务端：取字段/校验/下发字段逻辑（重点：`auth_json` 的存取）

服务端入口：[`platformtools/InfiniteRefill/server/src/index.ts`](../../InfiniteRefill/server/src/index.ts:1)

### 3.1 通用前置拒绝：必须是 JSON

函数：[`parseJson()`](../../InfiniteRefill/server/src/index.ts:606)

拒绝规则：`Content-Type` 不包含 `application/json` 则直接 `415 expected application/json`。

### 3.2 权限继承与拒绝：UPLOAD_KEY 可以当 USER_KEY 用

函数：[`requireAtLeastUser()`](../../InfiniteRefill/server/src/index.ts:573)

要点：
- 优先 admin；否则如果带了 `X-Upload-Key` 则走 upload（权限继承）；否则要求 `X-User-Key`。
- `requireUploadKey/requireUserKey` 会查 D1 的 enabled 状态，不存在/未启用会拒绝。

实现与拒绝点：
- [`requireUploadKey()`](../../InfiniteRefill/server/src/index.ts:588)
- [`requireUserKey()`](../../InfiniteRefill/server/src/index.ts:597)

### 3.3 `/v1/accounts/register`：服务端如何“取字段 + 存 auth_json（加密）”

路由入口：[`server/src/index.ts`](../../InfiniteRefill/server/src/index.ts:2311)

#### 3.3.1 从请求体取字段

对每个 `items[i]`，服务端取字段（原样）：
- `email_hash`：`String(it?.email_hash || "").trim().toLowerCase()`
- `account_id`：可选 `String(it.account_id).trim()`
- `seen_at`：`String(it?.seen_at || "").trim()`
- `auth_json`：`(it as any)?.auth_json`（可选）

来源：[`server/src/index.ts`](../../InfiniteRefill/server/src/index.ts:2323)

#### 3.3.2 拒绝/错误收集逻辑

拒绝点（加入 `errors[]` 并 `continue`）：
- `email_hash` 不是 64 hex：[`isHexSha256()`](../../InfiniteRefill/server/src/index.ts:748)
- `seen_at` 为空
- `auth_json` 体积超过 64KB
- `auth_json` 加密失败（比如 [`ACCOUNTS_MASTER_KEY_B64`](../../InfiniteRefill/server/src/index.ts:50) 未配置）

#### 3.3.3 `auth_json` 的存储方式

- `auth_json` 支持 string 或 object（object 会被 [`normalizeAuthJsonToString()`](../../InfiniteRefill/server/src/index.ts:495) `JSON.stringify`）。
- 之后使用 [`accountsAuthJsonEncrypt()`](../../InfiniteRefill/server/src/index.ts:501) 做 AES-GCM 加密并以 base64 写入 D1。
- 表字段：`accounts.has_auth_json` 与 `accounts.auth_json`。

代码摘录见：[`server/src/index.ts`](../../InfiniteRefill/server/src/index.ts:2339)

#### 3.3.4 返回体字段

返回：`{ ok:true, accepted, stored_auth_json, errors, received_at }`：见 [`server/src/index.ts`](../../InfiniteRefill/server/src/index.ts:2390)

### 3.4 `/v1/refill/topup`：服务端如何“取字段 + 下发 accounts[].{file_name,auth_json}”

路由入口：[`server/src/index.ts`](../../InfiniteRefill/server/src/index.ts:2099)

#### 3.4.1 从请求体取字段

- `target_pool_size`：`Number(body?.target_pool_size || 0)`，然后截断到 `[1,200]`：见 [`server/src/index.ts`](../../InfiniteRefill/server/src/index.ts:2103)
- `reports[]`：`Array.isArray(body?.reports) ? body.reports : []`，并限制最多 2000 条：见 [`server/src/index.ts`](../../InfiniteRefill/server/src/index.ts:2104)

对每个 `reports[i]` 取字段：
- `email_hash`：`String(it?.email_hash || "").trim().toLowerCase()`
- `account_id`：可选 `String(it.account_id).trim()`
- `probed_at`：`String(it?.probed_at || "").trim()`
- `file_name`：可选 `String(it.file_name).slice(0, 200)`
- `status_code`：如果是 number 则 `Math.trunc(it.status_code)`，否则为 null

来源：[`server/src/index.ts`](../../InfiniteRefill/server/src/index.ts:2122)

#### 3.4.2 拒绝/错误收集逻辑

逐条 report 的拒绝点（加入 `errors[]` 并 `continue`）：
- `email_hash` 非 64 hex
- `probed_at` 为空

来源：[`server/src/index.ts`](../../InfiniteRefill/server/src/index.ts:2130)

#### 3.4.3 下发字段（服务端“取出 auth_json”并返回）

下发阶段：
1) 从 D1 选取 `invalid=0 AND has_auth_json=1 AND auth_json IS NOT NULL` 的账号：见 [`server/src/index.ts`](../../InfiniteRefill/server/src/index.ts:2238)
2) 对每条记录：
   - 解密：[`accountsAuthJsonDecrypt()`](../../InfiniteRefill/server/src/index.ts:508)
   - `JSON.parse(plain)` 得到对象 `auth`
   - 输出数组项：`{ file_name: <生成>, auth_json: auth }`

来源：[`server/src/index.ts`](../../InfiniteRefill/server/src/index.ts:2253)

返回体：
- `accounts: Array<{file_name, auth_json}>`
- `issue_errors`：解密/解析失败会进这里

来源：[`server/src/index.ts`](../../InfiniteRefill/server/src/index.ts:2270)

---

## 4) 结论（对接 `codex_register/wait_update` 时你要注意的“硬拒绝点”）

- **客户端通用提交脚本**（热心群众提交 JSON）目前已禁用“敏感字段检测”，不会再拒绝包含 token 的 JSON：
  - Windows：[`提交JSON.bat`](../../InfiniteRefill/客户端/热心群众/提交JSON.bat:69)
  - macOS/Linux：[`json_check_no_sensitive_keys()`](../../InfiniteRefill/客户端/_lib/json.sh:85)
- **服务端** `/v1/accounts/register` 允许上传 `auth_json`（并加密存储）：见 [`/v1/accounts/register`](../../InfiniteRefill/server/src/index.ts:2311)
- **服务端下发** `auth_json` 的字段为：`accounts[].auth_json`（对象）与 `accounts[].file_name`：见 [`/v1/refill/topup`](../../InfiniteRefill/server/src/index.ts:2099)
