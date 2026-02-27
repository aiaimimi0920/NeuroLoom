# MailCreate（临时邮箱能力）客户端

该目录提供“统一的邮箱 API 客户端”，供后续多个自动化注册器复用。

- 服务端（Cloudflare Worker + D1 + Email Routing）源码将放在 [`platformtools/mailcreate/server`](platformtools/mailcreate/server:1)
- 本目录只放“调用服务端”的客户端封装，不绑定任何特定平台注册逻辑。

## 已部署的服务（当前临时环境）
### A) 我们自建服务（MailCreate / Cloudflare Worker）
- Base URL（自定义域名）：`https://mail.aiaimimi.com`
- 站点/API 访问鉴权：请求头 `x-custom-auth`

### B) GPTMail 公共服务（备用 Provider）
- Base URL：`https://mail.chatgpt.org.uk`
- 鉴权：请求头 `X-API-Key`
- 文档：`https://www.chatgpt.org.uk/2025/11/gptmailapiapi.html`

> 注意：公共测试 Key `gpt-test` 可能触发配额（我们本地实测已返回 `Daily quota exceeded`）。
> 建议使用多 Key（配置文件轮换）或自备 Key。

> 默认收件域建议使用 `aiaimimi.com`：因为 Email Routing 的 Catch-all 是 zone 级别规则。
> 你目前在 Cloudflare 配置的是 Catch-All for `aiaimimi.com` → Worker `cloudflare_temp_email`，因此 `*@aiaimimi.com` 会被投递到 Worker。
> 若改用 `*@mx.aiaimimi.com`，需要额外在 Email Routing → Settings 添加子域并配置对应规则（且 Catch-all 通常无法对“单独子域”生效）。

服务端入口与鉴权逻辑：[`worker.ts`](../../_cf_temp_email/cloudflare_temp_email/worker/src/worker.ts:38)

## Python 调用示例
文件：[`mailcreate_client.py`](mailcreate_client.py:1)

```python
from platformtools.mailcreate.client.mailcreate_client import (
    MailCreateClient,
    MailCreateConfig,
    wait_for_6digit_code,
)

cfg = MailCreateConfig(
    base_url="https://mail.aiaimimi.com",
    custom_auth="<你的 x-custom-auth 值>",
)
client = MailCreateClient(cfg)

print(client.health_check())
settings = client.open_settings()
print(settings["domains"])

addr = client.new_address(domain="aiaimimi.com")
# addr = { "address": "tmpxxxx@aiaimimi.com", "jwt": "...", "password"?: "..." }
print(addr)

code = wait_for_6digit_code(client, jwt=addr["jwt"], from_contains="openai", timeout_seconds=180)
print("code=", code)
```

## 服务端最小 API（供其他语言实现）
> 下面路径都以 `https://mail.aiaimimi.com` 为 base。

- `GET /health_check`
  - Header：`x-custom-auth`
- `GET /open_api/settings`
  - Header：`x-custom-auth`
- `POST /api/new_address`
  - Header：`x-custom-auth`
  - JSON：`{"name": "optional", "domain": "aiaimimi.com"}`
  - 返回：`{ address, jwt, password? }`
- `GET /api/mails?limit=20&offset=0`
  - Header：`x-custom-auth` + `Authorization: Bearer <jwt>`
- `GET /api/mail/:mail_id`
  - Header：`x-custom-auth` + `Authorization: Bearer <jwt>`

## Email Routing 配置（由控制台完成）
目标：让 `*@aiaimimi.com` 的入站邮件触发 Worker 的 email handler：[`email()`](../../_cf_temp_email/cloudflare_temp_email/worker/src/email/index.ts:16)

建议：Email → Email Routing → Routes → Catch-all → Action=Send to a Worker → 选择 Worker。
